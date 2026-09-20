use dioxus::prelude::*;

use crate::pages::{gauntlet::Gauntlet, gauntlet::state::GauntletState, home::Home};

const RUNESIM_LOGO: Asset = asset!("/assets/runesim_logo.png");

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[layout(Layout)]
    #[route("/")]
    Home {},
    #[route("/gauntlet")]
    Gauntlet {},
}

#[component]
fn Layout() -> Element {
    // Page state lives here (not in the page components) so it survives
    // navigating between pages.
    use_context_provider(|| Signal::new(crate::pages::home::initial_player()));
    use_context_provider(crate::pages::home::HomeState::new);
    use_context_provider(GauntletState::new);

    let route = use_route::<Route>();
    let page_title = match route {
        Route::Home {} => "DPS Calculator",
        Route::Gauntlet {} => "Corrupted Gauntlet Simulator",
    };

    rsx! {
        div { class: "app-shell",
            a { class: "skip-link", href: "#main-content", "Skip to content" }
            header { class: "app-header app-width",
                div { class: "app-brand",
                    Link { to: Route::Home {},
                        img {
                            src: RUNESIM_LOGO,
                            alt: "RuneSim",
                            class: "app-logo",
                        }
                    }
                    h1 { class: "app-title", "{page_title}" }
                }

                nav { class: "app-nav", aria_label: "Main navigation",
                    Link {
                        to: Route::Home {},
                        aria_current: if route == (Route::Home {}) { "page" } else { "false" },
                        "DPS Calculator"
                    }
                    Link {
                        to: Route::Gauntlet {},
                        aria_current: if route == (Route::Gauntlet {}) { "page" } else { "false" },
                        "Gauntlet"
                    }
                }
            }

            main { id: "main-content", tabindex: "-1", class: "app-width",
                Outlet::<Route> {}
            }
        }
    }
}
