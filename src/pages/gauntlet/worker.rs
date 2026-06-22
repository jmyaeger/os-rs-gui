use osrs::calc::analysis::SimulationStats;
use osrs::calc::rolls::calc_active_player_rolls;
use osrs::combat::simulation::simulate_n_fights;
use osrs::sims::hunleff::HunllefConfig;
use osrs::types::equipment::{CombatStyle, Gear};
use osrs::types::monster::Monster;
use osrs::types::player::{GearSwitch, Player, SwitchType};
use osrs::types::prayers::Prayer;
use osrs::types::stats::PlayerStats;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub id: u32,
    pub input: SimulationInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub id: u32,
    pub output: SimulationOutput,
}

/// Serializable input for the simulation worker
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

/// Serializable output from the simulation worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutput {
    pub success: bool,
    pub stats: Option<SimulationStats>,
    pub error: Option<String>,
}

/// Check if we're running in a Web Worker context
pub fn is_worker_context() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        web_sys::window().is_none()
            && js_sys::global()
                .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
                .is_ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// Run simulation with the given input - called from worker
pub fn run_simulation_from_input(input: SimulationInput) -> SimulationOutput {
    let result = run_simulation_inner(input);
    match result {
        Ok(stats) => SimulationOutput {
            success: true,
            stats: Some(stats),
            error: None,
        },
        Err(e) => SimulationOutput {
            success: false,
            stats: None,
            error: Some(format!("{:?}", e)),
        },
    }
}

fn run_simulation_inner(
    input: SimulationInput,
) -> Result<SimulationStats, osrs::error::SimulationError> {
    // Build gear for each switch
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
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{:?}", e)))?;

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
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{:?}", e)))?;

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
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{:?}", e)))?;

    // Build players for each switch
    let mut melee_switch = Player::builder()
        .player_stats(input.player_stats)
        .gear(melee_gear)
        .active_style(input.melee_style)
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{:?}", e)))?;

    for prayer in input.melee_prayers {
        melee_switch.add_prayer(prayer);
    }

    let mut ranged_switch = Player::builder()
        .player_stats(input.player_stats)
        .gear(ranged_gear)
        .active_style(input.ranged_style)
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{:?}", e)))?;

    for prayer in input.ranged_prayers {
        ranged_switch.add_prayer(prayer);
    }

    let mut magic_switch = Player::builder()
        .player_stats(input.player_stats)
        .gear(magic_gear)
        .active_style(input.magic_style)
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{:?}", e)))?;

    for prayer in input.magic_prayers {
        magic_switch.add_prayer(prayer);
    }

    let hunllef = Monster::new("Corrupted Hunllef", None)
        .map_err(|e| osrs::error::SimulationError::MonsterCreationError(format!("{:?}", e)))?;

    // Calculate rolls
    calc_active_player_rolls(&mut melee_switch, &hunllef);
    calc_active_player_rolls(&mut ranged_switch, &hunllef);
    calc_active_player_rolls(&mut magic_switch, &hunllef);

    // Build the main player with switches
    let mut player = Player::builder()
        .player_stats(input.player_stats)
        .build()
        .map_err(|e| osrs::error::SimulationError::ConfigError(format!("{:?}", e)))?;

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

    // Run simulation
    let fight = osrs::sims::hunleff::HunllefFight::new(player, input.sim_config.clone())?;
    let results = simulate_n_fights(
        Box::new(fight),
        input.num_trials,
        input.sim_config.only_success_stats,
    )?;
    let stats = SimulationStats::new(&results);

    Ok(stats)
}

/// Entry point for the web worker - called from worker.js
#[wasm_bindgen]
pub fn start_simulation_worker() {
    use serde_wasm_bindgen as swb;
    use web_sys::DedicatedWorkerGlobalScope;

    let global = js_sys::global();
    let scope = DedicatedWorkerGlobalScope::unchecked_from_js(global.into());

    let scope_clone = scope.clone();
    let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        web_sys::console::log_1(&"Worker: Received message".into());
        let data = event.data();

        let req: WorkerRequest = match swb::from_value(data) {
            Ok(v) => v,
            Err(e) => {
                let resp = WorkerResponse {
                    id: 0,
                    output: SimulationOutput {
                        success: false,
                        stats: None,
                        error: Some(format!("Failed to decode request: {e}")),
                    },
                };

                if let Ok(js_resp) = swb::to_value(&resp) {
                    let _ = scope_clone.post_message(&js_resp);
                }
                return;
            }
        };

        web_sys::console::log_1(
            &format!(
                "Worker: Running {} trials (id={})",
                req.input.num_trials, req.id
            )
            .into(),
        );

        let output = run_simulation_from_input(req.input);
        web_sys::console::log_1(&"Worker: Simulation complete".to_string().into());

        let resp = WorkerResponse { id: req.id, output };

        match swb::to_value(&resp) {
            Ok(js_resp) => {
                let _ = scope_clone.post_message(&js_resp);
            }
            Err(e) => {
                let fallback = WorkerResponse {
                    id: req.id,
                    output: SimulationOutput {
                        success: false,
                        stats: None,
                        error: Some(format!("Failed to encode response: {e}")),
                    },
                };

                if let Ok(js_resp) = swb::to_value(&fallback) {
                    let _ = scope_clone.post_message(&js_resp);
                }
            }
        }
    }) as Box<dyn Fn(web_sys::MessageEvent)>);

    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget(); // Prevent the closure from being dropped
    web_sys::console::log_1(&"Worker: onmessage handler registered".into());
}
