use dioxus::prelude::*;

use crate::components::{EquipmentGrid, EquipmentSelect, PotionSelect, PrayerSelect, SkillsSelect};

#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "w-full max-w-[380px] lg:max-w-7xl mx-auto",
            div { class: "card p-4 max-w-[270px]",
                h2 { class: "card-title mb-4", "Loadout" }
                div { class: "flex flex-col gap-4",
                    EquipmentGrid {}
                    EquipmentSelect {}
                    SkillsSelect {}
                    PrayerSelect {}
                    PotionSelect {}
                }
            }
        }
    }
}
