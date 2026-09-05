use crate::pages::gauntlet::state::{GauntletState, SimulationMode};
use dioxus::prelude::*;

#[component]
pub fn ModeToggle() -> Element {
    let mut mode = use_context::<GauntletState>().simulation_mode;
    let current_mode = mode.cloned();

    let button_base = "px-5 py-2 text-sm font-medium transition-all duration-150";
    let active = "btn-accent";
    let inactive = "bg-slate-900 text-gray-200 hover:bg-slate-600/40";

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
            div { class: "inline-flex rounded-xl overflow-hidden border border-slate-700",
                button {
                    class: "{two_t3_class}",
                    onclick: move |_| {
                        mode.set(SimulationMode::TwoT3);
                    },
                    "Two T3 Weapons"
                }
                button {
                    class: "{five_one_class}",
                    onclick: move |_| {
                        mode.set(SimulationMode::FiveOne);
                    },
                    "5:1"
                }
            }
        }
    }
}
