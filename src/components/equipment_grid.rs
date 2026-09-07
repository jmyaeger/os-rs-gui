use crate::components::equipment_slot::EquipmentGridSlot;
use dioxus::prelude::*;
use osrs::types::equipment::GearSlot;

/// Paperdoll positions: (slot, row, column) in a 3x5 grid. Spacing is owned by
/// the page stylesheet so the grid can be tightened without touching Tailwind.
const GRID: [(GearSlot, u8, u8); 11] = [
    (GearSlot::Head, 1, 2),
    (GearSlot::Cape, 2, 1),
    (GearSlot::Neck, 2, 2),
    (GearSlot::Ammo, 2, 3),
    (GearSlot::Weapon, 3, 1),
    (GearSlot::Body, 3, 2),
    (GearSlot::Shield, 3, 3),
    (GearSlot::Legs, 4, 2),
    (GearSlot::Hands, 5, 1),
    (GearSlot::Feet, 5, 2),
    (GearSlot::Ring, 5, 3),
];

#[component]
pub fn EquipmentGrid() -> Element {
    rsx! {
        div { class: "equipment-paperdoll",
            for (slot, row, column) in GRID {
                div {
                    key: "{slot}",
                    class: "equipment-paperdoll-cell",
                    style: "grid-row: {row}; grid-column: {column}",
                    EquipmentGridSlot { slot_type: slot }
                }
            }
        }
    }
}
