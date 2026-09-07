//! Monster selection, scaling, and custom NPCs.

use crate::components::SearchBar;
use dioxus::prelude::*;
use osrs::types::monster::Monster;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::LazyLock;

const MONSTER_JSON: &str = include_str!("../../../assets/json/monsters.json");
static MONSTERS: LazyLock<Vec<MonsterRecord>> = LazyLock::new(|| {
    serde_json::from_str(MONSTER_JSON).unwrap_or_else(|error| {
        log::error!("Could not read monster catalog: {error}");
        Vec::new()
    })
});

/// Attributes the engine reacts to, in the order they are offered for custom NPCs.
pub const ATTRIBUTES: [&str; 15] = [
    "dragon", "demon", "undead", "fiery", "kalphite", "golem", "leafy", "rat", "shade", "spectral",
    "xerician", "penance", "vampyre1", "vampyre2", "vampyre3",
];
const ELEMENTS: [&str; 4] = ["air", "water", "earth", "fire"];

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
struct MonsterInfo {
    id: Option<i32>,
    name: String,
    version: Option<String>,
    combat_level: u32,
    size: u32,
    attributes: Option<Vec<String>>,
    weakness: Option<Weakness>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
struct Weakness {
    element: String,
    severity: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
struct MonsterStats {
    attack: u32,
    strength: u32,
    defence: u32,
    ranged: u32,
    magic: u32,
    hitpoints: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
struct DefenceBonuses {
    stab: i32,
    slash: i32,
    crush: i32,
    light: i32,
    standard: i32,
    heavy: i32,
    magic: i32,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
struct MonsterBonuses {
    defence: DefenceBonuses,
    #[serde(default)]
    flat_armour: i32,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
struct MonsterRecord {
    info: MonsterInfo,
    stats: MonsterStats,
    bonuses: MonsterBonuses,
    #[serde(default)]
    image: String,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl MonsterRecord {
    fn label(&self) -> String {
        match self
            .info
            .version
            .as_deref()
            .filter(|version| !version.is_empty())
        {
            Some(version) => format!("{} · {version}", self.info.name),
            None => self.info.name.clone(),
        }
    }

    fn is_toa(&self) -> bool {
        self.info
            .id
            .is_some_and(|id| osrs::constants::TOA_MONSTERS.contains(&id))
    }
}

/// The selected monster plus the encounter's starting state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TargetConfig {
    monster: MonsterRecord,
    custom: bool,
    origin: Option<String>,
    starting_hp: u32,
    defence_reduction: u32,
    toa_level: u32,
    toa_path_level: u32,
    source_name: Option<String>,
    /// Scaled maximum HP, cached so summaries do not rebuild the engine monster.
    max_hp: u32,
}

impl Default for TargetConfig {
    fn default() -> Self {
        MONSTERS
            .iter()
            .find(|monster| {
                monster.info.name == "Vorkath"
                    && monster.info.version.as_deref() == Some("Post-quest")
            })
            .or_else(|| MONSTERS.first())
            .cloned()
            .map(Self::from_monster)
            .unwrap_or_else(Self::new_custom)
    }
}

impl TargetConfig {
    fn from_monster(monster: MonsterRecord) -> Self {
        let mut config = Self {
            starting_hp: monster.stats.hitpoints,
            source_name: Some(monster.info.name.clone()),
            monster,
            custom: false,
            origin: None,
            defence_reduction: 0,
            toa_level: 0,
            toa_path_level: 0,
            max_hp: 0,
        };
        config.refresh_max_hp();
        config.starting_hp = config.max_hp;
        // The catalog uses zero placeholders for Vardorvis' dynamic stats.
        if config.monster.info.name == "Vardorvis"
            && let Some(monster) = config.scaled_monster()
        {
            config.monster.stats.defence = monster.stats.defence.current;
            config.monster.stats.strength = monster.stats.strength.current;
        }
        config
    }

