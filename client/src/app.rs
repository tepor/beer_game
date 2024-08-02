use std::sync::{Arc, Mutex};

use game::{self, Game, GameListing, PlayerInfo};
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
}

impl Default for ClientApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            player_name: "Player 1".to_owned(),
            game_style: GameStyleChoice::NewMultiplayer,
            available_games: Arc::new(Mutex::new((vec![]))),
            current_game: Arc::new(Mutex::new(None)),
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

            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.player_name);
            });

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.game_style, GameStyleChoice::NewSingleplayer, "New Singleplayer");
                ui.selectable_value(&mut self.game_style, GameStyleChoice::NewMultiplayer, "New Multiplayer");
                ui.selectable_value(&mut self.game_style, GameStyleChoice::JoinMultiplayer, "Join Multiplayer");
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
                    if ui.button("Refresh").clicked() {
                        // Have to pass in an Arc Mutex of the games vector since the closure wants to consume it. Unsure of a better way to do this
                        let cloned_games = self.available_games.clone();
                        fetch(Request::get("http://127.0.0.1:8000/games"), move |response| { 
                            *cloned_games.lock().unwrap() = response.ok().unwrap().json::<Vec<GameListing>>().ok().unwrap();
                        });
                    };

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
                                };
                            }
                            ui.end_row();
                        });
                    };
                },
            }

            ui.separator();

            ui.code(format!("{:#?}", self.current_game.lock().unwrap()));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
