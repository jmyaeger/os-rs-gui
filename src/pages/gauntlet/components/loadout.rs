use super::prayers::PrayerSelect;
use super::select::Select;
use crate::pages::gauntlet::components::attack_styles::AttackStyleSelect;
use crate::pages::gauntlet::state::GauntletState;
use dioxus::prelude::*;
use osrs::types::equipment::{CombatStance, CombatStyle, GearSlot};

const MELEE_WEAPONS: [&str; 5] = [
    "Corrupted halberd (perfected)",
    "Corrupted halberd (attuned)",
    "Corrupted halberd (basic)",
    "Corrupted sceptre",
    "Unarmed",
];

const RANGED_WEAPONS: [&str; 3] = [
    "Corrupted bow (perfected)",
    "Corrupted bow (attuned)",
    "Corrupted bow (basic)",
];

const MAGIC_WEAPONS: [&str; 3] = [
    "Corrupted staff (perfected)",
    "Corrupted staff (attuned)",
    "Corrupted staff (basic)",
];

const STYLES: [&str; 3] = ["Melee", "Ranged", "Magic"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadoutStyle {
    Melee,
    Ranged,
    Magic,
}

impl LoadoutStyle {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "Melee" => Some(Self::Melee),
            "Ranged" => Some(Self::Ranged),
            "Magic" => Some(Self::Magic),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Melee => "Melee",
            Self::Ranged => "Ranged",
            Self::Magic => "Magic",
        }
    }
}