    fn new_custom() -> Self {
        let mut monster = MonsterRecord::default();
        monster.info.name = "Custom target".to_string();
        monster.info.size = 1;
        monster.stats = MonsterStats {
            attack: 1,
            strength: 1,
            defence: 1,
            ranged: 1,
            magic: 1,
            hitpoints: 100,
        };
        Self {
            custom: true,
            ..Self::from_monster(monster)
        }
    }

    /// Recompute cached values after deserialization.
    pub fn after_load(mut self) -> Self {
        self.refresh_max_hp();
        self
    }

    pub fn starting_hp(&self) -> u32 {
        self.starting_hp
    }

    /// Display name, including the catalog version for non-custom targets.
    pub fn label(&self) -> String {
        if self.custom {
            self.monster.info.name.clone()
        } else {
            self.monster.label()
        }
    }

    /// Non-default encounter settings worth calling out next to the name.
    pub fn overrides(&self) -> Vec<String> {
        let mut overrides = Vec::new();
        if self.starting_hp != self.max_hp {
            overrides.push(format!("{} starting HP", self.starting_hp));
        }
        if self.defence_reduction > 0 {
            overrides.push(format!("−{} Defence", self.defence_reduction));
        }
        if self.monster.is_toa() {
            if self.toa_level > 0 {
                overrides.push(format!("ToA {}", self.toa_level));
            }
            if self.toa_path_level > 0 {
                overrides.push(format!("Path {}", self.toa_path_level));
            }
        }
        if self.custom {
            overrides.push(match &self.origin {
                Some(origin) => format!("Custom from {origin}"),
                None => "Custom NPC".to_string(),
            });
        }
        overrides
    }

    pub(super) fn example(name: &str, toa_level: u32, path: u32, def_reduction: u32) -> Self {
        let matches = |monster: &&MonsterRecord| {
            monster.info.name.eq_ignore_ascii_case(name)
                || monster.label().eq_ignore_ascii_case(name)
        };
        let selected = MONSTERS
            .iter()
            .filter(matches)
            .find(|monster| {
                matches!(
                    monster.info.version.as_deref(),
                    None | Some("") | Some("Normal") | Some("Post-quest")
                )
            })
            .or_else(|| MONSTERS.iter().find(matches));
        let mut config = selected
            .cloned()
            .map(Self::from_monster)
            .unwrap_or_else(|| {
                let mut custom = Self::new_custom();
                custom.monster.info.name = name.to_string();
                custom
            });
        config.set_toa_scaling(toa_level, path);
        config.defence_reduction = def_reduction.min(config.monster.stats.defence);
        config
    }

    /// Rebuild from the local catalog record so custom copies retain fields the
    /// editor does not expose, including immunities and offensive bonuses.
    fn scaled_monster(&self) -> Option<Monster> {
        if !self.rolls_fit() {
            return None;
        }
        let mut record = serde_json::to_value(&self.monster).ok()?;
        let engine_name = if self.custom {
            if self.origin.is_some() {
                self.source_name.as_deref().unwrap_or("Custom target")
            } else {
                "Custom target"
            }
        } else {
            &self.monster.info.name
        };
        let info = record.get_mut("info")?.as_object_mut()?;
        info.insert("name".to_string(), json!(engine_name));
        info.entry("attack_speed").or_insert(json!(4));
        info.entry("attack_styles").or_insert(json!(["Crush"]));
        // Scaling is applied only after the unscaled rolls have been initialized.
        info.insert("toa_level".to_string(), json!(0));
        info.insert("toa_path_level".to_string(), json!(0));
        let bonuses = record.get_mut("bonuses")?.as_object_mut()?;
        bonuses
            .entry("attack")
            .or_insert(json!({"melee": 0, "ranged": 0, "magic": 0}));
        bonuses
            .entry("strength")
            .or_insert(json!({"melee": 0, "ranged": 0, "magic": 0}));
        record
            .as_object_mut()?
            .entry("immunities")
            .or_insert(json!({
                "poison": false, "venom": false, "freeze": 0, "burn": null,
            }));
        let json = serde_json::to_string(&vec![record]).ok()?;
        let mut monster =
            Monster::from_json_str(engine_name, self.monster.info.version.as_deref(), &json)
                .ok()?;
        if self.monster.is_toa() {
            monster.set_toa_level(self.toa_level.min(600), self.toa_path_level.min(6));
        }
        osrs::calc::monster_scaling::scale_monster_hp_only(&mut monster, false);
        Some(monster)
    }

