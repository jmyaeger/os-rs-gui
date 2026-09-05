use crate::pages::gauntlet::components::loadout::LoadoutStyle;
use dioxus::prelude::*;
use osrs::calc::analysis::SimulationStats;
use osrs::sims::hunleff::HunllefConfig;
use osrs::types::equipment::{CombatStyle, Gear};
use osrs::types::player::Player;
use osrs::types::stats::PlayerStats;

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

/// A plain snapshot of everything a simulation run needs. This is what
/// `build_simulation_input` consumes, and what tests construct directly.
#[derive(Debug, Clone)]
pub struct SimulationParams {
    pub player_stats: PlayerStats,
    pub melee_switch: Player,
    pub ranged_switch: Player,
    pub magic_switch: Player,
    pub simulation_mode: SimulationMode,
    pub two_t3_selections: TwoT3Selections,
    pub five_one_main_style: LoadoutStyle,
    pub sim_config: HunllefConfig,
    pub num_trials: u32,
}

impl Default for SimulationParams {
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
            player_stats: Player::default().stats,
            melee_switch,
            ranged_switch,
            magic_switch,
            simulation_mode: SimulationMode::default(),
            two_t3_selections: TwoT3Selections::default(),
            five_one_main_style: LoadoutStyle::Magic,
            sim_config: HunllefConfig::default(),
            num_trials: 10_000,
        }
    }
}

/// Shared Gauntlet page state as a set of independent signals, so components
/// subscribe only to the pieces they actually read.
#[derive(Clone, Copy)]
pub struct GauntletState {
    pub player: Signal<Player>,
    pub melee_switch: Signal<Player>,
    pub ranged_switch: Signal<Player>,
    pub magic_switch: Signal<Player>,
    pub results: Signal<Option<SimulationStats>>,
    pub simulation_mode: Signal<SimulationMode>,
    pub two_t3_selections: Signal<TwoT3Selections>,
    pub five_one_main_style: Signal<LoadoutStyle>,
    pub sim_config: Signal<HunllefConfig>,
    pub num_trials: Signal<u32>,
    /// False while any numeric option input holds text that doesn't parse or
    /// is out of range; the Simulate button is disabled in that case.
    pub options_valid: Signal<bool>,
}

impl GauntletState {
    /// Must be called from within a component (signals need a runtime scope).
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let defaults = SimulationParams::default();
        Self {
            player: Signal::new(Player::default()),
            melee_switch: Signal::new(defaults.melee_switch),
            ranged_switch: Signal::new(defaults.ranged_switch),
            magic_switch: Signal::new(defaults.magic_switch),
            results: Signal::new(None),
            simulation_mode: Signal::new(defaults.simulation_mode),
            two_t3_selections: Signal::new(defaults.two_t3_selections),
            five_one_main_style: Signal::new(defaults.five_one_main_style),
            sim_config: Signal::new(defaults.sim_config),
            num_trials: Signal::new(defaults.num_trials),
            options_valid: Signal::new(true),
        }
    }

    /// Clone the current state into a plain value for the simulation worker.
    pub fn snapshot(&self) -> SimulationParams {
        SimulationParams {
            player_stats: self.player.peek().stats,
            melee_switch: self.melee_switch.peek().clone(),
            ranged_switch: self.ranged_switch.peek().clone(),
            magic_switch: self.magic_switch.peek().clone(),
            simulation_mode: *self.simulation_mode.peek(),
            two_t3_selections: *self.two_t3_selections.peek(),
            five_one_main_style: *self.five_one_main_style.peek(),
            sim_config: self.sim_config.peek().clone(),
            num_trials: *self.num_trials.peek(),
        }
    }

    /// The switch player signal for a given loadout style.
    pub fn switch_signal(&self, style: LoadoutStyle) -> Signal<Player> {
        match style {
            LoadoutStyle::Melee => self.melee_switch,
            LoadoutStyle::Ranged => self.ranged_switch,
            LoadoutStyle::Magic => self.magic_switch,
        }
    }
}
