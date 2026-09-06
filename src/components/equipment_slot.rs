use dioxus::prelude::*;
use osrs::types::equipment::GearSlot;
use osrs::types::player::Player;

#[component]
pub fn EquipmentGridSlot(slot_type: GearSlot) -> Element {
    let mut player = use_context::<Signal<Player>>();
    let current_item = player.read().get_slot(&slot_type);
    let item_name = match current_item {
        Some(ref item) => item.name(),
        None => "",
    };
    let placeholder_image = format!("{}/{slot_type}.png", crate::PLACEHOLDERS_ASSETS);
    let equipped = current_item
        .as_ref()
        .is_some_and(|item| item.name() != "Unarmed");
    let can_remove = equipped
        && (slot_type == GearSlot::Weapon || !player.read().gear.weapon.combat_styles.is_empty());
    let slot_label = if equipped {
        format!("Unequip {item_name}")
    } else {
        format!("{slot_type}: Empty")
    };
    let button_class = format!(
        "equipment-slot-bg flex justify-center items-center h-[40px] w-[40px] {}",
        if equipped {
            "cursor-pointer"
        } else {
            "cursor-default"
        }
    );

    rsx! {
        button {
            "type": "button",
            class: "{button_class}",
            title: "{item_name}",
            aria_label: "{slot_label}",
            disabled: !can_remove,
            onclick: move |_| {
                if can_remove {
                    let mut player = player.write();
                    player.unequip_slot(&slot_type);
                    crate::components::ensure_style(&mut player);
                }
            },
            {
                match current_item {
                    Some(ref item) => {
                        if item_name == "Unarmed" {
                            rsx! { img { class: "opacity-30 filter grayscale invert", src: "{placeholder_image}", alt: "{slot_type}", draggable: "false" } }
                        } else {
                            let image_path = item.as_ref().get_image_path();
                            if image_path.is_empty() {
                                log::warn!("[GridSlot {slot_type:?}] Item '{item_name}' has empty image path. Showing placeholder.");
                                rsx! { img { class: "opacity-30 filter grayscale invert", src: "{placeholder_image}", alt: "{slot_type}", draggable: "false" } }
                            } else {
                                let cdn_image = format!("{}/{}", crate::EQUIPMENT_ASSETS, item.get_image_path());
                                rsx! { img { src: "{cdn_image}", alt: "{item_name}" } }
                            }
                        }
                    },
                    None => {
                        rsx! { img { class: "opacity-30 filter grayscale invert", src: "{placeholder_image}", alt: "{slot_type}", draggable: "false" } }
                    }
                }
            }
        }
    }
}
