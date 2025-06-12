use osrs::types::player::Player;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppState {
    pub player: Player,
}
