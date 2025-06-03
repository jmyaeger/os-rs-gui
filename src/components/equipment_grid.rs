use crate::components::equipment_slot::EquipmentGridSlot;
use dioxus::prelude::*;
use osrs::types::equipment::GearSlot;

#[component]
pub fn EquipmentGrid() -> Element {
    rsx! {
        div {
            div {
                class: "flex justify-center",
                EquipmentGridSlot { slot_type: GearSlot::Head }
            }
            div {
                class: "mt-1 flex justify-center gap-2",
                EquipmentGridSlot { slot_type: GearSlot::Cape }
                EquipmentGridSlot { slot_type: GearSlot::Neck }
                EquipmentGridSlot { slot_type: GearSlot::Ammo }
            }
            div {
                class: "mt-1 flex justify-center gap-6",
                EquipmentGridSlot { slot_type: GearSlot::Weapon }
                EquipmentGridSlot { slot_type: GearSlot::Body }
                EquipmentGridSlot { slot_type: GearSlot::Shield }
            }
            div {
                class: "mt-1 flex justify-center",
                EquipmentGridSlot { slot_type: GearSlot::Legs }
            }
            div {
                class: "mt-1 flex justify-center gap-6",
                EquipmentGridSlot { slot_type: GearSlot::Hands }
                EquipmentGridSlot { slot_type: GearSlot::Feet }
                EquipmentGridSlot { slot_type: GearSlot::Ring }
            }
        }
    }
}
