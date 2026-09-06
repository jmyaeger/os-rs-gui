use dioxus::prelude::*;
use dioxus_logger::tracing::Level;

use crate::routes::Route;
use crate::worker::is_worker_context;

mod components;
mod hiscores;
mod pages;
mod routes;
mod worker;

// Asset folders - these must be declared with asset!() to be included in the build
pub const EQUIPMENT_ASSETS: Asset = asset!("/assets/equipment");
pub const POTIONS_ASSETS: Asset = asset!("/assets/potions");
pub const PRAYERS_ASSETS: Asset = asset!("/assets/prayers");
pub const PLACEHOLDERS_ASSETS: Asset = asset!("/assets/placeholders");
pub const DEF_REDUCTIONS_ASSETS: Asset = asset!("/assets/def_reductions");
pub const STYLES_ASSETS: Asset = asset!("/assets/styles");
pub const BONUSES_ASSETS: Asset = asset!("/assets/bonuses");

fn main() {
    // Don't launch Dioxus if we're in a worker context
    // The worker will call start_simulation_worker() directly
    console_error_panic_hook::set_once();
    if is_worker_context() {
        return;
    }

    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}
