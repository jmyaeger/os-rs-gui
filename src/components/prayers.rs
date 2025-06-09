use crate::state::AppState;
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

#[component]
pub fn PrayerSelect() -> Element {
    let mut state = use_context::<Signal<AppState>>();

    rsx! {
        div {
            class: "px-4 mb-8",
            h2 {
                class: "text-md font-bold mb-4 text-accent text-center",
                "Prayers"
            }
            div {
                class: "flex flex-col gap-2 items-center",
                for (row_idx, prayer_row) in PRAYER_ROWS.iter().enumerate() {
                    div {
                        key: "prayer-row-{row_idx}",
                        class: "flex gap-2",
                        for (col_idx, prayer) in prayer_row.iter().enumerate() {
                            if *prayer != Prayer::None {
                                PrayerButton {
                                    key: "prayer-{row_idx}-{col_idx}",
                                    prayer: *prayer,
                                    is_active: state.read().player.prayers.contains_prayer(*prayer),
                                    on_click: move |prayer: Prayer| {
                                        let mut app_state = state.write();
                                        if app_state.player.prayers.contains_prayer(prayer) {
                                            app_state.player.prayers.remove(prayer);
                                        } else {
                                            app_state.player.prayers.add(prayer);
                                        }
                                    }
                                }
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
                alt: "{prayer}"
            }
            if is_active {
                img {
                    class: "absolute inset-0 w-full h-full object-contain pointer-events-none opacity-30",
                    src: "/assets/prayers/selected.png",
                    alt: "Selected"
                }
            }
        }
    }
}

fn get_prayer_img_path(prayer: Prayer) -> String {
    format!(
        "/assets/prayers/{}.png",
        prayer.to_string().replace(" ", "_")
    )
}
