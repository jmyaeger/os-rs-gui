//! Monte Carlo single-way simulation of a setup, special attacks included.
//!
//! Runs inside the shared worker (see `crate::worker`); off the web target it
//! runs inline, which is how the tests exercise it.

use super::metrics::{calculate_against, check_magic_is_supported};
use super::spec::{LoadoutSpec, TRACKED_PRAYERS};
use super::strategy::{DeathCharge, RestorePolicy, SpecPlan, SpecStep, spec_cost, spec_weapons};
use super::target::TargetConfig;
use crate::components::preferred_style;
use osrs::calc::rolls::calc_active_player_rolls;
use osrs::combat::attacks::specs::get_spec_attack_function;
use osrs::combat::attacks::standard::get_attack_functions;
use osrs::combat::simulation::Simulation;
use osrs::combat::spec::{self, CoreCondition, SpecConfig, SpecStrategy};
use osrs::combat::thralls::Thrall;
use osrs::constants::SECONDS_PER_TICK;
use osrs::error::SimulationError;
use osrs::sims::single_way::{SingleWayConfig, SingleWayFight};
use osrs::types::equipment::{CombatStance, Weapon};
use osrs::types::player::{GearSwitch, Player, SwitchType};
use osrs::types::stats::SpecEnergy;
use osrs::utils::logging::FightRecorder;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::LazyLock;

/// Spec weapons whose special attack the engine implements. For the others the
/// engine silently falls back to a normal attack, which would mislead.
static IMPLEMENTED_SPECS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    spec_weapons()
        .iter()
        .filter_map(|item| {
            let mut player = Player::default();
            let weapon = Weapon::new(&item.name, item.version.as_deref()).ok()?;
            player.equip_item(Box::new(weapon)).ok()?;
            let spec = get_spec_attack_function(&player);
            let attack = get_attack_functions(&player);
            (!std::ptr::fn_addr_eq(spec, attack)).then(|| item.name.clone())
        })
        .collect()
});

pub fn spec_implemented(weapon_name: &str) -> bool {
    IMPLEMENTED_SPECS.contains(weapon_name)
}

pub const TRIAL_CHOICES: [u32; 4] = [1_000, 10_000, 50_000, 100_000];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThrallChoice {
    LesserMelee,
    LesserRanged,
    LesserMagic,
    SuperiorMelee,
    SuperiorRanged,
    SuperiorMagic,
    GreaterMelee,
    GreaterRanged,
    GreaterMagic,
}

impl ThrallChoice {
    pub const ALL: [ThrallChoice; 9] = [
        Self::LesserMelee,
        Self::LesserRanged,
        Self::LesserMagic,
        Self::SuperiorMelee,
        Self::SuperiorRanged,
        Self::SuperiorMagic,
        Self::GreaterMelee,
        Self::GreaterRanged,
        Self::GreaterMagic,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::LesserMelee => "lesser-melee",
            Self::LesserRanged => "lesser-ranged",
            Self::LesserMagic => "lesser-magic",
            Self::SuperiorMelee => "superior-melee",
            Self::SuperiorRanged => "superior-ranged",
            Self::SuperiorMagic => "superior-magic",
            Self::GreaterMelee => "greater-melee",
            Self::GreaterRanged => "greater-ranged",
            Self::GreaterMagic => "greater-magic",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::LesserMelee => "Lesser melee",
            Self::LesserRanged => "Lesser ranged",
            Self::LesserMagic => "Lesser magic",
            Self::SuperiorMelee => "Superior melee",
            Self::SuperiorRanged => "Superior ranged",
            Self::SuperiorMagic => "Superior magic",
            Self::GreaterMelee => "Greater melee",
            Self::GreaterRanged => "Greater ranged",
            Self::GreaterMagic => "Greater magic",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|choice| choice.key() == key)
    }

    pub(super) fn engine(self) -> Thrall {
        match self {
            Self::LesserMelee => Thrall::LesserMelee,
            Self::LesserRanged => Thrall::LesserRanged,
            Self::LesserMagic => Thrall::LesserMagic,
            Self::SuperiorMelee => Thrall::SuperiorMelee,
            Self::SuperiorRanged => Thrall::SuperiorRanged,
            Self::SuperiorMagic => Thrall::SuperiorMagic,
            Self::GreaterMelee => Thrall::GreaterMelee,
            Self::GreaterRanged => Thrall::GreaterRanged,
            Self::GreaterMagic => Thrall::GreaterMagic,
        }
    }
}

