use crate::pages::gauntlet::components::loadout::LoadoutStyle;
use crate::pages::gauntlet::components::plots::{FoodHistogram, TimeUnit, TtkCdf};
use crate::pages::gauntlet::state::{AppState, SimulationMode};
use crate::pages::gauntlet::worker::{SimulationInput, WorkerRequest, WorkerResponse};
use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info};
use osrs::sims::hunleff::AttackStrategy;
use osrs::types::player::SwitchType;
use osrs::types::prayers::Prayer;
use serde_wasm_bindgen as swb;
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

/// Holds the worker and its event handler closures.
/// Closures are stored here (not forgotten) so they can be replaced each simulation.
struct WorkerState {
    worker: Worker,
    // Store closures to keep them alive and allow replacement
    _onmessage: Closure<dyn FnMut(MessageEvent)>,
    _onerror: Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

thread_local! {
    static WORKER_STATE: RefCell<Option<WorkerState>> = const { RefCell::new(None) };
}

thread_local! {
    static NEXT_REQUEST_ID: RefCell<u32> = const { RefCell::new(1) };
}

fn next_request_id() -> u32 {
    NEXT_REQUEST_ID.with(|cell| {
        let mut id = cell.borrow_mut();
        let out = *id;
        *id = id.wrapping_add(1);
        out
    })
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

fn collect_active_prayers(p: &osrs::types::player::Player) -> Vec<Prayer> {
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

#[cfg(target_arch = "wasm32")]
fn normalize_url(s: String) -> String {
    if let Some(rest) = s.strip_prefix("/./") {
        format!("/{rest}")
    } else if let Some(rest) = s.strip_prefix("./") {
        format!("/{rest}")
    } else if !s.starts_with('/') && !s.starts_with("http://") && !s.starts_with("https://") {
        format!("/{s}")
    } else {
        s
    }
}

#[cfg(target_arch = "wasm32")]
fn find_js_bundle_url() -> Result<String, JsValue> {
    use web_sys::window;

    let document = window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    // Match any of: "/assets/os-rs-gui-*.js", "/./assets/...", "./assets/...", "assets/..."
    let js_url = if let Some(el) =
        document.query_selector(r#"script[type="module"][src*="os-rs-gui-"][src$=".js"]"#)?
    {
        normalize_url(
            el.get_attribute("src")
                .ok_or_else(|| JsValue::from_str("module script missing src"))?,
        )
    } else {
        "/wasm/os-rs-gui.js".to_string()
    };

    web_sys::console::log_1(&format!("Worker init JS URL: {js_url}").into());
    Ok(js_url)
}

/// Creates a new Worker instance (without handlers).
fn create_worker() -> Result<Worker, JsValue> {
    let opts = WorkerOptions::new();
    opts.set_type(WorkerType::Module);

    let worker_url = asset!("/assets/worker.js").to_string();
    let worker = Worker::new_with_options(&worker_url, &opts)?;

    // Send init config so the worker can import the hashed bundle in release
    #[cfg(target_arch = "wasm32")]
    {
        let js_url = find_js_bundle_url()?;

        let init_msg = js_sys::Object::new();
        js_sys::Reflect::set(&init_msg, &"type".into(), &"init".into())?;
        js_sys::Reflect::set(&init_msg, &"js_url".into(), &js_url.into())?;

        worker.post_message(&init_msg)?;
    }

    Ok(worker)
}

/// Runs a simulation with fresh closures that capture current signals.
/// Closures are stored (not forgotten) and replaced each call to avoid memory leaks.
fn run_simulation(
    req: WorkerRequest,
    app_state: Signal<AppState>,
    in_progress: Signal<bool>,
    error_msg: Signal<Option<String>>,
) -> Result<(), JsValue> {
    WORKER_STATE.with(|state_cell| {
        let mut state_opt = state_cell.borrow_mut();

        // Get existing worker or create new one
        let worker = if let Some(state) = state_opt.as_ref() {
            state.worker.clone()
        } else {
            create_worker()?
        };

        // Wrap signals in Rc<RefCell<>> for interior mutability in Fn closures
        let mut app_state = app_state;
        let mut in_progress = in_progress;
        let mut error_msg = error_msg;

        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            let data = event.data();

            let resp: WorkerResponse = match swb::from_value(data) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to decode worker response: {e:?}");
                    error_msg.set(Some(format!("Failed to decode response: {e}")));
                    in_progress.set(false);
                    return;
                }
            };

            if resp.id != req.id {
                return;
            }

            if resp.output.success {
                if let Some(stats) = resp.output.stats {
                    info!("Simulation completed successfully!");
                    app_state.write().results = Some(stats);
                }
            } else {
                let err = resp
                    .output
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string());
                error!("Simulation failed: {err}");
                error_msg.set(Some(err));
            }

            in_progress.set(false);
        }) as Box<dyn FnMut(_)>);

