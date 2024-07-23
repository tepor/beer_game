use server::game::{GameSettings, GameState, PlayerRequest, PlayerRole, PlayerState};

fn main() {

    let game_settings = GameSettings {
        max_weeks: 5,
        initial_request: 4,
        stock_cost: 5,
        deficit_cost: 25,
    };

    let mut game_state = GameState {
        week: 0,
        game_end: false,
        players: [PlayerState {
            stock: 4,
            deficit: 0,
            incoming: 4,
            outgoing: 4,
            incoming_request: 4,
            outgoing_request: None,
            costs: 0,
        }; 4],
        production: 4,
    };

    println!("{:#?}", game_state);

    game_state.receive_request(PlayerRequest{role: PlayerRole::Retailer, request: 1});
    game_state.receive_request(PlayerRequest{role: PlayerRole::Wholesaler, request: 1});
    game_state.receive_request(PlayerRequest{role: PlayerRole::Distributor, request: 1});
    game_state.receive_request(PlayerRequest{role: PlayerRole::Manufacturer, request: 1});

    println!("{:#?}", game_state);

    game_state.take_turn(&game_settings);

    println!("{:#?}", game_state);

    game_state.take_turn(&game_settings);

    println!("{:#?}", game_state);

    game_state.take_turn(&game_settings);

    println!("{:#?}", game_state);

}
