use std::{cell::Cell, default, ops::{Index, IndexMut}};

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

pub struct GameSettings {
    max_weeks: u32,
    initial_request: u32,
    stock_cost: u32,
    deficit_cost: u32
}

pub struct PlayerRequest {
    role: PlayerRole,
    request: u32,
}

#[derive(Clone, Copy)]
pub struct PlayerState {
    outgoing: u32,
    incoming_request: u32,
    outgoing_request: Option<u32>,
    stock: u32,
    deficit: u32,
    incoming: u32,
    costs: u32,
}

pub struct GameState {
    week: u32,
    game_end: bool,
    players: [PlayerState; 4],
    production: u32
}

// Lots of boilerplate to make this array indexable by an enum. Might just be an outdated habit of mine, but surely there is a nicer way to do this
impl Index<PlayerRole> for [PlayerState] {
    type Output = PlayerState;

    fn index(&self, role: PlayerRole) -> &PlayerState {
        match role {
            PlayerRole::Retailer => &self[0],
            PlayerRole::Wholesaler => &self[1],
            PlayerRole::Distributor => &self[2],
            PlayerRole::Manufacturer => &self[3],
        }
    }
}

impl IndexMut<PlayerRole> for [PlayerState] {
    fn index_mut(&mut self, role: PlayerRole) -> &mut PlayerState {
        match role {
            PlayerRole::Retailer => &mut self[0],
            PlayerRole::Wholesaler => &mut self[1],
            PlayerRole::Distributor => &mut self[2],
            PlayerRole::Manufacturer => &mut self[3],
        }
    }
}

impl GameState {
    pub fn receive_request(&mut self, request: PlayerRequest) {
        self.players[request.role].outgoing_request = Some(request.request);
    }

    // Unsure how but could make this return a "ready" game state as opposed to a None to enforce type-level correctness
    // Maybe do that at the receive request level
    // Could also just attempt to take a turn and have it fail, but that feels awkward
    pub fn get_ready_state(&self) -> bool {
        self.players.iter().all(|r| r.outgoing_request.is_some())
    }

    pub fn take_turn(&mut self, settings: GameSettings) -> GameState {
        let mut players = self.players;

        // Generate request for the first player
        let customer_request = 5;

        // Propagate requests
        let mut carried_request = customer_request;
        for mut p in players {
            p.incoming_request = carried_request;
            carried_request = p.outgoing_request.unwrap();
        }

        // Warehouse incoming stock
        for mut p in players {
            p.stock += p.incoming;
        }

        // Move player's outgoing stock to the next player
        // First is the manufacturer receiving from production queue
        let mut carried_stock = self.production;
        for mut p in players.into_iter().rev() {
            p.incoming = carried_stock;
            carried_stock = p.outgoing;
        }

        // Handle the manufacturers production queue
        players[PlayerRole::Manufacturer].incoming = self.production;
        self.production = players[PlayerRole::Manufacturer].outgoing_request.unwrap();

        // Send out requested goods and calculate any deficit
        for mut p in players {
            let mut to_send = p.deficit + p.incoming_request;
            if to_send > p.stock {
                p.deficit = to_send - p.stock;
                to_send = p.stock;
            }
            p.stock -= to_send;
            p.outgoing = to_send;
        }

        // Calculate costs
        for mut p in players {
            p.costs = p.stock * settings.stock_cost 
                    + p.deficit * settings.deficit_cost;
        }

        GameState {
            week: &self.week + 1,
            game_end: self.week >= settings.max_weeks,
            players: self.players,
            production: self.production,
        }
    }
}