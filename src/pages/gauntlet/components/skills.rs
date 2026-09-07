use crate::hiscores::fetch_player_stats;
use crate::pages::gauntlet::state::GauntletState;
use dioxus::prelude::*;
use osrs::types::player::Player;

#[component]
pub fn SkillSelect() -> Element {
    let mut player = use_context::<GauntletState>().player;
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

                let result = { lookup_stats(&mut player, &rsn).await };

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
    let mut player = use_context::<GauntletState>().player;
    let base_level = get_skill_level(&player.read(), skill);
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
                        set_skill_base_level(&mut player.write(), skill, new_level.clamp(1, 99));
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

async fn lookup_stats(player: &mut Signal<Player>, rsn: &str) -> Result<(), String> {
    let stats = fetch_player_stats(rsn).await?;
    let mut player = player.write();
    player.stats = stats;
    player.attrs.name = Some(rsn.to_string());
    Ok(())
}

fn get_skill_level(player: &Player, skill: Skill) -> u32 {
    match skill {
        Skill::Attack => player.stats.attack.current,
        Skill::Strength => player.stats.strength.current,
        Skill::Defence => player.stats.defence.current,
        Skill::Ranged => player.stats.ranged.current,
        Skill::Magic => player.stats.magic.current,
        Skill::Hitpoints => player.stats.hitpoints.current,
    }
}

fn set_skill_base_level(player: &mut Player, skill: Skill, level: u32) {
    match skill {
        Skill::Attack => player.stats.attack.base = level,
        Skill::Strength => player.stats.strength.base = level,
        Skill::Defence => player.stats.defence.base = level,
        Skill::Ranged => player.stats.ranged.base = level,
        Skill::Magic => player.stats.magic.base = level,
        Skill::Hitpoints => player.stats.hitpoints.base = level,
    }
    player.reset_current_stats(true);
}
