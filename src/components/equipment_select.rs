// src/components/equipment_select.rs
#![allow(non_snake_case)]

use crate::state::AppState;
use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;
use osrs::types::equipment::EquipmentJson;
use std::rc::Rc;

const EQUIPMENT_JSON_STRING: &str = include_str!("../../assets/json/equipment.json");

fn image_asset_path(image_name: &str) -> String {
    format!("/assets/equipment/{}", image_name)
}

fn generate_list_item_id(index: usize) -> String {
    format!("equip-select-item-{}", index)
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
    #[cfg(not(target_arch = "wasm32"))]
    {
    }
}

#[component]
pub fn EquipmentSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut search_term = use_signal(String::new);
    let mut show_dropdown = use_signal(|| false);
    let mut highlighted_index = use_signal(|| None);

    let items: Signal<Option<Rc<Vec<EquipmentJson>>>> =
        use_signal(
            || match serde_json::from_str::<Vec<EquipmentJson>>(EQUIPMENT_JSON_STRING) {
                Ok(items) => {
                    let valid_items: Vec<EquipmentJson> = items
                        .into_iter()
                        .filter(|item| {
                            item.name != "Unarmed" && (
                                item.slot != "Weapon" || (
                                    item.category.is_some() &&
                                    item.speed.is_some() &&
                                    item.attack_range.is_some() &&
                                    item.is_two_handed.is_some()
                                )
                            )
                        })
                        .collect();
                    Some(Rc::new(valid_items))
                }
                Err(_) => None,
            },
        );

    let filtered_options = use_memo(move || {
        let term = search_term.read().to_lowercase();
        if term.is_empty() {
            return Rc::new(Vec::<EquipmentJson>::new());
        }

        if let Some(items_rc) = &*items.read() {
            let filtered: Vec<EquipmentJson> = items_rc
                .iter()
                .filter(|opt| {
                    opt.name.to_lowercase().contains(&term)
                        || opt
                            .version
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(&term)
                })
                .take(30)
                .cloned()
                .collect();
            Rc::new(filtered)
        } else {
            Rc::new(Vec::new())
        }
    });

    use_effect(move || {
        let current_filtered = filtered_options.read();
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

    let mut equip_item = move |item: EquipmentJson| {
        let mut state = app_state.write();
        let result = match item.slot.as_str() {
            "weapon" => item.clone().into_weapon()
                .map_err(|_| format!("Failed to convert '{}' to weapon", item.name))
                .and_then(|weapon| state.player.equip_item(Box::new(weapon))
                    .map_err(|e| format!("Failed to equip weapon: {}", e))),
            _ => item.clone().into_armor()
                .map_err(|_| format!("Failed to convert '{}' to armor", item.name))
                .and_then(|armor| state.player.equip_item(Box::new(armor))
                    .map_err(|e| format!("Failed to equip armor: {}", e))),
        };
        
        if let Err(e) = result {
            log::error!("{}", e);
        }
        search_term.set(String::new());
        show_dropdown.set(false);
        highlighted_index.set(None);
    };

    let items_guard = items.read();
    match &*items_guard {
        Some(_) => {
            rsx! {
                div { class: "relative flex-grow",
                    input {
                        "type": "text",
                        id: "equipment-select",
                        class: "w-full p-1 rounded bg-slate-600 border-slate-400 text-white placeholder-gray-400 focus:ring-1 focus:ring-blue-500 focus:border-blue-500",
                        placeholder: "Search for equipment...",
                        value: "{search_term}",
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
                            if !search_term.read().is_empty() && !filtered_options.read().is_empty() {
                                show_dropdown.set(true);
                            }
                        },
                        onblur: move |_| {
                            let mut show_dropdown_signal = show_dropdown;
                            dioxus::prelude::spawn(async move {
                                gloo_timers::future::TimeoutFuture::new(150).await;
                                show_dropdown_signal.set(false);
                            });
                        },
                        onkeydown: move |evt| {
                            let current_filtered = filtered_options.read();
                            if current_filtered.is_empty() && evt.key() != Key::Escape {
                                return;
                            }

                            let mut new_highlighted_idx_val: Option<usize> = None;

                            match evt.key() {
                                Key::ArrowDown => {
                                    evt.prevent_default(); // Prevent page scroll
                                    let current_idx_opt = *highlighted_index.read();
                                    let next_idx = match current_idx_opt {
                                        Some(idx) => (idx + 1) % current_filtered.len(),
                                        None => 0,
                                    };
                                    new_highlighted_idx_val = Some(next_idx);
                                }
                                Key::ArrowUp => {
                                    evt.prevent_default(); // Prevent page scroll
                                    let current_idx_opt = *highlighted_index.read();
                                    let next_idx = match current_idx_opt {
                                        Some(0) => current_filtered.len() - 1,
                                        Some(idx) => idx - 1,
                                        None if !current_filtered.is_empty() => current_filtered.len() - 1,
                                        None => 0,
                                    };
                                    new_highlighted_idx_val = Some(next_idx);
                                }
                                Key::Enter => {
                                    evt.prevent_default();
                                    let idx_to_equip = *highlighted_index.read();
                                    if let Some(idx) = idx_to_equip {
                                        if let Some(selected_item) = current_filtered.get(idx) {
                                        (equip_item)(selected_item.clone());
                                    }
                                    }
                                }
                                Key::Escape => {
                                    show_dropdown.set(false);
                                    new_highlighted_idx_val = None;
                                }
                                _ => {}
                            }
                            if let Some(val_to_set) = new_highlighted_idx_val {
                                highlighted_index.set(Some(val_to_set));
                                let id_to_scroll = generate_list_item_id(val_to_set);
                                scroll_element_into_view(&id_to_scroll);
                            } else if evt.key() == Key::Escape {
                                highlighted_index.set(None);
                            }
                        }
                    }

                    if *show_dropdown.read() && !filtered_options.read().is_empty() {
                        div {
                            class: "absolute z-10 w-full mt-1 bg-gray-700 border border-gray-600 rounded-md shadow-lg max-h-60 overflow-y-auto",
                            ul { class: "py-1",
                                for (idx, item) in filtered_options.read().iter().enumerate() {
                                    {
                                        let item_clone = item.clone();
                                        let is_highlighted = *highlighted_index.read() == Some(idx);
                                        let item_id = generate_list_item_id(idx);
                                        let highlight_class = if is_highlighted { "bg-gray-600" } else { "hover:bg-gray-500" };
                                        rsx! {
                                            li {
                                                id: "{item_id}",
                                                key: "{item_clone.name}-{item_clone.version.as_deref().unwrap_or(\"novariant\")}",
                                                class: "flex items-center gap-2 px-3 py-2 cursor-pointer text-sm text-white {highlight_class}",
                                                onmousedown: move |_| {
                                                    (equip_item)(item_clone.clone());
                                                },
                                                onmouseenter: move |_| {
                                                    highlighted_index.set(Some(idx));
                                                },
                                                div { class: "flex-shrink-0 h-[20px] w-[20px] flex justify-center items-center",
                                                    img {
                                                        class: "max-h-full max-w-full object-contain",
                                                        src: "{image_asset_path(&item.image)}",
                                                        alt: "{item.name}"
                                                    }
                                                }
                                                div {
                                                    "{item.name}"
                                                    if let Some(version) = &item.version {
                                                        " "
                                                        span { class: "text-xs text-gray-400 dark:text-gray-300", "#", "{version}" }
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
        None => {
            rsx! {
                div { class: "text-red-500 p-2.5",
                    "Error: Could not parse embedded equipment data. Check console for details."
                }
            }
        }
    }
}
