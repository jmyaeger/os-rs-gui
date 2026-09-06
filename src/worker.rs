//! One shared Web Worker that runs simulations off the main thread.
//!
//! Every page submits a `Job`; the worker answers with a `JobOutput`, an error,
//! or interim progress. Off the web target jobs run inline on the caller's thread.

use crate::pages::gauntlet::simulation::{
    SimulationInput, SimulationOutput, run_simulation_from_input,
};
use crate::pages::home::simulation::{SingleWayInput, SingleWayOutput, run_single_way};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Job {
    Gauntlet(SimulationInput),
    SingleWay(SingleWayInput),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobOutput {
    Gauntlet(SimulationOutput),
    SingleWay(SingleWayOutput),
}

/// Run a job on the current thread. `progress` receives values in `0.0..=1.0`.
pub fn run_job_blocking(job: Job, progress: &mut dyn FnMut(f64)) -> Result<JobOutput, String> {
    match job {
        Job::Gauntlet(input) => Ok(JobOutput::Gauntlet(run_simulation_from_input(input))),
        Job::SingleWay(input) => run_single_way(&input, progress).map(JobOutput::SingleWay),
    }
}

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
pub async fn run_job(job: Job, on_progress: impl Fn(f64) + 'static) -> Result<JobOutput, String> {
    run_job_blocking(job, &mut |value| on_progress(value))
}

/// Stop every queued or running job. Pending callers receive `Err("Cancelled")`.
#[cfg(not(target_arch = "wasm32"))]
pub fn cancel_jobs() {}

pub const CANCELLED: &str = "Cancelled";

#[cfg(target_arch = "wasm32")]
mod web_worker {
    use super::{CANCELLED, Job, JobOutput, run_job_blocking};
    use dioxus::prelude::{asset, manganis};
    use futures_channel::oneshot;
    use serde::{Deserialize, Serialize};
    use serde_wasm_bindgen as swb;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

    /// Jobs and outputs cross the worker boundary as JSON text. serde_json keeps
    /// integers and maps exact, where a direct JsValue conversion turns catalog
    /// integers into floats that the engine then rejects.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct WorkerRequest {
        id: u32,
        job: String,
    }

    /// Everything the worker can send back. Exactly one of the optional fields is set.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct WorkerMessage {
        id: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        progress: Option<f64>,
    }

    struct Pending {
        sender: oneshot::Sender<Result<JobOutput, String>>,
        on_progress: Box<dyn Fn(f64)>,
    }

    struct WorkerState {
        worker: Worker,
        pending: HashMap<u32, Pending>,
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
                    for pending in state.pending.into_values() {
                        let _ = pending.sender.send(Err(message.clone()));
                    }
                }
            } else if let Some(state) = state_opt.as_mut() {
                for pending in std::mem::take(&mut state.pending).into_values() {
                    let _ = pending.sender.send(Err(message.clone()));
                }
            }
        });
    }

    fn handle_message(message: WorkerMessage) {
        WORKER_STATE.with(|state_cell| {
            let mut state_opt = state_cell.borrow_mut();
            let Some(state) = state_opt.as_mut() else {
                return;
            };
            if let Some(progress) = message.progress {
                if let Some(pending) = state.pending.get(&message.id) {
                    (pending.on_progress)(progress);
                }
                return;
            }
            if let Some(pending) = state.pending.remove(&message.id) {
                let result = match (message.output, message.error) {
                    (Some(output), _) => serde_json::from_str::<JobOutput>(&output)
                        .map_err(|e| format!("failed to decode worker output: {e}")),
                    (None, Some(error)) => Err(error),
                    (None, None) => Err("worker sent an empty response".to_string()),
                };
                let _ = pending.sender.send(result);
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
            match swb::from_value::<WorkerMessage>(event.data()) {
                Ok(message) => handle_message(message),
                Err(e) => fail_all_pending(format!("failed to decode worker response: {e}"), true),
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

    pub async fn run_job(
        job: Job,
        on_progress: impl Fn(f64) + 'static,
    ) -> Result<JobOutput, String> {
        ensure_worker()?;

        let id = next_request_id();
        let job = serde_json::to_string(&job).map_err(|e| format!("failed to encode job: {e}"))?;
        let request = WorkerRequest { id, job };
        let js_request =
            swb::to_value(&request).map_err(|e| format!("failed to encode worker request: {e}"))?;
        let (sender, receiver) = oneshot::channel();

        WORKER_STATE.with(|state_cell| {
            let mut state_opt = state_cell.borrow_mut();
            let state = state_opt
                .as_mut()
                .ok_or_else(|| "simulation worker is not available".to_string())?;

            state.pending.insert(
                id,
                Pending {
                    sender,
                    on_progress: Box::new(on_progress),
                },
            );
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

    /// Terminate the worker, failing every pending job with `CANCELLED`. The next
    /// job creates a fresh worker.
    pub fn cancel_jobs() {
        fail_all_pending(CANCELLED.to_string(), true);
    }

    #[wasm_bindgen]
    pub fn start_simulation_worker() {
        use web_sys::DedicatedWorkerGlobalScope;

        let global = js_sys::global();
        let scope = DedicatedWorkerGlobalScope::unchecked_from_js(global.into());

        let post =
            move |scope: &DedicatedWorkerGlobalScope, message: WorkerMessage| match swb::to_value(
                &message,
            ) {
                Ok(value) => {
                    let _ = scope.post_message(&value);
                }
                Err(e) => {
                    let fallback = WorkerMessage {
                        id: message.id,
                        error: Some(format!("Failed to encode response: {e}")),
                        ..Default::default()
                    };
                    if let Ok(value) = swb::to_value(&fallback) {
                        let _ = scope.post_message(&value);
                    }
                }
            };

        let scope_clone = scope.clone();
        let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let data = event.data();
            let request: WorkerRequest = match swb::from_value(data.clone()) {
                Ok(v) => v,
                Err(e) => {
                    let id = js_sys::Reflect::get(&data, &"id".into())
                        .ok()
                        .and_then(|value| value.as_f64())
                        .map(|id| id as u32)
                        .unwrap_or_default();
                    post(
                        &scope_clone,
                        WorkerMessage {
                            id,
                            error: Some(format!("Failed to decode request: {e}")),
                            ..Default::default()
                        },
                    );
                    return;
                }
            };

            let id = request.id;
            let progress_scope = scope_clone.clone();
            let mut report = |progress: f64| {
                post(
                    &progress_scope,
                    WorkerMessage {
                        id,
                        progress: Some(progress),
                        ..Default::default()
                    },
                );
            };
            let outcome = serde_json::from_str::<Job>(&request.job)
                .map_err(|e| format!("Failed to decode job: {e}"))
                .and_then(|job| run_job_blocking(job, &mut report))
                .and_then(|output| {
                    serde_json::to_string(&output)
                        .map_err(|e| format!("Failed to encode output: {e}"))
                });
            let message = match outcome {
                Ok(output) => WorkerMessage {
                    id,
                    output: Some(output),
                    ..Default::default()
                },
                Err(error) => WorkerMessage {
                    id,
                    error: Some(error),
                    ..Default::default()
                },
            };
            post(&scope_clone, message);
        }) as Box<dyn Fn(web_sys::MessageEvent)>);

        scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
    }
}

#[cfg(target_arch = "wasm32")]
pub use web_worker::{cancel_jobs, run_job};
