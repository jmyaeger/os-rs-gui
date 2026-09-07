//! Special-attack strategy: which weapons to spec with, when, and in what gear.
//!
//! The model mirrors the engine's `SpecStrategy`/`SpecConfig` so that wiring the
//! simulation is a translation, not a redesign.

use super::HomeState;
use super::metrics::spec_metrics;
use super::simulation::{spec_implemented, spec_player};
use super::spec::{GearItem, OFFENSIVE_PRAYERS, SLOTS};
use crate::components::{SearchBar, equipment_catalog};
use dioxus::prelude::*;
use osrs::combat::spec::CoreCondition;
use osrs::constants::{
    CRUSH_SPEC_WEAPONS, MAGIC_SPEC_WEAPONS, SLASH_SPEC_WEAPONS, SPEC_COSTS, STAB_SPEC_WEAPONS,
};
use osrs::types::equipment::{CombatStance, CombatStyle, CombatType, EquipmentJson, GearSlot};
use osrs::types::monster::Monster;
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

/// The subset whose special attack the simulation actually models. Offering the
/// rest would silently fall back to a normal attack.
static SIMULATED_SPEC_WEAPONS: LazyLock<Vec<EquipmentJson>> = LazyLock::new(|| {
    SPEC_WEAPONS
        .iter()
        .filter(|item| spec_implemented(&item.name))
        .cloned()
        .collect()
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

pub fn simulated_spec_weapons() -> &'static [EquipmentJson] {
    &SIMULATED_SPEC_WEAPONS
}

/// The defence roll a special attack is checked against, and whether the weapon
/// fixes it. Mirrors `dps_calc::get_normal_accuracy` in the engine: most spec
/// weapons roll against one combat type whatever style is selected, and only the
/// unlisted ones follow the style. Reads the engine's own tables so the two
/// cannot drift apart.
pub fn spec_defence_type(weapon_name: &str, style_type: CombatType) -> (CombatType, bool) {
    for (weapons, combat_type) in [
        (&STAB_SPEC_WEAPONS[..], CombatType::Stab),
        (&SLASH_SPEC_WEAPONS[..], CombatType::Slash),
        (&CRUSH_SPEC_WEAPONS[..], CombatType::Crush),
        (&MAGIC_SPEC_WEAPONS[..], CombatType::Magic),
    ] {
        if weapons.contains(&weapon_name) {
            return (combat_type, true);
        }
    }
    (style_type, false)
}

/// Same preference order as the main weapon picker in `components::preferred_style`.
fn default_style(styles: &[(CombatStyle, CombatType, CombatStance)]) -> Option<CombatStyle> {
    styles
        .iter()
        .min_by_key(|(style, _, stance)| {
            let rank = match stance {
                CombatStance::Rapid => 0,
                CombatStance::Aggressive => 1,
                CombatStance::Accurate => 2,
                CombatStance::Controlled => 3,
                CombatStance::Autocast => 4,
                _ => 5,
            };
            (rank, style.to_string())
        })
        .map(|(style, _, _)| *style)
}

pub fn spec_cost(weapon_name: &str) -> Option<u8> {
    SPEC_COSTS
        .iter()
        .find(|(name, _)| *name == weapon_name)
        .map(|(_, cost)| *cost)
}

/// The combat styles a catalog weapon offers, sorted for a stable dropdown.
pub fn weapon_styles(item: &GearItem) -> Vec<(CombatStyle, CombatType, CombatStance)> {
    let Some(entry) = catalog_weapon(item).cloned() else {
        return Vec::new();
    };
    let Ok(weapon) = entry.into_weapon() else {
        return Vec::new();
    };
    let mut styles: Vec<_> = weapon
        .combat_styles
        .iter()
        .map(|(style, option)| (*style, option.combat_type, option.stance))
        .collect();
    styles.sort_by_key(|(style, _, _)| style.to_string());
    styles
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
    pub prayers: Vec<Prayer>,
    /// Attack style for the special attack. `None` follows the weapon's default.
    #[serde(default)]
    pub style: Option<CombatStyle>,
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
            prayers: Vec::new(),
            style: None,
            conditions: vec![SpecCondition::FirstAttackOnly],
            max_attempts: Some(1),
            min_successes: None,
        }
    }

    pub fn spec_cost(&self) -> Option<u8> {
        spec_cost(&self.weapon.name)
    }

    /// The style this step attacks with: the chosen one when the weapon still
    /// offers it, otherwise the weapon's default.
    pub fn resolved_style(&self) -> Option<CombatStyle> {
        let styles = weapon_styles(&self.weapon);
        self.style
            .filter(|chosen| styles.iter().any(|(style, _, _)| style == chosen))
            .or_else(|| default_style(&styles))
    }

    /// The combat type the resolved style attacks with.
    pub fn style_combat_type(&self) -> Option<CombatType> {
        let chosen = self.resolved_style()?;
        weapon_styles(&self.weapon)
            .into_iter()
            .find(|(style, _, _)| *style == chosen)
            .map(|(_, combat_type, _)| combat_type)
    }

    /// The defence roll this step's special attack is checked against.
    pub fn defence_type(&self) -> Option<(CombatType, bool)> {
        let style_type = self.style_combat_type()?;
        Some(spec_defence_type(&self.weapon.name, style_type))
    }

    /// Short summary of the style and the defence it rolls against.
    pub fn style_label(&self) -> Option<String> {
        let chosen = self.resolved_style()?;
        let (defence, _) = self.defence_type()?;
        Some(format!("{chosen} · vs {defence} def"))
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

/// When special attack energy is restored during a batch of kills.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestorePolicy {
    #[default]
    EveryKill,
    /// Restore once every N kills.
    EveryNKills(u32),
    Never,
}

