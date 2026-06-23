use crate::pages::gauntlet::components::loadout::LoadoutStyle;
use crate::pages::gauntlet::state::{AppState, SimulationMode};
use osrs::calc::analysis::SimulationStats;
use osrs::calc::rolls::calc_active_player_rolls;
use osrs::combat::simulation::simulate_n_fights;
use osrs::sims::hunleff::{AttackStrategy, HunllefConfig};
use osrs::types::equipment::{CombatStyle, Gear};
use osrs::types::monster::Monster;
use osrs::types::player::{GearSwitch, Player, SwitchType};
use osrs::types::prayers::Prayer;
use osrs::types::stats::PlayerStats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationInput {
    pub player_stats: PlayerStats,
    pub melee_weapon: String,
    pub melee_style: CombatStyle,
    pub melee_prayers: Vec<Prayer>,
    pub ranged_weapon: String,
    pub ranged_style: CombatStyle,
    pub ranged_prayers: Vec<Prayer>,
    pub magic_weapon: String,
    pub magic_style: CombatStyle,
    pub magic_prayers: Vec<Prayer>,
    pub armor_body: Option<String>,
    pub armor_head: Option<String>,
    pub armor_legs: Option<String>,
    pub sim_config: HunllefConfig,
    pub num_trials: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutput {
    pub success: bool,
    pub stats: Option<SimulationStats>,
    pub error: Option<String>,
}

const ALL_PRAYERS: [Prayer; 21] = [
    Prayer::ClarityOfThought,
    Prayer::BurstOfStrength,
    Prayer::ThickSkin,
    Prayer::SharpEye,
    Prayer::MysticWill,
    Prayer::ImprovedReflexes,
    Prayer::SuperhumanStrength,
    Prayer::RockSkin,
    Prayer::HawkEye,
    Prayer::MysticLore,
    Prayer::IncredibleReflexes,
    Prayer::UltimateStrength,
    Prayer::SteelSkin,
    Prayer::EagleEye,
    Prayer::MysticMight,
    Prayer::Chivalry,
    Prayer::Deadeye,
    Prayer::MysticVigour,
    Prayer::Piety,
    Prayer::Rigour,
    Prayer::Augury,
];

fn collect_active_prayers(p: &Player) -> Vec<Prayer> {
    ALL_PRAYERS
        .into_iter()
        .filter(|pr| p.prayers.contains_prayer(*pr))
        .collect()
}

fn to_switch_type(style: LoadoutStyle) -> SwitchType {
    match style {
        LoadoutStyle::Magic => SwitchType::Magic,
        LoadoutStyle::Ranged => SwitchType::Ranged,
        LoadoutStyle::Melee => SwitchType::Melee,
    }
}

pub fn build_simulation_input(state: &AppState) -> SimulationInput {
    let attack_strategy = match state.simulation_mode {
        SimulationMode::TwoT3 => {
            let styles = state.two_t3_selections;
            AttackStrategy::TwoT3Weapons {
                style1: to_switch_type(
                    styles.loadout1_style.expect("loadout1_style should be set"),
                ),
                style2: to_switch_type(
                    styles.loadout2_style.expect("loadout2_style should be set"),
                ),
            }
        }
        SimulationMode::FiveOne => {
            let main_style = to_switch_type(state.five_one_main_style);
            let (other_style1, other_style2) = match main_style {
                SwitchType::Magic => (SwitchType::Ranged, SwitchType::Melee),
                SwitchType::Ranged => (SwitchType::Magic, SwitchType::Melee),
                SwitchType::Melee => (SwitchType::Magic, SwitchType::Ranged),
                _ => unreachable!(),
            };
            AttackStrategy::FiveToOne {
                main_style,
                other_style1,
                other_style2,
            }
        }
    };

    let mut sim_config = state.sim_config.clone();
    sim_config.attack_strategy = attack_strategy;

    SimulationInput {
        player_stats: state.player.stats,
        melee_weapon: state.melee_switch.gear.weapon.name.clone(),
        melee_style: state.melee_switch.attrs.active_style,
        melee_prayers: collect_active_prayers(&state.melee_switch),
        ranged_weapon: state.ranged_switch.gear.weapon.name.clone(),
        ranged_style: state.ranged_switch.attrs.active_style,
        ranged_prayers: collect_active_prayers(&state.ranged_switch),
        magic_weapon: state.magic_switch.gear.weapon.name.clone(),
        magic_style: state.magic_switch.attrs.active_style,
        magic_prayers: collect_active_prayers(&state.magic_switch),
        armor_body: state
            .melee_switch
            .gear
            .body
            .as_ref()
            .map(|a| a.name.clone()),
        armor_head: state
            .melee_switch
            .gear
            .head
            .as_ref()
            .map(|a| a.name.clone()),
        armor_legs: state
            .melee_switch
            .gear
            .legs
            .as_ref()
            .map(|a| a.name.clone()),
        sim_config,
        num_trials: state.num_trials,
    }
}

pub fn run_simulation_from_input(input: SimulationInput) -> SimulationOutput {
    match run_simulation_inner(input) {
        Ok(stats) => SimulationOutput {
            success: true,
            stats: Some(stats),
            error: None,
        },
        Err(e) => SimulationOutput {
            success: false,
            stats: None,
            error: Some(format!("{e:?}")),
        },
    }
}

fn run_simulation_inner(
    input: SimulationInput,
) -> Result<SimulationStats, osrs::error::SimulationError> {
    let mut melee_builder = Gear::builder();
    if let Some(body) = input.armor_body.as_deref() {
        melee_builder = melee_builder.body(body, None);
    }
    if let Some(head) = input.armor_head.as_deref() {
        melee_builder = melee_builder.head(head, None);
    }
    if let Some(legs) = input.armor_legs.as_deref() {
        melee_builder = melee_builder.legs(legs, None);
    }
    if input.melee_weapon != "Unarmed" {
        melee_builder = melee_builder.weapon(input.melee_weapon.as_str(), None);
    }
    let melee_gear = melee_builder
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{e:?}")))?;

    let mut ranged_builder = Gear::builder().weapon(&input.ranged_weapon, None);
    if let Some(body) = input.armor_body.as_deref() {
        ranged_builder = ranged_builder.body(body, None);
    }
    if let Some(head) = input.armor_head.as_deref() {
        ranged_builder = ranged_builder.head(head, None);
    }
    if let Some(legs) = input.armor_legs.as_deref() {
        ranged_builder = ranged_builder.legs(legs, None);
    }
    let ranged_gear = ranged_builder
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{e:?}")))?;

    let mut magic_builder = Gear::builder().weapon(&input.magic_weapon, None);
    if let Some(body) = input.armor_body.as_deref() {
        magic_builder = magic_builder.body(body, None);
    }
    if let Some(head) = input.armor_head.as_deref() {
        magic_builder = magic_builder.head(head, None);
    }
    if let Some(legs) = input.armor_legs.as_deref() {
        magic_builder = magic_builder.legs(legs, None);
    }
    let magic_gear = magic_builder
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{e:?}")))?;

    let mut melee_switch = Player::builder()
        .player_stats(input.player_stats)
        .gear(melee_gear)
        .active_style(input.melee_style)
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{e:?}")))?;
    for prayer in input.melee_prayers {
        melee_switch.add_prayer(prayer);
    }

    let mut ranged_switch = Player::builder()
        .player_stats(input.player_stats)
        .gear(ranged_gear)
        .active_style(input.ranged_style)
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{e:?}")))?;
    for prayer in input.ranged_prayers {
        ranged_switch.add_prayer(prayer);
    }

    let mut magic_switch = Player::builder()
        .player_stats(input.player_stats)
        .gear(magic_gear)
        .active_style(input.magic_style)
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{e:?}")))?;
    for prayer in input.magic_prayers {
        magic_switch.add_prayer(prayer);
    }

    let hunllef = Monster::new("Corrupted Hunllef", None)
        .map_err(|e| osrs::error::SimulationError::MonsterCreationError(format!("{e:?}")))?;

    calc_active_player_rolls(&mut melee_switch, &hunllef);
    calc_active_player_rolls(&mut ranged_switch, &hunllef);
    calc_active_player_rolls(&mut magic_switch, &hunllef);

    let mut player = Player::builder()
        .player_stats(input.player_stats)
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{e:?}")))?;

    player.switches.clear();
    player
        .switches
        .push(GearSwitch::new(SwitchType::Melee, &melee_switch, &hunllef));
    player.switches.push(GearSwitch::new(
        SwitchType::Ranged,
        &ranged_switch,
        &hunllef,
    ));
    player
        .switches
        .push(GearSwitch::new(SwitchType::Magic, &magic_switch, &hunllef));
    let _ = player.switch(&SwitchType::Magic);

    let fight = osrs::sims::hunleff::HunllefFight::new(player, input.sim_config.clone())?;
    let results = simulate_n_fights(
        Box::new(fight),
        input.num_trials,
        input.sim_config.only_success_stats,
    )?;

    Ok(SimulationStats::new(&results))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_input_uses_selected_attack_style() {
        let mut state = AppState::default();
        state.melee_switch.set_active_style(CombatStyle::Jab);

        let input = build_simulation_input(&state);

        assert_eq!(input.melee_style, CombatStyle::Jab);
    }

    #[test]
    fn default_high_trial_simulation_completes() {
        let mut state = AppState::default();
        state.num_trials = 100_000;

        let output = run_simulation_from_input(build_simulation_input(&state));

        assert!(
            output.success,
            "simulation failed: {}",
            output.error.unwrap_or_default()
        );
    }
}
