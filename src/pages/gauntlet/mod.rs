use dioxus::prelude::*;

use self::components::{
    armor::ArmorSelect,
    loadout::{LoadoutCard, LoadoutStyle},
    mode_toggle::ModeToggle,
    simulation_options::SimulationOptions,
    skills::SkillSelect,
};
use self::simulate::SimulationResults;
use self::state::{AppState, SimulationMode};

pub mod components;
pub mod simulate;
pub mod simulation;
pub mod state;
pub mod worker;

#[component]
pub fn Gauntlet() -> Element {
    use_context_provider(|| Signal::new(AppState::default()));
    let app_state = use_context::<Signal<AppState>>();
    let simulation_mode = app_state.read().simulation_mode;
    let loadout_group_key = match simulation_mode {
        SimulationMode::TwoT3 => "two-t3",
        SimulationMode::FiveOne => "five-one",
    };

    rsx! {
        div { class: "w-full max-w-[380px] lg:max-w-7xl mx-auto",
            div { class: "flex flex-col lg:flex-row gap-4",
                // Left column: Stats, Armour, Options
                div { class: "lg:w-72 shrink-0 space-y-4",
                    div { class: "card p-4",
                        div { class: "card-title mb-2", "Stats" }
                        SkillSelect {}
                    }

                    div { class: "card p-4",
                        div { class: "card-title mb-2", "Armour" }
                        ArmorSelect {}
                    }

                    div { class: "card p-4",
                        div { class: "card-title mb-2", "Options" }
                        SimulationOptions {}
                    }
                }

                // Right column: Mode toggle, Loadouts, Results
                div { class: "flex-1 space-y-3",
                    // Mode toggle
                    div { class: "mb-4", ModeToggle {} }

                    // Loadouts section
                    div { class: "space-y-3",
                        div {
                            key: "{loadout_group_key}",
                            class: "flex flex-col lg:flex-row gap-4 justify-center items-center min-h-100",
                            match simulation_mode {
                                SimulationMode::TwoT3 => rsx! {
                                    LoadoutCard { loadout_number: 1 }
                                    LoadoutCard { loadout_number: 2 }
                                },
                                SimulationMode::FiveOne => rsx! {
                                    LoadoutCard { fixed_style: LoadoutStyle::Magic }
                                    LoadoutCard { fixed_style: LoadoutStyle::Ranged }
                                    LoadoutCard { fixed_style: LoadoutStyle::Melee }
                                },
                            }
                        }
                    }

                    // Results section
                    div { class: "space-y-3 m-2",
                        div { class: "section-title flex items-center gap-1.5",
                            "Results"
                            div { class: "relative group",
                                span { class: "text-xs text-gray-400 cursor-help w-4 h-4 rounded-full border border-gray-500 inline-flex items-center justify-center",
                                    "?"
                                }
                                div { class: "absolute bottom-full left-1/2 -translate-x-1/2 mb-1 px-2 py-1 text-xs text-gray-200 bg-gray-800 rounded whitespace-nowrap opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity z-10 normal-case font-normal",
                                    "Average stats are from successful kills only."
                                }
                            }
                        }
                        div { class: "card p-4", SimulationResults {} }
                    }
                }
            }
        }
    }
}
