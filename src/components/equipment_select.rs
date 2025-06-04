// src/components/equipment_select.rs
#![allow(non_snake_case)]

use crate::state::AppState;
use dioxus::prelude::*;
use dioxus_logger::tracing;
use osrs::types::equipment::EquipmentJson; // Only import what's directly used
use std::rc::Rc;

const EQUIPMENT_JSON_STRING: &str = include_str!("../../assets/json/equipment.json");
// Make sure this path is correct! If it's wrong, it will be a compile-time error.

// Helper for image asset paths if they are served via /assets/equipment/
fn image_asset_path(image_name: &str) -> String {
    format!("/assets/equipment/{}", image_name)
}

#[component]
pub fn EquipmentSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut search_term = use_signal(String::new);
    let mut show_dropdown = use_signal(|| false);

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

        // Directly access the parsed data from the signal
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
            Rc::new(Vec::new()) // Empty if parsing failed
        }
    });

    let all_items_read_guard = all_items_parsed.read();
    // The main UI structure now depends on whether all_items_parsed is Some or None
    match &*all_items_read_guard {
        Some(_) => {
            // Data successfully parsed (actual data used via filtered_options)
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
                        }
                    }
                    if *show_dropdown.read() && !filtered_options.read().is_empty() {
                        div {
                            class: "absolute z-10 w-full mt-1 bg-gray-700 border border-gray-600 rounded-md shadow-lg max-h-60 overflow-y-auto",
                            ul { class: "py-1",
                                for item_json_from_list in filtered_options.read().iter() {
                                    { // Scope for item_clone
                                        let item_for_closure = item_json_from_list.clone();
                                        rsx! {
                                            li {
                                                key: "{item_for_closure.name}-{item_for_closure.version.as_deref().unwrap_or(\"novariant\")}",
                                                class: "flex items-center gap-2 px-3 py-2 hover:bg-gray-600 cursor-pointer text-sm text-white",
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
            // Parsing failed
            rsx! {
                div { class: "text-red-500 p-2.5",
                    "Error: Could not parse embedded equipment data. Check console for details."
                }
            }
        }
    }
}