/// Settings that only the simulation uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SimOptions {
    pub trials: u32,
    pub thrall: Option<ThrallChoice>,
}

impl Default for SimOptions {
    fn default() -> Self {
        Self {
            trials: 10_000,
            thrall: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SingleWayInput {
    pub loadout: LoadoutSpec,
    pub target: TargetConfig,
    pub plan: SpecPlan,
    pub options: SimOptions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SingleWayOutput {
    pub trials: u32,
    /// Probability of the kill landing on each tick; index is the tick.
    pub ticks: Vec<f64>,
    /// Mean TTK in seconds.
    pub mean: f64,
    /// Median TTK in seconds
    pub median: f64,
    /// Fraction of attacks, special attacks included, that hit.
    pub accuracy: f64,
    pub attacks_per_kill: f64,
}

fn death_charge(choice: DeathCharge) -> Option<spec::DeathCharge> {
    match choice {
        DeathCharge::None => None,
        DeathCharge::Single => Some(spec::DeathCharge::Single),
        DeathCharge::Double => Some(spec::DeathCharge::Double),
    }
}

/// The main loadout with one step's spec weapon, overrides and prayer applied.
pub(super) fn spec_player(main: &Player, step: &SpecStep) -> Result<Player, String> {
    let mut player = main.clone();
    player.switches.clear();
    player.current_switch = None;
    for item in &step.switches {
        item.equip_onto(&mut player)?;
    }
    step.weapon.equip_onto(&mut player)?;
    if !step.prayers.is_empty() {
        for tracked in TRACKED_PRAYERS {
            player.remove_prayer(tracked);
        }
        for prayer in &step.prayers {
            player.add_prayer(*prayer);
        }
    }
    let style = step
        .style
        .filter(|style| player.gear.weapon.combat_styles.contains_key(style))
        .or_else(|| preferred_style(&player.gear.weapon))
        .ok_or_else(|| format!("{} has no attack styles", step.weapon.name))?;
    let casting = player
        .gear
        .weapon
        .combat_styles
        .get(&style)
        .is_some_and(|option| {
            matches!(
                option.stance,
                CombatStance::Autocast | CombatStance::DefensiveAutocast | CombatStance::ManualCast
            )
        });
    if !casting {
        player.attrs.spell = None;
    }
    player.set_active_style(style);
    player.update_bonuses();
    player.update_set_effects();
    Ok(player)
}

fn build_fight(input: &SingleWayInput) -> Result<SingleWayFight, String> {
    let restored = input.loadout.to_player();
    if !restored.warnings.is_empty() {
        return Err(format!("Unknown items: {}", restored.warnings.join(", ")));
    }
    let mut player = restored.player;
    let monster = input
        .target
        .simulation_monster()
        .ok_or("The target is invalid or its starting HP exceeds its maximum")?;
    // Reuse the calculator's validation so the simulation fails for the same reasons.
    // Validation only: a thrall never makes a setup invalid, so it is left out.
    calculate_against(&player, &monster, None)?;
    calc_active_player_rolls(&mut player, &monster);
    player.switches.clear();
    player.current_switch = None;

    let mut strategies = Vec::with_capacity(input.plan.steps.len());
    for (index, step) in input.plan.steps.iter().enumerate() {
        if spec_cost(&step.weapon.name).is_none() {
            return Err(format!(
                "{} has no special attack in the engine",
                step.weapon.name
            ));
        }
        if !spec_implemented(&step.weapon.name) {
            return Err(format!(
                "The engine does not simulate {}'s special attack yet",
                step.weapon.name
            ));
        }
        let switch_player = spec_player(&player, step)?;
        // GearSwitch::new calculates rolls, which panics for a magic weapon the
        // engine has no max hit for.
        check_magic_is_supported(&switch_player)?;
        let label: Rc<str> = Rc::from(format!("{} #{}", step.weapon.name, index + 1));
        let switch = GearSwitch::new(SwitchType::Spec(label), &switch_player, &monster);
        let conditions: Vec<CoreCondition> = step
            .conditions
            .iter()
            .map(|condition| condition.to_core())
            .collect();
        let mut strategy = SpecStrategy::new(&switch, Some(conditions));
        strategy.max_attempts = step.max_attempts;
        strategy.min_successes = step.min_successes;
        player.switches.push(switch);
        strategies.push(strategy);
    }
    let spec_config = (!strategies.is_empty()).then(|| {
        SpecConfig::new(
            strategies,
            input.plan.restore.into(),
            death_charge(input.plan.death_charge),
            input.plan.surge_potion,
        )
    });
    if let Some(config) = &spec_config {
        config.validate()?;
    }
    let config = SingleWayConfig {
        thralls: input.options.thrall.map(ThrallChoice::engine),
        remove_final_attack_delay: true,
        reset_soulreaper_stacks: Some(input.loadout.conditions.soulreaper_stacks),
    };
    SingleWayFight::new(player, monster, config, spec_config).map_err(|error| error.to_string())
}

/// Run the fight `options.trials` times. `progress` receives values in `0.0..=1.0`.
pub fn run_single_way(
    input: &SingleWayInput,
    progress: &mut dyn FnMut(f64),
) -> Result<SingleWayOutput, String> {
    let trials = input.options.trials.clamp(1, 1_000_000);
    let mut fight = build_fight(input)?;
    if fight.is_immune() {
        return Err(format!(
            "{} is immune to this attack style",
            fight.monster.info.name
        ));
    }
    fight.set_attack_function();
    calc_active_player_rolls(&mut fight.player, &fight.monster);
    let starting_energy = SpecEnergy::new(input.plan.starting_energy);
    fight.player.stats.spec = starting_energy;
    input.target.apply_starting_state(&mut fight.monster);

    let mut ttks: Vec<usize> = Vec::with_capacity(trials as usize);
    let mut attempts = 0_u64;
    let mut hits = 0_u64;
    let report_every = (trials / 50).max(1);
    for trial in 0..trials {
        match fight.simulate(&mut FightRecorder::Disabled) {
            Ok(result) => {
                ttks.push(result.ttk_ticks.max(0) as usize);
                attempts += u64::from(result.hit_attempts);
                hits += u64::from(result.hit_count);
            }
            Err(SimulationError::PlayerDeathError(_)) => {}
            Err(error) => return Err(error.to_string()),
        }
        fight.reset();
        if input.plan.restore == RestorePolicy::EveryKill {
            fight.player.stats.spec = starting_energy;
        }
        input.target.apply_starting_state(&mut fight.monster);
        if trial % report_every == 0 {
            progress(f64::from(trial) / f64::from(trials));
        }
    }
    progress(1.0);

    if ttks.is_empty() {
        return Err("No fight finished with a kill".into());
    }
    let count = ttks.len() as f64;
    let last = ttks.iter().copied().max().unwrap_or(0);
    let mut ticks = vec![0.0; last + 1];
    for tick in &ttks {
        ticks[*tick] += 1.0 / count;
    }
    ttks.sort_unstable();
    let quantile = |q: f64| ttks[((count * q) as usize).min(ttks.len() - 1)] as f64;
    Ok(SingleWayOutput {
        trials: ttks.len() as u32,
        mean: ttks.iter().sum::<usize>() as f64 / count * SECONDS_PER_TICK,
        median: quantile(0.5) * SECONDS_PER_TICK,
        accuracy: if attempts > 0 {
            hits as f64 / attempts as f64
        } else {
            0.0
        },
        attacks_per_kill: attempts as f64 / count,
        ticks,
    })
}

#[cfg(test)]
mod tests {
    use super::super::examples::examples;
    use super::*;

    fn quiet() -> impl FnMut(f64) {
        |_| {}
    }

    #[test]
    fn opening_bgs_spec_shortens_the_fight() {
        let example = examples().into_iter().next().unwrap();
        assert!(!example.plan.steps.is_empty());
        let with_spec = SingleWayInput {
            loadout: example.loadout.clone(),
            target: example.target.clone(),
            plan: example.plan.clone(),
            options: SimOptions {
                trials: 400,
                thrall: None,
            },
        };
        let without_spec = SingleWayInput {
            plan: SpecPlan::default(),
            ..with_spec.clone()
        };
        let specced = run_single_way(&with_spec, &mut quiet()).unwrap();
        let plain = run_single_way(&without_spec, &mut quiet()).unwrap();
        assert!((specced.ticks.iter().sum::<f64>() - 1.0).abs() < 1e-6);
        assert!(
            specced.mean < plain.mean,
            "spec {} vs plain {}",
            specced.mean,
            plain.mean
        );

        let monster = example.target.combat_monster().unwrap();
        let expected = calculate_against(&example.loadout.to_player().player, &monster, None)
            .unwrap()
            .expected_ttk
            .unwrap();
        assert!(
            (plain.mean - expected).abs() / expected < 0.15,
            "simulated {} vs analytic {expected}",
            plain.mean
        );
    }

    #[test]
    fn starting_state_persists_across_trials() {
        let example = examples().into_iter().nth(1).unwrap();
        let reduced = SingleWayInput {
            loadout: example.loadout.clone(),
            target: TargetConfig::example("General Graardor", 0, 0, 40),
            plan: SpecPlan::default(),
            options: SimOptions {
                trials: 300,
                thrall: None,
            },
        };
        let full = SingleWayInput {
            target: TargetConfig::example("General Graardor", 0, 0, 0),
            ..reduced.clone()
        };
        let mut reports = 0;
        let a = run_single_way(&reduced, &mut |_| reports += 1).unwrap();
        let b = run_single_way(&full, &mut quiet()).unwrap();
        assert!(reports > 10);
        assert!(a.mean < b.mean, "reduced {} vs full {}", a.mean, b.mean);
    }

    #[test]
    fn spec_step_uses_the_chosen_attack_style() {
        use super::super::strategy::weapon_styles;
        use osrs::types::equipment::CombatStyle;

        let bgs = super::super::strategy::simulated_spec_weapons()
            .iter()
            .find(|weapon| weapon.name == "Bandos godsword")
            .map(super::super::spec::GearItem::from_catalog)
            .expect("Bandos godsword in the catalog");
        let mut step = SpecStep::new(1, bgs);

        let default_style = step.resolved_style().expect("a default style");
        let other = weapon_styles(&step.weapon)
            .into_iter()
            .map(|(style, _, _)| style)
            .find(|style| *style != default_style)
            .expect("a second style");

        let main = LoadoutSpec::default().to_player().player;
        assert_eq!(
            spec_player(&main, &step).unwrap().attrs.active_style,
            default_style
        );

        step.style = Some(other);
        assert_eq!(step.resolved_style(), Some(other));
        let switched = spec_player(&main, &step).unwrap();
        assert_eq!(switched.attrs.active_style, other);

        // A style the weapon does not offer falls back rather than panicking.
        step.style = Some(CombatStyle::Rapid);
        assert_eq!(step.resolved_style(), Some(default_style));
        assert_eq!(
            spec_player(&main, &step).unwrap().attrs.active_style,
            default_style
        );
    }

    #[test]
    fn spec_defence_type_matches_the_engines_tables() {
        use super::super::strategy::spec_defence_type;
        use osrs::types::equipment::CombatType;

        // Listed weapons ignore the selected style's type.
        assert_eq!(
            spec_defence_type("Bandos godsword", CombatType::Crush),
            (CombatType::Slash, true)
        );
        assert_eq!(
            spec_defence_type("Arclight", CombatType::Slash),
            (CombatType::Stab, true)
        );
        assert_eq!(
            spec_defence_type("Voidwaker", CombatType::Slash),
            (CombatType::Magic, true)
        );
        // Unlisted weapons follow the style, which is how the engine behaves.
        assert_eq!(
            spec_defence_type("Soulreaper axe", CombatType::Slash),
            (CombatType::Slash, false)
        );
        assert_eq!(
            spec_defence_type("Burning claws", CombatType::Stab),
            (CombatType::Stab, false)
        );
    }

    #[test]
    fn spec_style_changes_the_attack_roll_but_not_a_fixed_defence() {
        use super::super::strategy::weapon_styles;

        let bgs = super::super::strategy::simulated_spec_weapons()
            .iter()
            .find(|weapon| weapon.name == "Bandos godsword")
            .map(super::super::spec::GearItem::from_catalog)
            .expect("Bandos godsword in the catalog");
        let mut step = SpecStep::new(1, bgs);

        // A godsword offers both slash and crush styles.
        let styles = weapon_styles(&step.weapon);
        let slash = styles
            .iter()
            .find(|(_, combat_type, _)| *combat_type == osrs::types::equipment::CombatType::Slash)
            .map(|(style, _, _)| *style)
            .expect("a slash style");
        let crush = styles
            .iter()
            .find(|(_, combat_type, _)| *combat_type == osrs::types::equipment::CombatType::Crush)
            .map(|(style, _, _)| *style)
            .expect("a crush style");

        let main = LoadoutSpec::default().to_player().player;
        step.style = Some(slash);
        let on_slash = spec_player(&main, &step).unwrap();
        step.style = Some(crush);
        let on_crush = spec_player(&main, &step).unwrap();
        assert_ne!(on_slash.combat_type(), on_crush.combat_type());

        // ...but the defence rolled against stays slash either way.
        step.style = Some(crush);
        assert_eq!(
            step.defence_type(),
            Some((osrs::types::equipment::CombatType::Slash, true))
        );
    }

    /// The GUI mirrors the engine's private spec attack-roll table so the roll can
    /// be shown. If the engine's factors change, the accuracy implied by our roll
    /// stops matching the engine's own, and this fails.
    #[test]
    fn spec_attack_roll_matches_engine_accuracy() {
        use super::super::metrics::spec_metrics;
        use super::super::strategy::spec_defence_type;

        let target = TargetConfig::example("General Graardor", 0, 0, 0);
        let monster = target.combat_monster().unwrap();
        let main = examples()
            .into_iter()
            .next()
            .unwrap()
            .loadout
            .to_player()
            .player;

        for name in ["Bandos godsword", "Dragon warhammer", "Dragon dagger"] {
            let weapon = super::super::strategy::simulated_spec_weapons()
                .iter()
                .find(|weapon| weapon.name == name)
                .map(super::super::spec::GearItem::from_catalog)
                .unwrap_or_else(|| panic!("{name} in the catalog"));
            let step = SpecStep::new(1, weapon);
            let player = spec_player(&main, &step).unwrap();
            let (defence, _) =
                spec_defence_type(name, step.style_combat_type().expect("a style type"));
            let metrics = spec_metrics(&player, &monster, defence).unwrap();

            // The engine's standard accuracy formula, applied to the rolls we show.
            let (attack, defence_roll) = (
                f64::from(metrics.attack_roll),
                f64::from(metrics.defence_roll),
            );
            let implied = if attack > defence_roll {
                1.0 - (defence_roll + 2.0) / (2.0 * (attack + 1.0))
            } else {
                attack / (2.0 * (defence_roll + 1.0))
            };
            assert!(
                (implied - metrics.accuracy).abs() < 1e-6,
                "{name}: shown rolls imply {implied} but the engine reports {}",
                metrics.accuracy
            );
        }
    }

    /// Every weapon the picker offers must either produce metrics or explain
    /// itself with the friendly message — never a raw engine error, and never a
    /// silently empty row. Which weapons have a closed form is the engine's
    /// business and changes over time, so this asserts the contract, not the
    /// coverage.
    #[test]
    fn every_offered_spec_weapon_reports_metrics_or_a_reason() {
        use super::super::metrics::spec_metrics;
        use super::super::strategy::{simulated_spec_weapons, spec_defence_type};

        let target = TargetConfig::example("General Graardor", 0, 0, 0);
        let monster = target.combat_monster().unwrap();
        let main = examples()
            .into_iter()
            .next()
            .unwrap()
            .loadout
            .to_player()
            .player;

        for entry in simulated_spec_weapons() {
            let step = SpecStep::new(1, super::super::spec::GearItem::from_catalog(entry));
            let style_type = step
                .style_combat_type()
                .unwrap_or_else(|| panic!("{} has no attack styles", entry.name));
            let (defence, _) = spec_defence_type(&entry.name, style_type);
            let player =
                spec_player(&main, &step).unwrap_or_else(|error| panic!("{}: {error}", entry.name));
            if let Err(reason) = spec_metrics(&player, &monster, defence) {
                // A refusal is fine as long as it explains itself; the catch-all
                // arm means an engine error kind we do not handle yet.
                assert!(
                    !reason.starts_with("The engine could not build"),
                    "{}: unhandled engine error: {reason}",
                    entry.name
                );
            }
        }
    }

    /// A special attack the calculator cannot solve must still be usable, because
    /// the simulation models it through `specs.rs` rather than `dps_calc`.
    #[test]
    fn a_spec_weapon_still_simulates() {
        let claws = super::super::strategy::simulated_spec_weapons()
            .iter()
            .find(|weapon| weapon.name == "Burning claws")
            .map(super::super::spec::GearItem::from_catalog)
            .expect("Burning claws is offered as a spec weapon");
        let step = SpecStep::new(1, claws);
        let example = examples().into_iter().next().unwrap();

        let mut plan = SpecPlan::default();
        plan.steps.push(step);
        let input = SingleWayInput {
            loadout: example.loadout.clone(),
            target: example.target.clone(),
            plan,
            options: SimOptions {
                trials: 200,
                thrall: None,
            },
        };
        let simulated = run_single_way(&input, &mut |_| {}).expect("the fight simulates");
        assert!(simulated.mean > 0.0);
        assert!((simulated.ticks.iter().sum::<f64>() - 1.0).abs() < 1e-6);
    }

    /// Specs whose distribution is several hitsplats must not collapse into a
    /// plain attack. The engine once reset the distribution for every special
    /// attack, which made claws and halberds silently report auto-attack numbers
    /// while every other weapon looked fine.
    #[test]
    fn multi_hitsplat_specs_beat_a_normal_attack() {
        use super::super::metrics::{calculate_against, spec_metrics};
        use super::super::strategy::{simulated_spec_weapons, spec_defence_type};

        let target = TargetConfig::example("General Graardor", 0, 0, 0);
        let monster = target.combat_monster().unwrap();
        let main = examples()
            .into_iter()
            .next()
            .unwrap()
            .loadout
            .to_player()
            .player;

        for name in ["Dragon claws", "Burning claws", "Dragon halberd"] {
            let entry = simulated_spec_weapons()
                .iter()
                .find(|weapon| weapon.name == name)
                .unwrap_or_else(|| panic!("{name} is offered as a spec weapon"));
            let step = SpecStep::new(1, super::super::spec::GearItem::from_catalog(entry));
            let player = spec_player(&main, &step).unwrap();
            let (defence, _) = spec_defence_type(name, step.style_combat_type().unwrap());

            let spec = spec_metrics(&player, &monster, defence).unwrap();
            let normal = calculate_against(&player, &monster, None).unwrap();
            assert!(
                spec.expected_hit > normal.expected_hit * 1.5,
                "{name}: spec expects {:.2} but a normal hit expects {:.2}",
                spec.expected_hit,
                normal.expected_hit
            );
            assert!(
                spec.max_hit > normal.max_hit,
                "{name}: spec max {} is not above the normal max {}",
                spec.max_hit,
                normal.max_hit
            );
        }
    }

    /// A thrall adds its own damage on its own timer, so it raises DPS without
    /// touching accuracy, max hit or the rolls. Values come from the engine, so
    /// this also catches the engine changing a thrall's damage or cadence.
    #[test]
    fn thralls_add_damage_per_second_without_changing_the_weapon() {
        use super::super::metrics::calculate_against;
        use osrs::constants::{SECONDS_PER_TICK, THRALL_ATTACK_SPEED};

        let example = examples().into_iter().next().unwrap();
        let player = example.loadout.to_player().player;
        let monster = example.target.combat_monster().unwrap();

        let plain = calculate_against(&player, &monster, None).unwrap();
        let interval = f64::from(THRALL_ATTACK_SPEED) * SECONDS_PER_TICK;

        for choice in [
            ThrallChoice::LesserMelee,
            ThrallChoice::SuperiorRanged,
            ThrallChoice::GreaterMagic,
        ] {
            let thrall = choice.engine();
            let with = calculate_against(&player, &monster, Some(thrall)).unwrap();
            let expected = f64::from(thrall.max_hit()) / 2.0 / interval;
            assert!(
                (with.dps - plain.dps - expected).abs() < 1e-9,
                "{}: dps {} should be {} above {}",
                choice.label(),
                with.dps,
                expected,
                plain.dps
            );
            // The thrall fights on its own; it must not touch the weapon's numbers.
            // Summation order inside the engine varies, so compare damage loosely.
            assert_eq!(with.max_hit, plain.max_hit);
            assert_eq!(with.attack_roll, plain.attack_roll);
            assert!((with.accuracy - plain.accuracy).abs() < 1e-9);
            assert!((with.expected_hit - plain.expected_hit).abs() < 1e-6);
        }

        // A greater thrall must out-damage a lesser one.
        let lesser =
            calculate_against(&player, &monster, Some(ThrallChoice::LesserMelee.engine())).unwrap();
        let greater =
            calculate_against(&player, &monster, Some(ThrallChoice::GreaterMelee.engine()))
                .unwrap();
        assert!(greater.dps > lesser.dps);
    }

    #[test]
    fn implemented_specs_are_detected() {
        assert!(spec_implemented("Bandos godsword"));
        assert!(spec_implemented("Dragon warhammer"));
        assert!(!spec_implemented("Not a weapon"));
    }

    #[test]
    fn thralls_help() {
        let example = examples().into_iter().nth(2).unwrap();
        let base = SingleWayInput {
            loadout: example.loadout.clone(),
            target: example.target.clone(),
            plan: SpecPlan::default(),
            options: SimOptions {
                trials: 200,
                thrall: None,
            },
        };
        let with_thrall = SingleWayInput {
            options: SimOptions {
                trials: 200,
                thrall: Some(ThrallChoice::GreaterMagic),
            },
            ..base.clone()
        };
        let a = run_single_way(&base, &mut quiet()).unwrap();
        let b = run_single_way(&with_thrall, &mut quiet()).unwrap();
        assert!(b.mean < a.mean, "thrall {} vs none {}", b.mean, a.mean);
    }
}
