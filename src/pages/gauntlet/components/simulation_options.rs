use dioxus::prelude::*;
use osrs::sims::hunleff::{HunllefEatStrategy, HunllefRedemptionStrat};

use crate::pages::gauntlet::components::select::Select;
use crate::pages::gauntlet::state::AppState;

const INPUT_CLASS: &str = "w-16 h-7 text-sm px-1 input-field rounded text-center text-white num focus:outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none";
const VALID_BORDER: &str = "";

#[component]
pub fn SimulationOptions() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();

    // Local signals for select components - synced from app state
    let eat_strategy_label = use_memo(move || match &app_state.read().sim_config.eat_strategy {
        HunllefEatStrategy::EatAtHp(_) => "At HP threshold".to_string(),
        HunllefEatStrategy::TickEatOnly => "Tick eat only".to_string(),
        HunllefEatStrategy::EatToFullDuringNadoes => "Eat during tornadoes".to_string(),
    });

    let eat_strategy_select = use_signal(move || Some(eat_strategy_label()));

    let redemption_enabled =
        use_memo(move || app_state.read().sim_config.redemption_strategy.is_some());

    let redemption_label = use_memo(move || {
        match &app_state.read().sim_config.redemption_strategy {
            Some(HunllefRedemptionStrat::BeforeEating(_)) => "Before eating".to_string(),
            Some(HunllefRedemptionStrat::NoFoodLeft(_)) => "No food left".to_string(),
            None => "Before eating".to_string(), // Default for when enabled
        }
    });

    let mut redemption_select = use_signal(move || Some(redemption_label()));

    // Local input signals for validation
    let mut num_trials_input = use_signal(move || app_state.read().num_trials.to_string());
    let mut lost_ticks_input =
        use_signal(move || app_state.read().sim_config.lost_ticks.to_string());
    let mut food_count_input =
        use_signal(move || app_state.read().sim_config.food_count.to_string());
    let mut hp_threshold_input =
        use_signal(move || match &app_state.read().sim_config.eat_strategy {
            HunllefEatStrategy::EatAtHp(hp) => hp.to_string(),
            _ => "50".to_string(),
        });
    let mut max_procs_input =
        use_signal(
            move || match &app_state.read().sim_config.redemption_strategy {
                Some(HunllefRedemptionStrat::BeforeEating(n)) => n.to_string(),
                Some(HunllefRedemptionStrat::NoFoodLeft(n)) => n.to_string(),
                None => "1".to_string(),
            },
        );

    // Validation states
    let num_trials_valid = use_memo(move || {
        num_trials_input()
            .parse::<u32>()
            .map(|v| (1..=100_000).contains(&v))
            .unwrap_or(false)
    });
    let lost_ticks_valid = use_memo(move || {
        lost_ticks_input()
            .parse::<i32>()
            .map(|v| (0..=1000).contains(&v))
            .unwrap_or(false)
    });
    let food_count_valid = use_memo(move || {
        food_count_input()
            .parse::<u32>()
            .map(|v| v <= 27)
            .unwrap_or(false)
    });
    let hp_threshold_valid = use_memo(move || {
        let max_hp = app_state.read().player.stats.hitpoints.base;
        hp_threshold_input()
            .parse::<u32>()
            .map(|v| v >= 1 && v <= max_hp)
            .unwrap_or(false)
    });
    let max_procs_valid = use_memo(move || {
        max_procs_input()
            .parse::<u32>()
            .map(|v| (1..=20).contains(&v))
            .unwrap_or(false)
    });

    // Get current values for display
    let max_hp = app_state.read().player.stats.hitpoints.base;
    let redemption_max_procs = match &app_state.read().sim_config.redemption_strategy {
        Some(HunllefRedemptionStrat::BeforeEating(n)) => *n,
        Some(HunllefRedemptionStrat::NoFoodLeft(n)) => *n,
        None => 1,
    };
    let is_eat_at_hp = matches!(
        app_state.read().sim_config.eat_strategy,
        HunllefEatStrategy::EatAtHp(_)
    );

    // Compute input classes based on validation
    let num_trials_class = format!(
        "{INPUT_CLASS} {}",
        if num_trials_valid() {
            VALID_BORDER
        } else {
            "input-invalid"
        }
    );
    let lost_ticks_class = format!(
        "{INPUT_CLASS} {}",
        if !lost_ticks_valid() {
            "input-invalid"
        } else {
            ""
        }
    );
    let food_count_class = format!(
        "{INPUT_CLASS} {}",
        if !food_count_valid() {
            "input-invalid"
        } else {
            ""
        }
    );
    let hp_threshold_class = format!(
        "{INPUT_CLASS} {}",
        if !hp_threshold_valid() {
            "input-invalid"
        } else {
            ""
        }
    );
    let max_procs_class = format!(
        "{INPUT_CLASS} {}",
        if !max_procs_valid() {
            "input-invalid"
        } else {
            ""
        }
    );

    rsx! {
        div { class: "space-y-0.5",
            // Number of Trials
            div { class: "flex items-center justify-between py-1 px-1 rounded",
                span { class: "text-sm text-gray-400", "Number of trials" }
                input {
                    "type": "number",
                    class: "{num_trials_class}",
                    min: "1",
                    max: "100000",
                    value: "{num_trials_input}",
                    oninput: move |e| {
                        let val = e.value();
                        num_trials_input.set(val.clone());
                        if let Ok(v) = val.parse::<u32>() && (1..=100_000).contains(&v) {
                            app_state.write().num_trials = v;
                        }
                    },
                }
            }

            // Lost Ticks
            div { class: "flex items-center justify-between py-1 px-1 rounded",
                span { class: "text-sm text-gray-400", "Lost ticks" }
                input {
                    "type": "number",
                    class: "{lost_ticks_class}",
                    min: "0",
                    max: "1000",
                    value: "{lost_ticks_input}",
                    oninput: move |e| {
                        let val = e.value();
                        lost_ticks_input.set(val.clone());
                        if let Ok(v) = val.parse::<i32>() && (0..=1000).contains(&v) {
                            app_state.write().sim_config.lost_ticks = v;
                        }
                    },
                }
            }

            // Paddlefish
            div { class: "flex items-center justify-between py-1 px-1 rounded",
                span { class: "text-sm text-gray-400", "Paddlefish" }
                input {
                    "type": "number",
                    class: "{food_count_class}",
                    min: "0",
                    max: "27",
                    value: "{food_count_input}",
                    oninput: move |e| {
                        let val = e.value();
                        food_count_input.set(val.clone());
                        if let Ok(v) = val.parse::<u32>() && v <= 27 {
                            app_state.write().sim_config.food_count = v;
                        }
                    },
                }
            }

            // Eat Strategy
            div { class: "flex items-center justify-between py-1 px-1 rounded",
                span { class: "text-sm text-gray-400 shrink-0", "Eat strategy" }
                div { class: "w-40",
                    Select {
                        options: vec![
                            "At HP threshold".to_string(),
                            "Tick eat only".to_string(),
                            "Eat during tornadoes".to_string(),
                        ],
                        value: eat_strategy_select,
                        placeholder: "Select...",
                        on_change: move |selected: String| {
                            let new_strategy = match selected.as_str() {
                                "At HP threshold" => {
                                    // Use the value from hp_threshold_input to preserve user's previous selection
                                    let threshold = hp_threshold_input().parse::<u32>().unwrap_or(50);
                                    HunllefEatStrategy::EatAtHp(threshold)
                                }
                                "Tick eat only" => HunllefEatStrategy::TickEatOnly,
                                "Eat during tornadoes" => HunllefEatStrategy::EatToFullDuringNadoes,
                                _ => return,
                            };
                            app_state.write().sim_config.eat_strategy = new_strategy;
                        },
                    }
                }
            }
            // Show HP threshold input when "Eat at HP" is selected
            if is_eat_at_hp {
                div { class: "flex items-center justify-between py-1 px-1 rounded ml-1",
                    span { class: "text-xs text-gray-500", "HP threshold" }
                    input {
                        "type": "number",
                        class: "{hp_threshold_class}",
                        min: "1",
                        max: "{max_hp}",
                        value: "{hp_threshold_input}",
                        oninput: move |e| {
                            let val = e.value();
                            hp_threshold_input.set(val.clone());
                            if let Ok(v) = val.parse::<u32>() {
                                let max = app_state.read().player.stats.hitpoints.base;
                                if v >= 1 && v <= max {
                                    app_state.write().sim_config.eat_strategy = HunllefEatStrategy::EatAtHp(
                                        v,
                                    );
                                }
                            }
                        },
                    }
                }
            }

            // Redemption Strategy
            div { class: "flex items-center justify-between py-1 px-1 rounded",
                span { class: "text-sm text-gray-400", "Redemption" }
                input {
                    "type": "checkbox",
                    class: "w-4 h-4 accent-blue-500 cursor-pointer",
                    checked: redemption_enabled(),
                    onchange: move |e| {
                        if e.checked() {
                            // Enable with default: BeforeEating(1)
                            app_state.write().sim_config.redemption_strategy = Some(
                                HunllefRedemptionStrat::BeforeEating(1),
                            );
                            redemption_select.set(Some("Before eating".to_string()));
                        } else {
                            app_state.write().sim_config.redemption_strategy = None;
                        }
                    },
                }
            }
            if redemption_enabled() {
                div { class: "flex items-center justify-between py-1 px-1 rounded ml-1",
                    span { class: "text-xs text-gray-500 shrink-0", "Strategy" }
                    div { class: "w-32",
                        Select {
                            options: vec!["Before eating".to_string(), "No food left".to_string()],
                            value: redemption_select,
                            placeholder: "Select...",
                            on_change: move |selected: String| {
                                let new_strategy = match selected.as_str() {
                                    "Before eating" => HunllefRedemptionStrat::BeforeEating(redemption_max_procs),
                                    "No food left" => HunllefRedemptionStrat::NoFoodLeft(redemption_max_procs),
                                    _ => return,
                                };
                                app_state.write().sim_config.redemption_strategy = Some(new_strategy);
                            },
                        }
                    }
                }
                div { class: "flex items-center justify-between py-1 px-1 rounded ml-1",
                    span { class: "text-xs text-gray-500", "Max procs" }
                    input {
                        "type": "number",
                        class: "{max_procs_class}",
                        min: "1",
                        max: "20",
                        value: "{max_procs_input}",
                        oninput: move |e| {
                            let val = e.value();
                            max_procs_input.set(val.clone());
                            if let Ok(v) = val.parse::<u32>() && (1..=20).contains(&v) {
                                let mut state = app_state.write();
                                state.sim_config.redemption_strategy = match &state
                                    .sim_config
                                    .redemption_strategy
                                {
                                    Some(HunllefRedemptionStrat::BeforeEating(_)) => {
                                        Some(HunllefRedemptionStrat::BeforeEating(v))
                                    }
                                    Some(HunllefRedemptionStrat::NoFoodLeft(_)) => {
                                        Some(HunllefRedemptionStrat::NoFoodLeft(v))
                                    }
                                    None => None,
                                };
                            }
                        },
                    }
                }
            }
        }
    }
}
