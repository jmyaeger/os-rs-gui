use dioxus::prelude::*;
use osrs::calc::dps_calc::{
    get_distribution, get_dps, get_expected_damage, get_hit_chance, get_ttk, get_ttk_distribution,
    spec_att_roll_factor,
};
use osrs::calc::hit_dist::AttackDistribution;
use osrs::calc::rolls::calc_active_player_rolls;
use osrs::combat::thralls::Thrall;
use osrs::constants::{SECONDS_PER_TICK, THRALL_ATTACK_SPEED, USES_OWN_AMMO};
use osrs::error::DpsCalcError;
use osrs::types::equipment::{CombatStance, CombatType};
use osrs::types::monster::Monster;
use osrs::types::player::Player;
use osrs::types::spells::{Spell, StandardSpell};
use serde::{Deserialize, Serialize};

use super::simulation::ThrallChoice;
use super::state::HomeState;
#[cfg(test)]
use super::target::TargetConfig;

/// Calculated metrics for the main weapon, including thralls and burn but not specs.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CombatMetrics {
    pub dps: f64,
    pub accuracy: f64,
    pub max_hit: u32,
    pub attack_roll: i32,
    pub defence_roll: i32,
    #[serde(default)]
    pub expected_hit: f64,
    #[serde(default)]
    pub expected_ttk: Option<f64>,
}

/// Calculated metrics for a special attack. These act like the special attack is used
/// repeatedly with no consideration for special attack energy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpecMetrics {
    pub dps: f64,
    pub accuracy: f64,
    pub max_hit: u32,
    pub expected_hit: f64,
    pub attack_roll: i32,
    pub defence_roll: i32,
    pub always_hits: bool,
}

/// Get the special attack metrics of `player`'s special attack against `monster`.
/// `defence_type` is the combat type the spec rolls against, since most specs roll against
/// a fixed style (exceptions being burning claws and the soulreaper axe).
pub fn get_spec_metrics(
    player: &Player,
    monster: &Monster,
    defence_type: CombatType,
) -> Result<SpecMetrics, String> {
    let mut player = player.clone();
    player.update_bonuses();
    player.update_set_effects();
    calc_active_player_rolls(&mut player, monster).map_err(|e| e.to_string())?;
    let distribution = get_distribution(&player, monster, true).map_err(|error| match error {
        DpsCalcError::SpecNotImplemented(_) => {
            "No formula for this special attack yet — the simulation still models it.".to_string()
        }
        other => format!("The engine could not build this special attack: {other:?}"),
    })?;
    let accuracy = get_hit_chance(&player, monster, true)
        .map_err(|error| format!("Error calculating the hit chance: {error:?}."))?;
    let combat_type = player.combat_type();
    let base_roll = player
        .att_rolls
        .get(combat_type)
        .map_err(|error| format!("{error:?}"))?;
    let factor = spec_att_roll_factor(&player);
    let mut attack_roll = factor.multiply_to_int(base_roll);
    if player.is_wearing("Keris partisan of the sun", None)
        && monster.is_toa_monster()
        && monster.stats.hitpoints.current < monster.stats.hitpoints.base / 4
    {
        attack_roll = attack_roll * 5 / 4;
    }
    let expected_hit = get_expected_damage(&distribution, &player, monster, true)
        .map_err(|error| format!("Error calculating expected hit: {error:?}"))?;
    let dps = get_dps(&distribution, &player, monster, true)
        .map_err(|error| format!("Error calculating DPS: {error:?}"))?;

    Ok(SpecMetrics {
        always_hits: player.is_wearing_any(osrs::constants::ALWAYS_HITS_SPEC),
        dps,
        accuracy: accuracy.clamp(0.0, 1.0),
        max_hit: distribution.get_max(),
        expected_hit,
        attack_roll,
        defence_roll: monster.def_rolls.get(defence_type),
    })
}

/// Time-to-kill distribution for one result, in ticks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TtkSummary {
    pub mean: f64,
    pub p90: f64,
    /// Probability of the kill landing on each tick; index is the tick.
    pub ticks: Vec<f64>,
}

pub(super) struct Prepared {
    pub(super) player: Player,
    distribution: AttackDistribution,
}

