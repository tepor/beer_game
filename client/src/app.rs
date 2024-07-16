#[derive(PartialEq)]
pub enum GameStyleChoice {
    NewSingleplayer,
    NewMultiplayer,
    JoinMultiplayer
}

pub struct ClientApp {
    // Example stuff:
    player_name: String,
    game_style: GameStyleChoice
}

impl Default for ClientApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            player_name: "Player 1".to_owned(),
            game_style: GameStyleChoice::NewSingleplayer,
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
                ui.selectable_value(&mut self.game_style, GameStyleChoice::NewSingleplayer, "Singleplayer");
                ui.selectable_value(&mut self.game_style, GameStyleChoice::NewMultiplayer, "Multiplayer");
            });

            if ui.button("Start game").clicked() {
                log::info!("player_name = {:?}", self.player_name);
            }

            ui.separator();

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