        let onerror = Closure::wrap(Box::new(move |_e: web_sys::ErrorEvent| {
            error!("Worker error occurred");
            error_msg.set(Some(
                "Worker error occurred. Check console for details.".to_string(),
            ));
            in_progress.set(false);
        }) as Box<dyn FnMut(_)>);

        // Set handlers on worker
        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        // Store state (replaces old closures, which get dropped)
        *state_opt = Some(WorkerState {
            worker: worker.clone(),
            _onmessage: onmessage,
            _onerror: onerror,
        });

        // Send message to worker
        let js_req = swb::to_value(&req)
            .map_err(|e| JsValue::from_str(&format!("Serialize request failed: {e}")))?;
        worker.post_message(&js_req)
    })
}

/// Builds SimulationInput from the current AppState
fn build_simulation_input(state: &AppState) -> SimulationInput {
    // Get gear names from the switches
    let melee_weapon = state.melee_switch.gear.weapon.name.clone();
    let ranged_weapon = state.ranged_switch.gear.weapon.name.clone();
    let magic_weapon = state.magic_switch.gear.weapon.name.clone();

    // Get armor names
    let armor_body = state
        .melee_switch
        .gear
        .body
        .as_ref()
        .map(|a| a.name.clone());
    let armor_head = state
        .melee_switch
        .gear
        .head
        .as_ref()
        .map(|a| a.name.clone());
    let armor_legs = state
        .melee_switch
        .gear
        .legs
        .as_ref()
        .map(|a| a.name.clone());

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

    let mut config = state.sim_config.clone();
    config.attack_strategy = attack_strategy;

    SimulationInput {
        player_stats: state.player.stats,
        melee_weapon,
        melee_style: state.melee_switch.attrs.active_style,
        melee_prayers: collect_active_prayers(&state.melee_switch),
        ranged_weapon,
        ranged_style: state.ranged_switch.attrs.active_style,
        ranged_prayers: collect_active_prayers(&state.ranged_switch),
        magic_weapon,
        magic_style: state.magic_switch.attrs.active_style,
        magic_prayers: collect_active_prayers(&state.magic_switch),
        armor_body,
        armor_head,
        armor_legs,
        sim_config: config,
        num_trials: state.num_trials,
    }
}

#[component]
fn SimulateButton(mut in_progress: Signal<bool>, mut error_msg: Signal<Option<String>>) -> Element {
    let app_state = use_context::<Signal<AppState>>();

    rsx! {
        div { class: "flex justify-center",
            button {
                class: "inline-flex items-center gap-2 px-5 py-2.5 rounded-lg btn-accent disabled:bg-gray-600 disabled:cursor-not-allowed",
                disabled: in_progress(),
                onclick: move |_| {
                    info!("Simulate button clicked");
                    error_msg.set(None);
                    in_progress.set(true);

                    let input = build_simulation_input(&app_state.read());
                    let req = WorkerRequest {

                        id: next_request_id(),
                        input,
                    };
                    if let Err(e) = run_simulation(req, app_state, in_progress, error_msg) {
                        error!("Failed to run simulation: {:?}", e);
                        error_msg.set(Some(format!("Failed to run simulation: {:?}", e)));
                        in_progress.set(false);
                    }
                },

                if in_progress() {
                    // spinner
                    span { class: "inline-block w-4 h-4 rounded-full border-2 border-white/30 border-t-white animate-spin" }
                    "Simulating..."
                } else {
                    "Simulate"
                }
            }
        }
    }
}

fn mean_from_pmf(pmf: &[f64]) -> f64 {
    pmf.iter().enumerate().map(|(i, p)| (i as f64) * p).sum()
}

fn quantile_from_pmf(pmf: &[f64], q: f64) -> usize {
    let mut c = 0.0;
    for (i, p) in pmf.iter().enumerate() {
        c += *p;
        if c >= q {
            return i;
        }
    }
    pmf.len().saturating_sub(1)
}