/// Validate the loadout and build its hit distribution against the target's starting state.
pub(super) fn prepare(player: &Player, monster: &Monster) -> Result<Prepared, String> {
    let style = player
        .gear
        .weapon
        .combat_styles
        .get(&player.attrs.active_style)
        .ok_or("Choose an attack style")?;
    if player.gear.weapon.base_speed <= 0 {
        return Err("This weapon has no attack speed".into());
    }
    if player.stats.hitpoints.base == 0 || player.stats.hitpoints.current == 0 {
        return Err("Hitpoints must be above zero".into());
    }
    if !matches!(
        style.combat_type,
        CombatType::Stab
            | CombatType::Slash
            | CombatType::Crush
            | CombatType::Light
            | CombatType::Standard
            | CombatType::Heavy
            | CombatType::Magic
    ) {
        return Err("Choose an attack style".into());
    }

    let casting = matches!(
        style.stance,
        CombatStance::Autocast | CombatStance::DefensiveAutocast | CombatStance::ManualCast
    );
    if casting {
        let spell = player.attrs.spell.ok_or("Select a spell to cast")?;
        if matches!(spell, Spell::Standard(StandardSpell::None)) {
            return Err("Select a spell to cast".into());
        }
        if player.stats.magic.current < spell.required_level() {
            return Err(format!("{spell} needs {} Magic", spell.required_level()));
        }
    }

    let mut player = player.clone();
    if !casting {
        // A previously selected spell must not affect another weapon's opening damage.
        player.attrs.spell = None;
    }
    player.set_active_style(player.attrs.active_style);
    if player.gear.weapon.speed <= 0 {
        return Err("This weapon has no attack speed".into());
    }
    if !has_required_ammunition(&player) {
        return Err("This weapon needs matching ammunition".into());
    }
    // TODO: replace this with validation on the engine side
    // if (player.is_using_magic() || player.is_using_ranged())
    //     && player
    //         .gear
    //         .weapon
    //         .version
    //         .as_deref()
    //         .is_some_and(|version| {
    //             let version = version.to_lowercase();
    //             ["uncharged", "inactive", "empty", "broken"]
    //                 .iter()
    //                 .any(|state| version.contains(state))
    //         })
    // {
    //     return Err("This weapon is uncharged".into());
    // }
    player.update_bonuses();
    player.update_set_effects();
    if monster.stats.hitpoints.base == 0 || monster.stats.hitpoints.current == 0 {
        return Err("The target has no hitpoints".into());
    }
    calc_active_player_rolls(&mut player, monster).map_err(|e| e.to_string())?;
    let distribution = get_distribution(&player, monster, false)
        .map_err(|error| format!("Engine could not build a hit distribution: {error:?}"))?;
    Ok(Prepared {
        player,
        distribution,
    })
}

/// A thrall's contribution in damage per second.
///
/// Thralls attack on their own timer, every `THRALL_ATTACK_SPEED` ticks, and roll
/// uniformly between 0 and their maximum, so their mean hit is half the maximum.
/// Values come from the engine so the calculator and the simulation agree.
fn thrall_dps(thrall: Option<Thrall>, monster: &Monster) -> f64 {
    thrall
        .filter(|thrall| !monster.is_immune_to_thrall(*thrall))
        .map(|thrall| {
            let mean_hit = f64::from(thrall.max_hit()) / 2.0;
            mean_hit / (f64::from(THRALL_ATTACK_SPEED) * SECONDS_PER_TICK)
        })
        .unwrap_or(0.0)
}

/// Main-weapon metrics against the target's opening state. Special attacks are not applied.
pub fn calculate_against(
    player: &Player,
    monster: &Monster,
    thralls: Option<Thrall>,
) -> Result<CombatMetrics, String> {
    let Prepared {
        player,
        distribution,
    } = prepare(player, monster)?;
    let accuracy = get_hit_chance(&player, monster, false)
        .map_err(|error| format!("Error calculating the hit chance: {error:?}."))?;
    // The distribution's own damage decides whether the fight can progress; the
    // burn-inclusive figure below is for display and for DPS.
    let immediate_damage = distribution.get_expected_damage();
    let expected_damage = get_expected_damage(&distribution, &player, monster, false)
        .map_err(|error| format!("Error calculating expected hit: {error:?}"))?;
    let dps = get_dps(&distribution, &player, monster, false)
        .map_err(|error| format!("Error calculating DPS: {error:?}"))?
        + thrall_dps(thralls, monster);
    let combat_type = player.combat_type();
    let mut attack_roll = player
        .att_rolls
        .get(combat_type)
        .map_err(|error| format!("{error:?}"))?;
    if player.is_wearing("Keris partisan of the sun", None)
        && monster.is_toa_monster()
        && monster.stats.hitpoints.current < monster.stats.hitpoints.base / 4
    {
        attack_roll = attack_roll * 5 / 4;
    }
    let expected_ttk = if immediate_damage > 0.0 {
        get_ttk(&distribution, &player, monster, false, false)
            .ok()
            .filter(|ttk| ttk.is_finite() && *ttk > 0.0)
    } else {
        None
    };
    Ok(CombatMetrics {
        dps,
        expected_hit: expected_damage,
        accuracy: accuracy.clamp(0.0, 1.0),
        max_hit: distribution.get_max(),
        attack_roll,
        defence_roll: monster.def_rolls.get(combat_type),
        expected_ttk,
    })
}

