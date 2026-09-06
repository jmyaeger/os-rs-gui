use crate::components::plots::{FoodHistogram, TimeUnit, TtkCdf};
use crate::pages::gauntlet::simulation::build_simulation_input;
use crate::pages::gauntlet::state::GauntletState;
use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info};

#[component]
fn SimulateButton(mut simulate: Action<(), ()>) -> Element {
    let state = use_context::<GauntletState>();
    let is_pending = simulate.pending();
    let options_valid = state.options_valid.cloned();

    rsx! {
        div { class: "flex justify-center",
            button {
                class: "inline-flex items-center gap-2 px-5 py-2.5 rounded-lg btn-accent disabled:bg-gray-600 disabled:cursor-not-allowed",
                disabled: is_pending || !options_valid,
                title: if !options_valid { "Fix the highlighted options before simulating" },
                onclick: move |_| {
                    info!("Simulate button clicked");
                    simulate.call();
                },

                if is_pending {
                    span { class: "inline-block w-4 h-4 rounded-full border-2 border-white/30 border-t-white animate-spin" }
                    "Simulating..."
                } else {
                    "Simulate"
                }
            }
        }
    }
}

fn simulation_error(simulate: &Action<(), ()>) -> Option<String> {
    simulate.value().and_then(|result| match result {
        Ok(_) => None,
        Err(err) => Some(err.to_string()),
    })
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
    let state = use_context::<GauntletState>();
    let simulate = use_action(move || async move {
        let input = build_simulation_input(&state.snapshot());

        let output = match crate::worker::run_job(crate::worker::Job::Gauntlet(input), |_| {})
            .await
            .map_err(anyhow::Error::msg)?
        {
            crate::worker::JobOutput::Gauntlet(output) => output,
            other => return Err(anyhow::anyhow!("unexpected worker response: {other:?}")),
        };

        if output.success {
            let stats = output
                .stats
                .ok_or_else(|| anyhow::anyhow!("Simulation completed without stats"))?;
            let mut results = state.results;
            results.set(Some(stats));
            info!("Simulation completed successfully");
            Ok(())
        } else {
            let err = output
                .error
                .unwrap_or_else(|| "Unknown simulation error".to_string());
            error!("Simulation failed: {err}");
            Err(anyhow::anyhow!(err))
        }
    });

    let mut time_unit = use_signal(|| TimeUnit::Seconds);
    let error_msg = simulation_error(&simulate);

    rsx! {
        div { class: "space-y-4",

            SimulateButton { simulate }

            if let Some(err) = error_msg {
                div { class: "px-4 py-3 rounded-xl bg-red-500/10 border border-red-500/30 text-red-200 text-sm",
                    "{err}"
                }
            }

            if let Some(results) = state.results.read().clone() {

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
