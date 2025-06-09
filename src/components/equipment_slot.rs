use crate::state::AppState;
use dioxus::prelude::*;
use osrs::types::equipment::GearSlot;

#[component]
pub fn EquipmentGridSlot(slot_type: GearSlot) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let current_item = state.read().player.get_slot(&slot_type);
    let placeholder_image = format!("/assets/placeholders/{slot_type}.png");

    let button_class = format!(
        "flex justify-center items-center h-[40px] w-[40px] bg-slate-600 border-slate-400 border rounded {}",
        if current_item.is_ok() { "cursor-pointer hover:border-red" } else { "" }
    );

    let tooltip_content = match current_item {
        Ok(ref item) => item.name(),
        Err(_) => "",
    };

    rsx! {
        div { class: "h-[40px] w-[40px] relative",
            // UserIssueWarning could be added here if needed
            button {
                "type": "button",
                class: "{button_class}",
                title: "{tooltip_content}",
                onmousedown: move |_| {
                    if current_item.is_ok() {
                        state.write().player.unequip_slot(&slot_type);
                    }
                },
                {
                    match current_item {
                        Ok(ref item) => {
                            if item.as_ref().name() == "Unarmed" {
                                rsx! { img { class: "opacity-60 filter", src: "{placeholder_image}", alt: "{slot_type}", draggable: "false" } }
                            } else {
                                let image_path = item.as_ref().get_image_path();
                                if image_path.is_empty() {
                                    log::warn!("[GridSlot {:?}] Item '{}' has empty image path. Showing placeholder.", slot_type, item.as_ref().name());
                                    rsx! { img { class: "opacity-60 filter", src: "{placeholder_image}", alt: "{slot_type}", draggable: "false" } }
                                } else {
                                    let cdn_image = format!("/assets/equipment/{}", item.get_image_path());
                                    rsx! { img { src: "{cdn_image}", alt: "{item.name()}" } }
                                }

                            }
                        },
                        Err(_) => {
                            rsx! { img { class: "opacity-60 filter", src: "{placeholder_image}", alt: "{slot_type}", draggable: "false" } }
                        }
                    }
                }
            }
        }
    }
}
