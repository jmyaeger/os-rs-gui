use crate::state::AppState;
use dioxus::prelude::*;
use osrs::types::equipment::GearSlot;

#[component]
pub fn EquipmentGridSlot(slot_type: GearSlot) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let current_item = state.read().player.get_slot(&slot_type);
    let item_name = match current_item {
        Some(ref item) => item.name(),
        None => "",
    };
    let placeholder_image = format!("/assets/placeholders/{slot_type}.png");
    let button_class = format!(
        "equipment-slot-bg flex justify-center items-center h-[40px] w-[40px] {}",
        if current_item.is_some() {
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
            onmousedown: move |_| {
                if current_item.is_some() {
                    state.write().player.unequip_slot(&slot_type);
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
                                let cdn_image = format!("/assets/equipment/{}", item.get_image_path());
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