#[cfg(test)]
pub fn calculate(
    player: &Player,
    target: &TargetConfig,
    thralls: Option<Thrall>,
) -> Result<CombatMetrics, String> {
    let monster = target
        .combat_monster()
        .ok_or("The target is invalid or its starting HP exceeds its maximum")?;
    calculate_against(player, &monster, thralls)
}

/// Full time-to-kill distribution for the main weapon. More expensive than
/// `calculate_against`; run it when a result is added, not on every edit.
pub fn ttk_distribution(player: &Player, monster: &Monster) -> Result<TtkSummary, String> {
    let Prepared {
        player,
        mut distribution,
    } = prepare(player, monster)?;
    if distribution.get_expected_damage() <= 0.0 {
        return Err("This loadout cannot damage the target".into());
    }
    let by_tick = get_ttk_distribution(&mut distribution, &player, monster, false, true)
        .map_err(|error| format!("Engine could not build a TTK distribution: {error:?}"))?;
    let last = by_tick.keys().copied().max().unwrap_or(0);
    let mut ticks = vec![0.0; last + 1];
    for (tick, probability) in by_tick {
        ticks[tick] = probability;
    }
    let total: f64 = ticks.iter().sum();
    if total <= 0.0 {
        return Err("The kill did not converge within the engine's limit".into());
    }
    let mean = ticks
        .iter()
        .enumerate()
        .map(|(tick, probability)| tick as f64 * probability)
        .sum::<f64>()
        / total
        * SECONDS_PER_TICK;
    let mut cumulative = 0.0;
    let mut p90 = last;
    for (tick, probability) in ticks.iter().enumerate() {
        cumulative += probability / total;
        if cumulative >= 0.9 {
            p90 = tick;
            break;
        }
    }
    Ok(TtkSummary {
        mean,
        p90: p90 as f64 * SECONDS_PER_TICK,
        ticks,
    })
}

fn has_required_ammunition(player: &Player) -> bool {
    let weapon = &player.gear.weapon;
    let name = weapon.name.to_lowercase();
    let salamander = player.is_wearing_salamander();
    if (!player.is_using_ranged() && !salamander)
        || USES_OWN_AMMO.contains(&(weapon.name.as_str(), weapon.version.as_deref()))
    {
        return true;
    }

    let ammo = player.gear.ammo.as_ref();
    if name.contains("crossbow") {
        ammo.is_some_and(|ammo| ammo.is_bolt())
    } else if name.contains("ballista") {
        ammo.is_some_and(|ammo| ammo.name.contains("javelin"))
    } else if name.contains("atlatl") {
        ammo.is_some_and(|ammo| ammo.name == "Atlatl dart")
    } else if salamander {
        ammo.is_some_and(|ammo| ammo.name.ends_with(" tar"))
    } else if name.contains("bow") && !name.contains("blowpipe") {
        ammo.is_some_and(|ammo| ammo.is_arrow())
    } else {
        // Thrown weapons and dart-loaded blowpipes carry their own projectile.
        true
    }
}

pub fn format_seconds(seconds: f64) -> String {
    format!("{seconds:.2}")
}