#[component]
fn StatChip(label: String, value: String) -> Element {
    rsx! {
        div { class: "px-3 py-2 rounded-lg",
            div { class: "text-[11px] text-gray-500 text-left", "{label}" }
            div { class: "text-sm font-medium text-gray-200 num text-left", "{value}" }
        }
    }
}

#[component]
fn SectionTitle(text: String) -> Element {
    rsx! {
        div { class: "card-title", "{text}" }
    }
}

#[component]
pub fn SimulationResults() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let in_progress = use_signal(|| false);
    let error_msg = use_signal(|| None::<String>);

    let mut time_unit = use_signal(|| TimeUnit::Seconds);

    rsx! {
        div { class: "space-y-4",

            // Primary action
            SimulateButton { in_progress, error_msg }

            // Error banner
            if let Some(err) = error_msg() {
                div { class: "px-4 py-3 rounded-xl bg-red-500/10 border border-red-500/30 text-red-200 text-sm",
                    "{err}"
                }
            }

            // Results content
            // Clone results to avoid holding the read borrow across render
            if let Some(results) = (!in_progress())
                .then(|| app_state.read().results.clone())
                .flatten()
            {

                // Summary chips
                {
                    let ttk_pmf = &results.ttk_dist;
                    let food_pmf = &results.food_eaten_dist;

                    let median_ttk_ticks = quantile_from_pmf(ttk_pmf, 0.50) as f64;

                    let (median_ttk, avg_ttk, unit_label) = match time_unit() {
                        TimeUnit::Seconds => (median_ttk_ticks * 0.6, results.ttk, "s"),
                        TimeUnit::Ticks => (median_ttk_ticks, results.ttk / 0.6, "t"),
                    };

                    let mean_food = mean_from_pmf(food_pmf);
                    let median_food = quantile_from_pmf(food_pmf, 0.50);

                    rsx! {
                        div { class: "justify-items-center",
                            div { class: "grid grid-cols-3 lg:grid-cols-6 gap-3 justify-items-between",
                                StatChip {
                                    label: "Avg TTK".to_string(),
                                    value: format!("{:.2}{unit_label}", avg_ttk),
                                }
                                StatChip {
                                    label: "Median TTK".to_string(),
                                    value: format!("{:.0}{unit_label}", median_ttk),
                                }
                                StatChip {
                                    label: "Success Rate".to_string(),
                                    value: format!("{:.2}%", results.success_rate * 100.0),
                                }
                                StatChip {
                                    label: "Avg Damage Taken".to_string(),
                                    value: format!("{:.2}", results.avg_damage_taken),
                                }
                                StatChip { label: "Avg Food".to_string(), value: format!("{:.2}", mean_food) }
                                StatChip {
                                    label: "Median Food".to_string(),
                                    value: format!("{}", median_food),
                                }
                            }
                        }
                    }
                }

                // Plots grid
                div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",

                    // TTK panel
                    div { class: "card p-4 space-y-3",
                        div { class: "flex items-center justify-between gap-3",
                            SectionTitle { text: "Time to Kill".to_string() }

                            // time unit toggle
                            div { class: "flex items-center gap-0 rounded-lg overflow-hidden border border-slate-700",
                                button {
                                    class: if time_unit() == TimeUnit::Seconds { "px-2 py-1 text-xs btn-accent" } else { "px-2 py-1 text-xs bg-slate-800 text-gray-400 hover:bg-slate-700/40" },
                                    onclick: move |_| time_unit.set(TimeUnit::Seconds),
                                    "Seconds"
                                }
                                button {
                                    class: if time_unit() == TimeUnit::Ticks { "px-2 py-1 text-xs btn-accent" } else { "px-2 py-1 text-xs bg-slate-800 text-gray-400 hover:bg-slate-700/40" },
                                    onclick: move |_| time_unit.set(TimeUnit::Ticks),
                                    "Ticks"
                                }
                            }
                        }

                        TtkCdf {
                            distributions: vec![results.ttk_dist.clone()],
                            time_unit,
                            labels: None,
                        }
                    }

                    // Food panel
                    div { class: "card p-4 space-y-3",
                        SectionTitle { text: "Food Eaten".to_string() }
                        FoodHistogram { distribution: results.food_eaten_dist.clone() }
                    }
                }
            }
        }
    }
}