    fn rolls_fit(&self) -> bool {
        // The engine calculates rolls in i32, including multiplication before
        // dividing for raid scaling. Reject oversized custom inputs first.
        let stats = &self.monster.stats;
        let level = [
            stats.attack,
            stats.strength,
            stats.defence,
            stats.ranged,
            stats.magic,
        ]
        .into_iter()
        .max()
        .unwrap_or(0) as i64
            + 9;
        let defence = &self.monster.bonuses.defence;
        let largest_defence = [
            defence.stab,
            defence.slash,
            defence.crush,
            defence.magic,
            defence.light,
            defence.standard,
            defence.heavy,
        ]
        .into_iter()
        .map(|bonus| (i64::from(bonus) + 64).abs())
        .max()
        .unwrap_or(64);
        let scaling = if self.monster.is_toa() {
            1000 + i64::from(self.toa_level.min(600)) * 4
        } else {
            1
        };
        let largest_offence = ["attack", "strength"]
            .into_iter()
            .filter_map(|name| {
                self.monster
                    .bonuses
                    .extra
                    .get(name)
                    .and_then(Value::as_object)
            })
            .flat_map(|bonuses| bonuses.values())
            .filter_map(Value::as_i64)
            .map(|bonus| (bonus + 64).abs())
            .max()
            .unwrap_or(64);
        level * largest_defence * scaling <= i64::from(i32::MAX)
            && level * largest_offence + 320 <= i64::from(i32::MAX)
    }

    fn refresh_max_hp(&mut self) {
        self.max_hp = self
            .scaled_monster()
            .map(|monster| monster.stats.hitpoints.current)
            .unwrap_or(self.monster.stats.hitpoints)
            .max(1);
    }

    fn set_toa_scaling(&mut self, level: u32, path: u32) {
        let use_default_hp = self.starting_hp == self.max_hp;
        self.toa_level = level.min(600);
        self.toa_path_level = path.min(6);
        self.refresh_max_hp();
        if use_default_hp {
            self.starting_hp = self.max_hp;
        }
    }

    /// The engine monster at the encounter's starting state, for the calculator.
    pub fn combat_monster(&self) -> Option<Monster> {
        let mut monster = self.scaled_monster()?;
        // HP-dependent attacks need the encounter's scaled maximum HP.
        monster.stats.hitpoints.base = monster.stats.hitpoints.current;
        if self.starting_hp > monster.stats.hitpoints.base {
            return None;
        }
        self.apply_starting_state(&mut monster);
        Some(monster)
    }

    /// The engine monster for simulations. Base stats stay unscaled so the
    /// engine's per-fight reset re-applies raid scaling correctly.
    pub fn simulation_monster(&self) -> Option<Monster> {
        let mut monster = self.scaled_monster()?;
        if self.starting_hp > monster.stats.hitpoints.current {
            return None;
        }
        self.apply_starting_state(&mut monster);
        Some(monster)
    }

    /// Put a freshly reset monster into the encounter's starting state. This
    /// describes an explicit starting state, independent of the attack or
    /// strategy that produced the reduction.
    pub fn apply_starting_state(&self, monster: &mut Monster) {
        let maximum = monster.stats.hitpoints.current.max(1);
        monster.stats.hitpoints.current = self.starting_hp.clamp(1, maximum);
        osrs::calc::monster_scaling::scale_monster_hp_only(monster, false);
        monster.stats.defence.current = monster
            .stats
            .defence
            .current
            .saturating_sub(self.defence_reduction);
        monster.recalculate_def_rolls();
    }

