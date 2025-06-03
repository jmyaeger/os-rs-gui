use dioxus::prelude::*;
use osrs::types::player::Player;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppState {
    pub player: Player,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            player: Player::new(),
        }
    }
}
