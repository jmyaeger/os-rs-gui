//! Plain, serializable description of a player loadout.
//!
//! The editor works on the engine's `Player`, which carries caches and
//! function pointers and cannot be serialized. Results, persistence and
//! future permalinks store this spec instead and rebuild a `Player` from it.

use crate::components::preferred_style;
use osrs::types::equipment::{Armor, CombatStyle, Equipment, EquipmentJson, GearSlot, Weapon};
use osrs::types::player::{Player, StatusBoosts};
use osrs::types::potions::Potion;
use osrs::types::prayers::Prayer;
use osrs::types::spells::{AncientSpell, ArceuusSpell, SpecialSpell, Spell, StandardSpell};
use osrs::types::stats::{PlayerStats, Stat};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

pub const SLOTS: [GearSlot; 11] = [
    GearSlot::Head,
    GearSlot::Cape,
    GearSlot::Neck,
    GearSlot::Ammo,
    GearSlot::Weapon,
    GearSlot::Shield,
    GearSlot::Body,
    GearSlot::Legs,
    GearSlot::Hands,
    GearSlot::Feet,
    GearSlot::Ring,
];

/// Prayers the calculator tracks, in the order they are summarized.
pub const TRACKED_PRAYERS: [Prayer; 21] = [
    Prayer::Piety,
    Prayer::Rigour,
    Prayer::Augury,
    Prayer::Chivalry,
    Prayer::Deadeye,
    Prayer::MysticVigour,
    Prayer::IncredibleReflexes,
    Prayer::UltimateStrength,
    Prayer::SteelSkin,
    Prayer::EagleEye,
    Prayer::MysticMight,
    Prayer::ImprovedReflexes,
    Prayer::SuperhumanStrength,
    Prayer::RockSkin,
    Prayer::HawkEye,
    Prayer::MysticLore,
    Prayer::ClarityOfThought,
    Prayer::BurstOfStrength,
    Prayer::ThickSkin,
    Prayer::SharpEye,
    Prayer::MysticWill,
];