impl RestorePolicy {
    pub fn key(self) -> &'static str {
        match self {
            Self::EveryKill => "every",
            Self::EveryNKills(_) => "every-n",
            Self::Never => "never",
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::EveryKill => "Restore every kill".to_string(),
            Self::EveryNKills(n) => format!("Restore every {n} kills"),
            Self::Never => "Never restore".to_string(),
        }
    }
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
        if self.restore != RestorePolicy::EveryKill {
            parts.push(self.restore.label().to_lowercase());
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
    if spec_cost(&main.name).is_some() && spec_implemented(&main.name) {
        return player
            .get_slot(&GearSlot::Weapon)
            .map(|item| GearItem::from_equipped(GearSlot::Weapon, item.as_ref()));
    }
    simulated_spec_weapons()
        .iter()
        .find(|weapon| weapon.name == "Dragon warhammer")
        .or_else(|| simulated_spec_weapons().first())
        .map(GearItem::from_catalog)
}

fn filter_item(item: &EquipmentJson, term: &str) -> bool {
    item.name.to_lowercase().contains(term)
        || item
            .version
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(term)
}

fn item_key(item: &EquipmentJson) -> String {
    format!(
        "{}-{}-{}",
        item.slot,
        item.name,
        item.version.as_deref().unwrap_or_default()
    )
}

fn render_item(item: &EquipmentJson) -> Element {
    rsx! {
        div { class: "home-search-item",
            img { src: "{crate::EQUIPMENT_ASSETS}/{item.image}", alt: "" }
            span { "{item.name}" }
            small {
                "{item.slot}"
                if let Some(version) = &item.version {
                    " · {version}"
                }
            }
        }
    }
}

