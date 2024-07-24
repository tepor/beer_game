
use server::game::{GameSettings, GameState, PlayerRequest, PlayerRole, PlayerState};
#[macro_use] extern crate rocket;


#[get("/gamestate")]
fn serve_gamestate() -> &'static str {
    "Hello game"
}

#[post("/submitrequest", format="application/json", data="<pr>")]
fn receive_request(pr: rocket::serde::json::Json<PlayerRequest>) {
    println!("Player {:?} requested {:?}", pr.role, pr.request);
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![serve_gamestate, receive_request])
}


// fn main() {

//     let game_settings = GameSettings {
//         max_weeks: 5,
//         initial_request: 4,
//         stock_cost: 5,
//         deficit_cost: 25,
//     };

//     let mut game_state = GameState {
//         week: 0,
//         game_end: false,
//         players: [PlayerState {
//             stock: 4,
//             deficit: 0,
//             incoming: 4,
//             outgoing: 4,
//             incoming_request: 4,
//             outgoing_request: None,
//             costs: 0,
//         }; 4],
//         production: 4,
//     };

//     println!("{:#?}", game_state);

//     game_state.receive_request(PlayerRequest{role: PlayerRole::Retailer, request: 1});
//     game_state.receive_request(PlayerRequest{role: PlayerRole::Wholesaler, request: 1});
//     game_state.receive_request(PlayerRequest{role: PlayerRole::Distributor, request: 1});
//     game_state.receive_request(PlayerRequest{role: PlayerRole::Manufacturer, request: 1});

//     let serialized = serde_json::to_string(&game_state).unwrap();
//     println!("serialized = {}", serialized);

// }
