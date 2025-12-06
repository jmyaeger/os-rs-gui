use crate::components::search_bar::SearchBar;
use crate::state::AppState;
use dioxus::prelude::*;
use osrs::types::equipment::EquipmentJson;

const EQUIPMENT_JSON_STRING: &str = include_str!("../../assets/json/equipment.json");

fn filter_equipment(item: &EquipmentJson, term: &str) -> bool {
    item.name.to_lowercase().contains(term)
        || item
            .version
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(term)
}

fn render_equipment_item(item: &EquipmentJson) -> Element {
    let image_path = format!("/assets/equipment/{}", item.image);
    rsx! {
        div { class: "flex items-center h-10 gap-3 px-4 py-3 text-sm",
            div { class: "flex-shrink-0 h-8 w-8 flex justify-center items-center p-1",
                img {
                    class: "max-h-full max-w-full object-contain",
                    src: "{image_path}",
                    alt: "{item.name}"
                }
            }
            div { class: "flex-grow",
                div { class: "font-small",
                    "{item.name}"
                }
                if let Some(version) = &item.version {
                    div { class: "text-xs text-subtle",
                        "Version: {version}"
                    }
                }
            }
        }
    }
}

fn get_equipment_key(item: &EquipmentJson) -> String {
    format!(
        "{}-{}",
        item.name,
        item.version.as_deref().unwrap_or("novariant")
    )
}

#[component]
pub fn EquipmentSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();

    // Load and parse equipment data once
    let items = use_signal(|| {
        match serde_json::from_str::<Vec<EquipmentJson>>(EQUIPMENT_JSON_STRING) {
            Ok(items) => {
                let valid_items: Vec<EquipmentJson> = items
                    .into_iter()
                    .filter(|item| {
                        item.name != "Unarmed"
                            && (item.slot != "Weapon"
                                || (item.category.is_some()
                                    && item.speed.is_some()
                                    && item.attack_range.is_some()
                                    && item.is_two_handed.is_some()))
                    })
                    .collect();
                Some(valid_items)
            }
            Err(_) => None,
        }
    });

    let items_read = items.read();
    match &*items_read {
        Some(equipment_list) => {
            rsx! {
                SearchBar {
                    items: equipment_list.clone(),
                    filter_fn: filter_equipment,
                    render_item: render_equipment_item,
                    get_key: get_equipment_key,
                    on_select: move |item: EquipmentJson| {
                        let mut state = app_state.write();
                        let result = if item.slot.eq_ignore_ascii_case("weapon") {
                            item.clone()
                                .into_weapon()
                                .map_err(|_| format!("Failed to convert '{}' to weapon", item.name))
                                .and_then(|weapon| {
                                    state
                                        .player
                                        .equip_item(Box::new(weapon))
                                        .map_err(|e| format!("Failed to equip weapon: {e}"))
                                })
                        } else {
                            item.clone()
                                .into_armor()
                                .map_err(|_| format!("Failed to convert '{}' to armor", item.name))
                                .and_then(|armor| {
                                    state
                                        .player
                                        .equip_item(Box::new(armor))
                                        .map_err(|e| format!("Failed to equip armor: {e}"))
                                })
                        };

                        if let Err(e) = result {
                            log::error!("{e}");
                        }
                    },
                    placeholder: "Search for equipment...".to_string(),
                }
            }
        }
        None => {
            rsx! {
                div { class: "panel p-4 text-error",
                    "Error: Could not parse embedded equipment data. Check console for details."
                }
            }
        }
    }
}
