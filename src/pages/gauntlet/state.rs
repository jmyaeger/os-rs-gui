use crate::pages::gauntlet::components::loadout::LoadoutStyle;
use osrs::calc::analysis::SimulationStats;
use osrs::sims::hunleff::HunllefConfig;
use osrs::types::equipment::{CombatStyle, Gear};
use osrs::types::{monster::Monster, player::Player};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SimulationMode {
    #[default]
    TwoT3,
    FiveOne,
}

/// Tracks selected styles for the two T3 weapon loadout cards
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoT3Selections {
    pub loadout1_style: Option<LoadoutStyle>,
    pub loadout2_style: Option<LoadoutStyle>,
}

impl Default for TwoT3Selections {
    fn default() -> Self {
        Self {
            loadout1_style: Some(LoadoutStyle::Magic),
            loadout2_style: Some(LoadoutStyle::Ranged),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub player: Player,
    pub melee_switch: Player,
    pub ranged_switch: Player,
    pub magic_switch: Player,
    pub hunllef: Monster,
    pub results: Option<SimulationStats>,
    pub simulation_mode: SimulationMode,
    pub two_t3_selections: TwoT3Selections,
    /// The main combat style for 5:1 mode (defaults to Magic)
    pub five_one_main_style: LoadoutStyle,
    pub sim_config: HunllefConfig,
    pub num_trials: u32,
}

impl Default for AppState {
    fn default() -> Self {
        let melee_gear = Gear::builder()
            .body("Corrupted body (basic)", None)
            .head("Corrupted helm (basic)", None)
            .legs("Corrupted legs (basic)", None)
            .weapon("Corrupted halberd (perfected)", None)
            .build()
            .unwrap();

        let ranged_gear = Gear::builder()
            .body("Corrupted body (basic)", None)
            .head("Corrupted helm (basic)", None)
            .legs("Corrupted legs (basic)", None)
            .weapon("Corrupted bow (perfected)", None)
            .build()
            .unwrap();

        let magic_gear = Gear::builder()
            .body("Corrupted body (basic)", None)
            .head("Corrupted helm (basic)", None)
            .legs("Corrupted legs (basic)", None)
            .weapon("Corrupted staff (perfected)", None)
            .build()
            .unwrap();

        let melee_switch = Player::builder()
            .gear(melee_gear)
            .active_style(CombatStyle::Swipe)
            .build()
            .unwrap();

        let ranged_switch = Player::builder()
            .gear(ranged_gear)
            .active_style(CombatStyle::Rapid)
            .build()
            .unwrap();

        let magic_switch = Player::builder()
            .gear(magic_gear)
            .active_style(CombatStyle::Accurate)
            .build()
            .unwrap();

        Self {
            player: Player::default(),
            melee_switch,
            ranged_switch,
            magic_switch,
            hunllef: Monster::new("Corrupted Hunllef", None).unwrap(),
            results: None,
            simulation_mode: SimulationMode::default(),
            two_t3_selections: TwoT3Selections::default(),
            five_one_main_style: LoadoutStyle::Magic,
            sim_config: HunllefConfig::default(),
            num_trials: 10_000,
        }
    }
}
