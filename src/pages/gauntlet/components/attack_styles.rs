use crate::pages::gauntlet::components::loadout::LoadoutStyle;
use crate::pages::gauntlet::components::select::Select;
use crate::pages::gauntlet::state::GauntletState;
use dioxus::prelude::*;
use osrs::types::equipment::CombatStyle;

const HALBERD_STYLES: [CombatStyle; 3] = [CombatStyle::Jab, CombatStyle::Swipe, CombatStyle::Fend];

const BOW_STYLES: [CombatStyle; 3] = [
    CombatStyle::Accurate,
    CombatStyle::Rapid,
    CombatStyle::Longrange,
];

const STAFF_STYLES: [CombatStyle; 2] = [CombatStyle::Accurate, CombatStyle::Longrange];

const UNARMED_STYLES: [CombatStyle; 3] =
    [CombatStyle::Punch, CombatStyle::Kick, CombatStyle::Block];

const SCEPTRE_STYLES: [CombatStyle; 3] =
    [CombatStyle::Pound, CombatStyle::Pummel, CombatStyle::Block];

fn get_styles_for_weapon(weapon: &str) -> (Vec<CombatStyle>, CombatStyle) {
    if weapon.contains("halberd") {
        (Vec::from(HALBERD_STYLES), HALBERD_STYLES[1])
    } else if weapon.contains("bow") {
        (Vec::from(BOW_STYLES), BOW_STYLES[1])
    } else if weapon.contains("staff") {
        (Vec::from(STAFF_STYLES), STAFF_STYLES[0])
    } else if weapon.contains("sceptre") {
        (Vec::from(SCEPTRE_STYLES), SCEPTRE_STYLES[1])
    } else {
        (Vec::from(UNARMED_STYLES), UNARMED_STYLES[1])
    }
}

#[component]
pub fn AttackStyleSelect(style: LoadoutStyle, weapon: ReadSignal<Option<String>>) -> Element {
    let mut switch = use_context::<GauntletState>().switch_signal(style);
    let (styles, _default) = use_memo(move || {
        weapon()
            .map(|w| get_styles_for_weapon(&w))
            .unwrap_or_default()
    })();

    let mut current_value = use_signal(move || {
        let active_style = switch.peek().attrs.active_style;
        Some(active_style.to_string())
    });

    use_effect(move || {
        if let Some(w) = weapon() {
            let (_, new_default) = get_styles_for_weapon(&w);
            current_value.set(Some(new_default.to_string()));
            switch.write().set_active_style(new_default);
        }
    });

    rsx! {
        Select {
            options: styles.iter().map(|s| s.to_string()).collect(),
            value: current_value,
            placeholder: "Select attack style...",
            on_change: move |selected: String| {
                if let Some(new_style) = styles.iter().copied().find(|s| s.to_string() == selected) {
                    switch.write().set_active_style(new_style);
                }
            },
        }
    }
}
