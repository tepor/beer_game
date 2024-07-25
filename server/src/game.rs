use std::ops::{Index, IndexMut};

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};


#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub enum PlayerRole {
    Retailer,
    Wholesaler,
    Distributor,
    Manufacturer
}

#[derive(Serialize, Deserialize)]
pub struct PlayerInfo {
    name: String,
    role: PlayerRole
}

#[derive(Serialize, Deserialize)]
pub struct GameSettings {
    pub name: String,
    pub max_weeks: u32,
    pub initial_request: u32,
    pub stock_cost: u32,
    pub deficit_cost: u32
}

#[derive(Serialize, Deserialize)]
pub struct PlayerRequest {
    pub role: PlayerRole,
    pub request: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub stock: u32,
    pub deficit: u32,
    pub incoming: u32,
    pub outgoing: u32,
    pub incoming_request: u32,
    pub outgoing_request: Option<u32>,
    pub costs: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    pub week: u32,
    pub game_end: bool,
    pub players: [PlayerState; 4],
    pub production: u32
}

pub struct Game {
    pub settings: GameSettings,
    pub state: GameState,
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

    pub fn take_turn(&mut self, settings: &GameSettings) {

        // Warehouse incoming stock
        for p in self.players.iter_mut() {
            p.stock += p.incoming;
        }

        // Move player's outgoing stock to the next player
        // First is the manufacturer receiving from production queue
        let mut carried_stock = self.production;
        for p in self.players.iter_mut().rev() {
            p.incoming = carried_stock;
            carried_stock = p.outgoing;
        }

        // Handle the manufacturers production queue
        self.players[PlayerRole::Manufacturer].incoming = self.production;
        self.production = self.players[PlayerRole::Manufacturer].outgoing_request.unwrap();

        // Propagate requests
        // Generate request for the first player
        let customer_request = 1;
        let mut carried_request = customer_request;
        for p in self.players.iter_mut() {
            p.incoming_request = carried_request;
            carried_request = p.outgoing_request.unwrap();
        }

        // Send out requested goods and calculate any deficit
        for p in self.players.iter_mut() {
            let mut to_send = p.deficit + p.incoming_request;
            if to_send > p.stock {
                p.deficit = to_send - p.stock;
                to_send = p.stock;
            }
            p.stock -= to_send;
            p.outgoing = to_send;
        }

        // Calculate costs
        for p in self.players.iter_mut() {
            p.costs = p.stock * settings.stock_cost 
                    + p.deficit * settings.deficit_cost;
        }

        self.week += 1;
        self.game_end = self.week >= settings.max_weeks;
    }
}