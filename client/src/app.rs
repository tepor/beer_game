use std::{collections::HashMap, sync::{Arc, Mutex}};

use game::{self, Game, GameListing, GameSettings, PlayerInfo, PlayerRequest, PlayerRole};
use ehttp::{fetch,Request};

#[derive(PartialEq)]
pub enum GameStyleChoice {
    NewSingleplayer,
    NewMultiplayer,
    JoinMultiplayer
}

pub struct ClientApp {
    // Example stuff:
    player_name: String,
    game_style: GameStyleChoice,
    available_games: Arc<Mutex<Vec<game::GameListing>>>,
    current_game: Arc<Mutex<Option<Game>>>,
    current_game_id: Option<i64>,
    player_info: Option<PlayerInfo>,
    new_game_settings: GameSettings,
    outgoing_request: u32,
}

impl Default for ClientApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            player_name: "Player 1".to_owned(),
            game_style: GameStyleChoice::NewMultiplayer,
            available_games: Arc::new(Mutex::new(vec![])),
            current_game: Arc::new(Mutex::new(None)),
            current_game_id: None,
            player_info: None,
            new_game_settings: GameSettings {
                name: "Default Game".to_owned(),
                max_weeks: 5,
                initial_request: 4,
                stock_cost: 5,
                deficit_cost: 25,
                players: HashMap::from([
                    (PlayerRole::Retailer, None),
                    (PlayerRole::Wholesaler, None),
                    (PlayerRole::Distributor, None),
                    (PlayerRole::Manufacturer, None),
                ]),
            },
            outgoing_request: 4
        }
    }
}

impl ClientApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Default::default()
    }

    fn update_games_list(&mut self) {
        // Have to pass in an Arc Mutex of the games vector since the closure wants to consume it. Unsure of a better way to do this
        let cloned_games = self.available_games.clone();
        fetch(Request::get("http://127.0.0.1:8000/games"), move |response| { 
            *cloned_games.lock().unwrap() = response.ok().unwrap().json::<Vec<GameListing>>().ok().unwrap();
        });
    }
}

impl eframe::App for ClientApp {

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("The Beer Distribution Game");
            ui.separator();

            // Get a copy of the current game to do UI things
            let game = self.current_game.lock().unwrap().clone();

            match (game, &self.player_info) {
                // Game play UI
                (Some(game), Some(pi)) => {
                    let state = game.states.last().unwrap();

                    ui.heading(format!("{} (ID: {})", game.settings.name, self.current_game_id.unwrap()));
                    ui.heading(format!("{:?}", pi.role));
                    ui.label(&pi.name);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.heading("Downstream");
                            ui.label(format!("Requested: {}", state.players[pi.role].incoming_request));
                            ui.label(format!("Outgoing: {}", state.players[pi.role].outgoing));
                        });
                        ui.vertical(|ui| {
                            ui.heading("Current");
                            ui.label(format!("Stock: {}", state.players[pi.role].stock));
                            ui.label(format!("Deficit: {}", state.players[pi.role].deficit));
                            ui.label(format!("Costs: {}", state.players[pi.role].costs));
                        });
                        ui.vertical(|ui| {
                            ui.heading("Upstream");
                            ui.horizontal(|ui| {
                                ui.add(egui::widgets::DragValue::new(&mut self.outgoing_request));
                                if ui.button("Submit").clicked() {
                                    // Submit a request
                                    let r = PlayerRequest {
                                        game_id: self.current_game_id.unwrap(),
                                        week: state.week,
                                        role: pi.role,
                                        amount: self.outgoing_request,
                                    };
                                    fetch(Request::json("http://127.0.0.1:8000/submitrequest", &r).unwrap(), |_| {});
                                };
                            });
                            ui.label(format!("Incoming: {}", state.players[pi.role].incoming));
                        });
                    });
                }

                // Game selection UI
                _ => {
                    ui.horizontal(|ui| {
                        ui.label("Your name: ");
                        ui.text_edit_singleline(&mut self.player_name);
                    });

                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.game_style, GameStyleChoice::NewSingleplayer, "New Singleplayer");
                        ui.selectable_value(&mut self.game_style, GameStyleChoice::NewMultiplayer, "New Multiplayer");
                        if ui.selectable_value(&mut self.game_style, GameStyleChoice::JoinMultiplayer, "Join Multiplayer").clicked() {
                            self.update_games_list()
                        };
                    });

                    match &self.game_style {
                        GameStyleChoice::NewSingleplayer => {},
                        GameStyleChoice::NewMultiplayer => {
                            if ui.button("Start game").clicked() {
                                log::info!("player_name = {:?}", self.player_name);
                            }
                        },
                        GameStyleChoice::JoinMultiplayer => {
                            // Fetch available games
                            if ui.button("Refresh").clicked() {self.update_games_list()};

                            ui.separator();

                            // List available games
                            for game in self.available_games.lock().unwrap().to_owned() {
                                ui.horizontal(|ui| {
                                    ui.label(game.name);
                                    for role in game.available_roles {
                                        if ui.button(format!("Join as {:?}", role)).clicked() { 
                                            let pi = PlayerInfo {
                                                                                name: self.player_name.clone(), 
                                                                                role: role,
                                                                            };
                                            let cloned_game = self.current_game.clone();
                                            let url = format!("http://127.0.0.1:8000/joingame/{}", game.id);
                                            fetch(Request::json(url, &pi).unwrap(), move |response| {
                                                let r = response.unwrap();
                                                match r.ok {
                                                    true => *cloned_game.lock().unwrap() = Some(r.json::<Game>().ok().unwrap()),
                                                    false => {}
                                                };
                                            });
                                            // Update our state
                                            self.player_info = Some(pi.clone());
                                            self.current_game_id = Some(game.id);
                                        };
                                    }
                                    ui.end_row();
                                });
                            };
                        },
                    }
                }
            }

            

            ui.separator();

            ui.code(format!("{:#?}", self.current_game.lock().unwrap()));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
