use crate::PRAYERS_ASSETS;
use crate::pages::gauntlet::state::GauntletState;
use dioxus::prelude::*;
use osrs::types::prayers::Prayer;

const PRAYER_ROWS: [[Prayer; 5]; 5] = [
    [
        Prayer::ClarityOfThought,
        Prayer::BurstOfStrength,
        Prayer::ThickSkin,
        Prayer::SharpEye,
        Prayer::MysticWill,
    ],
    [
        Prayer::ImprovedReflexes,
        Prayer::SuperhumanStrength,
        Prayer::RockSkin,
        Prayer::HawkEye,
        Prayer::MysticLore,
    ],
    [
        Prayer::IncredibleReflexes,
        Prayer::UltimateStrength,
        Prayer::SteelSkin,
        Prayer::EagleEye,
        Prayer::MysticMight,
    ],
    [
        Prayer::Chivalry,
        Prayer::Deadeye,
        Prayer::MysticVigour,
        Prayer::None,
        Prayer::None,
    ],
    [
        Prayer::Piety,
        Prayer::Rigour,
        Prayer::Augury,
        Prayer::None,
        Prayer::None,
    ],
];

use crate::pages::gauntlet::components::loadout::LoadoutStyle;

#[component]
pub fn PrayerSelect(style: LoadoutStyle) -> Element {
    let mut switch = use_context::<GauntletState>().switch_signal(style);

    let active_prayers = switch.read().prayers.clone();
    let is_prayer_active = |p: Prayer| active_prayers.contains_prayer(p);

    rsx! {
        div { class: "flex flex-col gap-0.5 items-center",
            for (row_idx , prayer_row) in PRAYER_ROWS.iter().enumerate() {
                div { key: "prayer-row-{row_idx}", class: "flex gap-0.5",
                    for (col_idx , prayer) in prayer_row.iter().enumerate() {
                        if *prayer != Prayer::None {
                            PrayerButton {
                                key: "prayer-{row_idx}-{col_idx}",
                                prayer: *prayer,
                                is_active: is_prayer_active(*prayer),
                                on_click: move |p: Prayer| {
                                    let mut player = switch.write();
                                    if player.prayers.contains_prayer(p) {
                                        player.remove_prayer(p);
                                    } else {
                                        player.add_prayer(p);
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PrayerButton(prayer: Prayer, is_active: bool, on_click: EventHandler<Prayer>) -> Element {
    let img_path = get_prayer_img_path(prayer);
    let button_class = if is_active {
        "relative w-8 h-8 bg-gray-700 rounded-full cursor-pointer transition-all duration-150 hover:scale-105 flex items-center justify-center"
    } else {
        "relative w-8 h-8 bg-gray-800 rounded-full cursor-pointer transition-all duration-150 hover:bg-gray-700 hover:scale-105 flex items-center justify-center"
    };

    rsx! {
        div {
            class: "{button_class}",
            title: "{prayer}",
            onclick: move |_| on_click.call(prayer),
            img {
                class: "p-1 object-contain",
                src: "{img_path}",
                alt: "{prayer}",
            }
            if is_active {
                img {
                    class: "absolute inset-0 w-full h-full object-contain pointer-events-none opacity-30",
                    src: format!("{PRAYERS_ASSETS}/selected.png"),
                    alt: "Selected",
                }
            }
        }
    }
}

fn get_prayer_img_path(prayer: Prayer) -> String {
    format!(
        "{}/{}.png",
        PRAYERS_ASSETS,
        prayer.to_string().replace(" ", "_")
    )
}
