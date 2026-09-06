use crate::components::search_bar::SearchBar;
use dioxus::prelude::*;
use osrs::types::equipment::{CombatStance, CombatStyle, EquipmentJson, Weapon};
use osrs::types::player::Player;
use std::sync::LazyLock;

const EQUIPMENT_JSON_STRING: &str = include_str!("../../assets/json/equipment.json");

static EQUIPMENT_ITEMS: LazyLock<Option<Vec<EquipmentJson>>> = LazyLock::new(|| {
    serde_json::from_str::<Vec<EquipmentJson>>(EQUIPMENT_JSON_STRING)
        .ok()
        .map(|items| {
            items
                .into_iter()
                .filter(|item| {
                    item.name != "Unarmed"
                        && (item.slot != "Weapon"
                            || (item.category.is_some()
                                && item.speed.is_some()
                                && item.attack_range.is_some()
                                && item.is_two_handed.is_some()))
                })
                .collect()
        })
});

/// Every catalog item the search can equip; empty if the catalog failed to parse.
pub fn equipment_catalog() -> &'static [EquipmentJson] {
    EQUIPMENT_ITEMS.as_deref().unwrap_or(&[])
}

/// The style a weapon should default to when the current one does not exist on it.
pub fn preferred_style(weapon: &Weapon) -> Option<CombatStyle> {
    weapon
        .combat_styles
        .iter()
        .min_by_key(|(style, option)| {
            let rank = match option.stance {
                CombatStance::Rapid => 0,
                CombatStance::Aggressive => 1,
                CombatStance::Accurate => 2,
                CombatStance::Controlled => 3,
                CombatStance::Autocast => 4,
                _ => 5,
            };
            (rank, style.to_string())
        })
        .map(|(style, _)| *style)
}

/// Keep the active style valid for the equipped weapon and refresh the cached combat type.
pub fn ensure_style(player: &mut Player) {
    let current = player.attrs.active_style;
    let style = if player.gear.weapon.combat_styles.contains_key(&current) {
        Some(current)
    } else {
        preferred_style(&player.gear.weapon)
    };
    if let Some(style) = style {
        player.set_active_style(style);
    }
}

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
    let image_path = format!("{}/{}", crate::EQUIPMENT_ASSETS, item.image);
    rsx! {
        div { class: "flex items-center h-10 gap-3 px-4 py-3 text-sm",
            div { class: "flex-shrink-0 h-8 w-8 flex justify-center items-center p-1",
                img {
                    class: "max-h-full max-w-full object-contain",
                    src: "{image_path}",
                    alt: "{item.name}",
                }
            }
            div { class: "flex-grow",
                div { class: "font-small", "{item.name}" }
                if let Some(version) = &item.version {
                    div { class: "text-xs text-subtle", "Version: {version}" }
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
    let mut player = use_context::<Signal<Player>>();

    match &*EQUIPMENT_ITEMS {
        Some(equipment_list) => {
            rsx! {
                SearchBar {
                    items: equipment_list.clone(),
                    filter_fn: filter_equipment,
                    render_item: render_equipment_item,
                    get_key: get_equipment_key,
                    on_select: move |item: EquipmentJson| {
                        let mut player = player.write();
                        let result = if item.slot.eq_ignore_ascii_case("weapon") {
                            item.clone()
                                .into_weapon()
                                .map_err(|_| format!("Failed to convert '{}' to weapon", item.name))
                                .and_then(|weapon| {
                                    player
                                        .equip_item(Box::new(weapon))
                                        .map_err(|e| format!("Failed to equip weapon: {e}"))
                                })
                        } else {
                            item.clone()
                                .into_armor()
                                .map_err(|_| format!("Failed to convert '{}' to armor", item.name))
                                .and_then(|armor| {
                                    player
                                        .equip_item(Box::new(armor))
                                        .map_err(|e| format!("Failed to equip armor: {e}"))
                                })
                        };

                        match result {
                            Ok(()) => ensure_style(&mut player),
                            Err(e) => log::error!("{e}"),
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
