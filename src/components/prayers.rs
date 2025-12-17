use crate::state::AppState;
use crate::PRAYERS_ASSETS;
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
    let mut is_collapsed = use_signal(|| false);

    // Read state once and collect active prayers
    let player_prayers = state.read().player.prayers.clone();
    let is_prayer_active = |prayer: Prayer| player_prayers.contains_prayer(prayer);

    rsx! {
        div {
            // Toggle header
            div {
                class: "flex items-center justify-between cursor-pointer p-2 hover:bg-gray-800 rounded transition-colors",
                onclick: move |_| is_collapsed.set(!is_collapsed()),
                div {
                    class: "flex items-center gap-4",
                    h3 {
                        class: "text-sm font-semibold text-accent w-12",
                        "Prayers"
                    }
                    if is_collapsed() {
                        {
                            let active_prayers: Vec<Prayer> = PRAYER_ROWS
                                .iter()
                                .flatten()
                                .filter(|&prayer| *prayer != Prayer::None && is_prayer_active(*prayer))
                                .copied()
                                .collect();
                            if !active_prayers.is_empty() {
                                rsx! {
                                    div {
                                        class: "flex gap-2",
                                        for prayer in active_prayers.iter() {
                                            img {
                                                class: "w-5 h-5 object-contain",
                                                src: "{get_prayer_img_path(*prayer)}",
                                                alt: "{prayer}",
                                                title: "{prayer}"
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                    }
                }
                div {
                    class: "text-xs text-gray-400 transform transition-transform",
                    class: if is_collapsed() { "" } else { "rotate-180" },
                    "▼"
                }
            }

            // Expanded prayer grid
            if !is_collapsed() {
                div {
                    class: "flex flex-col gap-2 items-center mt-2",
                    for (row_idx, prayer_row) in PRAYER_ROWS.iter().enumerate() {
                        div {
                            key: "prayer-row-{row_idx}",
                            class: "flex gap-2",
                            for (col_idx, prayer) in prayer_row.iter().enumerate() {
                                if *prayer != Prayer::None {
                                    PrayerButton {
                                        key: "prayer-{row_idx}-{col_idx}",
                                        prayer: *prayer,
                                        is_active: is_prayer_active(*prayer),
                                        on_click: move |prayer: Prayer| {
                                            let mut app_state = state.write();
                                            if app_state.player.prayers.contains_prayer(prayer) {
                                                app_state.player.remove_prayer(prayer);
                                            } else {
                                                app_state.player.add_prayer(prayer);
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
                    src: format!("{PRAYERS_ASSETS}/selected.png"),
                    alt: "Selected"
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
