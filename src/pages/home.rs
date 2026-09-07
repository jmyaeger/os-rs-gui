mod examples;
mod loadout;
mod metrics;
mod results;
pub mod simulation;
mod spec;
mod state;
mod storage;
mod strategy;
mod target;

pub use state::{HomeState, initial_player};

use dioxus::prelude::*;
use loadout::{BoostsPanel, EquipmentPanel, PlayerPanel};
use metrics::MetricsStrip;
use osrs::types::player::Player;
use results::ResultsPanel;
use simulation::TRIAL_CHOICES;
use state::result_letter;
use strategy::StrategyPanel;
use target::TargetPanel;

#[component]
pub fn Home() -> Element {
    let mut state = use_context::<HomeState>();
    let mut player = use_context::<Signal<Player>>();
    let mut notice = use_signal(String::new);
    let monster = use_memo(move || state.target.read().combat_monster());
    use_effect(move || state.persist_results());
    use_effect(move || state.persist_draft(&player.read()));
    let origin = state.origin(&player.read());
    let warnings = state.warnings.read().clone();

    rsx! {
        document::Stylesheet { href: asset!("/assets/home.css") }
        div { class: "w-full max-w-[380px] lg:max-w-7xl mx-auto home",
            div { class: "card home-toolbar",
                div { class: "home-toolbar-name",
                    span { class: "card-title", "Setup" }
                    input {
                        class: "home-name-input",
                        aria_label: "Setup name",
                        placeholder: "Name this setup",
                        value: "{state.name}",
                        maxlength: "80",
                        oninput: move |event| state.name.set(event.value()),
                    }
                    if let Some((index, unchanged)) = origin {
                        span { class: "home-muted home-toolbar-status",
                            if unchanged {
                                "Same as result {result_letter(index)}"
                            } else {
                                "Edited since result {result_letter(index)}"
                            }
                        }
                    }
                }
                div { class: "home-toolbar-actions",
                    button {
                        class: "home-button",
                        onclick: move |_| {
                            state.reset(&mut player);
                            notice.set(String::new());
                        },
                        "New setup"
                    }
                }
            }
            if !warnings.is_empty() {
                p { class: "home-error", role: "alert", "Not restored: {warnings.join(\", \")}" }
            }

            div { class: "home-editor",
                div { class: "home-editor-loadout",
                    section {
                        class: "card home-panel loadout-card",
                        aria_label: "Loadout",
                        header { class: "home-panel-header",
                            h2 { class: "card-title", "Loadout" }
                        }
                        div { class: "loadout-columns",
                            EquipmentPanel {}
                            PlayerPanel {}
                            BoostsPanel {}
                        }
                        MetricsStrip { monster }
                    }
                }
                div { class: "home-editor-target",
                    TargetPanel { target: state.target, monster }
                }
                div { class: "home-editor-strategy",
                    StrategyPanel { monster }
                }
            }

            div { class: "home-run-bar",
                button {
                    class: "btn-accent home-run-button",
                    onclick: move |_| {
                        let id = state.add_result(&player.peek());
                        state.simulate_entry(id);
                        let index = state
                            .results
                            .peek()
                            .iter()
                            .position(|entry| entry.id == id)
                            .unwrap_or(0);
                        notice.set(format!("Added result {} · simulating", result_letter(index)));
                    },
                    "Add & simulate"
                }
                label { class: "home-run-option",
                    span { "Trials" }
                    select {
                        class: "input-field",
                        aria_label: "Simulation trials",
                        value: "{state.sim.read().trials}",
                        onchange: move |event| {
                            if let Ok(value) = event.value().parse::<u32>() {
                                state.sim.write().trials = value;
                            }
                        },
                        for trials in TRIAL_CHOICES {
                            option {
                                value: "{trials}",
                                selected: trials == state.sim.read().trials,
                                "{trials / 1000}k"
                            }
                        }
                    }
                }
                if !notice.read().is_empty() {
                    span { class: "home-notice", role: "status", "{notice}" }
                }
            }

            ResultsPanel {}
        }
    }
}