/// Live read-out of the current editor state, shown at the foot of the Loadout
/// card. Deliberately excludes anything the simulation is responsible for.
#[component]
pub fn MetricsStrip(monster: ReadSignal<Option<Monster>>) -> Element {
    let player = use_context::<Signal<Player>>();
    let state = use_context::<HomeState>();
    let metrics = use_memo(move || {
        let thrall = state.sim.read().thrall.map(ThrallChoice::engine);
        match &*monster.read() {
            Some(monster) => calculate_against(&player.read(), monster, thrall),
            None => Err("The target is invalid or its starting HP exceeds its maximum".to_string()),
        }
    });

    rsx! {
        section {
            class: "metrics-strip",
            aria_label: "Calculated main-weapon stats",
            p { class: "metrics-note",
                "Main weapon against the target's starting state. Special attacks are not included."
            }
            match &*metrics.read() {
                Ok(metrics) => rsx! {
                    dl { class: "metrics-grid",
                        MetricTile {
                            label: "DPS",
                            value: format!("{:.2}", metrics.dps),
                            help: "Average damage per second, including burn and thralls.",
                        }
                        MetricTile {
                            label: "Expected hit",
                            value: format!("{:.2}", metrics.expected_hit),
                            help: "Average damage per attack, including burn but not thralls.",
                        }
                        MetricTile {
                            label: "Max hit",
                            value: metrics.max_hit.to_string(),
                            help: "Maximum combined damage of one hit, not including burn or thralls.",
                        }
                        MetricTile {
                            label: "Accuracy",
                            value: format!("{:.2}", metrics.accuracy * 100.0),
                            unit: "%",
                            help: "Hit chance of a single accuracy roll",
                        }
                        MetricTile { label: "Attack roll", value: metrics.attack_roll.to_string() }
                        MetricTile {
                            label: "Defence roll",
                            value: metrics.defence_roll.to_string(),
                            help: "Target defence roll against the selected combat style.",
                        }
                    }
                },
                Err(reason) => rsx! {
                    p { class: "metrics-empty", "{reason}" }
                },
            }
        }
    }
}

#[component]
fn MetricTile(
    label: &'static str,
    value: String,
    #[props(default = "")] unit: &'static str,
    #[props(default = false)] emphasis: bool,
    #[props(default = "")] help: &'static str,
) -> Element {
    rsx! {
        div {
            class: if emphasis { "metric-tile is-emphasis" } else { "metric-tile" },
            title: "{help}",
            dt { "{label}" }
            dd { class: "num",
                "{value}"
                if !unit.is_empty() {
                    small { "{unit}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osrs::types::equipment::{CombatStyle, Weapon};

    #[test]
    fn invocation_scaling_reaches_displayed_defence_roll() {
        let player = Player::default();
        let normal = calculate(&player, &TargetConfig::example("Zebak", 0, 0, 0), None).unwrap();
        let raid = calculate(&player, &TargetConfig::example("Zebak", 300, 2, 0), None).unwrap();
        assert_eq!(raid.defence_roll, normal.defence_roll * 2200 / 1000);
        assert!(raid.accuracy < normal.accuracy);
    }

    #[test]
    fn starting_reduction_updates_accuracy_without_changing_player() {
        let player = Player::default();
        let before_stats = player.stats;
        let normal = calculate(
            &player,
            &TargetConfig::example("General Graardor", 0, 0, 0),
            None,
        )
        .unwrap();
        let reduced = calculate(
            &player,
            &TargetConfig::example("General Graardor", 0, 0, 40),
            None,
        )
        .unwrap();
        assert_eq!(reduced.defence_roll, (210 + 9) * (90 + 64));
        assert!(reduced.accuracy > normal.accuracy);
        assert_eq!(reduced.max_hit, normal.max_hit);
        assert_eq!(player.stats, before_stats);
    }

    #[test]
    fn casting_without_a_spell_is_incomplete() {
        let mut player = Player::default();
        player
            .equip_item(Box::new(Weapon::new("Staff of air", None).unwrap()))
            .unwrap();
        player.set_active_style(CombatStyle::Spell);
        assert!(calculate(&player, &TargetConfig::default(), None).is_err());
    }

    #[test]
    fn ttk_distribution_mean_matches_expected_ttk() {
        let mut player = Player::default();
        player
            .equip_item(Box::new(Weapon::new("Abyssal whip", None).unwrap()))
            .unwrap();
        player.set_active_style(CombatStyle::Lash);
        let target = TargetConfig::example("General Graardor", 0, 0, 0);
        let monster = target.combat_monster().unwrap();
        let metrics = calculate_against(&player, &monster, None).unwrap();
        let summary = ttk_distribution(&player, &monster).unwrap();
        let expected = metrics.expected_ttk.unwrap();
        assert!(
            (summary.mean - expected).abs() < 0.5,
            "{} vs {expected}",
            summary.mean
        );
        assert!(summary.p90 > summary.mean);
        assert!((summary.ticks.iter().sum::<f64>() - 1.0).abs() < 1e-3);
    }
}
