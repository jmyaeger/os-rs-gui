use crate::state::AppState;
use dioxus::prelude::*;
use osrs::types::player::parse_player_data;

// Define skill types and order
#[derive(Clone, Copy, Debug, PartialEq)]
enum Skill {
    Attack,
    Strength,
    Defence,
    Ranged,
    Magic,
    Hitpoints,
    Prayer,
    Mining,
    Herblore,
}

impl Skill {
    fn name(&self) -> &'static str {
        match self {
            Skill::Attack => "Attack",
            Skill::Strength => "Strength", 
            Skill::Defence => "Defence",
            Skill::Ranged => "Ranged",
            Skill::Magic => "Magic",
            Skill::Hitpoints => "Hitpoints",
            Skill::Prayer => "Prayer",
            Skill::Mining => "Mining",
            Skill::Herblore => "Herblore",
        }
    }

    fn icon_path(&self) -> String {
        format!("/assets/bonuses/{}.png", self.name().to_lowercase())
    }

    fn is_combat_skill(&self) -> bool {
        matches!(self, Skill::Attack | Skill::Strength | Skill::Defence | Skill::Ranged | Skill::Magic | Skill::Hitpoints)
    }
}

const ALL_SKILLS: [Skill; 9] = [
    Skill::Attack,
    Skill::Strength,
    Skill::Defence,
    Skill::Ranged,
    Skill::Magic,
    Skill::Hitpoints,
    Skill::Prayer,
    Skill::Mining,
    Skill::Herblore,
];

const COMBAT_SKILLS: [Skill; 6] = [
    Skill::Attack,
    Skill::Strength,
    Skill::Defence,
    Skill::Ranged,
    Skill::Magic,
    Skill::Hitpoints,
];

// Web-compatible fetch function that uses reqwest with the proxy
async fn fetch_player_data_web(rsn: &str) -> Result<String, reqwest::Error> {
    // Use absolute URL pointing to the Dioxus dev server
    let url = format!("http://localhost:8080/m=hiscore_oldschool/index_lite.ws?player={}", rsn);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let data = response.text().await?;
    Ok(data)
}

// Web-compatible lookup function
async fn lookup_stats_web(app_state: &mut AppState, rsn: &str) -> Result<(), Box<dyn std::error::Error>> {
    let stats_data = fetch_player_data_web(rsn).await?;
    app_state.player.stats = parse_player_data(stats_data);
    app_state.player.attrs.name = Some(rsn.to_string());
    Ok(())
}

