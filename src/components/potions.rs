use crate::components::search_bar::SearchBar;
use crate::state::AppState;
use dioxus::prelude::*;
use osrs::types::potions::Potion;
use strum::IntoEnumIterator;

const MAX_ACTIVE_POTIONS: usize = 4;

fn get_active_potions_from_state(app_state: &AppState) -> Vec<Potion> {
    let mut active_potions = Vec::new();
    let potions = &app_state.player.potions;

    if let Some(attack_potions) = &potions.attack {
        for boost in attack_potions {
            if !active_potions.contains(&boost.potion_type) {
                active_potions.push(boost.potion_type);
            }
        }
    }
    if let Some(strength_potions) = &potions.strength {
        for boost in strength_potions {
            if !active_potions.contains(&boost.potion_type) {
                active_potions.push(boost.potion_type);
            }
        }
    }
    if let Some(defence_potions) = &potions.defence {
        for boost in defence_potions {
            if !active_potions.contains(&boost.potion_type) {
                active_potions.push(boost.potion_type);
            }
        }
    }
    if let Some(ranged_potions) = &potions.ranged {
        for boost in ranged_potions {
            if !active_potions.contains(&boost.potion_type) {
                active_potions.push(boost.potion_type);
            }
        }
    }
    if let Some(magic_potions) = &potions.magic {
        for boost in magic_potions {
            if !active_potions.contains(&boost.potion_type) {
                active_potions.push(boost.potion_type);
            }
        }
    }

    active_potions
}

fn filter_potion(potion: &Potion, term: &str) -> bool {
    potion.to_string().to_lowercase().contains(term)
}

fn render_potion_item(potion: &Potion) -> Element {
    let img_path = get_potion_img_path(*potion);
    rsx! {
        div { class: "flex items-center gap-3 px-4 py-3 text-sm h-10",
            div { class: "flex-shrink-0 h-[24px] w-[24px] flex justify-center items-center",
                img {
                    class: "max-h-full max-w-full object-contain",
                    src: "{img_path}",
                    alt: "{potion}"
                }
            }
            div { class: "flex-grow font-medium",
                "{potion}"
            }
        }
    }
}

fn get_potion_key(potion: &Potion) -> String {
    potion.to_string()
}

fn get_potion_img_path(potion: Potion) -> String {
    let potion_name = potion.to_string().replace(" (-)", "").replace(" (+)", "");
    format!("/assets/potions/{potion_name}.png")
}

#[component]
pub fn PotionSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut is_collapsed = use_signal(|| false);

    // Derive active potions from app_state (single source of truth)
    let active_potions = use_memo(move || {
        let state = app_state.read();
        get_active_potions_from_state(&state)
    });

    // Compute available potions (all potions minus active ones)
    let available_potions: Vec<Potion> = {
        let active = active_potions.read();
        Potion::iter()
            .filter(|p| *p != Potion::None && !active.contains(p))
            .collect()
    };

    let is_at_max = active_potions.read().len() >= MAX_ACTIVE_POTIONS;

    rsx! {
        div {
            // Toggle header
            div {
                class: "flex items-center justify-between cursor-pointer p-2 hover:bg-gray-800 rounded transition-colors",
                onclick: move |_| is_collapsed.set(!is_collapsed()),
                div {
                    class: "flex items-center gap-4",
                    h3 {
                        class: "text-sm font-semibold text-accent w-12",
                        "Boosts"
                    }
                    if !active_potions.read().is_empty() && is_collapsed() {
                        div {
                            class: "flex gap-2",
                            for potion in active_potions.read().iter() {
                                img {
                                    class: "w-5 h-5 object-contain",
                                    src: "{get_potion_img_path(*potion)}",
                                    alt: "{potion}",
                                    title: "{potion}"
                                }
                            }
                        }
                    }
                }
                div {
                    class: "text-xs text-gray-400 transform transition-transform",
                    class: if is_collapsed() { "" } else { "rotate-180" },
                    "▼"
                }
            }

            // Expanded potion interface
            if !is_collapsed() {
                div {
                    class: "mt-2",
                    // Active potion slots
                    div { class: "flex gap-2 justify-center mb-4",
                        for (idx, potion) in active_potions.read().iter().enumerate() {
                            ActivePotionSlot {
                                key: "active-potion-{idx}",
                                potion: *potion,
                                on_remove: move |potion: Potion| {
                                    app_state.write().player.remove_potion(potion);
                                }
                            }
                        }
                        // Empty slots
                        for idx in active_potions.read().len()..MAX_ACTIVE_POTIONS {
                            EmptyPotionSlot { key: "empty-potion-{idx}" }
                        }
                    }

                    // Search input
                    div { class: "max-w-md mx-auto",
                        SearchBar {
                            items: available_potions,
                            filter_fn: filter_potion,
                            render_item: render_potion_item,
                            get_key: get_potion_key,
                            on_select: move |potion: Potion| {
                                if active_potions.read().len() < MAX_ACTIVE_POTIONS {
                                    app_state.write().player.add_potion(potion);
                                }
                            },
                            placeholder: "Search for boosts...".to_string(),
                            max_results: 10,
                            disabled: is_at_max,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ActivePotionSlot(potion: Potion, on_remove: EventHandler<Potion>) -> Element {
    rsx! {
        div {
            class: "relative group",
            div {
                class: "equipment-slot-bg flex justify-center items-center h-[40px] w-[40px] p-1",
                img {
                    class: "max-h-full max-w-full object-contain",
                    src: "{get_potion_img_path(potion)}",
                    alt: "{potion}",
                    title: "{potion}"
                }
            }
            button {
                "type": "button",
                class: "absolute -top-1 -right-1 w-4 h-4 bg-red-600 hover:bg-red-700 rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-150",
                style: "font-size: 10px; line-height: 1; font-family: monospace;",
                onclick: move |_| on_remove.call(potion),
                "×"
            }
        }
    }
}

#[component]
fn EmptyPotionSlot() -> Element {
    rsx! {
        div {
            class: "equipment-slot-bg flex justify-center items-center h-[40px] w-[40px] p-1 opacity-50",
            img {
                class: "max-h-full max-w-full object-contain opacity-50 filter grayscale",
                src: "/assets/potions/Vial.png",
                alt: "Empty potion slot"
            }
        }
    }
}
