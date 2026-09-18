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

    /// Everything the worker can send back. `Ready` and `InitFailed` come from the
    /// bootstrap in `assets/worker.js`; the rest come from `start_simulation_worker`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum Outbound {
        Ready,
        InitFailed { error: String },
        Progress { id: u32, value: f64 },
        Finished { id: u32, output: String },
        Failed { id: u32, error: String },
    }

    struct Pending {
        sender: oneshot::Sender<Result<JobOutput, String>>,
        on_progress: Box<dyn Fn(f64)>,
    }

    /// Jobs submitted before the bootstrap reports `Ready` wait here; the worker
    /// has no message handler installed until then.
    enum Readiness {
        Waiting(Vec<oneshot::Sender<Result<(), String>>>),
        Ready,
    }

    struct WorkerState {
        worker: Worker,
        readiness: Readiness,
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

    /// Locate the app's wasm-bindgen glue module. `dx` emits it as the document's
    /// module script, unhashed at `/wasm/os-rs-gui.js` in dev and hashed under
    /// `/assets/` in release, so the selector matches on the stem alone.
    fn find_js_bundle_url() -> Result<String, String> {
        let document = web_sys::window()
            .ok_or_else(|| "no window".to_string())?
            .document()
            .ok_or_else(|| "no document".to_string())?;

        let selector = r#"script[type="module"][src*="os-rs-gui"][src$=".js"]"#;
        document
            .query_selector(selector)
            .map_err(|e| format!("failed to query module script: {e:?}"))?
            .and_then(|el| el.get_attribute("src"))
            .map(normalize_url)
            .ok_or_else(|| {
                "could not find the app's module script; the simulation worker \
                 cannot load the engine"
                    .to_string()
            })
    }

    /// Tear the worker down and fail everyone waiting on it. The next job builds a
    /// fresh instance, so a failed start is recoverable.
    fn teardown(message: String) {
        // Take the state out before touching anything that could re-enter.
        let Some(state) = WORKER_STATE.with(|cell| cell.borrow_mut().take()) else {
            return;
        };
        state.worker.terminate();
        if let Readiness::Waiting(waiters) = state.readiness {
            for waiter in waiters {
                let _ = waiter.send(Err(message.clone()));
            }
        }
        for pending in state.pending.into_values() {
            let _ = pending.sender.send(Err(message.clone()));
        }
    }

    fn handle_outbound(message: Outbound) {
        match message {
            Outbound::Ready => {
                let waiters = WORKER_STATE.with(|cell| {
                    let mut state = cell.borrow_mut();
                    let state = state.as_mut()?;
                    match std::mem::replace(&mut state.readiness, Readiness::Ready) {
                        Readiness::Waiting(waiters) => Some(waiters),
                        Readiness::Ready => None,
                    }
                });
                for waiter in waiters.unwrap_or_default() {
                    let _ = waiter.send(Ok(()));
                }
            }
            Outbound::InitFailed { error } => {
                teardown(format!("simulation worker failed to start: {error}"));
            }
            Outbound::Progress { id, value } => {
                WORKER_STATE.with(|cell| {
                    let state = cell.borrow();
                    if let Some(pending) = state.as_ref().and_then(|s| s.pending.get(&id)) {
                        (pending.on_progress)(value);
                    }
                });
            }
            Outbound::Finished { id, output } => {
                if let Some(pending) = take_pending(id) {
                    let result = serde_json::from_str::<JobOutput>(&output)
                        .map_err(|e| format!("failed to decode worker output: {e}"));
                    let _ = pending.sender.send(result);
                }
            }
            Outbound::Failed { id, error } => {
                if let Some(pending) = take_pending(id) {
                    let _ = pending.sender.send(Err(error));
                }
            }
        }
    }

    fn take_pending(id: u32) -> Option<Pending> {
        WORKER_STATE.with(|cell| {
            cell.borrow_mut()
                .as_mut()
                .and_then(|state| state.pending.remove(&id))
        })
    }

    /// Create the worker and ask it to load the engine. Its bootstrap answers with
    /// `Ready` or `InitFailed`.
    fn start_worker() -> Result<(), String> {
        let js_url = find_js_bundle_url()?;

        let opts = WorkerOptions::new();
        opts.set_type(WorkerType::Module);
        let worker_url = asset!("/assets/worker.js").to_string();
        let worker = Worker::new_with_options(&worker_url, &opts)
            .map_err(|e| format!("failed to create simulation worker: {e:?}"))?;

        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            match swb::from_value::<Outbound>(event.data()) {
                Ok(message) => handle_outbound(message),
                Err(e) => teardown(format!("failed to decode worker response: {e}")),
            }
        }) as Box<dyn FnMut(_)>);

        let onerror = Closure::wrap(Box::new(move |event: web_sys::ErrorEvent| {
            let message = if event.message().is_empty() {
                "worker error occurred".to_string()
            } else {
                format!("worker error occurred: {}", event.message())
            };
            teardown(message);
        }) as Box<dyn FnMut(_)>);

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        WORKER_STATE.with(|cell| {
            *cell.borrow_mut() = Some(WorkerState {
                worker: worker.clone(),
                readiness: Readiness::Waiting(Vec::new()),
                pending: HashMap::new(),
                _onmessage: onmessage,
                _onerror: onerror,
            });
        });

        let init = js_sys::Object::new();
        let set = |key: &str, value: &JsValue| {
            js_sys::Reflect::set(&init, &key.into(), value)
                .map(|_| ())
                .map_err(|e| format!("failed to build worker init message: {e:?}"))
        };
        let result = set("type", &"init".into())
            .and_then(|()| set("js_url", &js_url.into()))
            .and_then(|()| {
                worker
                    .post_message(&init)
                    .map_err(|e| format!("failed to initialize simulation worker: {e:?}"))
            });

        if let Err(error) = result {
            teardown(error.clone());
            return Err(error);
        }
        Ok(())
    }

    /// Resolve once the worker is able to serve jobs, starting one if needed.
    async fn ensure_ready() -> Result<(), String> {
        if WORKER_STATE.with(|cell| cell.borrow().is_none()) {
            start_worker()?;
        }

        let waiter = WORKER_STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            let state = state
                .as_mut()
                .ok_or_else(|| "simulation worker is not available".to_string())?;
            Ok::<_, String>(match &mut state.readiness {
                Readiness::Ready => None,
                Readiness::Waiting(waiters) => {
                    let (sender, receiver) = oneshot::channel();
                    waiters.push(sender);
                    Some(receiver)
                }
            })
        })?;

        match waiter {
            None => Ok(()),
            Some(receiver) => receiver
                .await
                .map_err(|_| "simulation worker stopped before it was ready".to_string())?,
        }
    }

    pub async fn run_job(
        job: Job,
        on_progress: impl Fn(f64) + 'static,
    ) -> Result<JobOutput, String> {
        ensure_ready().await?;

        let id = next_request_id();
        let job = serde_json::to_string(&job).map_err(|e| format!("failed to encode job: {e}"))?;
        let request = WorkerRequest { id, job };
        let js_request =
            swb::to_value(&request).map_err(|e| format!("failed to encode worker request: {e}"))?;
        let (sender, receiver) = oneshot::channel();

        WORKER_STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            let state = state
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
        teardown(CANCELLED.to_string());
    }

    #[wasm_bindgen]
    pub fn start_simulation_worker() {
        use web_sys::DedicatedWorkerGlobalScope;

        let global = js_sys::global();
        let scope = DedicatedWorkerGlobalScope::unchecked_from_js(global.into());

        let post = move |scope: &DedicatedWorkerGlobalScope, message: Outbound| {
            match swb::to_value(&message) {
                Ok(value) => {
                    let _ = scope.post_message(&value);
                }
                Err(e) => {
                    let id = match message {
                        Outbound::Progress { id, .. }
                        | Outbound::Finished { id, .. }
                        | Outbound::Failed { id, .. } => id,
                        _ => 0,
                    };
                    let fallback = Outbound::Failed {
                        id,
                        error: format!("Failed to encode response: {e}"),
                    };
                    if let Ok(value) = swb::to_value(&fallback) {
                        let _ = scope.post_message(&value);
                    }
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
                        Outbound::Failed {
                            id,
                            error: format!("Failed to decode request: {e}"),
                        },
                    );
                    return;
                }
            };

            let id = request.id;
            let progress_scope = scope_clone.clone();
            let mut report = |value: f64| {
                post(&progress_scope, Outbound::Progress { id, value });
            };
            let outcome = serde_json::from_str::<Job>(&request.job)
                .map_err(|e| format!("Failed to decode job: {e}"))
                .and_then(|job| run_job_blocking(job, &mut report))
                .and_then(|output| {
                    serde_json::to_string(&output)
                        .map_err(|e| format!("Failed to encode output: {e}"))
                });
            let message = match outcome {
                Ok(output) => Outbound::Finished { id, output },
                Err(error) => Outbound::Failed { id, error },
            };
            post(&scope_clone, message);
        }) as Box<dyn Fn(web_sys::MessageEvent)>);

        scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
    }
}

#[cfg(target_arch = "wasm32")]
pub use web_worker::{cancel_jobs, run_job};
