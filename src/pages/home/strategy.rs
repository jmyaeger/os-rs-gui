//! Special-attack strategy: which weapons to spec with, when, and in what gear.
//!
//! The model mirrors the engine's `SpecStrategy`/`SpecConfig` so that wiring the
//! simulation later is a translation, not a redesign.

use super::HomeState;
use super::simulation::spec_implemented;
use super::spec::{GearItem, OFFENSIVE_PRAYERS, SLOTS};
use crate::components::{SearchBar, equipment_catalog};
use dioxus::prelude::*;
use osrs::combat::spec::CoreCondition;
use osrs::constants::SPEC_COSTS;
use osrs::types::equipment::{EquipmentJson, GearSlot};
use osrs::types::player::Player;
use osrs::types::prayers::Prayer;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Catalog weapons the engine knows a special attack for.
static SPEC_WEAPONS: LazyLock<Vec<EquipmentJson>> = LazyLock::new(|| {
    let mut weapons: Vec<EquipmentJson> = equipment_catalog()
        .iter()
        .filter(|item| {
            item.slot.eq_ignore_ascii_case("weapon")
                && SPEC_COSTS.iter().any(|(name, _)| *name == item.name)
        })
        .cloned()
        .collect();
    weapons.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    weapons
});

static SWITCH_ITEMS: LazyLock<Vec<EquipmentJson>> = LazyLock::new(|| {
    equipment_catalog()
        .iter()
        .filter(|item| !item.slot.eq_ignore_ascii_case("weapon"))
        .cloned()
        .collect()
});

pub fn spec_weapons() -> &'static [EquipmentJson] {
    &SPEC_WEAPONS
}

pub fn spec_cost(weapon_name: &str) -> Option<u8> {
    SPEC_COSTS
        .iter()
        .find(|(name, _)| *name == weapon_name)
        .map(|(_, cost)| *cost)
}

fn catalog_weapon(item: &GearItem) -> Option<&'static EquipmentJson> {
    SPEC_WEAPONS
        .iter()
        .find(|weapon| weapon.name == item.name && weapon.version == item.version)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecCondition {
    FirstAttackOnly,
    NotFirstAttack,
    TargetHpBelow(u32),
    TargetHpAbove(u32),
    TargetHpPercentBelow(u8),
    TargetHpPercentAbove(u8),
    UntilDefenceReduced(u32),
}

impl SpecCondition {
    /// Stable keys and labels for the "add condition" menu.
    pub const KINDS: [(&'static str, &'static str); 7] = [
        ("first", "First attack only"),
        ("not-first", "Not on first attack"),
        ("hp-below", "Target HP below"),
        ("hp-above", "Target HP above"),
        ("pct-below", "Target HP below %"),
        ("pct-above", "Target HP above %"),
        ("def-reduced", "Until Defence reduced by"),
    ];

    pub fn from_kind(kind: &str, starting_hp: u32) -> Option<Self> {
        Some(match kind {
            "first" => Self::FirstAttackOnly,
            "not-first" => Self::NotFirstAttack,
            "hp-below" => Self::TargetHpBelow(starting_hp / 2),
            "hp-above" => Self::TargetHpAbove(starting_hp / 2),
            "pct-below" => Self::TargetHpPercentBelow(50),
            "pct-above" => Self::TargetHpPercentAbove(50),
            "def-reduced" => Self::UntilDefenceReduced(30),
            _ => return None,
        })
    }

