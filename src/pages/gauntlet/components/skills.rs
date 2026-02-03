use anyhow::Result;
use gloo_net::http::Request;
use js_sys::encode_uri_component;
use osrs::types::stats::{PlayerStats, SpecEnergy, Stat};
use std::collections::HashMap;

use crate::pages::gauntlet::state::AppState;
use dioxus::prelude::*;

const HISCORES_API_URL: &str = "https://hiscores-proxy.jmyaeger.workers.dev";

#[component]
pub fn SkillSelect() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut rsn_input = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    let mut perform_lookup = move || {
        let rsn = rsn_input.read().trim().to_string();
        if !rsn.is_empty() {
            is_loading.set(true);
            spawn(async move {
                // Clear any previous error
                error_message.set(None);

                let result = { lookup_stats(&mut app_state, &rsn).await };

                match result {
                    Ok(()) => {
                        rsn_input.set(String::new()); // Clear the input on success
                    }
                    Err(e) => {
                        error_message.set(Some(format!("Failed to lookup stats: {e}")));
                    }
                }

                is_loading.set(false);
            });
        }
    };

    let lookup_stats = move |_| perform_lookup();

    rsx! {
        div { class: "grid grid-cols-2 gap-1",
            div { class: "col-span-2 flex items-center gap-2 rounded px-1 py-1",
                input {
                    "type": "text",
                    class: "flex-1 min-w-0 h-7 text-sm px-2 input-field rounded text-white placeholder-gray-400 focus:outline-none",
                    placeholder: "Enter RSN...",
                    value: "{rsn_input}",
                    disabled: is_loading(),
                    oninput: move |evt| rsn_input.set(evt.value()),
                    onkeydown: move |evt| {
                        let rsn = rsn_input.read();
                        if rsn.trim().is_empty() {
                            return;
                        }
                        if evt.key() == Key::Enter {
                            perform_lookup();
                        }
                    },
                }
                button {
                    class: "h-7 px-3 btn-accent disabled:bg-gray-500 disabled:cursor-not-allowed rounded text-xs shrink-0",
                    disabled: rsn_input.read().trim().is_empty() || is_loading(),
                    onclick: lookup_stats,
                    if is_loading() {
                        "..."
                    } else {
                        "Lookup"
                    }
                }
            }

            // Error message display
            if let Some(error) = error_message.read().as_ref() {
                div { class: "col-span-2 p-2 bg-red-500/10 border border-red-500/30 rounded text-red-300 text-sm",
                    "{error}"
                }
            }

            // All skills grid
            for skill in COMBAT_SKILLS.iter() {
                SkillDisplay { skill: *skill }
            }

        }
    }
}

#[component]
pub fn SkillDisplay(skill: Skill) -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let base_level = get_skill_level(&app_state.read(), skill);
    rsx! {
        div { class: "flex items-center justify-between py-1 px-1 rounded",
            span { class: "text-sm text-gray-400", "{skill.name()}" }
            input {
                "type": "number",
                class: "w-10 h-7 text-sm px-1 input-field rounded text-center text-white num focus:outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none",
                min: "1",
                max: "99",
                value: "{base_level}",
                oninput: move |evt| {
                    if let Ok(new_level) = evt.value().parse::<u32>() {
                        set_skill_base_level(&mut app_state.write(), skill, new_level.clamp(1, 99));
                    }
                },
            }
        }
    }
}

// Define skill types and order
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Skill {
    Attack,
    Strength,
    Defence,
    Ranged,
    Magic,
    Hitpoints,
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
        }
    }
}

const COMBAT_SKILLS: [Skill; 6] = [
    Skill::Attack,
    Skill::Strength,
    Skill::Defence,
    Skill::Ranged,
    Skill::Magic,
    Skill::Hitpoints,
];

async fn fetch_player_data(rsn: String) -> Result<String> {
    let encoded_rsn = encode_uri_component(&rsn);
    let url = format!("{}/?player={}", HISCORES_API_URL, encoded_rsn);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Network error: {e}"))?;

    let status = response.status();

    if status == 404 {
        return Err(anyhow::anyhow!("Player not found: {rsn}"));
    }

    if status != 200 {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "API request failed ({}): {}",
            status,
            error_text
        ));
    }

    let data = response
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read response: {e}"))?;

    Ok(data)
}

pub fn parse_player_data(data: String) -> Result<PlayerStats> {
    // Parses player data and creates a PlayerStats struct from it
    let skills = [
        "attack",
        "defence",
        "strength",
        "hitpoints",
        "ranged",
        "prayer",
        "magic",
    ];
    let data_lines: Vec<&str> = data.lines().collect();
    let mut skill_map = HashMap::new();

    for (i, skill) in skills.iter().enumerate() {
        let line_parts: Vec<&str> = data_lines[i + 1].split(',').collect();
        let level = line_parts[1].parse::<u32>()?;
        skill_map.insert(*skill, level);
    }

    let mining_lvl = data_lines[15].split(',').collect::<Vec<&str>>()[1];
    skill_map.insert("mining", mining_lvl.parse::<u32>()?);
    let herblore_lvl = data_lines[16].split(',').collect::<Vec<&str>>()[1];
    skill_map.insert("herblore", herblore_lvl.parse::<u32>()?);

    Ok(PlayerStats {
        hitpoints: Stat::new(skill_map["hitpoints"], None),
        attack: Stat::new(skill_map["attack"], None),
        strength: Stat::new(skill_map["strength"], None),
        defence: Stat::new(skill_map["defence"], None),
        ranged: Stat::new(skill_map["ranged"], None),
        magic: Stat::new(skill_map["magic"], None),
        prayer: Stat::new(skill_map["prayer"], None),
        mining: Stat::new(skill_map["mining"], None),
        herblore: Stat::new(skill_map["herblore"], None),
        spec: SpecEnergy::default(),
    })
}

async fn lookup_stats(app_state: &mut Signal<AppState>, rsn: &str) -> Result<()> {
    let stats_data = fetch_player_data(rsn.to_string()).await?;
    let mut state = app_state.write();
    state.player.stats = parse_player_data(stats_data)?;
    state.player.attrs.name = Some(rsn.to_string());
    Ok(())
}

fn get_skill_level(app_state: &AppState, skill: Skill) -> u32 {
    match skill {
        Skill::Attack => app_state.player.stats.attack.current,
        Skill::Strength => app_state.player.stats.strength.current,
        Skill::Defence => app_state.player.stats.defence.current,
        Skill::Ranged => app_state.player.stats.ranged.current,
        Skill::Magic => app_state.player.stats.magic.current,
        Skill::Hitpoints => app_state.player.stats.hitpoints.current,
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
    }
    app_state.player.reset_current_stats(true);
}