#[component]
pub fn StrategyPanel(monster: ReadSignal<Option<Monster>>) -> Element {
    let mut state = use_context::<HomeState>();
    let player = use_context::<Signal<Player>>();
    let plan = state.plan.read().clone();
    let total = plan.steps.len();
    let restore_kills = match plan.restore {
        RestorePolicy::EveryNKills(n) => n,
        _ => 5,
    };
    rsx! {
        section {
            class: "card home-panel strategy-panel",
            aria_label: "Special attack strategy",
            header { class: "home-panel-header",
                h2 { class: "card-title", "Special attacks" }
                button {
                    class: "home-button",
                    disabled: simulated_spec_weapons().is_empty(),
                    onclick: move |_| {
                        let Some(weapon) = default_spec_weapon(&player.peek()) else { return };
                        let id = state.plan.peek().steps.iter().map(|step| step.id).max().unwrap_or(0)
                            + 1;
                        state.plan.write().steps.push(SpecStep::new(id, weapon));
                    },
                    "+ Add special attack"
                }
            }
            div { class: "strategy-settings",
                label {
                    span { "Starting energy" }
                    div { class: "strategy-energy-field",
                        input {
                            class: "input-field num",
                            r#type: "number",
                            min: "0",
                            max: "100",
                            step: "5",
                            value: "{plan.starting_energy}",
                            oninput: move |event| {
                                if let Ok(value) = event.value().parse::<i32>() {
                                    state.plan.write().starting_energy = value.clamp(0, 100) as u8;
                                }
                            },
                        }
                        span { class: "home-muted", "%" }
                    }
                }
                label {
                    span { "Spec restore policy" }
                    select {
                        class: "input-field",
                        value: plan.restore.key(),
                        onchange: move |event| {
                            let mut plan = state.plan.write();
                            plan.restore = match event.value().as_str() {
                                "never" => RestorePolicy::Never,
                                "every-n" => RestorePolicy::EveryNKills(restore_kills.max(2)),
                                _ => RestorePolicy::EveryKill,
                            };
                        },
                        option {
                            value: "every",
                            selected: plan.restore == RestorePolicy::EveryKill,
                            "Every kill"
                        }
                        option {
                            value: "every-n",
                            selected: matches!(plan.restore, RestorePolicy::EveryNKills(_)),
                            "Every X kills"
                        }
                        option {
                            value: "never",
                            selected: plan.restore == RestorePolicy::Never,
                            "Never"
                        }
                    }
                }
                if matches!(plan.restore, RestorePolicy::EveryNKills(_)) {
                    label {
                        span { "Kills between restores" }
                        input {
                            class: "input-field num",
                            r#type: "number",
                            min: "2",
                            max: "999",
                            value: "{restore_kills}",
                            oninput: move |event| {
                                if let Ok(value) = event.value().parse::<u32>() {
                                    state.plan.write().restore = RestorePolicy::EveryNKills(value.clamp(2, 999));
                                }
                            },
                        }
                    }
                }
                label {
                    span { "Death Charge" }
                    select {
                        class: "input-field",
                        value: match plan.death_charge {
                            DeathCharge::None => "none",
                            DeathCharge::Single => "single",
                            DeathCharge::Double => "double",
                        },
                        onchange: move |event| {
                            state.plan.write().death_charge = match event.value().as_str() {
                                "single" => DeathCharge::Single,
                                "double" => DeathCharge::Double,
                                _ => DeathCharge::None,
                            };
                        },
                        option {
                            value: "none",
                            selected: plan.death_charge == DeathCharge::None,
                            "None"
                        }
                        option {
                            value: "single",
                            selected: plan.death_charge == DeathCharge::Single,
                            "Single"
                        }
                        option {
                            value: "double",
                            selected: plan.death_charge == DeathCharge::Double,
                            "Double"
                        }
                    }
                }
                label { class: "home-check",
                    input {
                        r#type: "checkbox",
                        checked: plan.surge_potion,
                        onchange: move |event| state.plan.write().surge_potion = event.checked(),
                    }
                    span { "Surge potion" }
                }
            }
            if plan.steps.is_empty() {
                p { class: "strategy-empty", "No special attacks." }
            } else {
                div { class: "strategy-steps",
                    for (index , step) in plan.steps.iter().enumerate() {
                        StrategyRow {
                            key: "{step.id}",
                            index,
                            step: step.clone(),
                            total,
                            monster,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StrategyRow(
    index: usize,
    step: SpecStep,
    total: usize,
    monster: ReadSignal<Option<Monster>>,
) -> Element {
    let mut state = use_context::<HomeState>();
    let player = use_context::<Signal<Player>>();
    let mut editing_switch = use_signal(|| false);
    let mut editing_weapon = use_signal(|| false);
    let number = index + 1;
    let two_handed = step.is_two_handed();
    let starting_hp = state.target.read().starting_hp();
    let switch_items: Vec<_> = SWITCH_ITEMS
        .iter()
        .filter(|item| !two_handed || !item.slot.eq_ignore_ascii_case("shield"))
        .cloned()
        .collect();
    let cost = step
        .spec_cost()
        .map(|cost| format!("{cost}% energy"))
        .unwrap_or_else(|| "No spec data".to_string());
    // Recomputed on each render rather than memoised, so it always reflects the
    // current gear, target and style; one hit distribution per step is cheap.
    let metrics = match &*monster.read() {
        Some(monster) => step.defence_type().map(|(defence, _)| {
            spec_player(&player.read(), &step)
                .and_then(|player| spec_metrics(&player, monster, defence))
        }),
        None => None,
    };
    let styles = weapon_styles(&step.weapon);
    let resolved_style = step.resolved_style();
    let weapon_for_styles = step.weapon.clone();
    let available_prayers: Vec<_> = OFFENSIVE_PRAYERS
        .into_iter()
        .filter(|prayer| !step.prayers.contains(prayer))
        .collect();
    // Signals are Copy; taking a local copy keeps this closure `Fn`.
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
                div { class: "strategy-field strategy-weapon",
                    span { class: "home-eyebrow", "Weapon" }
                    div { class: "strategy-weapon-pick",
                        img { src: step.weapon.image_src(), alt: "" }
                        strong { "{step.weapon.name}" }
                        span { class: "home-muted", "{cost}" }
                        button {
                            class: "home-text-button",
                            aria_expanded: editing_weapon(),
                            onclick: move |_| editing_weapon.toggle(),
                            if editing_weapon() {
                                "Done"
                            } else {
                                "Change"
                            }
                        }
                    }
                }
                div { class: "strategy-field strategy-style",
                    span {
                        class: "home-eyebrow",
                        title: "Sets the attack roll and max hit for the special attack",
                        "Attack style"
                    }
                    select {
                        class: "input-field",
                        aria_label: "Special attack {number} attack style",
                        disabled: styles.is_empty(),
                        value: resolved_style.map(|style| style.to_string()).unwrap_or_default(),
                        onchange: move |event| {
                            let chosen = weapon_styles(&weapon_for_styles)
                                .into_iter()
                                .find(|(style, _, _)| style.to_string() == event.value())
                                .map(|(style, _, _)| style);
                            update(&|step| step.style = chosen);
                        },
                        for (style , combat_type , stance) in styles.clone() {
                            option {
                                value: "{style}",
                                selected: Some(style) == resolved_style,
                                "{style} · {combat_type} · {stance_label(stance)}"
                            }
                        }
                    }
                    if let Some((defence, fixed)) = step.defence_type() {
                        span {
                            class: "home-muted strategy-defence-note",
                            title: if fixed { "This weapon's special attack always rolls against the same defence, whatever style is selected" } else { "The engine has no fixed defence type for this special attack, so it uses the selected style's" },
                            "vs {defence} def"
                            if fixed {
                                " (fixed)"
                            }
                        }
                    }
                }
                div { class: "strategy-field strategy-limit",
                    span { class: "home-eyebrow", "Max uses" }
                    input {
                        class: "input-field num",
                        r#type: "number",
                        min: "1",
                        max: "99",
                        placeholder: "∞",
                        value: step.max_attempts.map(|n| n.to_string()).unwrap_or_default(),
                        aria_label: "Special attack {number} maximum uses",
                        oninput: move |event| {
                            let value = event.value().parse::<u8>().ok().filter(|n| *n > 0);
                            update(&|step| step.max_attempts = value);
                        },
                    }
                }
                div { class: "strategy-field strategy-limit",
                    span {
                        class: "home-eyebrow",
                        title: "Stop once this many special attacks have hit",
                        "Hits needed"
                    }
                    input {
                        class: "input-field num",
                        r#type: "number",
                        min: "1",
                        max: "99",
                        placeholder: "—",
                        value: step.min_successes.map(|n| n.to_string()).unwrap_or_default(),
                        aria_label: "Special attack {number} successful hits before stopping",
                        oninput: move |event| {
                            let value = event.value().parse::<u8>().ok().filter(|n| *n > 0);
                            update(&|step| step.min_successes = value);
                        },
                    }
                }
                div { class: "strategy-order-buttons",
                    button {
                        class: "home-icon-button",
                        disabled: index == 0,
                        title: "Move earlier",
                        aria_label: "Move special attack {number} earlier",
                        onclick: move |_| {
                            if index > 0 {
                                state.plan.write().steps.swap(index, index - 1);
                            }
                        },
                        "↑"
                    }
                    button {
                        class: "home-icon-button",
                        disabled: index + 1 >= total,
                        title: "Move later",
                        aria_label: "Move special attack {number} later",
                        onclick: move |_| {
                            if index + 1 < total {
                                state.plan.write().steps.swap(index, index + 1);
                            }
                        },
                        "↓"
                    }
                    button {
                        class: "home-icon-button",
                        title: "Remove",
                        aria_label: "Remove special attack {number}",
                        onclick: move |_| {
                            state.plan.write().steps.remove(index);
                        },
                        "×"
                    }
                }
            }
            if editing_weapon() {
                div { class: "strategy-search",
                    SearchBar {
                        items: simulated_spec_weapons().to_vec(),
                        filter_fn: filter_item,
                        render_item,
                        get_key: item_key,
                        placeholder: "Search special attack weapons…".to_string(),
                        max_results: 8,
                        on_select: move |item: EquipmentJson| {
                            let item = GearItem::from_catalog(&item);
                            update(
                                &|step| { // The previous style may not exist on the new weapon.
                                    step.weapon = item.clone();
                                    step.style = None;
                                    if step.is_two_handed() {
                                        step.switches
                                            .retain(|switch| !switch.slot.eq_ignore_ascii_case("shield"));
                                    }
                                },
                            );
                            editing_weapon.set(false);
                        },
                    }
                }
            }
            div { class: "strategy-line",
                span { class: "home-eyebrow strategy-line-label", "Use when" }
                if step.conditions.is_empty() {
                    span { class: "home-muted", "Whenever energy allows" }
                }
                for (condition_index , condition) in step.conditions.iter().copied().enumerate() {
                    span {
                        class: "home-chip strategy-condition",
                        key: "{condition.kind()}-{condition_index}",
                        "{condition.parts().0}"
                        if let Some(value) = condition.value() {
                            input {
                                class: "strategy-condition-value num",
                                r#type: "number",
                                min: "0",
                                value: "{value}",
                                aria_label: "{condition.parts().0} value",
                                oninput: move |event| {
                                    if let Ok(value) = event.value().parse::<u32>() {
                                        update(
                                            &|step| {
                                                if let Some(c) = step.conditions.get_mut(condition_index) {
                                                    *c = c.with_value(value);
                                                }
                                            },
                                        );
                                    }
                                },
                            }
                            if !condition.parts().1.is_empty() {
                                span { "{condition.parts().1}" }
                            }
                        }
                        button {
                            class: "home-chip-remove",
                            aria_label: "Remove condition",
                            onclick: move |_| update(
                                &|step| {
                                    step.conditions.remove(condition_index);
                                },
                            ),
                            "×"
                        }
                    }
                }
                select {
                    key: "add-condition-{step.conditions.len()}",
                    class: "home-add-select",
                    aria_label: "Add condition",
                    value: "",
                    onchange: move |event| {
                        if let Some(condition) = SpecCondition::from_kind(&event.value(), starting_hp) {
                            update(
                                &|step| {
                                    if !step.conditions.iter().any(|c| c.kind() == condition.kind()) {
                                        step.conditions.push(condition);
                                    }
                                },
                            );
                        }
                    },
                    option { value: "", "+ Condition" }
                    for (kind , label) in SpecCondition::KINDS {
                        option { value: kind, "{label}" }
                    }
                }
            }
            div { class: "strategy-line",
                span { class: "home-eyebrow strategy-line-label", "Prayers" }
                if step.prayers.is_empty() {
                    span { class: "home-muted", "Same as main" }
                }
                for (prayer_index , prayer) in step.prayers.iter().copied().enumerate() {
                    span { class: "home-chip", key: "{prayer}",
                        img {
                            src: "{crate::PRAYERS_ASSETS}/{prayer}.png",
                            alt: "",
                        }
                        "{prayer}"
                        button {
                            class: "home-chip-remove",
                            aria_label: "Remove {prayer}",
                            onclick: move |_| update(
                                &|step| {
                                    step.prayers.remove(prayer_index);
                                },
                            ),
                            "×"
                        }
                    }
                }
                select {
                    key: "add-prayer-{step.prayers.len()}",
                    class: "home-add-select",
                    aria_label: "Add prayer for special attack {number}",
                    value: "",
                    onchange: move |event| {
                        let chosen = OFFENSIVE_PRAYERS
                            .into_iter()
                            .find(|prayer| prayer.to_string() == event.value());
                        if let Some(prayer) = chosen {
                            update(
                                &|step| {
                                    if !step.prayers.contains(&prayer) {
                                        step.prayers.push(prayer);
                                    }
                                },
                            );
                        }
                    },
                    option { value: "", "+ Prayer" }
                    for prayer in available_prayers {
                        option { value: "{prayer}", "{prayer}" }
                    }
                }
            }
            div { class: "strategy-line",
                span { class: "home-eyebrow strategy-line-label", "Gear" }
                if two_handed {
                    span { class: "home-chip is-note", "Shield removed" }
                }
                for (item_index , item) in step.switches.iter().enumerate() {
                    span {
                        class: "home-chip strategy-switch",
                        key: "{item.slot}",
                        img { src: item.image_src(), alt: "" }
                        "{item.label()}"
                        button {
                            class: "home-chip-remove",
                            aria_label: "Use main gear for {item.slot}",
                            title: "Use main gear for this slot",
                            onclick: move |_| update(
                                &|step| {
                                    step.switches.remove(item_index);
                                },
                            ),
                            "×"
                        }
                    }
                }
                button {
                    class: "home-text-button",
                    aria_expanded: editing_switch(),
                    onclick: move |_| editing_switch.toggle(),
                    if editing_switch() {
                        "Done"
                    } else {
                        "Add gear switch"
                    }
                }
            }
            if let Some(Err(reason)) = &metrics {
                div { class: "strategy-metrics",
                    span { class: "home-eyebrow strategy-line-label", "This spec" }
                    span { class: "home-muted", "{reason}" }
                }
            }
            if let Some(Ok(metrics)) = metrics {
                div { class: "strategy-metrics",
                    span { class: "home-eyebrow strategy-line-label", "This spec" }
                    div {
                        span { "DPS" }
                        strong { class: "num", "{metrics.dps:.2}" }
                    }
                    div {
                        span { "Accuracy" }
                        strong { class: "num",
                            if metrics.always_hits {
                                "Always hits"
                            } else {
                                "{metrics.accuracy * 100.0:.2}%"
                            }
                        }
                    }
                    div {
                        span { "Expected hit" }
                        strong { class: "num", "{metrics.expected_hit:.2}" }
                    }
                    div {
                        span { "Max hit" }
                        strong { class: "num", "{metrics.max_hit}" }
                    }
                    if !metrics.always_hits {
                        div {
                            span { "Attack roll" }
                            strong { class: "num", "{metrics.attack_roll}" }
                        }
                        div {
                            span { "Defence roll" }
                            strong { class: "num", "{metrics.defence_roll}" }
                        }
                    }
                }
            }
            if editing_switch() {
                div { class: "strategy-search",
                    SearchBar {
                        items: switch_items,
                        filter_fn: filter_item,
                        render_item,
                        get_key: item_key,
                        placeholder: "Search gear to switch to…".to_string(),
                        max_results: 8,
                        on_select: move |item: EquipmentJson| {
                            let item = GearItem::from_catalog(&item);
                            update(
                                &|step| {
                                    step.switches
                                        .retain(|existing| !existing.slot.eq_ignore_ascii_case(&item.slot));
                                    step.switches.push(item.clone());
                                },
                            );
                        },
                    }
                }
            }
        }
    }
}

fn stance_label(stance: CombatStance) -> &'static str {
    match stance {
        CombatStance::Accurate => "Accurate",
        CombatStance::Aggressive => "Aggressive",
        CombatStance::Defensive => "Defensive",
        CombatStance::Controlled => "Controlled",
        CombatStance::Rapid => "Rapid",
        CombatStance::Longrange => "Long range",
        CombatStance::ShortFuse => "Short fuse",
        CombatStance::MediumFuse => "Medium fuse",
        CombatStance::LongFuse => "Long fuse",
        CombatStance::DefensiveAutocast => "Defensive cast",
        CombatStance::Autocast => "Autocast",
        CombatStance::ManualCast => "Manual cast",
        CombatStance::None => "None",
    }
}

/// Slots a step leaves untouched, for summaries.
#[allow(dead_code)]
pub fn inherited_slots(step: &SpecStep) -> Vec<GearSlot> {
    SLOTS
        .iter()
        .copied()
        .filter(|slot| {
            *slot != GearSlot::Weapon
                && !step
                    .switches
                    .iter()
                    .any(|item| item.slot.eq_ignore_ascii_case(&slot.to_string()))
        })
        .collect()
}