    pub fn kind(self) -> &'static str {
        match self {
            Self::FirstAttackOnly => "first",
            Self::NotFirstAttack => "not-first",
            Self::TargetHpBelow(_) => "hp-below",
            Self::TargetHpAbove(_) => "hp-above",
            Self::TargetHpPercentBelow(_) => "pct-below",
            Self::TargetHpPercentAbove(_) => "pct-above",
            Self::UntilDefenceReduced(_) => "def-reduced",
        }
    }

    pub fn value(self) -> Option<u32> {
        match self {
            Self::FirstAttackOnly | Self::NotFirstAttack => None,
            Self::TargetHpBelow(v) | Self::TargetHpAbove(v) | Self::UntilDefenceReduced(v) => {
                Some(v)
            }
            Self::TargetHpPercentBelow(v) | Self::TargetHpPercentAbove(v) => Some(u32::from(v)),
        }
    }

    pub fn with_value(self, value: u32) -> Self {
        let percent = value.min(100) as u8;
        match self {
            Self::TargetHpBelow(_) => Self::TargetHpBelow(value),
            Self::TargetHpAbove(_) => Self::TargetHpAbove(value),
            Self::TargetHpPercentBelow(_) => Self::TargetHpPercentBelow(percent),
            Self::TargetHpPercentAbove(_) => Self::TargetHpPercentAbove(percent),
            Self::UntilDefenceReduced(_) => Self::UntilDefenceReduced(value),
            other => other,
        }
    }

    /// Text before the editable number, and the unit after it.
    pub fn parts(self) -> (&'static str, &'static str) {
        match self {
            Self::FirstAttackOnly => ("First attack only", ""),
            Self::NotFirstAttack => ("Not on first attack", ""),
            Self::TargetHpBelow(_) => ("Target HP ≤", "HP"),
            Self::TargetHpAbove(_) => ("Target HP >", "HP"),
            Self::TargetHpPercentBelow(_) => ("Target HP ≤", "%"),
            Self::TargetHpPercentAbove(_) => ("Target HP >", "%"),
            Self::UntilDefenceReduced(_) => ("Until Defence −", ""),
        }
    }

    pub fn label(self) -> String {
        let (text, unit) = self.parts();
        match self.value() {
            Some(value) if unit.is_empty() => format!("{text}{value}"),
            Some(value) => format!("{text} {value}{}", if unit == "%" { "%" } else { " HP" }),
            None => text.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn to_core(self) -> CoreCondition {
        match self {
            Self::FirstAttackOnly => CoreCondition::FirstAttackOnly,
            Self::NotFirstAttack => CoreCondition::NotFirstAttack,
            Self::TargetHpBelow(v) => CoreCondition::MonsterHpBelow(v),
            Self::TargetHpAbove(v) => CoreCondition::MonsterHpAbove(v),
            Self::TargetHpPercentBelow(v) => CoreCondition::MonsterHpPercentBelow(v),
            Self::TargetHpPercentAbove(v) => CoreCondition::MonsterHpPercentAbove(v),
            Self::UntilDefenceReduced(v) => CoreCondition::TargetDefenceReduction(v),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecStep {
    pub id: u32,
    pub weapon: GearItem,
    #[serde(default)]
    pub switches: Vec<GearItem>,
    #[serde(default)]
    pub prayer: Option<Prayer>,
    #[serde(default)]
    pub conditions: Vec<SpecCondition>,
    #[serde(default)]
    pub max_attempts: Option<u8>,
    #[serde(default)]
    pub min_successes: Option<u8>,
}

impl SpecStep {
    pub fn new(id: u32, weapon: GearItem) -> Self {
        Self {
            id,
            weapon,
            switches: Vec::new(),
            prayer: None,
            conditions: vec![SpecCondition::FirstAttackOnly],
            max_attempts: Some(1),
            min_successes: None,
        }
    }

    pub fn spec_cost(&self) -> Option<u8> {
        spec_cost(&self.weapon.name)
    }

    pub fn is_two_handed(&self) -> bool {
        catalog_weapon(&self.weapon).is_some_and(|weapon| weapon.is_two_handed == Some(true))
    }

    pub fn condition_summary(&self) -> String {
        if self.conditions.is_empty() {
            "Whenever energy allows".to_string()
        } else {
            self.conditions
                .iter()
                .map(|condition| condition.label())
                .collect::<Vec<_>>()
                .join(" · ")
        }
    }

    pub fn limits_summary(&self) -> String {
        let mut parts = Vec::new();
        match self.max_attempts {
            Some(1) => parts.push("once".to_string()),
            Some(n) => parts.push(format!("up to {n}×")),
            None => {}
        }
        if let Some(n) = self.min_successes {
            parts.push(format!("until {n} hit{}", if n == 1 { "" } else { "s" }));
        }
        parts.join(", ")
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestorePolicy {
    #[default]
    EveryKill,
    Never,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathCharge {
    #[default]
    None,
    Single,
    Double,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpecPlan {
    pub steps: Vec<SpecStep>,
    pub starting_energy: u8,
    pub restore: RestorePolicy,
    pub death_charge: DeathCharge,
    pub surge_potion: bool,
}

impl Default for SpecPlan {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            starting_energy: 100,
            restore: RestorePolicy::EveryKill,
            death_charge: DeathCharge::None,
            surge_potion: false,
        }
    }
}

impl SpecPlan {
    pub fn settings_summary(&self) -> Vec<String> {
        let mut parts = vec![format!("{}% energy", self.starting_energy)];
        if self.restore == RestorePolicy::Never {
            parts.push("no restore between kills".into());
        }
        match self.death_charge {
            DeathCharge::None => {}
            DeathCharge::Single => parts.push("Death Charge".into()),
            DeathCharge::Double => parts.push("Death Charge ×2".into()),
        }
        if self.surge_potion {
            parts.push("Surge potion".into());
        }
        parts
    }
}

fn default_spec_weapon(player: &Player) -> Option<GearItem> {
    let main = &player.gear.weapon;
    if spec_cost(&main.name).is_some() {
        return player
            .get_slot(&GearSlot::Weapon)
            .map(|item| GearItem::from_equipped(GearSlot::Weapon, item.as_ref()));
    }
    spec_weapons()
        .iter()
        .find(|weapon| weapon.name == "Dragon warhammer")
        .or_else(|| spec_weapons().first())
        .map(GearItem::from_catalog)
}

fn main_gear_switches(player: &Player, two_handed: bool) -> Vec<GearItem> {
    SLOTS
        .iter()
        .filter(|slot| **slot != GearSlot::Weapon && !(two_handed && **slot == GearSlot::Shield))
        .filter_map(|slot| {
            player
                .get_slot(slot)
                .map(|item| GearItem::from_equipped(*slot, item.as_ref()))
        })
        .collect()
}

fn filter_switch(item: &EquipmentJson, term: &str) -> bool {
    item.name.to_lowercase().contains(term)
}

fn switch_key(item: &EquipmentJson) -> String {
    format!(
        "{}-{}-{}",
        item.slot,
        item.name,
        item.version.as_deref().unwrap_or_default()
    )
}

fn render_switch(item: &EquipmentJson) -> Element {
    rsx! {
        div { class: "home-search-item",
            img { src: "{crate::EQUIPMENT_ASSETS}/{item.image}", alt: "" }
            span { "{item.name}" }
            small { "{item.slot}", if let Some(version) = &item.version { " · {version}" } }
        }
    }
}

#[component]
pub fn StrategyPanel() -> Element {
    let mut state = use_context::<HomeState>();
    let player = use_context::<Signal<Player>>();
    let plan = state.plan.read().clone();
    let total = plan.steps.len();
    rsx! {
        section { class: "card home-panel strategy-panel", aria_label: "Special attack strategy",
            header { class: "home-panel-header",
                h2 { class: "card-title", "Special attacks" }
                button {
                    class: "home-button",
                    disabled: spec_weapons().is_empty(),
                    onclick: move |_| {
                        let Some(weapon) = default_spec_weapon(&player.peek()) else { return };
                        let id = state.plan.peek().steps.iter().map(|step| step.id).max().unwrap_or(0) + 1;
                        state.plan.write().steps.push(SpecStep::new(id, weapon));
                    },
                    "+ Add special attack"
                }
            }
            div { class: "strategy-settings",
                label {
                    span { "Starting energy" }
                    select { class: "input-field", value: "{plan.starting_energy}",
                        onchange: move |event| { if let Ok(value) = event.value().parse() { state.plan.write().starting_energy = value; } },
                        for energy in [0_u8, 25, 50, 75, 100] { option { value: "{energy}", selected: energy == plan.starting_energy, "{energy}%" } }
                    }
                }
                label {
                    span { "Between kills" }
                    select { class: "input-field",
                        value: match plan.restore { RestorePolicy::EveryKill => "every", RestorePolicy::Never => "never" },
                        onchange: move |event| {
                            state.plan.write().restore = if event.value() == "never" { RestorePolicy::Never } else { RestorePolicy::EveryKill };
                        },
                        option { value: "every", selected: plan.restore == RestorePolicy::EveryKill, "Restore to full" }
                        option { value: "never", selected: plan.restore == RestorePolicy::Never, "Keep remaining" }
                    }
                }
                label {
                    span { "Death Charge" }
                    select { class: "input-field",
                        value: match plan.death_charge { DeathCharge::None => "none", DeathCharge::Single => "single", DeathCharge::Double => "double" },
                        onchange: move |event| {
                            state.plan.write().death_charge = match event.value().as_str() {
                                "single" => DeathCharge::Single,
                                "double" => DeathCharge::Double,
                                _ => DeathCharge::None,
                            };
                        },
                        option { value: "none", selected: plan.death_charge == DeathCharge::None, "Off" }
                        option { value: "single", selected: plan.death_charge == DeathCharge::Single, "On" }
                        option { value: "double", selected: plan.death_charge == DeathCharge::Double, "Double (Arceuus)" }
                    }
                }
                label { class: "home-check",
                    input { r#type: "checkbox", checked: plan.surge_potion,
                        onchange: move |event| state.plan.write().surge_potion = event.checked() }
                    span { "Surge potion" }
                }
            }
            if plan.steps.is_empty() {
                p { class: "strategy-empty", "No special attacks. The main weapon is used for the whole fight." }
            } else {
                div { class: "strategy-steps",
                    for (index, step) in plan.steps.iter().enumerate() {
                        StrategyRow { key: "{step.id}", index, step: step.clone(), total }
                    }
                }
            }
        }
    }
}

#[component]
fn StrategyRow(index: usize, step: SpecStep, total: usize) -> Element {
    let mut state = use_context::<HomeState>();
    let player = use_context::<Signal<Player>>();
    let mut editing_switch = use_signal(|| false);
    let number = index + 1;
    let two_handed = step.is_two_handed();
    let starting_hp = state.target.read().starting_hp();
    let weapon_value = step.weapon.label();
    let switch_items: Vec<_> = SWITCH_ITEMS
        .iter()
        .filter(|item| !two_handed || !item.slot.eq_ignore_ascii_case("shield"))
        .cloned()
        .collect();
    let mut cost = step
        .spec_cost()
        .map(|cost| format!("{cost}% energy"))
        .unwrap_or_else(|| "No spec data".to_string());
    let simulated = spec_implemented(&step.weapon.name);
    if !simulated {
        cost.push_str(" · not simulated by the engine yet");
    }
    // Copy the signal handle so the closure stays `Fn` and can be shared by every handler.
    let update = move |f: &dyn Fn(&mut SpecStep)| {
        let mut plan = state.plan;
        let mut plan = plan.write();
        if let Some(step) = plan.steps.get_mut(index) {
            f(step);
        }
    };

    rsx! {
        div { class: "strategy-row",
            div { class: "strategy-row-main",
                span { class: "strategy-order num", "{number}" }
                label { class: "strategy-field strategy-weapon",
                    span { class: "home-eyebrow", "Weapon" }
                    div { class: "strategy-weapon-pick",
                        img { src: step.weapon.image_src(), alt: "" }
                        select { class: "input-field", aria_label: "Special attack {number} weapon", value: "{weapon_value}",
                            onchange: move |event| {
                                let chosen = spec_weapons().iter().find(|weapon| GearItem::from_catalog(weapon).label() == event.value()).map(GearItem::from_catalog);
                                if let Some(weapon) = chosen {
                                    let mut plan = state.plan.write();
                                    if let Some(step) = plan.steps.get_mut(index) {
                                        step.weapon = weapon;
                                        if step.is_two_handed() {
                                            step.switches.retain(|item| !item.slot.eq_ignore_ascii_case("shield"));
                                        }
                                    }
                                }
                            },
                            for weapon in spec_weapons() {
                                option {
                                    value: GearItem::from_catalog(weapon).label(),
                                    selected: GearItem::from_catalog(weapon).label() == weapon_value,
                                    "{GearItem::from_catalog(weapon).label()}",
                                    if !spec_implemented(&weapon.name) { " (not simulated)" }
                                }
                            }
                        }
                    }
                    span { class: if simulated { "home-muted" } else { "home-error" }, "{cost}" }
                }
                label { class: "strategy-field strategy-limit",
                    span { class: "home-eyebrow", "Max uses" }
                    input { class: "input-field num", r#type: "number", min: "1", max: "99", placeholder: "∞",
                        value: step.max_attempts.map(|n| n.to_string()).unwrap_or_default(),
                        aria_label: "Special attack {number} maximum uses",
                        oninput: move |event| {
                            let value = event.value().parse::<u8>().ok().filter(|n| *n > 0);
                            update(&|step| step.max_attempts = value);
                        }
                    }
                }
                label { class: "strategy-field strategy-limit",
                    span { class: "home-eyebrow", title: "Stop once this many special attacks have hit", "Hits needed" }
                    input { class: "input-field num", r#type: "number", min: "1", max: "99", placeholder: "—",
                        value: step.min_successes.map(|n| n.to_string()).unwrap_or_default(),
                        aria_label: "Special attack {number} successful hits before stopping",
                        oninput: move |event| {
                            let value = event.value().parse::<u8>().ok().filter(|n| *n > 0);
                            update(&|step| step.min_successes = value);
                        }
                    }
                }
                label { class: "strategy-field strategy-prayer",
                    span { class: "home-eyebrow", "Prayer" }
                    select { class: "input-field", aria_label: "Special attack {number} prayer",
                        value: step.prayer.map(|p| p.to_string()).unwrap_or_default(),
                        onchange: move |event| {
                            let prayer = OFFENSIVE_PRAYERS.iter().copied().find(|p| p.to_string() == event.value());
                            update(&|step| step.prayer = prayer);
                        },
                        option { value: "", selected: step.prayer.is_none(), "Same as main" }
                        for prayer in OFFENSIVE_PRAYERS { option { value: "{prayer}", selected: step.prayer == Some(prayer), "{prayer}" } }
                    }
                }
                div { class: "strategy-order-buttons",
                    button { class: "home-icon-button", disabled: index == 0, title: "Move earlier", aria_label: "Move special attack {number} earlier",
                        onclick: move |_| { if index > 0 { state.plan.write().steps.swap(index, index - 1); } }, "↑" }
                    button { class: "home-icon-button", disabled: index + 1 >= total, title: "Move later", aria_label: "Move special attack {number} later",
                        onclick: move |_| { if index + 1 < total { state.plan.write().steps.swap(index, index + 1); } }, "↓" }
                    button { class: "home-icon-button", title: "Remove", aria_label: "Remove special attack {number}",
                        onclick: move |_| { state.plan.write().steps.remove(index); }, "×" }
                }
            }
            div { class: "strategy-line",
                span { class: "home-eyebrow strategy-line-label", "Use when" }
                if step.conditions.is_empty() { span { class: "home-muted", "Whenever energy allows" } }
                for (condition_index, condition) in step.conditions.iter().copied().enumerate() {
                    span { class: "home-chip strategy-condition", key: "{condition.kind()}-{condition_index}",
                        "{condition.parts().0}"
                        if let Some(value) = condition.value() {
                            input { class: "strategy-condition-value num", r#type: "number", min: "0", value: "{value}",
                                aria_label: "{condition.parts().0} value",
                                oninput: move |event| {
                                    if let Ok(value) = event.value().parse::<u32>() {
                                        update(&|step| { if let Some(c) = step.conditions.get_mut(condition_index) { *c = c.with_value(value); } });
                                    }
                                }
                            }
                            if !condition.parts().1.is_empty() { span { "{condition.parts().1}" } }
                        }
                        button { class: "home-chip-remove", aria_label: "Remove condition", onclick: move |_| update(&|step| { step.conditions.remove(condition_index); }), "×" }
                    }
                }
                select { key: "add-{step.conditions.len()}", class: "home-add-select", aria_label: "Add condition", value: "",
                    onchange: move |event| {
                        if let Some(condition) = SpecCondition::from_kind(&event.value(), starting_hp) {
                            update(&|step| { if !step.conditions.iter().any(|c| c.kind() == condition.kind()) { step.conditions.push(condition); } });
                        }
                    },
                    option { value: "", "+ Condition" }
                    for (kind, label) in SpecCondition::KINDS { option { value: kind, "{label}" } }
                }
            }
            div { class: "strategy-line",
                span { class: "home-eyebrow strategy-line-label", "Gear" }
                if step.switches.is_empty() { span { class: "home-muted", "Main gear with the spec weapon" } }
                if two_handed { span { class: "home-chip is-note", "Shield removed" } }
                for (item_index, item) in step.switches.iter().enumerate() {
                    span { class: "home-chip strategy-switch", key: "{item.slot}",
                        img { src: item.image_src(), alt: "" }
                        "{item.label()}"
                        button { class: "home-chip-remove", aria_label: "Use main gear for {item.slot}", title: "Use main gear for this slot",
                            onclick: move |_| update(&|step| { step.switches.remove(item_index); }), "×" }
                    }
                }
                button { class: "home-text-button",
                    onclick: move |_| {
                        let switches = main_gear_switches(&player.peek(), two_handed);
                        update(&|step| step.switches = switches.clone());
                    },
                    "Copy main gear"
                }
                button { class: "home-text-button", aria_expanded: editing_switch(),
                    onclick: move |_| editing_switch.toggle(),
                    if editing_switch() { "Done" } else { "+ Override slot" }
                }
            }
            if editing_switch() {
                div { class: "strategy-switch-editor",
                    SearchBar {
                        items: switch_items, filter_fn: filter_switch, render_item: render_switch, get_key: switch_key,
                        placeholder: "Search gear to override a slot…".to_string(), max_results: 8,
                        on_select: move |item: EquipmentJson| {
                            let item = GearItem::from_catalog(&item);
                            update(&|step| {
                                step.switches.retain(|existing| !existing.slot.eq_ignore_ascii_case(&item.slot));
                                step.switches.push(item.clone());
                            });
                        }
                    }
                }
            }
        }
    }
}
