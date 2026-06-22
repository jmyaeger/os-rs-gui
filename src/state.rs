use osrs::types::player::Player;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub player: Player,
}
