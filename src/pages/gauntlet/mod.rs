use dioxus::prelude::*;

use self::components::{
    armor::ArmorSelect,
    loadout::{LoadoutCard, LoadoutStyle},
    mode_toggle::ModeToggle,
    simulation_options::SimulationOptions,
    skills::SkillSelect,
};
use self::simulate::SimulationResults;
use self::state::{GauntletState, SimulationMode};

pub mod components;
pub mod simulate;
pub mod simulation;
pub mod state;

#[component]
pub fn Gauntlet() -> Element {
    let state = use_context::<GauntletState>();
    let simulation_mode = state.simulation_mode.cloned();
    let loadout_group_key = match simulation_mode {
        SimulationMode::TwoT3 => "two-t3",
        SimulationMode::FiveOne => "five-one",
    };

    rsx! {
        div { class: "gauntlet",
            div { class: "gauntlet-layout",
                // Left column: Stats, Armour, Options
                div { class: "gauntlet-sidebar",
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
                div { class: "gauntlet-workspace space-y-3",
                    // Mode toggle
                    div { class: "mb-4", ModeToggle {} }

                    // Loadouts section
                    div { class: "space-y-3",
                        div {
                            key: "{loadout_group_key}",
                            class: "gauntlet-loadouts",
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
                                button {
                                    class: "home-icon-button",
                                    aria_label: "About result averages",
                                    aria_describedby: "gauntlet-results-help",
                                    "?"
                                }
                                div {
                                    id: "gauntlet-results-help",
                                    role: "tooltip",
                                    class: "absolute bottom-full left-1/2 -translate-x-1/2 mb-1 px-2 py-1 text-xs text-foreground bg-surface2 rounded w-48 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 pointer-events-none transition-opacity z-10 normal-case font-normal",
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
