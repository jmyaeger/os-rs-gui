// The dioxus prelude contains a ton of common items used in dioxus apps. It's a good idea to import wherever you
// need dioxus
use components::{EquipmentGrid, EquipmentSelect};
use dioxus::prelude::*;
use dioxus_logger::tracing::Level;
use state::AppState;

/// Define a components module that contains all shared components for our app.
mod components;
mod state;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    // The `launch` function is the main entry point for a dioxus app. It takes a component and renders it with the platform feature
    // you have enabled
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each component is a function
/// that takes some props and returns an Element. In this case, App takes no props because it is the root of our app.
///
/// Components should be annotated with `#[component]` to support props, better error messages, and autocomplete
#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(AppState::default()));

    let app_state = use_context::<Signal<AppState>>();
    let str_bonus = use_memo(move || app_state.read().player.bonuses.strength.melee);

    // The `rsx!` macro lets us define HTML inside of rust. It expands to an Element with all of our HTML inside.
    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div {
            class: "px-4 flex",
            div {
                div {
                    class: "mt-4",
                    EquipmentGrid {}
                },
                div {
                    class: "mt-4 flex grow gap-0.5",
                    div {
                        class: "basis-full", EquipmentSelect {}
                    }
                }
                div {
                    p { class: "text-white", "Strength bonus: {str_bonus}" }
                }
            }
        }
    }
}
