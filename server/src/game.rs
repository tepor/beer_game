use std::{default, ops::{Index, IndexMut}};

use chrono::{DateTime, Utc};
use rocket::futures::io::Copy;


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

pub struct PlayerRequest {
    role: PlayerRole,
    request: i32,
}

#[derive(Clone, Copy)]
pub struct PlayerState {
    outgoing: i32,
    request: Option<i32>,
    stock: i32,
    incoming: i32,
}

pub struct GameState {
    week: i32,
    players: [PlayerState; 4],
    production: i32
}

// Lots of boilerplate to make this array indexable by an enum. Might just be an outdated habit of mine, but surely there is a nicer way to do this
impl Index<PlayerRole> for GameState {
    type Output = PlayerState;

    fn index(&self, role: PlayerRole) -> &PlayerState {
        match role {
            PlayerRole::Retailer => &self.players[0],
            PlayerRole::Wholesaler => &self.players[1],
            PlayerRole::Distributor => &self.players[2],
            PlayerRole::Manufacturer => &self.players[3],
        }
    }
}

impl IndexMut<PlayerRole> for GameState {
    fn index_mut(&mut self, role: PlayerRole) -> &mut PlayerState {
        match role {
            PlayerRole::Retailer => &mut self.players[0],
            PlayerRole::Wholesaler => &mut self.players[1],
            PlayerRole::Distributor => &mut self.players[2],
            PlayerRole::Manufacturer => &mut self.players[3],
        }
    }
}

impl GameState {
    pub fn receive_request(&mut self, request: PlayerRequest) {
        self[request.role].request = Some(request.request);
    }

    // Attempt to use the type system to enforce readiness before taking a turn. Unsure if this needs another layer of abstraction
    pub fn get_ready_state(&self) -> Option<&GameState> {
        if self.players.iter().all(|r| r.request.is_some()) {
            Some(self)
        } else {
            None
        }
    }

    pub fn take_turn(&mut self) {

    }
}