use dioxus::prelude::*;
use osrs::types::player::Player;

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
    use_context_provider(|| Signal::new(Player::default()));
    use_context_provider(GauntletState::new);

    let route = use_route::<Route>();
    let page_title = match route {
        Route::Home {} => "DPS Calculator",
        Route::Gauntlet {} => "Corrupted Gauntlet Simulator",
    };

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }

        div { class: "min-h-screen px-4 pb-6 mt-2",
            // Header with logo, title, and navigation
            header { class: "w-full max-w-[380px] lg:max-w-7xl mx-auto mb-4 flex items-center justify-between",
                // Left: Logo + page title
                div { class: "flex items-center gap-3",
                    Link { to: Route::Home {},
                        img {
                            src: RUNESIM_LOGO,
                            alt: "RuneSim",
                            class: "h-6 lg:h-8",
                        }
                    }
                    span { class: "text-sm lg:text-xl text-gray-200 font-light", "{page_title}" }
                }

                // Right: Navigation
                nav { class: "flex items-center gap-4 text-xs lg:text-sm",
                    Link {
                        to: Route::Home {},
                        class: "text-gray-400 hover:text-gray-200 transition-colors",
                        "DPS Calculator"
                    }

                    // Simulations dropdown
                    div { class: "relative group",
                        button { class: "text-gray-400 hover:text-gray-200 transition-colors flex items-center gap-1",
                            "Simulations"
                            svg {
                                class: "w-3 h-3 transition-transform group-hover:rotate-180",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2",
                                view_box: "0 0 24 24",
                                path { d: "M19 9l-7 7-7-7" }
                            }
                        }
                        // Dropdown menu
                        div { class: "absolute right-0 top-full pt-1 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all",
                            div { class: "bg-gray-800 border border-gray-700 rounded-lg py-1 min-w-[140px] shadow-lg",
                                Link {
                                    to: Route::Gauntlet {},
                                    class: "block px-4 py-2 text-gray-400 hover:text-gray-200 hover:bg-gray-700 transition-colors",
                                    "Gauntlet"
                                }
                            }
                        }
                    }
                }
            }

            // Page content
            Outlet::<Route> {}
        }
    }
}