/// A loadout card component that displays weapon, prayer, and attack style selection.
///
/// If `fixed_style` is provided, the style is locked and shown as a header.
/// If `fixed_style` is None, a style dropdown is shown for user selection.
#[component]
pub fn LoadoutCard(
    #[props(default)] fixed_style: Option<LoadoutStyle>,
    #[props(default = 1)] loadout_number: u8,
) -> Element {
    let state = use_context::<GauntletState>();

    // For non-fixed cards, get the current style from shared state
    let current_style = if let Some(style) = fixed_style {
        Some(style)
    } else {
        let selections = state.two_t3_selections.cloned();
        match loadout_number {
            1 => selections.loadout1_style,
            2 => selections.loadout2_style,
            _ => None,
        }
    };

    // Get the style selected by the other loadout (for filtering)
    let other_loadout_style = if fixed_style.is_none() {
        let selections = state.two_t3_selections.cloned();
        match loadout_number {
            1 => selections.loadout2_style,
            2 => selections.loadout1_style,
            _ => None,
        }
    } else {
        None
    };

    // Available styles (filter out the one selected by the other loadout)
    let available_styles: Vec<String> = STYLES
        .iter()
        .filter(|&s| {
            other_loadout_style
                .map(|other| other.as_str() != *s)
                .unwrap_or(true)
        })
        .map(|s| s.to_string())
        .collect();

    // Weapons based on current style
    let weapons: Vec<String> = current_style
        .map(|style| match style {
            LoadoutStyle::Melee => MELEE_WEAPONS.iter().map(|s| s.to_string()).collect(),
            LoadoutStyle::Ranged => RANGED_WEAPONS.iter().map(|s| s.to_string()).collect(),
            LoadoutStyle::Magic => MAGIC_WEAPONS.iter().map(|s| s.to_string()).collect(),
        })
        .unwrap_or_default();

    let default_weapon = if let Some(style) = fixed_style {
        match style {
            LoadoutStyle::Magic => Some(MAGIC_WEAPONS[0].to_string()),
            LoadoutStyle::Ranged => Some(RANGED_WEAPONS[1].to_string()),
            LoadoutStyle::Melee => Some(MELEE_WEAPONS[4].to_string()),
        }
    } else {
        current_style.map(|style| match style {
            LoadoutStyle::Melee => MELEE_WEAPONS[0].to_string(),
            LoadoutStyle::Ranged => RANGED_WEAPONS[0].to_string(),
            LoadoutStyle::Magic => MAGIC_WEAPONS[0].to_string(),
        })
    };

    let mut selected_weapon = use_signal(move || default_weapon.clone());

    // Signal for style selection in the Select component
    let selected_style_str = use_signal(move || current_style.map(|s| s.as_str().to_string()));

    // Update app state when style signal changes, and reset weapon to default for new style
    use_effect(move || {
        if fixed_style.is_some() {
            return;
        }

        if let Some(style_str) = selected_style_str()
            && let Some(style) = LoadoutStyle::from_str(&style_str)
        {
            // Update the selected weapon to the default for this style
            let new_default = match style {
                LoadoutStyle::Melee => MELEE_WEAPONS[0].to_string(),
                LoadoutStyle::Ranged => RANGED_WEAPONS[0].to_string(),
                LoadoutStyle::Magic => MAGIC_WEAPONS[0].to_string(),
            };
            selected_weapon.set(Some(new_default));

            let mut selections = state.two_t3_selections;
            match loadout_number {
                1 if selections.peek().loadout1_style != Some(style) => {
                    selections.write().loadout1_style = Some(style);
                }
                2 if selections.peek().loadout2_style != Some(style) => {
                    selections.write().loadout2_style = Some(style);
                }
                _ => {}
            }
        }
    });

    let on_weapon_change = use_callback(move |weapon: String| {
        if let Some(style) = current_style {
            let mut switch = state.switch_signal(style);
            let mut player = switch.write();

            if weapon.trim() == "Unarmed" {
                player.unequip_slot(&GearSlot::Weapon);
            } else {
                let _ = player.equip(&weapon, None);
                let attack_style = match style {
                    LoadoutStyle::Melee => {
                        *player
                            .gear
                            .weapon
                            .combat_styles
                            .iter()
                            .find(|(_, s)| s.stance == CombatStance::Aggressive)
                            .or_else(|| player.gear.weapon.combat_styles.iter().next())
                            .expect("equipped melee weapon should have at least one combat style")
                            .0
                    }
                    LoadoutStyle::Ranged => CombatStyle::Rapid,
                    LoadoutStyle::Magic => CombatStyle::Accurate,
                };
                player.set_active_style(attack_style);
            }
        }
    });

    // Apply the default/initial selection to AppState (and keep in sync)
    use_effect(move || {
        if let Some(w) = selected_weapon() {
            on_weapon_change.call(w);
        }
    });

    let title = if let Some(style) = fixed_style {
        style.as_str().to_string()
    } else {
        format!("Style {loadout_number}")
    };

    // Check if this card is the main style in 5:1 mode
    let is_main_style = fixed_style
        .map(|style| state.five_one_main_style.cloned() == style)
        .unwrap_or(false);

    let on_main_style_click = move |_| {
        if let Some(style) = fixed_style {
            let mut main_style = state.five_one_main_style;
            main_style.set(style);
        }
    };

    // Get style-specific classes
    let style_class = current_style
        .map(|s| match s {
            LoadoutStyle::Melee => "loadout-melee",
            LoadoutStyle::Ranged => "loadout-ranged",
            LoadoutStyle::Magic => "loadout-magic",
        })
        .unwrap_or("");

    let title_color = current_style
        .map(|s| match s {
            LoadoutStyle::Melee => "text-melee",
            LoadoutStyle::Ranged => "text-ranged",
            LoadoutStyle::Magic => "text-magic",
        })
        .unwrap_or("text-gray-200");

    let base_class = format!(
        "card rounded-lg p-3 flex flex-col gap-1 w-68 min-h-72 {}{}",
        style_class,
        if is_main_style {
            " border-2 border-white"
        } else {
            ""
        }
    );

    rsx! {
        div { class: "{base_class}",
            // Header section
            div { class: "flex flex-col gap-1 min-h-14",
                h3 { class: "text-base font-semibold {title_color} text-center", "{title}" }

                // 5:1 mode: main style selector
                if fixed_style.is_some() {
                    div { class: "flex justify-center",
                        button {
                            class: "flex items-center gap-1.5 text-xs px-2 py-0.5 rounded hover:bg-slate-700/40 transition-colors",
                            onclick: on_main_style_click,
                            title: "Set as main style for 5:1",
                            span { class: if is_main_style { "w-3 h-3 rounded-full border-2 border-white bg-white" } else { "w-3 h-3 rounded-full border-2 border-slate-700" } }
                            span { class: if is_main_style { "text-slate-100" } else { "text-gray-400" },
                                "Main style"
                            }
                        }
                    }
                }

                // Two T3 mode: style selector
                if fixed_style.is_none() {
                    Select {
                        options: available_styles,
                        value: selected_style_str,
                        placeholder: "Select style...",
                    }
                }
            }

            // Weapon selector
            if current_style.is_some() {
                div {
                    label { class: "text-xs text-gray-400", "Weapon" }
                    Select {
                        options: weapons,
                        value: selected_weapon,
                        placeholder: "Select weapon...",
                        on_change: on_weapon_change,
                    }
                }

                // Attack style
                if selected_weapon().is_some() {
                    div {
                        label { class: "text-xs text-gray-400", "Attack Style" }
                        AttackStyleSelect {
                            style: current_style.unwrap(),
                            weapon: selected_weapon,
                        }
                    }
                }

                // Prayer selector
                div {
                    label { class: "text-xs text-gray-400", "Prayers" }
                    PrayerSelect { style: current_style.unwrap() }
                }
            }
        
        }
    }
}