/// One equipped item, identified the way the equipment catalog identifies it.
/// Prayers worth swapping to for a special attack. Defence-only prayers are
/// excluded because nothing in a spec calculation reads them.
pub const OFFENSIVE_PRAYERS: [Prayer; 18] = [
    Prayer::Piety,
    Prayer::Rigour,
    Prayer::Augury,
    Prayer::Chivalry,
    Prayer::Deadeye,
    Prayer::MysticVigour,
    Prayer::IncredibleReflexes,
    Prayer::UltimateStrength,
    Prayer::EagleEye,
    Prayer::MysticMight,
    Prayer::ImprovedReflexes,
    Prayer::SuperhumanStrength,
    Prayer::HawkEye,
    Prayer::MysticLore,
    Prayer::ClarityOfThought,
    Prayer::BurstOfStrength,
    Prayer::SharpEye,
    Prayer::MysticWill,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GearItem {
    pub slot: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub image: String,
}

impl GearItem {
    pub fn from_equipped(slot: GearSlot, item: &dyn Equipment) -> Self {
        let version = item
            .as_any()
            .downcast_ref::<Armor>()
            .and_then(|armor| armor.version.clone())
            .or_else(|| {
                item.as_any()
                    .downcast_ref::<Weapon>()
                    .and_then(|weapon| weapon.version.clone())
            });
        Self {
            slot: slot.to_string().to_lowercase(),
            name: item.name().to_string(),
            version,
            image: item.get_image_path().to_string(),
        }
    }

    pub fn from_catalog(item: &EquipmentJson) -> Self {
        Self {
            slot: item.slot.to_lowercase(),
            name: item.name.clone(),
            version: item.version.clone(),
            image: item.image.clone(),
        }
    }

    pub fn is_weapon(&self) -> bool {
        self.slot.eq_ignore_ascii_case("weapon")
    }

    pub fn label(&self) -> String {
        match &self.version {
            Some(version) => format!("{} ({version})", self.name),
            None => self.name.clone(),
        }
    }

    pub fn image_src(&self) -> String {
        format!("{}/{}", crate::EQUIPMENT_ASSETS, self.image)
    }

    pub(super) fn equip_onto(&self, player: &mut Player) -> Result<(), String> {
        // `Weapon::new`/`Armor::new` read the engine's own catalog, which is the
        // same data the search offers, so a stored name always resolves the way
        // it did when it was picked.
        let version = self.version.as_deref();
        let result = if self.is_weapon() {
            Weapon::new(&self.name, version)
                .map_err(|error| error.to_string())
                .and_then(|weapon| {
                    player
                        .equip_item(Box::new(weapon))
                        .map_err(|error| error.to_string())
                })
        } else {
            Armor::new(&self.name, version)
                .map_err(|error| error.to_string())
                .and_then(|armor| {
                    player
                        .equip_item(Box::new(armor))
                        .map_err(|error| error.to_string())
                })
        };
        result.map_err(|error| format!("{}: {error}", self.label()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseStats {
    pub attack: u32,
    pub strength: u32,
    pub defence: u32,
    pub ranged: u32,
    pub magic: u32,
    pub hitpoints: u32,
    pub prayer: u32,
    pub mining: u32,
    pub herblore: u32,
}

impl Default for BaseStats {
    fn default() -> Self {
        Self::from_stats(&Player::default().stats)
    }
}

impl BaseStats {
    pub fn from_stats(stats: &PlayerStats) -> Self {
        Self {
            attack: stats.attack.base,
            strength: stats.strength.base,
            defence: stats.defence.base,
            ranged: stats.ranged.base,
            magic: stats.magic.base,
            hitpoints: stats.hitpoints.base,
            prayer: stats.prayer.base,
            mining: stats.mining.base,
            herblore: stats.herblore.base,
        }
    }

    pub fn from_current(stats: &PlayerStats) -> Self {
        Self {
            attack: stats.attack.current,
            strength: stats.strength.current,
            defence: stats.defence.current,
            ranged: stats.ranged.current,
            magic: stats.magic.current,
            hitpoints: stats.hitpoints.current,
            prayer: stats.prayer.current,
            mining: stats.mining.current,
            herblore: stats.herblore.current,
        }
    }

    /// (label, icon, base, current) in display order.
    pub fn rows(&self, current: &Self) -> [(&'static str, u32, u32); 9] {
        [
            ("Attack", self.attack, current.attack),
            ("Strength", self.strength, current.strength),
            ("Defence", self.defence, current.defence),
            ("Ranged", self.ranged, current.ranged),
            ("Magic", self.magic, current.magic),
            ("Hitpoints", self.hitpoints, current.hitpoints),
            ("Prayer", self.prayer, current.prayer),
            ("Mining", self.mining, current.mining),
            ("Herblore", self.herblore, current.herblore),
        ]
    }

    fn apply(&self, stats: &mut PlayerStats) {
        stats.attack = Stat::from(self.attack);
        stats.strength = Stat::from(self.strength);
        stats.defence = Stat::from(self.defence);
        stats.ranged = Stat::from(self.ranged);
        stats.magic = Stat::from(self.magic);
        stats.hitpoints = Stat::from(self.hitpoints);
        stats.prayer = Stat::from(self.prayer);
        stats.mining = Stat::from(self.mining);
        stats.herblore = Stat::from(self.herblore);
    }
}

/// Situational boosts the calculator exposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Conditions {
    pub on_task: bool,
    pub in_wilderness: bool,
    pub in_multi: bool,
    pub forinthry_surge: bool,
    pub charge_active: bool,
    pub kandarin_diary: bool,
    pub mark_of_darkness: bool,
    pub sunfire: bool,
    pub soulreaper_stacks: u32,
}

impl Default for Conditions {
    fn default() -> Self {
        Self::from_boosts(&StatusBoosts::default())
    }
}

impl Conditions {
    pub fn from_boosts(boosts: &StatusBoosts) -> Self {
        Self {
            on_task: boosts.on_task,
            in_wilderness: boosts.in_wilderness,
            in_multi: boosts.in_multi,
            forinthry_surge: boosts.forinthry_surge,
            charge_active: boosts.charge_active,
            kandarin_diary: boosts.kandarin_diary,
            mark_of_darkness: boosts.mark_of_darkness,
            sunfire: boosts.sunfire.active,
            soulreaper_stacks: boosts.soulreaper_stacks,
        }
    }

    fn apply(&self, boosts: &mut StatusBoosts) {
        boosts.on_task = self.on_task;
        boosts.in_wilderness = self.in_wilderness;
        boosts.in_multi = self.in_multi;
        boosts.forinthry_surge = self.forinthry_surge;
        boosts.charge_active = self.charge_active;
        boosts.kandarin_diary = self.kandarin_diary;
        boosts.mark_of_darkness = self.mark_of_darkness;
        boosts.sunfire.active = self.sunfire;
        boosts.soulreaper_stacks = self.soulreaper_stacks;
    }

    /// Labels for the situational boosts that are switched on, in display order.
    pub fn active_labels(&self) -> Vec<String> {
        let mut labels = Vec::new();
        for (enabled, label) in [
            (self.on_task, "Slayer task"),
            (self.in_wilderness, "Wilderness"),
            (self.kandarin_diary, "Kandarin diary"),
            (self.in_multi, "Multicombat"),
            (self.forinthry_surge, "Forinthry surge"),
            (self.charge_active, "Charge"),
            (self.mark_of_darkness, "Mark of Darkness"),
            (self.sunfire, "Sunfire runes"),
        ] {
            if enabled {
                labels.push(label.to_string());
            }
        }
        if self.soulreaper_stacks > 0 {
            labels.push(format!("{} Soulreaper stacks", self.soulreaper_stacks));
        }
        labels
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LoadoutSpec {
    pub gear: Vec<GearItem>,
    pub stats: BaseStats,
    pub prayers: Vec<Prayer>,
    pub potions: Vec<String>,
    pub conditions: Conditions,
    pub style: CombatStyle,
    pub spell: Option<String>,
    pub rsn: Option<String>,
}

impl Default for LoadoutSpec {
    fn default() -> Self {
        Self::from_player(&Player::default())
    }
}

/// A player rebuilt from a spec, with any items the engine no longer knows.
pub struct RestoredPlayer {
    pub player: Player,
    pub warnings: Vec<String>,
}

impl LoadoutSpec {
    pub fn from_player(player: &Player) -> Self {
        let gear = SLOTS
            .iter()
            .filter_map(|slot| {
                let item = player.get_slot(slot)?;
                if *slot == GearSlot::Weapon && item.name() == "Unarmed" {
                    return None;
                }
                Some(GearItem::from_equipped(*slot, item.as_ref()))
            })
            .collect();
        Self {
            gear,
            stats: BaseStats::from_stats(&player.stats),
            prayers: TRACKED_PRAYERS
                .into_iter()
                .filter(|prayer| player.prayers.contains_prayer(*prayer))
                .collect(),
            potions: active_potions(player)
                .into_iter()
                .map(|potion| potion.to_string())
                .collect(),
            conditions: Conditions::from_boosts(&player.boosts),
            style: player.attrs.active_style,
            spell: player.attrs.spell.map(|spell| spell.to_string()),
            rsn: player.attrs.name.clone(),
        }
    }

    pub fn to_player(&self) -> RestoredPlayer {
        let mut player = Player::default();
        let mut warnings = Vec::new();
        self.stats.apply(&mut player.stats);
        // Equip the weapon last so a two-handed weapon wins over a listed shield.
        let (weapons, others): (Vec<_>, Vec<_>) =
            self.gear.iter().partition(|item| item.is_weapon());
        for item in others.into_iter().chain(weapons) {
            if let Err(error) = item.equip_onto(&mut player) {
                warnings.push(error);
            }
        }
        for prayer in &self.prayers {
            player.add_prayer(*prayer);
        }
        for name in &self.potions {
            match Potion::iter().find(|potion| potion.to_string() == *name) {
                Some(potion) => player.add_potion(potion),
                None => warnings.push(format!("Unknown potion: {name}")),
            }
        }
        self.conditions.apply(&mut player.boosts);
        player.attrs.name = self.rsn.clone();
        player.attrs.spell = self.spell.as_deref().and_then(parse_spell);
        let style = if player.gear.weapon.combat_styles.contains_key(&self.style) {
            Some(self.style)
        } else {
            preferred_style(&player.gear.weapon)
        };
        if let Some(style) = style {
            player.set_active_style(style);
        }
        player.calc_potion_boosts();
        player.reset_current_stats(false);
        player.update_bonuses();
        player.update_set_effects();
        RestoredPlayer { player, warnings }
    }

    pub fn item(&self, slot: GearSlot) -> Option<&GearItem> {
        let slot = slot.to_string().to_lowercase();
        self.gear.iter().find(|item| item.slot == slot)
    }
}

pub fn active_potions(player: &Player) -> Vec<Potion> {
    let mut active = Vec::new();
    for boosts in [
        &player.potions.attack,
        &player.potions.strength,
        &player.potions.defence,
        &player.potions.ranged,
        &player.potions.magic,
    ]
    .into_iter()
    .flatten()
    {
        for boost in boosts {
            if !active.contains(&boost.potion_type) {
                active.push(boost.potion_type);
            }
        }
    }
    active
}

pub fn parse_spell(name: &str) -> Option<Spell> {
    all_spells()
        .into_iter()
        .find(|spell| spell.to_string() == name)
}

pub fn all_spells() -> Vec<Spell> {
    use AncientSpell as A;
    use ArceuusSpell as R;
    use StandardSpell as S;
    let standard = [
        S::WindStrike,
        S::WaterStrike,
        S::EarthStrike,
        S::FireStrike,
        S::WindBolt,
        S::WaterBolt,
        S::EarthBolt,
        S::FireBolt,
        S::WindBlast,
        S::WaterBlast,
        S::EarthBlast,
        S::FireBlast,
        S::WindWave,
        S::WaterWave,
        S::EarthWave,
        S::FireWave,
        S::WindSurge,
        S::WaterSurge,
        S::EarthSurge,
        S::FireSurge,
        S::CrumbleUndead,
        S::SaradominStrike,
        S::ClawsOfGuthix,
        S::FlamesOfZamorak,
        S::IbanBlast,
        S::MagicDart,
        S::Bind,
        S::Snare,
        S::Entangle,
    ];
    let ancient = [
        A::SmokeRush,
        A::ShadowRush,
        A::BloodRush,
        A::IceRush,
        A::SmokeBurst,
        A::ShadowBurst,
        A::BloodBurst,
        A::IceBurst,
        A::SmokeBlitz,
        A::ShadowBlitz,
        A::BloodBlitz,
        A::IceBlitz,
        A::SmokeBarrage,
        A::ShadowBarrage,
        A::BloodBarrage,
        A::IceBarrage,
    ];
    let arceuus = [
        R::GhostlyGrasp,
        R::SkeletalGrasp,
        R::UndeadGrasp,
        R::InferiorDemonbane,
        R::SuperiorDemonbane,
        R::DarkDemonbane,
    ];
    standard
        .into_iter()
        .map(Spell::Standard)
        .chain(ancient.into_iter().map(Spell::Ancient))
        .chain(arceuus.into_iter().map(Spell::Arceuus))
        .chain([
            Spell::Special(SpecialSpell::Invocate),
            Spell::Special(SpecialSpell::Immolate),
        ])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_round_trips_through_player() {
        let mut player = Player::default();
        for (name, version) in [
            ("Torva full helm", None),
            ("Osmumten's fang", None),
            ("Avernic defender", None),
            ("Bow of Faerdhinen", Some("Charged")),
        ] {
            let item = GearItem {
                slot: if name.contains("fang") || name.contains("Bow") {
                    "weapon".into()
                } else if name.contains("defender") {
                    "shield".into()
                } else {
                    "head".into()
                },
                name: name.into(),
                version: version.map(str::to_string),
                image: String::new(),
            };
            item.equip_onto(&mut player).unwrap();
        }
        player.stats.ranged = Stat::from(95);
        player.set_active_style(CombatStyle::Rapid);
        player.add_prayer(Prayer::Rigour);
        player.add_potion(Potion::Ranging);
        player.boosts.on_task = false;
        player.calc_potion_boosts();
        player.reset_current_stats(false);

        let spec = LoadoutSpec::from_player(&player);
        assert_eq!(
            spec.item(GearSlot::Weapon).unwrap().name,
            "Bow of Faerdhinen"
        );
        assert!(
            spec.item(GearSlot::Shield).is_none(),
            "2H weapon clears shield"
        );
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: LoadoutSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, spec);

        let restored = parsed.to_player();
        assert!(restored.warnings.is_empty(), "{:?}", restored.warnings);
        assert_eq!(restored.player.gear.weapon.name, "Bow of Faerdhinen");
        assert_eq!(restored.player.attrs.active_style, CombatStyle::Rapid);
        assert_eq!(restored.player.stats.ranged, player.stats.ranged);
        assert!(restored.player.prayers.contains_prayer(Prayer::Rigour));
        assert_eq!(active_potions(&restored.player), vec![Potion::Ranging]);
        assert!(!restored.player.boosts.on_task);
        assert_eq!(LoadoutSpec::from_player(&restored.player), spec);
    }

    #[test]
    fn unknown_items_are_reported_not_fatal() {
        let spec = LoadoutSpec {
            gear: vec![GearItem {
                slot: "head".into(),
                name: "Hat of nonexistence".into(),
                version: None,
                image: String::new(),
            }],
            ..LoadoutSpec::default()
        };
        let restored = spec.to_player();
        assert_eq!(restored.warnings.len(), 1);
        assert!(restored.player.gear.head.is_none());
    }
}
