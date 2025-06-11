use crate::state::AppState;
use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;
use osrs::types::potions::Potion;
use std::rc::Rc;
use strum::IntoEnumIterator;

const MAX_ACTIVE_POTIONS: usize = 4;

fn generate_list_item_id(index: usize) -> String {
    format!("potion-select-item-{}", index)
}

fn scroll_element_into_view(element_id: &str) {
    let js_code = format!(
        r#"
        const element = document.getElementById('{}');
        if (element) {{
            element.scrollIntoView({{ block: 'nearest', inline: 'nearest' }});
        }}
        "#,
        element_id
    );
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen(inline_js = r#"export function exec_js(code) {{ eval(code); }}"#)]
        extern "C" {
            fn exec_js(code: &str);
        }
        exec_js(&js_code);
    }
}

#[component]
pub fn PotionSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut search_term = use_signal(String::new);
    let mut show_dropdown = use_signal(|| false);
    let mut highlighted_index = use_signal(|| None);
    let mut is_collapsed = use_signal(|| false);

    // Get all available potions
    let all_potions: Vec<Potion> = Potion::iter().filter(|p| *p != Potion::None).collect();

    let mut active_potions = use_signal(Vec::<Potion>::new);

    let filtered_potions = use_memo(move || {
        let term = search_term.read().to_lowercase();
        let active = active_potions.read();

        if term.is_empty() {
            return Rc::new(Vec::<Potion>::new());
        }

        let filtered: Vec<Potion> = all_potions
            .iter()
            .filter(|potion| {
                !active.contains(potion) && potion.to_string().to_lowercase().contains(&term)
            })
            .take(10)
            .cloned()
            .collect();

        Rc::new(filtered)
    });

    use_effect(move || {
        let current_filtered = filtered_potions.read();
        let should_show = *show_dropdown.read();
        let mut current_highlight_val = highlighted_index.write();

        if should_show && !current_filtered.is_empty() {
            if current_highlight_val.map_or(true, |idx| idx >= current_filtered.len()) {
                *current_highlight_val = Some(0);
            }
        } else if current_highlight_val.is_some() {
            *current_highlight_val = None;
        }
    });

    let mut add_potion = move |potion: Potion| {
        let current_len = active_potions.read().len();
        if current_len < MAX_ACTIVE_POTIONS {
            let mut state = app_state.write();
            state.player.add_potion(potion);
            drop(state); // Release the app_state borrow before updating active_potions
            active_potions.write().push(potion);
        }
        search_term.set(String::new());
        show_dropdown.set(false);
        highlighted_index.set(None);
    };

    let mut remove_potion = move |potion: Potion| {
        let mut state = app_state.write();
        state.player.remove_potion(potion);
        drop(state); // Release the app_state borrow before updating active_potions
        active_potions.write().retain(|&p| p != potion);
    };

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
                                on_remove: move |potion| remove_potion(potion)
                            }
                        }
                        // Empty slots
                        for idx in active_potions.read().len()..MAX_ACTIVE_POTIONS {
                            EmptyPotionSlot { key: "empty-potion-{idx}" }
                        }
                    }

                    // Search input
                    div { class: "relative w-full max-w-md mx-auto",
                        input {
                            "type": "text",
                            id: "potion-select",
                            class: "input w-full h-10",
                            placeholder: "Search for boosts...",
                            value: "{search_term}",
                            autocomplete: "off",
                            disabled: active_potions.read().len() >= MAX_ACTIVE_POTIONS,
                            oninput: move |evt| {
                                let new_value = evt.value();
                                search_term.set(new_value.clone());
                                if !new_value.is_empty() {
                                    show_dropdown.set(true);
                                } else {
                                    show_dropdown.set(false);
                                    highlighted_index.set(None);
                                }
                            },
                            onfocusin: move |_| {
                                if !search_term.read().is_empty() && !filtered_potions.read().is_empty() {
                                    show_dropdown.set(true);
                                }
                            },
                            onblur: move |_| {
                                let mut show_dropdown_signal = show_dropdown;
                                spawn(async move {
                                    gloo_timers::future::TimeoutFuture::new(150).await;
                                    show_dropdown_signal.set(false);
                                });
                            },
                            onkeydown: move |evt| {
                                match evt.key() {
                                    Key::ArrowDown => {
                                        let current_filtered = filtered_potions.read();
                                        if current_filtered.is_empty() {
                                            return;
                                        }
                                        evt.prevent_default();
                                        let current_idx = *highlighted_index.read();
                                        let next_idx = match current_idx {
                                            Some(idx) => (idx + 1) % current_filtered.len(),
                                            None => 0,
                                        };
                                        drop(current_filtered); // Release borrow
                                        highlighted_index.set(Some(next_idx));
                                        scroll_element_into_view(&generate_list_item_id(next_idx));
                                    }
                                    Key::ArrowUp => {
                                        let current_filtered = filtered_potions.read();
                                        if current_filtered.is_empty() {
                                            return;
                                        }
                                        evt.prevent_default();
                                        let current_idx = *highlighted_index.read();
                                        let next_idx = match current_idx {
                                            Some(0) => current_filtered.len() - 1,
                                            Some(idx) => idx - 1,
                                            None => current_filtered.len() - 1,
                                        };
                                        drop(current_filtered); // Release borrow
                                        highlighted_index.set(Some(next_idx));
                                        scroll_element_into_view(&generate_list_item_id(next_idx));
                                    }
                                    Key::Enter => {
                                        evt.prevent_default();
                                        let current_idx = *highlighted_index.read();
                                        if let Some(idx) = current_idx {
                                            let current_filtered = filtered_potions.read();
                                            if let Some(selected_potion) = current_filtered.get(idx) {
                                                let potion_to_add = *selected_potion;
                                                drop(current_filtered); // Release borrow
                                                add_potion(potion_to_add);
                                            }
                                        }
                                    }
                                    Key::Escape => {
                                        show_dropdown.set(false);
                                        highlighted_index.set(None);
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Dropdown
                        if *show_dropdown.read() && !filtered_potions.read().is_empty() {
                            div {
                                class: "absolute z-10 w-full mt-2 panel border-2 shadow-lg max-h-48 overflow-y-auto",
                                ul { class: "py-2",
                                    for (idx, potion) in filtered_potions.read().iter().enumerate() {
                                        {
                                            let potion_clone = *potion;
                                            let is_highlighted = *highlighted_index.read() == Some(idx);
                                            let item_id = generate_list_item_id(idx);
                                            let highlight_class = if is_highlighted {
                                                "panel-elevated"
                                            } else {
                                                "hover:panel-elevated transition-all duration-100"
                                            };

                                            rsx! {
                                                li {
                                                    id: "{item_id}",
                                                    key: "{potion}",
                                                    class: "flex items-center gap-3 px-4 py-3 cursor-pointer text-sm mx-2 mb-1 rounded-md h-10 {highlight_class}",
                                                    style: "outline: none !important; box-shadow: none !important; transition: background-color 0.05s ease, box-shadow 0.05s ease !important;",
                                                    tabindex: "-1",
                                                    onfocus: move |_| {},
                                                    onmousedown: move |_| {
                                                        add_potion(potion_clone);
                                                    },
                                                    onmouseenter: move |_| {
                                                        highlighted_index.set(Some(idx));
                                                    },
                                                    div { class: "flex-shrink-0 h-[24px] w-[24px] flex justify-center items-center",
                                                        img {
                                                            class: "max-h-full max-w-full object-contain",
                                                            src: "{get_potion_img_path(*potion)}",
                                                            alt: "{potion}"
                                                        }
                                                    }
                                                    div { class: "flex-grow font-medium",
                                                        "{potion}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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

fn get_potion_img_path(potion: Potion) -> String {
    let potion_name = potion.to_string().replace(" (-)", "").replace(" (+)", "");
    format!("/assets/potions/{}.png", potion_name)
}
