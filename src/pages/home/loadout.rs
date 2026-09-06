//! Equipment and player panels of the editor.

use super::HomeState;
use super::examples::examples;
use super::simulation::ThrallChoice;
use super::spec::{Conditions, OFFENSIVE_PRAYERS, SLOTS, active_potions, all_spells};
use crate::components::{EquipmentGrid, EquipmentSelect, PrayerSelect, ensure_style};
use crate::hiscores::fetch_player_stats;
use dioxus::prelude::*;
use osrs::types::equipment::{CombatStance, CombatStyle, GearSlot};
use osrs::types::player::Player;
use osrs::types::potions::Potion;
use osrs::types::stats::Stat;
use strum::IntoEnumIterator;

#[component]
pub fn EquipmentPanel() -> Element {
    let mut player = use_context::<Signal<Player>>();
    let mut styles: Vec<_> = player
        .read()
        .gear
        .weapon
        .combat_styles
        .keys()
        .copied()
        .collect();
    styles.sort_by_key(ToString::to_string);
    let active_style = player.read().attrs.active_style;
    let empty = SLOTS.iter().all(|slot| {
        player
            .read()
            .get_slot(slot)
            .is_none_or(|item| item.name() == "Unarmed")
    });
    let style_details = player
        .read()
        .gear
        .weapon
        .combat_styles
        .get(&active_style)
        .map(|option| format!("{} · {}", option.combat_type, stance_label(option.stance)))
        .unwrap_or_else(|| "Choose an attack style".to_string());
    let speed = player.read().gear.weapon.speed;
    let casting = matches!(
        active_style,
        CombatStyle::Spell | CombatStyle::DefensiveSpell | CombatStyle::ManualCast
    );
    let spell_name = player
        .read()
        .attrs
        .spell
        .map(|spell| spell.to_string())
        .unwrap_or_default();

    rsx! {
        section { class: "card home-panel loadout-equipment",
            header { class: "home-panel-header",
                h2 { class: "card-title", "Equipment" }
                div { class: "home-panel-actions",
                    if empty {
                        button { class: "home-text-button",
                            onclick: move |_| {
                                if let Some(example) = examples().into_iter().next() {
                                    let restored = example.loadout.to_player();
                                    player.set(restored.player);
                                }
                            },
                            "Load example gear"
                        }
                    } else {
                        button { class: "home-text-button",
                            onclick: move |_| {
                                let mut player = player.write();
                                for slot in SLOTS {
                                    if slot != GearSlot::Weapon {
                                        player.unequip_slot(&slot);
                                    }
                                }
                                player.unequip_slot(&GearSlot::Weapon);
                            },
                            "Clear"
                        }
                    }
                }
            }
            div { class: "loadout-search", EquipmentSelect {} }
            div { class: "loadout-gear-row",
                div { class: "loadout-grid", EquipmentGrid {} }
                EquipmentBonuses {}
            }
            div { class: "loadout-style-row",
                label { r#for: "loadout-attack-style", "Attack style" }
                select {
                    id: "loadout-attack-style",
                    class: "input-field",
                    value: "{active_style}",
                    onchange: move |event| {
                        if let Some(style) = styles.iter().find(|style| style.to_string() == event.value()) {
                            player.write().set_active_style(*style);
                        }
                    },
                    for style in player.read().gear.weapon.combat_styles.keys().copied().collect::<Vec<_>>() {
                        option { value: "{style}", selected: style == active_style, "{style}" }
                    }
                }
            }
            div { class: "loadout-style-details",
                span { "{style_details}" }
                span { class: "num", "{speed}-tick" }
            }
            if casting || !spell_name.is_empty() {
                div { class: "loadout-style-row",
                    label { r#for: "loadout-spell", "Spell" }
                    select {
                        id: "loadout-spell",
                        class: "input-field",
                        value: "{spell_name}",
                        onchange: move |event| {
                            let spell = all_spells().into_iter().find(|spell| spell.to_string() == event.value());
                            let mut current = player.write();
                            current.attrs.spell = spell;
                            ensure_style(&mut current);
                        },
                        option { value: "", "Select a spell" }
                        for spell in all_spells() {
                            option { value: "{spell}", "{spell}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EquipmentBonuses() -> Element {
    let player = use_context::<Signal<Player>>();
    let bonuses = player.read().bonuses.clone();
    let styles = [
        ("Stab", "dagger", bonuses.attack.stab, bonuses.defence.stab),
        (
            "Slash",
            "scimitar",
            bonuses.attack.slash,
            bonuses.defence.slash,
        ),
        (
            "Crush",
            "warhammer",
            bonuses.attack.crush,
            bonuses.defence.crush,
        ),
        (
            "Range",
            "ranged",
            bonuses.attack.ranged,
            bonuses.defence.ranged,
        ),
        (
            "Magic",
            "magic",
            bonuses.attack.magic,
            bonuses.defence.magic,
        ),
    ];
    let other = [
        (
            "Melee str.",
            "strength",
            format!("{:+}", bonuses.strength.melee),
        ),
        (
            "Range str.",
            "ranged_strength",
            format!("{:+}", bonuses.strength.ranged),
        ),
        (
            "Magic dmg.",
            "magic_strength",
            format!("{:+}%", bonuses.strength.magic),
        ),
        ("Prayer", "prayer", format!("{:+}", bonuses.prayer)),
    ];
    rsx! {
        div { class: "loadout-bonuses",
            table {
                thead {
                    tr {
                        th { scope: "col", aria_label: "Combat style" }
                        th { scope: "col", title: "Attack bonus", "Atk" }
                        th { scope: "col", title: "Defence bonus", "Def" }
                    }
                }
                tbody {
                    for (label, icon, attack, defence) in styles {
                        tr { key: "{label}",
                            th { scope: "row",
                                img { src: format!("{}/{icon}.png", crate::BONUSES_ASSETS), alt: "" }
                                "{label}"
                            }
                            td { class: "num", "{attack:+}" }
                            td { class: "num", "{defence:+}" }
                        }
                    }
                }
            }
            dl { class: "loadout-other-bonuses",
                for (label, icon, value) in other {
                    div { key: "{label}",
                        dt {
                            img { src: format!("{}/{icon}.png", crate::BONUSES_ASSETS), alt: "" }
                            "{label}"
                        }
                        dd { class: "num", "{value}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn PlayerPanel() -> Element {
    let mut player = use_context::<Signal<Player>>();
    let mut state = use_context::<HomeState>();
    let thrall = state.sim.read().thrall;
    let mut show_import = use_signal(|| false);
    let mut show_more = use_signal(|| false);
    let prayers: Vec<_> = OFFENSIVE_PRAYERS
        .iter()
        .copied()
        .filter(|prayer| player.read().prayers.contains_prayer(*prayer))
        .collect();
    let potions = active_potions(&player.read());
    let available_potions: Vec<_> = Potion::iter()
        .filter(|potion| *potion != Potion::None && !potions.contains(potion))
        .collect();
    let conditions = Conditions::from_boosts(&player.read().boosts);
    let more_active = conditions.in_multi
        || conditions.forinthry_surge
        || conditions.charge_active
        || conditions.mark_of_darkness
        || conditions.sunfire
        || conditions.soulreaper_stacks > 0
        || thrall.is_some();

    rsx! {
        section { class: "card home-panel loadout-player",
            header { class: "home-panel-header",
                h2 { class: "card-title", "Player" }
                div { class: "home-panel-actions",
                    button {
                        class: "home-text-button",
                        aria_expanded: "{show_import}",
                        onclick: move |_| show_import.toggle(),
                        "Import stats"
                    }
                }
            }
            if show_import() { HiscoreImport {} }
            div { class: "loadout-section-heading",
                h3 { "Stats" }
                span { class: "home-muted", "base → boosted" }
            }
            div { class: "loadout-stats-grid",
                for skill in SKILLS {
                    SkillField { key: "{skill.name()}", skill }
                }
            }
            div { class: "loadout-boosts",
                div { class: "loadout-prayers",
                    div { class: "loadout-section-heading", h3 { "Prayers" } }
                    PrayerSelect { show_header: false }
                    p { class: "loadout-prayer-summary",
                        if prayers.is_empty() { "None" }
                        else { "{prayers.iter().map(ToString::to_string).collect::<Vec<_>>().join(\", \")}" }
                    }
                }
                div { class: "loadout-consumables",
                    div { class: "loadout-section-heading",
                        h3 { "Potions" }
                        select {
                            key: "add-potion-{potions.len()}",
                            class: "home-add-select",
                            aria_label: "Add potion or boost",
                            value: "",
                            onchange: move |event| {
                                if let Some(potion) = Potion::iter().find(|potion| potion.to_string() == event.value())
                                    && potion != Potion::None
                                    && !active_potions(&player.read()).contains(&potion)
                                {
                                    player.write().add_potion(potion);
                                }
                            },
                            option { value: "", "+ Add" }
                            for potion in available_potions {
                                option { value: "{potion}", "{potion}" }
                            }
                        }
                    }
                    div { class: "loadout-chips",
                        if potions.is_empty() { span { class: "home-muted", "None" } }
                        for potion in potions {
                            button {
                                class: "home-chip is-removable",
                                aria_label: "Remove {potion}",
                                onclick: move |_| player.write().remove_potion(potion),
                                img { src: format!("{}/{}.png", crate::POTIONS_ASSETS, potion.to_string().replace(" (-)", "").replace(" (+)", "")), alt: "" }
                                "{potion}"
                                span { class: "home-chip-remove", "×" }
                            }
                        }
                    }
                    div { class: "loadout-section-heading loadout-conditions-heading",
                        h3 { "Conditions" }
                        button {
                            class: "home-text-button",
                            aria_expanded: "{show_more}",
                            onclick: move |_| show_more.toggle(),
                            if show_more() { "Fewer" } else if more_active { "More (active)" } else { "More" }
                        }
                    }
                    div { class: "loadout-conditions",
                        ConditionToggle { label: "Slayer task", enabled: conditions.on_task, on_change: move |checked| player.write().boosts.on_task = checked }
                        ConditionToggle { label: "Wilderness", enabled: conditions.in_wilderness, on_change: move |checked| player.write().boosts.in_wilderness = checked }
                        ConditionToggle { label: "Kandarin hard diary", enabled: conditions.kandarin_diary, on_change: move |checked| player.write().boosts.kandarin_diary = checked }
                        if show_more() {
                            ConditionToggle { label: "Multicombat", enabled: conditions.in_multi, on_change: move |checked| player.write().boosts.in_multi = checked }
                            ConditionToggle { label: "Forinthry surge", enabled: conditions.forinthry_surge, on_change: move |checked| player.write().boosts.forinthry_surge = checked }
                            ConditionToggle { label: "Charge spell", enabled: conditions.charge_active, on_change: move |checked| player.write().boosts.charge_active = checked }
                            ConditionToggle { label: "Mark of Darkness", enabled: conditions.mark_of_darkness, on_change: move |checked| player.write().boosts.mark_of_darkness = checked }
                            ConditionToggle { label: "Sunfire runes", enabled: conditions.sunfire, on_change: move |checked| player.write().boosts.sunfire.active = checked }
                            label { class: "loadout-stack-field",
                                span { "Thrall" }
                                select {
                                    class: "input-field",
                                    value: thrall.map(ThrallChoice::key).unwrap_or(""),
                                    title: "Thralls only affect simulations, not the live main-weapon numbers",
                                    onchange: move |event| state.sim.write().thrall = ThrallChoice::from_key(&event.value()),
                                    option { value: "", selected: thrall.is_none(), "None" }
                                    for choice in ThrallChoice::ALL {
                                        option { value: choice.key(), selected: thrall == Some(choice), "{choice.label()}" }
                                    }
                                }
                            }
                            label { class: "loadout-stack-field",
                                span { "Soulreaper stacks" }
                                input {
                                    class: "input-field num",
                                    r#type: "number", min: "0", max: "5",
                                    value: "{conditions.soulreaper_stacks}",
                                    oninput: move |event| {
                                        if let Ok(stacks) = event.value().parse::<u32>() {
                                            player.write().boosts.soulreaper_stacks = stacks.min(5);
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Skill {
    Attack,
    Strength,
    Defence,
    Ranged,
    Magic,
    Hitpoints,
    Prayer,
    Mining,
    Herblore,
}

const SKILLS: [Skill; 9] = [
    Skill::Attack,
    Skill::Strength,
    Skill::Defence,
    Skill::Ranged,
    Skill::Magic,
    Skill::Hitpoints,
    Skill::Prayer,
    Skill::Mining,
    Skill::Herblore,
];

impl Skill {
    fn name(self) -> &'static str {
        match self {
            Self::Attack => "Attack",
            Self::Strength => "Strength",
            Self::Defence => "Defence",
            Self::Ranged => "Ranged",
            Self::Magic => "Magic",
            Self::Hitpoints => "Hitpoints",
            Self::Prayer => "Prayer",
            Self::Mining => "Mining",
            Self::Herblore => "Herblore",
        }
    }

    fn stat(self, player: &Player) -> Stat {
        match self {
            Self::Attack => player.stats.attack,
            Self::Strength => player.stats.strength,
            Self::Defence => player.stats.defence,
            Self::Ranged => player.stats.ranged,
            Self::Magic => player.stats.magic,
            Self::Hitpoints => player.stats.hitpoints,
            Self::Prayer => player.stats.prayer,
            Self::Mining => player.stats.mining,
            Self::Herblore => player.stats.herblore,
        }
    }

    fn set_base(self, player: &mut Player, level: u32) {
        let stat = match self {
            Self::Attack => &mut player.stats.attack,
            Self::Strength => &mut player.stats.strength,
            Self::Defence => &mut player.stats.defence,
            Self::Ranged => &mut player.stats.ranged,
            Self::Magic => &mut player.stats.magic,
            Self::Hitpoints => &mut player.stats.hitpoints,
            Self::Prayer => &mut player.stats.prayer,
            Self::Mining => &mut player.stats.mining,
            Self::Herblore => &mut player.stats.herblore,
        };
        stat.base = level;
        player.calc_potion_boosts();
        player.reset_current_stats(false);
    }
}

#[component]
fn SkillField(skill: Skill) -> Element {
    let mut player = use_context::<Signal<Player>>();
    let stat = skill.stat(&player.read());
    rsx! {
        label { class: "loadout-stat", title: "{skill.name()}",
            img { src: format!("{}/{}.png", crate::BONUSES_ASSETS, skill.name().to_lowercase()), alt: "{skill.name()}" }
            input {
                class: "input-field num",
                r#type: "number", min: "1", max: "99",
                aria_label: "Base {skill.name()} level",
                value: "{stat.base}",
                oninput: move |event| {
                    if let Ok(level) = event.value().parse::<u32>()
                        && (1..=99).contains(&level)
                    {
                        skill.set_base(&mut player.write(), level);
                    }
                },
            }
            span { class: if stat.current > stat.base { "loadout-stat-current num is-boosted" } else { "loadout-stat-current num" }, "{stat.current}" }
        }
    }
}

#[component]
fn ConditionToggle(label: &'static str, enabled: bool, on_change: EventHandler<bool>) -> Element {
    rsx! {
        label { class: if enabled { "home-check is-on" } else { "home-check" },
            input { r#type: "checkbox", checked: enabled, onchange: move |event| on_change.call(event.checked()) }
            span { "{label}" }
        }
    }
}

#[component]
fn HiscoreImport() -> Element {
    let mut player = use_context::<Signal<Player>>();
    let mut name = use_signal(String::new);
    let mut pending = use_signal(|| false);
    let mut status = use_signal(String::new);
    rsx! {
        form { class: "loadout-hiscores",
            onsubmit: move |event| {
                event.prevent_default();
                let rsn = name.read().trim().to_string();
                if rsn.is_empty() || pending() { return; }
                pending.set(true);
                status.set(String::new());
                spawn(async move {
                    match fetch_player_stats(&rsn).await {
                        Ok(stats) => {
                            let mut player = player.write();
                            player.stats = stats;
                            player.attrs.name = Some(rsn.clone());
                            player.calc_potion_boosts();
                            player.reset_current_stats(false);
                            status.set(format!("Imported {rsn}"));
                        }
                        Err(error) => status.set(error),
                    }
                    pending.set(false);
                });
            },
            div {
                input { class: "input-field", aria_label: "RuneScape username", placeholder: "RuneScape username", value: "{name}", oninput: move |event| name.set(event.value()) }
                button { class: "home-button", r#type: "submit", disabled: pending(), if pending() { "Importing…" } else { "Import" } }
            }
            if !status.read().is_empty() { p { class: "home-muted", role: "status", "{status}" } }
        }
    }
}

fn stance_label(stance: CombatStance) -> &'static str {
    match stance {
        CombatStance::None => "None",
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
    }
}
