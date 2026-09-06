//! Cheap engine calculations for the main weapon: rolls, DPS, and expected TTK.

use dioxus::prelude::*;
use osrs::calc::dps_calc::{get_distribution, get_dps, get_ttk, get_ttk_distribution};
use osrs::calc::hit_dist::AttackDistribution;
use osrs::calc::rolls::calc_active_player_rolls;
use osrs::constants::{SECONDS_PER_TICK, USES_OWN_AMMO};
use osrs::types::equipment::{CombatStance, CombatType};
use osrs::types::monster::Monster;
use osrs::types::player::Player;
use osrs::types::spells::{Spell, StandardSpell};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use super::target::TargetConfig;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CombatMetrics {
    pub dps: f64,
    /// Probability that the first hitsplat passes its accuracy check, including accurate zeros.
    pub accuracy: f64,
    /// Maximum combined immediate damage from one attack, including additional hitsplats.
    pub max_hit: u32,
    pub attack_roll: i32,
    pub defence_roll: i32,
    /// Expected interval between attacks, in seconds.
    pub attack_interval: f64,
    /// Expected time to kill with the main weapon only, in seconds.
    #[serde(default)]
    pub expected_ttk: Option<f64>,
}

/// Time-to-kill distribution for one result, in ticks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TtkSummary {
    pub mean: f64,
    pub p90: f64,
    /// Probability of the kill landing on each tick; index is the tick.
    pub ticks: Vec<f64>,
}

struct Prepared {
    player: Player,
    distribution: AttackDistribution,
}

/// Validate the loadout and build its hit distribution against the target's starting state.
fn prepare(player: &Player, monster: &Monster) -> Result<Prepared, String> {
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
    } else if style.combat_type == CombatType::Magic && !has_inbuilt_magic_attack(player) {
        // The library panics when asked for a magic max hit without a spell or supported staff.
        return Err("This staff needs a spell selected".into());
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
    if (player.is_using_magic() || player.is_using_ranged())
        && player
            .gear
            .weapon
            .version
            .as_deref()
            .is_some_and(|version| {
                let version = version.to_lowercase();
                ["uncharged", "inactive", "empty", "broken"]
                    .iter()
                    .any(|state| version.contains(state))
            })
    {
        return Err("This weapon is uncharged".into());
    }
    player.update_bonuses();
    player.update_set_effects();
    if player.set_effects.full_dharoks {
        // The library subtracts current HP from base HP without saturation.
        // Overhealing gives no Dharok bonus, rather than overflowing that subtraction.
        player.stats.hitpoints.current = player
            .stats
            .hitpoints
            .current
            .min(player.stats.hitpoints.base);
    }
    if monster.stats.hitpoints.base == 0 || monster.stats.hitpoints.current == 0 {
        return Err("The target has no hitpoints".into());
    }
    calc_active_player_rolls(&mut player, monster);
    let distribution = get_distribution(&player, monster, false)
        .map_err(|error| format!("Engine could not build a hit distribution: {error:?}"))?;
    Ok(Prepared {
        player,
        distribution,
    })
}

/// Main-weapon metrics against the target's opening state. Special attacks are not applied.
pub fn calculate_against(player: &Player, monster: &Monster) -> Result<CombatMetrics, String> {
    let Prepared {
        player,
        distribution,
    } = prepare(player, monster)?;
    let first_hit = distribution
        .dists
        .first()
        .ok_or("Engine returned an empty distribution")?;
    let accuracy: f64 = first_hit
        .hits
        .iter()
        .filter(|hit| hit.hitsplats.first().is_some_and(|splat| splat.accurate))
        .map(|hit| hit.probability)
        .sum();

    let mut attack_interval = player.gear.weapon.speed as f64 * SECONDS_PER_TICK;
    if player.set_effects.full_blood_moon {
        // Only this set needs the library's probabilistic-delay calculation.
        let immediate_damage = distribution.get_expected_damage();
        let immediate_dps = get_dps(&distribution, &player, false);
        if immediate_damage > 0.0 && immediate_dps > 0.0 {
            attack_interval = immediate_damage / immediate_dps;
        }
    }
    let expected_damage = distribution.get_expected_damage();
    let dps = expected_damage / attack_interval;
    if !dps.is_finite()
        || dps < 0.0
        || !attack_interval.is_finite()
        || attack_interval <= 0.0
        || !accuracy.is_finite()
        || !(-1e-9..=1.0 + 1e-9).contains(&accuracy)
    {
        return Err("Engine returned an invalid result".into());
    }

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
    let expected_ttk = if expected_damage > 0.0 {
        get_ttk(&distribution, &player, monster, false, false)
            .ok()
            .filter(|ttk| ttk.is_finite() && *ttk > 0.0)
    } else {
        None
    };
    Ok(CombatMetrics {
        dps,
        accuracy: accuracy.clamp(0.0, 1.0),
        max_hit: distribution.get_max(),
        attack_roll,
        defence_roll: monster.def_rolls.get(combat_type),
        attack_interval,
        expected_ttk,
    })
}

