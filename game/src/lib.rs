use std::{collections::HashMap, ops::{Index, IndexMut}};
use serde::{Serialize, Deserialize};

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy, Eq, Hash)]
pub enum PlayerRole {
    Retailer,
    Wholesaler,
    Distributor,
    Manufacturer
}

impl PlayerRole {
    const ROLES: [Self; 4] = [  PlayerRole::Retailer, 
                                PlayerRole::Wholesaler, 
                                PlayerRole::Distributor, 
                                PlayerRole::Manufacturer];
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub name: String,
    pub role: PlayerRole
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameSettings {
    pub name: String,
    pub max_weeks: u32,
    pub initial_request: u32,
    pub stock_cost: u32,
    pub deficit_cost: u32,
    pub players: HashMap<PlayerRole, Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerRequest {
    pub game_id: i64,
    pub week: u32,
    pub role: PlayerRole,
    pub amount: u32,
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct GameState {
    pub week: u32,
    pub game_end: bool,
    pub players: [PlayerState; 4],
    pub production: u32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameListing {
    pub id: i64,
    pub name: String,
    pub available_roles: Vec<PlayerRole>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    pub settings: GameSettings,
    pub states: Vec<GameState>,
}

impl TryFrom<u32> for PlayerRole {
    type Error = ();
    
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            role if role == PlayerRole::Retailer as u32 => Ok(PlayerRole::Retailer),
            role if role == PlayerRole::Wholesaler as u32 => Ok(PlayerRole::Wholesaler),
            role if role == PlayerRole::Distributor as u32 => Ok(PlayerRole::Distributor),
            role if role == PlayerRole::Manufacturer as u32 => Ok(PlayerRole::Manufacturer),
            _ => Err(())
        }
    }
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
        self.players[request.role].outgoing_request = Some(request.amount);
    }

    // Unsure how but could make this return a "ready" game state as opposed to a None to enforce type-level correctness
    // Maybe do that at the receive request level
    // Could also just attempt to take a turn and have it fail, but that feels awkward
    pub fn get_ready_state(&self) -> bool {
        self.players.iter().all(|r| r.outgoing_request.is_some())
    }

    pub fn take_turn(self, settings: &GameSettings) -> GameState {

        let mut state = self;

        // Warehouse incoming stock
        for p in state.players.iter_mut() {
            p.stock += p.incoming;
        }

        // Move player's outgoing stock to the next player
        // First is the manufacturer receiving from production queue
        let mut carried_stock = state.production;
        for p in state.players.iter_mut().rev() {
            p.incoming = carried_stock;
            carried_stock = p.outgoing;
        }

        // Handle the manufacturers production queue
        state.players[PlayerRole::Manufacturer].incoming = state.production;
        state.production = state.players[PlayerRole::Manufacturer].outgoing_request.unwrap();

        // Propagate requests
        // Generate request for the first player
        let customer_request = 1;
        let mut carried_request = customer_request;
        for p in state.players.iter_mut() {
            p.incoming_request = carried_request;
            carried_request = p.outgoing_request.unwrap();
        }

        // Send out requested goods and calculate any deficit
        for p in state.players.iter_mut() {
            let mut to_send = p.deficit + p.incoming_request;
            if to_send > p.stock {
                p.deficit = to_send - p.stock;
                to_send = p.stock;
            }
            p.stock -= to_send;
            p.outgoing = to_send;
        }

        // Calculate costs
        for p in state.players.iter_mut() {
            p.costs = p.stock * settings.stock_cost 
                    + p.deficit * settings.deficit_cost;
        }

        state.week += 1;
        state.game_end = state.week >= settings.max_weeks;

        state
    }
}

impl Game {
    pub fn new(settings: GameSettings) -> Game {
        let initial_state = GameState {
            week: 1,
            game_end: false,
            players: [PlayerState { 
                stock: settings.initial_request,
                deficit: 0,
                incoming: settings.initial_request,
                outgoing: settings.initial_request,
                incoming_request: settings.initial_request,
                outgoing_request: None,
                costs: 0,
            }; 4],
            production: settings.initial_request,
        };

        Game {
            settings: settings,
            states: vec![initial_state],
        }
    }

    pub fn get_current_week(&self) -> u32 {
        match &self.states.last() {
            Some(s) => s.week,
            None => 1,
        }
    }

    pub fn take_turn(&mut self) {
        let state = self.states.last().unwrap().take_turn(&self.settings);
        self.states.push(state);
    }

    pub fn get_available_roles(&self) -> Vec<PlayerRole> {
        let mut roles = vec![];
        for role in PlayerRole::ROLES {
            if self.settings.players.get(&role).unwrap().is_none() { roles.push(role) };
        }
        roles
    }
}