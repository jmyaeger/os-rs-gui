use crate::pages::gauntlet::state::{GauntletState, SimulationMode};
use dioxus::prelude::*;

#[component]
pub fn ModeToggle() -> Element {
    let mut mode = use_context::<GauntletState>().simulation_mode;
    let current_mode = mode.cloned();

    let button_base = "px-5 py-2 text-sm font-medium";
    let active = "is-active";
    let inactive = "";

    let two_t3_class = if current_mode == SimulationMode::TwoT3 {
        format!("{button_base} {active}")
    } else {
        format!("{button_base} {inactive}")
    };

    let five_one_class = if current_mode == SimulationMode::FiveOne {
        format!("{button_base} {active}")
    } else {
        format!("{button_base} {inactive}")
    };

    rsx! {
        div { class: "flex justify-center",
            div { class: "home-segmented", role: "group", aria_label: "Simulation mode",
                button {
                    class: "{two_t3_class}",
                    aria_pressed: current_mode == SimulationMode::TwoT3,
                    onclick: move |_| {
                        mode.set(SimulationMode::TwoT3);
                    },
                    "Two T3 Weapons"
                }
                button {
                    class: "{five_one_class}",
                    aria_pressed: current_mode == SimulationMode::FiveOne,
                    onclick: move |_| {
                        mode.set(SimulationMode::FiveOne);
                    },
                    "5:1"
                }
            }
        }
    }
}