#[cfg(test)]
pub fn calculate(player: &Player, target: &TargetConfig) -> Result<CombatMetrics, String> {
    let monster = target
        .combat_monster()
        .ok_or("The target is invalid or its starting HP exceeds its maximum")?;
    calculate_against(player, &monster)
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

fn has_inbuilt_magic_attack(player: &Player) -> bool {
    // Keep this aligned with osrs::calc::rolls::charged_staff_max_hit and
    // salamander_max_hit; unsupported equipment must not reach their panic arms.
    matches!(
        player.gear.weapon.name.as_str(),
        "Starter staff"
            | "Warped sceptre"
            | "Trident of the seas"
            | "Trident of the seas (e)"
            | "Thammaron's sceptre"
            | "Accursed sceptre"
            | "Trident of the swamp"
            | "Trident of the swamp (e)"
            | "Sanguinesti staff"
            | "Dawnbringer"
            | "Tumeken's shadow"
            | "Bone staff"
            | "Crystal staff (basic)"
            | "Corrupted staff (basic)"
            | "Crystal staff (attuned)"
            | "Corrupted staff (attuned)"
            | "Crystal staff (perfected)"
            | "Corrupted staff (perfected)"
            | "Swamp lizard"
            | "Orange salamander"
            | "Red salamander"
            | "Black salamander"
            | "Tecu salamander"
    )
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
    if seconds >= 100.0 {
        format!("{seconds:.0}")
    } else {
        format!("{seconds:.1}")
    }
}

/// Live read-out of the current editor state. Spans the editor so it reads as
/// the output of the whole configuration rather than of one panel.
#[component]
pub fn MetricsStrip(monster: ReadSignal<Option<Monster>>) -> Element {
    let player = use_context::<Signal<Player>>();
    let metrics = use_memo(move || match &*monster.read() {
        Some(monster) => calculate_against(&player.read(), monster),
        None => Err("The target is invalid or its starting HP exceeds its maximum".to_string()),
    });

    rsx! {
        section {
            class: "card metrics-strip",
            aria_label: "Live main-weapon combat metrics",
            div { class: "metrics-strip-heading",
                span { class: "card-title", "Main weapon" }
                span { class: "home-muted", "Against the target's starting state. Special attacks are not included." }
            }
            match &*metrics.read() {
                Ok(metrics) => rsx! {
                    dl { class: "metrics-grid",
                        MetricTile { label: "Expected TTK", value: metrics.expected_ttk.map(format_seconds).unwrap_or_else(|| "—".into()), unit: "s", emphasis: true, help: "Expected time to kill with the main weapon only." }
                        MetricTile { label: "DPS", value: format!("{:.2}", metrics.dps), help: "Immediate damage per second, including procs and multi-hit attacks. Excludes delayed burns and poison." }
                        MetricTile { label: "Accuracy", value: format!("{:.1}", metrics.accuracy * 100.0), unit: "%", help: "First hitsplat accuracy, including successful zero-damage hits." }
                        MetricTile { label: "Max hit", value: metrics.max_hit.to_string(), help: "Maximum combined immediate damage across all hitsplats in one attack." }
                        MetricTile { label: "Attack roll", value: metrics.attack_roll.to_string() }
                        MetricTile { label: "Defence roll", value: metrics.defence_roll.to_string(), help: "Target defence roll against the selected combat style." }
                        MetricTile { label: "Interval", value: format!("{:.1}", metrics.attack_interval), unit: "s", help: "Expected time between attacks." }
                    }
                },
                Err(reason) => rsx! { p { class: "metrics-empty", "{reason}" } },
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
        div { class: if emphasis { "metric-tile is-emphasis" } else { "metric-tile" }, title: "{help}",
            dt { "{label}" }
            dd { class: "num", "{value}", if !unit.is_empty() { small { "{unit}" } } }
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
        let normal = calculate(&player, &TargetConfig::example("Zebak", 0, 0, 0)).unwrap();
        let raid = calculate(&player, &TargetConfig::example("Zebak", 300, 2, 0)).unwrap();
        assert_eq!(raid.defence_roll, normal.defence_roll * 2200 / 1000);
        assert!(raid.accuracy < normal.accuracy);
    }

    #[test]
    fn starting_reduction_updates_accuracy_without_changing_player() {
        let player = Player::default();
        let before_stats = player.stats;
        let normal =
            calculate(&player, &TargetConfig::example("General Graardor", 0, 0, 0)).unwrap();
        let reduced = calculate(
            &player,
            &TargetConfig::example("General Graardor", 0, 0, 40),
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
        assert!(calculate(&player, &TargetConfig::default()).is_err());
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
        let metrics = calculate_against(&player, &monster).unwrap();
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
