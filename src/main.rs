use components::{EquipmentGrid, EquipmentSelect, PotionSelect, PrayerSelect, SkillsSelect};
use dioxus::prelude::*;
use dioxus_logger::tracing::Level;
use state::AppState;

mod components;
mod state;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(AppState::default()));

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div {
            class: "p-6 flex",
            div {
                class: "panel p-4 max-w-4xl",
                h1 {
                    class: "text-xl font-bold mb-4 text-accent text-center",
                    "Loadout"
                }
                div {
                    class: "mt-4 flex flex-col gap-4 w-full max-w-md mx-auto",
                    EquipmentGrid {}
                    EquipmentSelect {}
                    SkillsSelect {}
                    PrayerSelect {}
                    PotionSelect {}
                }
            }
        }
    }
}
