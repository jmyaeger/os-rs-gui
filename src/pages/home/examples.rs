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

pub fn examples() -> Vec<Example> {
    let torva_switch = gear(&[
        ("Torva full helm", None),
        ("Amulet of torture", None),
        ("Torva platebody", None),
        ("Avernic defender", None),
        ("Torva platelegs", None),
        ("Ferocious gloves", None),
        ("Primordial boots", None),
    ]);

    let melee = LoadoutSpec {
        gear: gear(&[
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
        ]),
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
        gear: gear(&[
            ("Crystal helm", Some("Active")),
            ("Ava's assembler", None),
            ("Necklace of anguish", None),
            ("Bow of faerdhinen", Some("Charged")),
            ("Crystal body", Some("Active")),
            ("Crystal legs", Some("Active")),
            ("Barrows gloves", None),
            ("Pegasian boots", None),
            ("Ring of suffering (i)", Some("Recoil")),
        ]),
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
        gear: gear(&[
            ("Ancestral hat", None),
            ("Imbued saradomin cape", None),
            ("Occult necklace", None),
            ("Tumeken's shadow", Some("Charged")),
            ("Ancestral robe top", None),
            ("Ancestral robe bottom", None),
            ("Tormented bracelet", None),
            ("Eternal boots", None),
            ("Magus ring", None),
        ]),
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
        step.prayer = Some(Prayer::Piety);
        magic_plan.steps.push(step);
    }
    if let Some(voidwaker) = item("Voidwaker", None) {
        let mut step = SpecStep::new(2, voidwaker);
        step.conditions = vec![SpecCondition::TargetHpBelow(150)];
        step.switches = torva_switch;
        step.prayer = Some(Prayer::Piety);
        magic_plan.steps.push(step);
    }

    vec![
        Example {
            name: "Fang · opening BGS",
            loadout: melee,
            target: TargetConfig::example("General Graardor", 0, 0, 0),
            plan: melee_plan,
        },
        Example {
            name: "Bowfa · 95 Ranged",
            loadout: ranged,
            target: TargetConfig::example("General Graardor", 0, 0, 40),
            plan: SpecPlan::default(),
        },
        Example {
            name: "Shadow · DWH then Voidwaker",
            loadout: magic,
            target: TargetConfig::example("Zebak", 300, 2, 0),
            plan: magic_plan,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_restore_without_warnings() {
        for example in examples() {
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
        }
    }
}
