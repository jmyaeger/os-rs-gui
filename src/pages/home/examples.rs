//! Ready-made setups used to demonstrate the page and to seed an empty editor.

use super::spec::{BaseStats, GearItem, LoadoutSpec};
use super::strategy::{SpecCondition, SpecPlan, SpecStep};
use super::target::TargetConfig;
use crate::components::equipment_catalog;
use osrs::types::equipment::CombatStyle;
use osrs::types::prayers::Prayer;

pub struct Example {
    pub name: &'static str,
    pub loadout: LoadoutSpec,
    pub target: TargetConfig,
    pub plan: SpecPlan,
    /// Catalog lookups that found nothing. Empty unless the item data has been
    /// regenerated with different names; the test below guards against that.
    #[cfg_attr(not(test), allow(dead_code))]
    pub missing: Vec<String>,
}

fn item(name: &str, version: Option<&str>) -> Option<GearItem> {
    equipment_catalog()
        .iter()
        .find(|item| item.name == name && item.version.as_deref() == version)
        .map(GearItem::from_catalog)
}

fn gear(items: &[(&str, Option<&str>)]) -> Vec<GearItem> {
    items
        .iter()
        .filter_map(|(name, version)| item(name, *version))
        .collect()
}

/// Names in `items` that the equipment catalog does not contain.
fn missing_items(items: &[&[(&str, Option<&str>)]]) -> Vec<String> {
    items
        .iter()
        .flat_map(|group| group.iter())
        .filter(|(name, version)| item(name, *version).is_none())
        .map(|(name, version)| match version {
            Some(version) => format!("{name} ({version})"),
            None => (*name).to_string(),
        })
        .collect()
}

const SWITCH_GEAR: [(&str, Option<&str>); 7] = [
    ("Torva full helm", None),
    ("Amulet of torture", None),
    ("Torva platebody", None),
    ("Avernic defender", None),
    ("Torva platelegs", None),
    ("Ferocious gloves", None),
    ("Primordial boots", None),
];

const MELEE_GEAR: [(&str, Option<&str>); 10] = [
    ("Torva full helm", None),
    ("Infernal cape", None),
    ("Amulet of torture", None),
    ("Osmumten's fang", None),
    ("Torva platebody", None),
    ("Avernic defender", None),
    ("Torva platelegs", None),
    ("Ferocious gloves", None),
    ("Primordial boots", None),
    ("Ultor ring", None),
];

// Names come from the engine's catalog, which is also what the search offers and
// what a restored loadout resolves against, so a name here is a name everywhere.
const RANGED_GEAR: [(&str, Option<&str>); 9] = [
    ("Crystal helm", Some("Active")),
    ("Ava's assembler", None),
    ("Necklace of anguish", None),
    ("Bow of Faerdhinen", Some("Charged")),
    ("Crystal body", Some("Active")),
    ("Crystal legs", Some("Active")),
    ("Barrows gloves", None),
    ("Pegasian boots", None),
    ("Ring of suffering (i)", Some("Recoil")),
];

const MAGIC_GEAR: [(&str, Option<&str>); 9] = [
    ("Ancestral hat", None),
    ("Imbued Saradomin cape", None),
    ("Occult necklace", None),
    ("Tumeken's shadow", Some("Charged")),
    ("Ancestral robe top", None),
    ("Ancestral robe bottom", None),
    ("Tormented bracelet", None),
    ("Eternal boots", None),
    ("Magus ring", None),
];

const MELEE_SPECS: [(&str, Option<&str>); 1] = [("Bandos godsword", None)];
const MAGIC_SPECS: [(&str, Option<&str>); 2] = [("Dragon warhammer", None), ("Voidwaker", None)];

pub fn examples() -> Vec<Example> {
    let torva_switch = gear(&SWITCH_GEAR);

    let melee = LoadoutSpec {
        gear: gear(&MELEE_GEAR),
        prayers: vec![Prayer::Piety],
        potions: vec!["Super combat".into()],
        style: CombatStyle::Lunge,
        ..LoadoutSpec::default()
    };
    let mut melee_plan = SpecPlan::default();
    if let Some(bgs) = item("Bandos godsword", None) {
        melee_plan.steps.push(SpecStep::new(1, bgs));
    }

    let ranged = LoadoutSpec {
        gear: gear(&RANGED_GEAR),
        stats: BaseStats {
            attack: 90,
            strength: 92,
            defence: 85,
            ranged: 95,
            magic: 80,
            hitpoints: 94,
            prayer: 77,
            mining: 72,
            herblore: 82,
        },
        prayers: vec![Prayer::Rigour],
        potions: vec!["Ranging".into()],
        style: CombatStyle::Rapid,
        ..LoadoutSpec::default()
    };

    let magic = LoadoutSpec {
        gear: gear(&MAGIC_GEAR),
        stats: BaseStats {
            prayer: 85,
            mining: 85,
            herblore: 90,
            ..BaseStats::default()
        },
        prayers: vec![Prayer::Augury],
        potions: vec!["Smelling salts".into()],
        style: CombatStyle::Accurate,
        ..LoadoutSpec::default()
    };
    let mut magic_plan = SpecPlan::default();
    if let Some(dwh) = item("Dragon warhammer", None) {
        let mut step = SpecStep::new(1, dwh);
        step.switches = torva_switch.clone();
        step.prayers = vec![Prayer::Piety];
        magic_plan.steps.push(step);
    }
    if let Some(voidwaker) = item("Voidwaker", None) {
        let mut step = SpecStep::new(2, voidwaker);
        step.conditions = vec![SpecCondition::TargetHpBelow(150)];
        step.switches = torva_switch;
        step.prayers = vec![Prayer::Piety];
        magic_plan.steps.push(step);
    }

    vec![
        Example {
            name: "Fang · opening BGS",
            loadout: melee,
            target: TargetConfig::example("General Graardor", 0, 0, 0),
            plan: melee_plan,
            missing: missing_items(&[&MELEE_GEAR, &MELEE_SPECS]),
        },
        Example {
            name: "Bowfa · 95 Ranged",
            loadout: ranged,
            target: TargetConfig::example("General Graardor", 0, 0, 40),
            plan: SpecPlan::default(),
            missing: missing_items(&[&RANGED_GEAR]),
        },
        Example {
            name: "Shadow · DWH then Voidwaker",
            loadout: magic,
            target: TargetConfig::example("Zebak", 300, 2, 0),
            plan: magic_plan,
            missing: missing_items(&[&MAGIC_GEAR, &MAGIC_SPECS, &SWITCH_GEAR]),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_resolve_and_restore_cleanly() {
        for example in examples() {
            assert!(
                example.missing.is_empty(),
                "{}: not in the equipment catalog: {:?}",
                example.name,
                example.missing
            );
            let restored = example.loadout.to_player();
            assert!(
                restored.warnings.is_empty(),
                "{}: {:?}",
                example.name,
                restored.warnings
            );
            let equipped = super::super::spec::SLOTS
                .iter()
                .filter(|slot| {
                    restored
                        .player
                        .get_slot(slot)
                        .is_some_and(|item| item.name() != "Unarmed")
                })
                .count();
            assert_eq!(example.loadout.gear.len(), equipped, "{}", example.name);
            assert_ne!(
                restored.player.gear.weapon.name, "Unarmed",
                "{} lost its weapon",
                example.name
            );
        }
    }
}
