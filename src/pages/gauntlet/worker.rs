#[cfg(not(target_arch = "wasm32"))]
use crate::pages::gauntlet::simulation::run_simulation_from_input;
use crate::pages::gauntlet::simulation::{SimulationInput, SimulationOutput};

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

#[cfg(not(target_arch = "wasm32"))]
pub async fn run_simulation(input: SimulationInput) -> Result<SimulationOutput, String> {
    Ok(run_simulation_from_input(input))
}

#[cfg(target_arch = "wasm32")]
mod web_worker {
    use super::{SimulationInput, SimulationOutput};
    use crate::pages::gauntlet::simulation::run_simulation_from_input;
    use dioxus::prelude::{asset, manganis};
    use futures_channel::oneshot;
    use serde::{Deserialize, Serialize};
    use serde_wasm_bindgen as swb;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

    type PendingSender = oneshot::Sender<Result<SimulationOutput, String>>;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct WorkerRequest {
        id: u32,
        input: SimulationInput,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct WorkerResponse {
        id: u32,
        output: SimulationOutput,
    }

    struct WorkerState {
        worker: Worker,
        pending: HashMap<u32, PendingSender>,
        _onmessage: Closure<dyn FnMut(MessageEvent)>,
        _onerror: Closure<dyn FnMut(web_sys::ErrorEvent)>,
    }

    thread_local! {
        static WORKER_STATE: RefCell<Option<WorkerState>> = const { RefCell::new(None) };
        static NEXT_REQUEST_ID: RefCell<u32> = const { RefCell::new(1) };
    }

    fn next_request_id() -> u32 {
        NEXT_REQUEST_ID.with(|cell| {
            let mut id = cell.borrow_mut();
            let out = *id;
            *id = id.wrapping_add(1);
            if *id == 0 {
                *id = 1;
            }
            out
        })
    }

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

    fn find_js_bundle_url() -> Result<String, String> {
        let document = web_sys::window()
            .ok_or_else(|| "no window".to_string())?
            .document()
            .ok_or_else(|| "no document".to_string())?;

        let selector = r#"script[type="module"][src*="os-rs-gui-"][src$=".js"]"#;
        let js_url = document
            .query_selector(selector)
            .map_err(|e| format!("failed to query module script: {e:?}"))?
            .and_then(|el| el.get_attribute("src"))
            .map(normalize_url)
            .unwrap_or_else(|| "/wasm/os-rs-gui.js".to_string());

        Ok(js_url)
    }

    fn create_worker() -> Result<Worker, String> {
        let opts = WorkerOptions::new();
        opts.set_type(WorkerType::Module);

        let worker_url = asset!("/assets/worker.js").to_string();
        Worker::new_with_options(&worker_url, &opts)
            .map_err(|e| format!("failed to create simulation worker: {e:?}"))
    }

    fn send_init_message(worker: &Worker) -> Result<(), String> {
        let js_url = find_js_bundle_url()?;
        let init_msg = js_sys::Object::new();
        js_sys::Reflect::set(&init_msg, &"type".into(), &"init".into())
            .map_err(|e| format!("failed to build worker init message: {e:?}"))?;
        js_sys::Reflect::set(&init_msg, &"js_url".into(), &js_url.into())
            .map_err(|e| format!("failed to set worker JS URL: {e:?}"))?;

        worker
            .post_message(&init_msg)
            .map_err(|e| format!("failed to initialize simulation worker: {e:?}"))
    }

    fn fail_all_pending(message: String, reset_worker: bool) {
        WORKER_STATE.with(|state_cell| {
            let mut state_opt = state_cell.borrow_mut();
            if reset_worker {
                if let Some(state) = state_opt.take() {
                    state.worker.terminate();
                    for sender in state.pending.into_values() {
                        let _ = sender.send(Err(message.clone()));
                    }
                }
            } else if let Some(state) = state_opt.as_mut() {
                for sender in std::mem::take(&mut state.pending).into_values() {
                    let _ = sender.send(Err(message.clone()));
                }
            }
        });
    }

    fn ensure_worker() -> Result<(), String> {
        let already_ready = WORKER_STATE.with(|state_cell| state_cell.borrow().is_some());
        if already_ready {
            return Ok(());
        }

        let worker = create_worker()?;

        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            let response = swb::from_value::<WorkerResponse>(event.data())
                .map_err(|e| format!("failed to decode worker response: {e}"));

            match response {
                Ok(resp) => {
                    WORKER_STATE.with(|state_cell| {
                        let mut state_opt = state_cell.borrow_mut();
                        let Some(state) = state_opt.as_mut() else {
                            return;
                        };

                        if let Some(sender) = state.pending.remove(&resp.id) {
                            let _ = sender.send(Ok(resp.output));
                        }
                    });
                }
                Err(message) => {
                    fail_all_pending(message, true);
                }
            }
        }) as Box<dyn FnMut(_)>);

        let onerror = Closure::wrap(Box::new(move |event: web_sys::ErrorEvent| {
            let message = if event.message().is_empty() {
                "worker error occurred".to_string()
            } else {
                format!("worker error occurred: {}", event.message())
            };
            fail_all_pending(message, true);
        }) as Box<dyn FnMut(_)>);

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        WORKER_STATE.with(|state_cell| {
            *state_cell.borrow_mut() = Some(WorkerState {
                worker: worker.clone(),
                pending: HashMap::new(),
                _onmessage: onmessage,
                _onerror: onerror,
            });
        });

        if let Err(err) = send_init_message(&worker) {
            fail_all_pending(err.clone(), true);
            return Err(err);
        }

        Ok(())
    }

    pub async fn run_simulation(input: SimulationInput) -> Result<SimulationOutput, String> {
        ensure_worker()?;

        let id = next_request_id();
        let request = WorkerRequest { id, input };
        let js_request =
            swb::to_value(&request).map_err(|e| format!("failed to encode worker request: {e}"))?;
        let (sender, receiver) = oneshot::channel();

        WORKER_STATE.with(|state_cell| {
            let mut state_opt = state_cell.borrow_mut();
            let state = state_opt
                .as_mut()
                .ok_or_else(|| "simulation worker is not available".to_string())?;

            state.pending.insert(id, sender);
            if let Err(e) = state.worker.post_message(&js_request) {
                state.pending.remove(&id);
                return Err(format!("failed to send worker request: {e:?}"));
            }

            Ok::<(), String>(())
        })?;

        receiver
            .await
            .map_err(|_| "simulation worker stopped before responding".to_string())?
    }

    #[wasm_bindgen]
    pub fn start_simulation_worker() {
        use web_sys::DedicatedWorkerGlobalScope;

        let global = js_sys::global();
        let scope = DedicatedWorkerGlobalScope::unchecked_from_js(global.into());

        let scope_clone = scope.clone();
        let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let data = event.data();
            let req: WorkerRequest = match swb::from_value(data.clone()) {
                Ok(v) => v,
                Err(e) => {
                    let id = js_sys::Reflect::get(&data, &"id".into())
                        .ok()
                        .and_then(|value| value.as_f64())
                        .map(|id| id as u32)
                        .unwrap_or_default();
                    let resp = WorkerResponse {
                        id,
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

            let output = run_simulation_from_input(req.input);
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
        onmessage.forget();
    }
}

#[cfg(target_arch = "wasm32")]
pub use web_worker::run_simulation;
