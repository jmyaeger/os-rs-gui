use crate::pages::gauntlet::components::select::Select;
use crate::pages::gauntlet::state::AppState;
use dioxus::prelude::*;
use osrs::types::equipment::{Armor, GearSlot};
use std::sync::LazyLock;

#[derive(Clone)]
struct GearOption {
    label: &'static str,
    armor: Option<Armor>,
}

fn build_options(labels: [&'static str; 3]) -> Vec<GearOption> {
    let mut opts = labels
        .into_iter()
        .map(|label| GearOption {
            label,
            armor: Some(Armor::new(label, None).unwrap()),
        })
        .collect::<Vec<_>>();

    opts.insert(
        0,
        GearOption {
            label: "None",
            armor: None,
        },
    );
    opts
}

static HELMET_OPTIONS: LazyLock<Vec<GearOption>> = LazyLock::new(|| {
    build_options([
        "Corrupted helm (basic)",
        "Corrupted helm (attuned)",
        "Corrupted helm (perfected)",
    ])
});

static BODY_OPTIONS: LazyLock<Vec<GearOption>> = LazyLock::new(|| {
    build_options([
        "Corrupted body (basic)",
        "Corrupted body (attuned)",
        "Corrupted body (perfected)",
    ])
});

static LEGS_OPTIONS: LazyLock<Vec<GearOption>> = LazyLock::new(|| {
    build_options([
        "Corrupted legs (basic)",
        "Corrupted legs (attuned)",
        "Corrupted legs (perfected)",
    ])
});

fn apply_slot(state: &mut AppState, slot: GearSlot, armor: Option<&Armor>) {
    state.melee_switch.unequip_slot(&slot);
    state.ranged_switch.unequip_slot(&slot);
    state.magic_switch.unequip_slot(&slot);

    if let Some(a) = armor {
        let _ = state.melee_switch.equip_item(Box::new(a.clone()));
        let _ = state.ranged_switch.equip_item(Box::new(a.clone()));
        let _ = state.magic_switch.equip_item(Box::new(a.clone()));
    }
}

fn find_option<'a>(opts: &'a [GearOption], label: &str) -> Option<&'a GearOption> {
    opts.iter().find(|o| o.label == label)
}

#[component]
pub fn ArmorSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();

    let mut selected_tier = use_signal(|| Some(1usize));
    let mut selected_helmet = use_signal(|| None::<String>);
    let mut selected_body = use_signal(|| None::<String>);
    let mut selected_legs = use_signal(|| None::<String>);

    // Tier buttons: set dropdown state; one effect below will apply
    use_effect(move || {
        let Some(tier) = selected_tier() else { return };
        if tier > 3 {
            return;
        }

        selected_helmet.set(Some(HELMET_OPTIONS[tier].label.to_string()));
        selected_body.set(Some(BODY_OPTIONS[tier].label.to_string()));
        selected_legs.set(Some(LEGS_OPTIONS[tier].label.to_string()));
    });

    // Any dropdown change: apply to switch loadouts (including None)
    use_effect(move || {
        let helmet_label = selected_helmet();
        let body_label = selected_body();
        let legs_label = selected_legs();

        let mut state = app_state.write();

        let helmet_armor = helmet_label
            .as_deref()
            .and_then(|l| find_option(&HELMET_OPTIONS, l))
            .and_then(|o| o.armor.as_ref());

        let body_armor = body_label
            .as_deref()
            .and_then(|l| find_option(&BODY_OPTIONS, l))
            .and_then(|o| o.armor.as_ref());

        let legs_armor = legs_label
            .as_deref()
            .and_then(|l| find_option(&LEGS_OPTIONS, l))
            .and_then(|o| o.armor.as_ref());

        apply_slot(&mut state, GearSlot::Head, helmet_armor);
        apply_slot(&mut state, GearSlot::Body, body_armor);
        apply_slot(&mut state, GearSlot::Legs, legs_armor);
    });

    let tier_btn = |tier: usize| {
        let is_active = selected_tier() == Some(tier);
        if is_active {
            "px-3 py-1 text-sm font-medium transition-all duration-150 rounded btn-accent"
        } else {
            "px-3 py-1 text-sm font-medium transition-all duration-150 rounded bg-slate-900 text-gray-200 hover:bg-slate-600/40 input-field"
        }
    };

    rsx! {
        div { class: "flex items-center justify-center gap-1 px-1 py-1 mb-1",
            button {
                class: "{tier_btn(0)}",
                onclick: move |_| selected_tier.set(Some(0)),
                "None"
            }
            button {
                class: "{tier_btn(1)}",
                onclick: move |_| selected_tier.set(Some(1)),
                "T1"
            }
            button {
                class: "{tier_btn(2)}",
                onclick: move |_| selected_tier.set(Some(2)),
                "T2"
            }
            button {
                class: "{tier_btn(3)}",
                onclick: move |_| selected_tier.set(Some(3)),
                "T3"
            }
        }

        div { class: "space-y-1",
            Select {
                options: HELMET_OPTIONS.iter().map(|o| o.label.to_string()).collect(),
                value: selected_helmet,
                placeholder: "Select helmet...",
                on_change: move |gear_name| selected_helmet.set(Some(gear_name)),
            }
            Select {
                options: BODY_OPTIONS.iter().map(|o| o.label.to_string()).collect(),
                value: selected_body,
                placeholder: "Select body...",
                on_change: move |gear_name| selected_body.set(Some(gear_name)),
            }
            Select {
                options: LEGS_OPTIONS.iter().map(|o| o.label.to_string()).collect(),
                value: selected_legs,
                placeholder: "Select legs",
                on_change: move |gear_name| selected_legs.set(Some(gear_name)),
            }
        }
    }
}