#[component]
pub fn SkillsSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut is_collapsed = use_signal(|| false);
    let mut rsn_input = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    rsx! {
        div {
            class: "min-w-60",
            // Toggle header
            div {
                class: "flex items-center justify-between cursor-pointer p-2 hover:bg-gray-800 rounded transition-colors",
                onclick: move |_| is_collapsed.set(!is_collapsed()),
                div {
                    class: "flex items-center gap-2",
                    h3 {
                        class: "text-sm font-semibold text-accent w-12",
                        "Skills"
                    }
                    if is_collapsed() {
                        div {
                            class: "flex flex-col gap-2",
                            div {
                                class: "flex gap-4 items-center",
                                for skill in COMBAT_SKILLS[0..3].iter() {
                                    SkillIconDisplay {
                                        skill: *skill
                                    }
                                }
                            }
                            div {
                                class: "flex gap-4 items-center",
                                for skill in COMBAT_SKILLS[3..6].iter() {
                                    SkillIconDisplay {
                                        skill: *skill
                                    }
                                }
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

            // Expanded skills interface
            if !is_collapsed() {
                div {
                    class: "mt-2",
                    
                    // RSN Lookup section
                    div {
                        class: "mb-3",
                        div {
                            class: "flex items-center gap-2 justify-between",
                            input {
                                "type": "text",
                                class: "input w-38 h-10 text-sm",
                                placeholder: "Enter RSN...",
                                value: "{rsn_input}",
                                disabled: is_loading(),
                                oninput: move |evt| rsn_input.set(evt.value())
                            }
                            button {
                                class: "flex btn btn-primary text-sm text-center p-0 h-10 min-h-0 w-20 justify-center",
                                disabled: rsn_input.read().trim().is_empty() || is_loading(),
                                onclick: move |_| {
                                    let rsn = rsn_input.read().trim().to_string();
                                    if !rsn.is_empty() {
                                        is_loading.set(true);
                                        let mut state_signal = app_state;
                                        spawn(async move {
                                            // Clear any previous error
                                            error_message.set(None);
                                            
                                            // Use our web-compatible lookup function
                                            let result = {
                                                let mut state = state_signal.write();
                                                lookup_stats_web(&mut *state, &rsn).await
                                            };
                                            
                                            match result {
                                                Ok(()) => {
                                                    // Success - stats were updated
                                                    rsn_input.set(String::new()); // Clear the input on success
                                                }
                                                Err(e) => {
                                                    // Handle the error gracefully
                                                    error_message.set(Some(format!("Failed to lookup stats: {}", e)));
                                                }
                                            }
                                            
                                            // Set loading to false after completion
                                            is_loading.set(false);
                                        });
                                    }
                                },
                                if is_loading() { "Loading..." } else { "Lookup" }
                            }
                        }
                        
                        // Error message display
                        if let Some(error) = error_message.read().as_ref() {
                            div {
                                class: "mt-2 p-2 bg-red-600/20 border border-red-600/30 rounded text-red-300 text-sm",
                                "{error}"
                            }
                        }
                    }

                    // All skills grid
                    div {
                        class: "grid grid-cols-2 gap-1 mx-auto w-fit",
                        for skill in ALL_SKILLS.iter() {
                            SkillDisplay {
                                skill: *skill
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SkillIconDisplay(skill: Skill) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    
    let (_base_level, current_level) = get_skill_levels(&app_state.read(), skill);
    
    rsx! {
        div {
            class: "flex items-center gap-1",
            img {
                class: "w-4 h-4 object-contain",
                src: "{skill.icon_path()}",
                alt: "{skill.name()}",
                title: "{skill.name()}"
            }
            span {
                class: "text-xs font-medium",
                "{current_level}"
            }
        }
    }
}

#[component]
fn SkillDisplay(skill: Skill) -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    
    let (base_level, current_level) = get_skill_levels(&app_state.read(), skill);
    
    rsx! {
        div {
            class: "flex items-center justify-center gap-1 p-1 rounded bg-gray-800/50",
            img {
                class: "w-5 h-5 object-contain flex-shrink-0",
                src: "{skill.icon_path()}",
                alt: "{skill.name()}",
                title: "{skill.name()}"
            }
            div {
                class: "flex items-center gap-1 text-sm min-w-0",
                span {
                    class: "font-bold",
                    "{current_level}"
                }
                span { "/" }
                input {
                    "type": "number",
                    class: "input w-12 h-5 text-center text-sm px-1 py-0 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none",
                    min: "0",
                    max: "99",
                    value: "{base_level}",
                    oninput: move |evt| {
                        if let Ok(new_level) = evt.value().parse::<u8>() {
                            if new_level <= 99 {
                                set_skill_base_level(&mut app_state.write(), skill, new_level as u32);
                            }
                        }
                    }
                }
            }
        }
    }
}

// Helper functions to get and set skill levels
fn get_skill_levels(app_state: &AppState, skill: Skill) -> (u32, u32) {
    match skill {
        Skill::Attack => (app_state.player.stats.attack.base, app_state.player.stats.attack.current),
        Skill::Strength => (app_state.player.stats.strength.base, app_state.player.stats.strength.current),
        Skill::Defence => (app_state.player.stats.defence.base, app_state.player.stats.defence.current),
        Skill::Ranged => (app_state.player.stats.ranged.base, app_state.player.stats.ranged.current),
        Skill::Magic => (app_state.player.stats.magic.base, app_state.player.stats.magic.current),
        Skill::Hitpoints => (app_state.player.stats.hitpoints.base, app_state.player.stats.hitpoints.current),
        Skill::Prayer => (app_state.player.stats.prayer.base, app_state.player.stats.prayer.current),
        Skill::Mining => (app_state.player.stats.mining.base, app_state.player.stats.mining.current),
        Skill::Herblore => (app_state.player.stats.herblore.base, app_state.player.stats.herblore.current),
    }
}

fn set_skill_base_level(app_state: &mut AppState, skill: Skill, level: u32) {
    match skill {
        Skill::Attack => app_state.player.stats.attack.base = level,
        Skill::Strength => app_state.player.stats.strength.base = level,
        Skill::Defence => app_state.player.stats.defence.base = level,
        Skill::Ranged => app_state.player.stats.ranged.base = level,
        Skill::Magic => app_state.player.stats.magic.base = level,
        Skill::Hitpoints => app_state.player.stats.hitpoints.base = level,
        Skill::Prayer => app_state.player.stats.prayer.base = level,
        Skill::Mining => app_state.player.stats.mining.base = level,
        Skill::Herblore => app_state.player.stats.herblore.base = level,
    }
}