    fn set_stat(&mut self, index: usize, value: u32) {
        let use_default_hp = self.starting_hp == self.max_hp;
        let stats = &mut self.monster.stats;
        match index {
            0 => stats.attack = value,
            1 => stats.strength = value,
            2 => {
                stats.defence = value;
                self.defence_reduction = self.defence_reduction.min(value);
            }
            3 => stats.ranged = value,
            4 => stats.magic = value,
            _ => {
                stats.hitpoints = value.max(1);
            }
        }
        if index == 5 {
            self.refresh_max_hp();
            if use_default_hp {
                self.starting_hp = self.max_hp;
            }
        }
    }

    fn set_defence_bonus(&mut self, index: usize, value: i32) {
        let defence = &mut self.monster.bonuses.defence;
        match index {
            0 => defence.stab = value,
            1 => defence.slash = value,
            2 => defence.crush = value,
            3 => defence.magic = value,
            4 => defence.light = value,
            5 => defence.standard = value,
            _ => defence.heavy = value,
        }
    }

    fn toggle_attribute(&mut self, attribute: &str) {
        let attributes = self.monster.info.attributes.get_or_insert_with(Vec::new);
        if let Some(position) = attributes.iter().position(|a| a == attribute) {
            attributes.remove(position);
        } else {
            attributes.push(attribute.to_string());
        }
    }
}

/// (label, icon file stem) for the six monster combat stats, in display order.
const STAT_ICONS: [(&str, &str); 6] = [
    ("Attack", "attack"),
    ("Strength", "strength"),
    ("Defence", "defence"),
    ("Ranged", "ranged"),
    ("Magic", "magic"),
    ("Hitpoints", "hitpoints"),
];

/// (label, icon file name) for the seven defensive bonuses, in display order.
const DEFENCE_ICONS: [(&str, &str); 7] = [
    ("Stab", "dagger.png"),
    ("Slash", "scimitar.png"),
    ("Crush", "warhammer.png"),
    ("Magic", "magic.png"),
    ("Light", "ranged_light.webp"),
    ("Standard", "ranged_standard.webp"),
    ("Heavy", "ranged_heavy.webp"),
];

fn filter_monster(monster: &MonsterRecord, term: &str) -> bool {
    monster.label().to_lowercase().contains(term)
}

fn monster_key(monster: &MonsterRecord) -> String {
    format!(
        "{}-{}-{}-{}",
        monster.info.id.unwrap_or_default(),
        monster.label(),
        monster.stats.hitpoints,
        monster.stats.defence
    )
}

fn render_monster(monster: &MonsterRecord) -> Element {
    let version = monster.info.version.as_deref().unwrap_or("Standard");
    rsx! {
        div { class: "home-search-item",
            span { "{monster.info.name}" }
            small { "{version} · {monster.stats.hitpoints} HP · Lv. {monster.info.combat_level}" }
        }
    }
}

#[component]
pub fn TargetPanel(
    mut target: Signal<TargetConfig>,
    monster: ReadSignal<Option<Monster>>,
) -> Element {
    let current = target.read().clone();
    let info = &current.monster.info;
    let version = info.version.as_deref().unwrap_or("Standard");
    let stats = &current.monster.stats;
    let defence = &current.monster.bonuses.defence;
    let stat_values = [
        stats.attack,
        stats.strength,
        stats.defence,
        stats.ranged,
        stats.magic,
        stats.hitpoints,
    ];
    let defence_values = [
        defence.stab,
        defence.slash,
        defence.crush,
        defence.magic,
        defence.light,
        defence.standard,
        defence.heavy,
    ];
    let attributes = info.attributes.clone().unwrap_or_default();
    let is_toa = current.monster.is_toa();
    let editable = current.custom;
    let starting_defence = monster
        .read()
        .as_ref()
        .map(|monster| monster.stats.defence.current)
        .unwrap_or_else(|| stats.defence.saturating_sub(current.defence_reduction));
    let invalid = editable && monster.read().is_none();
    let weakness_element = info
        .weakness
        .as_ref()
        .map(|weakness| weakness.element.clone())
        .unwrap_or_else(|| "none".to_string());
    let weakness_severity = info
        .weakness
        .as_ref()
        .map(|weakness| weakness.severity)
        .unwrap_or(0);
    let sprite = (!current.monster.image.is_empty())
        .then(|| format!("{}/{}", crate::MONSTERS_ASSETS, current.monster.image));

    rsx! {
        section {
            class: "card home-panel target-panel",
            aria_label: "Target configuration",
            header { class: "home-panel-header",
                h2 { class: "card-title", "Monster" }
                div { class: "home-panel-actions",
                    if !editable {
                        button {
                            class: "home-text-button",
                            r#type: "button",
                            title: "Edit a copy of this monster's stats",
                            onclick: move |_| {
                                let mut config = target.write();
                                config.origin = Some(config.monster.label());
                                config.custom = true;
                            },
                            "✎ Edit stats"
                        }
                    }
                    button {
                        class: "home-text-button",
                        r#type: "button",
                        onclick: move |_| target.set(TargetConfig::new_custom()),
                        "New custom"
                    }
                }
            }
            div { class: "target-search",
                SearchBar {
                    items: MONSTERS.clone(),
                    filter_fn: filter_monster,
                    render_item: render_monster,
                    get_key: monster_key,
                    on_select: move |monster| target.set(TargetConfig::from_monster(monster)),
                    placeholder: "Search monsters…".to_string(),
                    max_results: 20,
                }
                if MONSTERS.is_empty() {
                    p { class: "home-muted", "Monster catalog unavailable" }
                }
            }

            div { class: "target-identity",
                div { class: "target-identity-text",
                    if editable {
                        label { class: "target-custom-name",
                            span { class: "home-sr-only", "Custom target name" }
                            input {
                                class: "input-field",
                                value: "{info.name}",
                                maxlength: "80",
                                oninput: move |event| target.write().monster.info.name = event.value(),
                            }
                        }
                        p { class: "target-meta",
                            match &current.origin {
                                Some(origin) => rsx! { "Editing a copy of {origin}" },
                                None => rsx! { "Custom NPC" },
                            }
                        }
                    } else {
                        h3 { class: "target-name", "{info.name}" }
                        p { class: "target-meta",
                            "{version} · Lv. {info.combat_level} · {info.size}×{info.size}"
                        }
                    }
                }
                if let Some(sprite) = sprite {
                    img {
                        class: "target-sprite",
                        src: "{sprite}",
                        alt: "",
                        loading: "lazy",
                    }
                }
            }

            if editable {
                div { class: "target-section-label", "Attributes" }
                div { class: "target-attribute-picker",
                    for attribute in ATTRIBUTES {
                        button {
                            key: "{attribute}",
                            r#type: "button",
                            class: if attributes.iter().any(|a| a == attribute) { "home-chip is-toggle is-on" } else { "home-chip is-toggle" },
                            aria_pressed: attributes.iter().any(|a| a == attribute),
                            onclick: move |_| target.write().toggle_attribute(attribute),
                            "{attribute}"
                        }
                    }
                }
                div { class: "target-pair target-weakness-editor",
                    label {
                        span { "Elemental weakness" }
                        select {
                            class: "input-field",
                            value: "{weakness_element}",
                            onchange: move |event| {
                                let mut config = target.write();
                                let element = event.value();
                                config.monster.info.weakness = if element == "none" {
                                    None
                                } else {
                                    Some(Weakness {
                                        element,
                                        severity: config
                                            .monster
                                            .info
                                            .weakness
                                            .as_ref()
                                            .map(|w| w.severity)
                                            .unwrap_or(50),
                                    })
                                };
                            },
                            option {
                                value: "none",
                                selected: weakness_element == "none",
                                "None"
                            }
                            for element in ELEMENTS {
                                option {
                                    value: element,
                                    selected: weakness_element == element,
                                    "{element}"
                                }
                            }
                        }
                    }
                    if info.weakness.is_some() {
                        label {
                            span { "Severity" }
                            input {
                                class: "input-field num",
                                r#type: "number",
                                min: "0",
                                max: "500",
                                value: "{weakness_severity}",
                                oninput: move |event| {
                                    if let Ok(value) = event.value().parse::<u32>()
                                        && let Some(weakness) = target.write().monster.info.weakness.as_mut()
                                    {
                                        weakness.severity = value.min(500);
                                    }
                                },
                            }
                        }
                    }
                }
                if invalid {
                    p { class: "home-error", role: "status",
                        "The engine rejected this NPC. Check its stats, bonuses and starting HP."
                    }
                }
            } else {
                div { class: "target-attributes",
                    if attributes.is_empty() {
                        span { class: "home-muted", "No attributes" }
                    }
                    for attribute in attributes {
                        span { class: "home-chip", "{attribute}" }
                    }
                    if let Some(weakness) = &info.weakness {
                        if weakness.element != "none" {
                            span { class: "home-chip is-note",
                                "{weakness.element} weakness +{weakness.severity}%"
                            }
                        }
                    }
                }
            }

            div { class: "target-section-label", "Base stats" }
            div { class: "target-stats",
                for (index , (label , icon)) in STAT_ICONS.into_iter().enumerate() {
                    StatRow {
                        key: "stat-{label}",
                        icon: format!("{}/{icon}.png", crate::BONUSES_ASSETS),
                        label,
                        value: stat_values[index] as i64,
                        editable,
                        minimum: if index == 5 { 1 } else { 0 },
                        on_change: move |value: i64| target.write().set_stat(index, value as u32),
                    }
                }
            }
            div { class: "target-section-label", "Defensive bonuses" }
            div { class: "target-stats",
                for (index , (label , icon)) in DEFENCE_ICONS.into_iter().enumerate() {
                    StatRow {
                        key: "defence-{label}",
                        icon: format!("{}/{icon}", crate::BONUSES_ASSETS),
                        label,
                        value: defence_values[index] as i64,
                        editable,
                        minimum: -10000,
                        signed: true,
                        on_change: move |value: i64| target.write().set_defence_bonus(index, value as i32),
                    }
                }
                if editable {
                    StatRow {
                        icon: format!("{}/flat_armour.png", crate::BONUSES_ASSETS),
                        label: "Flat armour",
                        value: current.monster.bonuses.flat_armour as i64,
                        editable: true,
                        minimum: -10000,
                        signed: true,
                        on_change: move |value: i64| target.write().monster.bonuses.flat_armour = value as i32,
                    }
                    StatRow {
                        icon: format!("{}/attack_speed.png", crate::BONUSES_ASSETS),
                        label: "Size",
                        value: info.size as i64,
                        editable: true,
                        minimum: 1,
                        maximum: 10,
                        on_change: move |value: i64| target.write().monster.info.size = value as u32,
                    }
                } else if current.monster.bonuses.flat_armour != 0 {
                    StatRow {
                        icon: format!("{}/flat_armour.png", crate::BONUSES_ASSETS),
                        label: "Flat armour",
                        value: current.monster.bonuses.flat_armour as i64,
                        signed: true,
                        on_change: move |_: i64| {},
                    }
                }
            }

            div { class: "target-section-label", "Encounter" }
            div { class: "target-pair",
                TargetNumber {
                    label: "Starting HP",
                    value: current.starting_hp as i64,
                    minimum: 1,
                    maximum: current.max_hp as i64,
                    on_change: move |value: i64| target.write().starting_hp = value as u32,
                }
                TargetNumber {
                    label: "Defence reduction",
                    value: current.defence_reduction as i64,
                    minimum: 0,
                    maximum: stats.defence as i64,
                    on_change: move |value: i64| {
                        let mut config = target.write();
                        config.defence_reduction = (value as u32).min(config.monster.stats.defence);
                    },
                }
            }
            if is_toa {
                div { class: "target-pair",
                    TargetNumber {
                        label: "ToA invocation",
                        value: current.toa_level as i64,
                        minimum: 0,
                        maximum: 600,
                        on_change: move |value: i64| {
                            let mut config = target.write();
                            let path = config.toa_path_level;
                            config.set_toa_scaling(value as u32, path);
                        },
                    }
                    TargetNumber {
                        label: "Path level",
                        value: current.toa_path_level as i64,
                        minimum: 0,
                        maximum: 6,
                        on_change: move |value: i64| {
                            let mut config = target.write();
                            let level = config.toa_level;
                            config.set_toa_scaling(level, value as u32);
                        },
                    }
                }
            }
            p { class: "target-note",
                "Starting Defence {starting_defence}"
                if is_toa || current.starting_hp != current.max_hp {
                    " · max HP {current.max_hp}"
                }
            }
        }
    }
}

/// One `icon · label · value` row. The value becomes an input in custom mode so
/// the editable and read-only panels stay the same height.
#[component]
fn StatRow(
    icon: String,
    label: &'static str,
    value: i64,
    #[props(default = false)] editable: bool,
    #[props(default = false)] signed: bool,
    #[props(default = 0)] minimum: i64,
    #[props(default = 1_000_000)] maximum: i64,
    on_change: EventHandler<i64>,
) -> Element {
    rsx! {
        div { class: "target-stat",
            img { src: "{icon}", alt: "" }
            span { "{label}" }
            if editable {
                input {
                    class: "input-field num",
                    r#type: "number",
                    min: "{minimum}",
                    max: "{maximum}",
                    value: "{value}",
                    aria_label: "{label}",
                    oninput: move |event| {
                        if let Ok(value) = event.value().parse::<i64>() {
                            on_change.call(value.clamp(minimum, maximum));
                        }
                    },
                }
            } else if signed {
                strong { class: "num", "{value:+}" }
            } else {
                strong { class: "num", "{value}" }
            }
        }
    }
}

#[component]
fn TargetNumber(
    label: &'static str,
    value: i64,
    minimum: i64,
    #[props(default = 1_000_000)] maximum: i64,
    on_change: EventHandler<i64>,
) -> Element {
    rsx! {
        label { class: "target-number",
            span { "{label}" }
            input {
                class: "input-field num",
                r#type: "number",
                min: "{minimum}",
                max: "{maximum}",
                value: "{value}",
                oninput: move |event| {
                    if let Ok(value) = event.value().parse::<i64>() {
                        on_change.call(value.clamp(minimum, maximum));
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_hp_applies_vardorvis_scaling_before_reductions() {
        let mut target = TargetConfig::example("Vardorvis · Post-quest", 0, 0, 10);
        target.starting_hp = 350;
        let monster = target.combat_monster().unwrap();
        assert_eq!(monster.stats.strength.current, 315);
        assert_eq!(monster.stats.defence.current, 170);
    }

    #[test]
    fn toa_scaled_max_hp_is_used_for_sun_keris_threshold() {
        use osrs::types::{
            equipment::{CombatStyle, Weapon},
            player::Player,
        };
        let mut target = TargetConfig::example("Zebak", 300, 2, 0);
        let maximum = target.max_hp;
        assert!(maximum > target.monster.stats.hitpoints);
        let mut player = Player::default();
        player
            .equip_item(Box::new(
                Weapon::new("Keris partisan of the sun", None).unwrap(),
            ))
            .unwrap();
        player.set_active_style(CombatStyle::Lunge);
        target.starting_hp = maximum / 4;
        let before = super::super::metrics::calculate(&player, &target, None).unwrap();
        target.starting_hp -= 1;
        let after = super::super::metrics::calculate(&player, &target, None).unwrap();
        assert_eq!(after.attack_roll, before.attack_roll * 5 / 4);
        assert!(after.accuracy > before.accuracy);
    }

    #[test]
    fn custom_roll_overflow_returns_unavailable() {
        let mut target = TargetConfig::new_custom();
        target.monster.stats.defence = 1_000_000;
        target.monster.bonuses.defence.stab = 1_000_000;
        assert!(target.combat_monster().is_none());
    }

    #[test]
    fn target_round_trips_through_json() {
        let target = TargetConfig::example("Zebak", 300, 2, 0);
        let json = serde_json::to_string(&target).unwrap();
        let parsed: TargetConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.after_load(), target);
    }
}
