// src/components/equipment_select.rs
#![allow(non_snake_case)]

use crate::state::AppState;
use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;
use dioxus_logger::tracing;
use osrs::types::equipment::EquipmentJson;
use std::rc::Rc;

const EQUIPMENT_JSON_STRING: &str = include_str!("../../assets/json/equipment.json");

fn image_asset_path(image_name: &str) -> String {
    format!("/assets/equipment/{}", image_name)
}

fn generasi_list_item_id(index: usize) -> Option<String> {
    Some(format!("equip-select-item-{}", index))
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
        log::trace!("Attempted to scroll {} into view", element_id);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        log::trace!(
            "(Skipping scroll for non-wasm) Attempted to scroll {} into view",
            element_id
        );
    }
}

#[component]
pub fn EquipmentSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut search_term = use_signal(String::new);
    let mut show_dropdown = use_signal(|| false);
    let mut highlighted_index: Signal<Option<usize>> = use_signal(|| None);

    let all_items_parsed: Signal<Option<Rc<Vec<EquipmentJson>>>> =
        use_signal(
            || match serde_json::from_str::<Vec<EquipmentJson>>(EQUIPMENT_JSON_STRING) {
                Ok(items) => {
                    let valid_items: Vec<EquipmentJson> = items
                        .into_iter()
                        .filter(|item| {
                            if item.name == "Unarmed" {
                                return false;
                            }
                            if item.slot == "Weapon" {
                                item.category.is_some()
                                    && item.speed.is_some()
                                    && item.attack_range.is_some()
                                    && item.is_two_handed.is_some()
                            } else {
                                item.slot != "Weapon"
                            }
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

        if let Some(items_rc) = &*all_items_parsed.read() {
            let filtered: Vec<EquipmentJson> = items_rc
                .iter()
                .filter(|opt| {
                    opt.name.to_lowercase().contains(&term)
                        || opt
                            .version
                            .as_deref()
                            .unwrap_or("")
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
                log::debug!(
                    "Effect: Setting highlight to first item (0). List size: {}",
                    current_filtered.len()
                );
                *current_highlight_val = Some(0);
            }
        } else if current_highlight_val.is_some() {
            log::debug!(
                "Effect: Clearing highlight. Dropdown shown: {}, List empty: {}",
                should_show,
                current_filtered.is_empty()
            );
            *current_highlight_val = None;
        }
    });

    let mut equip_item_action = move |item_to_equip: EquipmentJson| {
        log::debug!("Equipping item: {}", item_to_equip.name);
        let mut player_manager = app_state.write();
        match item_to_equip.slot.as_str() {
            "weapon" => {
                if let Ok(weapon) = item_to_equip.clone().into_weapon() {
                    if let Err(e) = player_manager.player.equip_item(Box::new(weapon)) {
                        log::error!("Error equipping weapon: {}", e);
                    }
                } else {
                    log::error!(
                        "Failed to convert '{}' to Weapon for equip action",
                        item_to_equip.name
                    );
                }
            }
            _ => {
                if let Ok(armor) = item_to_equip.clone().into_armor() {
                    if let Err(e) = player_manager.player.equip_item(Box::new(armor)) {
                        log::error!("Error equipping armor: {}", e);
                    }
                } else {
                    log::error!(
                        "Failed to convert '{}' to Armor for equip action",
                        item_to_equip.name
                    );
                }
            }
        }
        // Reset UI
        search_term.set("".to_string());
        show_dropdown.set(false);
        highlighted_index.set(None);
    };

    let all_items_read_guard = all_items_parsed.read();
    match &*all_items_read_guard {
        Some(_) => {
            rsx! {
                div { class: "relative flex-grow",
                    input {
                        "type": "text",
                        id: "equipment-select",
                        class: "w-full p-2.5 rounded bg-dark-400 border-dark-400 text-white placeholder-gray-400 focus:ring-1 focus:ring-blue-500 focus:border-blue-500",
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
                                        Some(idx) => if idx == 0 { current_filtered.len() - 1 } else { idx - 1 },
                                        None => if !current_filtered.is_empty() { current_filtered.len() - 1 } else { 0 },
                                    };
                                    new_highlighted_idx_val = Some(next_idx);
                                }
                                Key::Enter => {
                                    evt.prevent_default();
                                    let idx_to_equip = *highlighted_index.read();
                                    if let Some(idx) = idx_to_equip {
                                        if let Some(selected_item) = current_filtered.get(idx) {
                                            (equip_item_action)(selected_item.clone());
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
                                if let Some(id_to_scroll) = generasi_list_item_id(val_to_set) {
                                    scroll_element_into_view(&id_to_scroll);
                                }
                            } else if evt.key() == Key::Escape {
                                highlighted_index.set(None);
                            }
                        }
                    }

                    if *show_dropdown.read() && !filtered_options.read().is_empty() {
                        div {
                            class: "absolute z-10 w-full mt-1 bg-gray-700 border border-gray-600 rounded-md shadow-lg max-h-60 overflow-y-auto",
                            ul { class: "py-1",
                                for (idx, item_json_from_list) in filtered_options.read().iter().enumerate() {
                                    {
                                        let item_for_closure = item_json_from_list.clone();
                                        let is_highlighted = *highlighted_index.read() == Some(idx);
                                        let item_id = generasi_list_item_id(idx).unwrap_or_default();
                                        let highlight_class = if is_highlighted { "bg-gray-600" } else { "hover:bg-gray-500" };
                                        rsx! {
                                            li {
                                                id: "{item_id}",
                                                key: "{item_for_closure.name}-{item_for_closure.version.as_deref().unwrap_or(\"novariant\")}",
                                                class: "flex items-center gap-2 px-3 py-2 cursor-pointer text-sm text-white {highlight_class}",
                                                onmousedown: move |_| {
                                                    tracing::info!("[onmousedown] Fired. Current search: '{}', Current DD: {}", search_term.read(), show_dropdown.read());
                                                    search_term.set("".to_string());
                                                    show_dropdown.set(false);
                                                    tracing::info!("[onmousedown] After set. New search: '{}', New DD: {}", search_term.read(), show_dropdown.read());

                                                    let mut player_manager = app_state.write();
                                                    let item_to_convert = item_for_closure.clone();
                                                    tracing::info!("Item slot name: {}", item_to_convert.slot);
                                                    match item_to_convert.slot.as_str() {
                                                        "weapon" => {
                                                            if let Ok(weapon) = item_to_convert.into_weapon() {
                                                                match player_manager.player.equip_item(Box::new(weapon)) {
                                                                    Ok(_) => {},
                                                                    Err(e) => tracing::error!("Error equipping weapon: {}", e)
                                                                }
                                                                let weapon_slot_after_equip = &player_manager.player.gear.weapon.name;
                                                                tracing::info!("[EquipmentSelect] After equip attempt, weapon slot contains: {:?}", weapon_slot_after_equip);
                                                            }
                                                        },
                                                        _ => {
                                                            if let Ok(armor) = item_to_convert.into_armor() {
                                                                let armor_slot = armor.slot;
                                                                match player_manager.player.equip_item(Box::new(armor)) {
                                                                    Ok(_) => {},
                                                                    Err(e) => tracing::error!("Error equipping armor: {}", e)
                                                                }
                                                                if let Ok(armor) = &player_manager.player.get_slot(&armor_slot) {
                                                                    tracing::info!("[EquipmentSelect] After equip attempt, armor slot contains: {:?}", armor.name());
                                                                } else {
                                                                    tracing::error!("[EquipmentSelect] Armor slot is empty after equip attempt.")
                                                                }

                                                            }
                                                        }
                                                    }
                                                    player_manager.player.update_bonuses();
                                                    player_manager.player.update_set_effects();
                                                },
                                                onmouseenter: move |_| {
                                                    highlighted_index.set(Some(idx));
                                                },
                                                div { class: "flex-shrink-0 h-[20px] w-[20px] flex justify-center items-center",
                                                    img {
                                                        class: "max-h-full max-w-full object-contain",
                                                        src: "{image_asset_path(&item_json_from_list.image)}",
                                                        alt: "{item_json_from_list.name}"
                                                    }
                                                }
                                                div {
                                                    "{item_json_from_list.name}"
                                                    if let Some(version) = &item_json_from_list.version {
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
