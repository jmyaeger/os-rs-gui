use components::{EquipmentGrid, EquipmentSelect, PotionSelect, PrayerSelect, SkillsSelect};
use dioxus::prelude::*;
use dioxus_logger::tracing::Level;
use state::AppState;

mod components;
mod state;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

// Asset folders - these must be declared with asset!() to be included in the build
pub const EQUIPMENT_ASSETS: Asset = asset!("/assets/equipment");
pub const POTIONS_ASSETS: Asset = asset!("/assets/potions");
pub const PRAYERS_ASSETS: Asset = asset!("/assets/prayers");
pub const PLACEHOLDERS_ASSETS: Asset = asset!("/assets/placeholders");
pub const DEF_REDUCTIONS_ASSETS: Asset = asset!("/assets/def_reductions");
pub const STYLES_ASSETS: Asset = asset!("/assets/styles");
pub const BONUSES_ASSETS: Asset = asset!("/assets/bonuses");

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
