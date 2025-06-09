use crate::components::equipment_slot::EquipmentGridSlot;
use dioxus::prelude::*;
use osrs::types::equipment::GearSlot;

#[component]
pub fn EquipmentGrid() -> Element {
    rsx! {
        div {
            class: "panel-elevated p-6 inline-block",
            h2 {
                class: "text-lg font-medium text-center mb-4 text-muted",
                "Equipment"
            }
            div {
                class: "space-y-3",
                div {
                    class: "flex justify-center",
                    EquipmentGridSlot { slot_type: GearSlot::Head }
                }
                div {
                    class: "flex justify-center gap-3",
                    EquipmentGridSlot { slot_type: GearSlot::Cape }
                    EquipmentGridSlot { slot_type: GearSlot::Neck }
                    EquipmentGridSlot { slot_type: GearSlot::Ammo }
                }
                div {
                    class: "flex justify-center gap-8",
                    EquipmentGridSlot { slot_type: GearSlot::Weapon }
                    EquipmentGridSlot { slot_type: GearSlot::Body }
                    EquipmentGridSlot { slot_type: GearSlot::Shield }
                }
                div {
                    class: "flex justify-center",
                    EquipmentGridSlot { slot_type: GearSlot::Legs }
                }
                div {
                    class: "flex justify-center gap-8",
                    EquipmentGridSlot { slot_type: GearSlot::Hands }
                    EquipmentGridSlot { slot_type: GearSlot::Feet }
                    EquipmentGridSlot { slot_type: GearSlot::Ring }
                }
            }
        }
    }
}
