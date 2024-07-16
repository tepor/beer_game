use chrono::{DateTime, Utc};


#[derive(PartialEq)]
pub enum PlayerRole {
    Retailer,
    Wholesaler,
    Distributor,
    Manufacturer
}

pub struct PlayerInfo {
    name: String,
    role: PlayerRole
}

pub struct GameInfo {
    start_time: DateTime<Utc>,
    name: String,
    player_info: [Option<PlayerInfo>; 4],
}