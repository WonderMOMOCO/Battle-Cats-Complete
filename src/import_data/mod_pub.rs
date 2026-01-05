use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::path::{Path, PathBuf};
use std::env;

use super::game_data_pub as game_data;
use super::sort;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ImportMode {
    None,
    Folder,
    Zip,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum DataTab {
    Import,
    Export,
}

pub struct ImportState {
    // Import Data
    selected_path: String,
    import_mode: ImportMode,
    
    // Export Data
    compression_level: i32,

    // Shared State
    active_tab: DataTab,
    status_message: String,
    log_content: String,
    rx: Option<Receiver<String>>,
    reset_trigger: Option<f64>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            selected_path: "No source selected".to_owned(),
            import_mode: ImportMode::None,
            compression_level: 6,
            active_tab: DataTab::Import,
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
            reset_trigger: None,
        }
    }
}

impl ImportState {
    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let mut finished_just_now = false;

        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.status_message = msg.clone();
                self.log_content.push_str(&format!("{}\n", msg));

                if self.status_message.contains("Success") || self.status_message.contains("Aborted") || self.status_message.contains("Error") {
                    let current_time = ctx.input(|i| i.time);
                    self.reset_trigger = Some(current_time + 5.0);
                    finished_just_now = true;
                }
            }
            ctx.request_repaint();
        }

        if let Some(trigger_time) = self.reset_trigger {
            let current_time = ctx.input(|i| i.time);
            if current_time >= trigger_time {
                self.status_message = "Ready".to_string();
                self.rx = None;
                self.reset_trigger = None;
                self.selected_path = "No source selected".to_string();
                self.import_mode = ImportMode::None;
            } else {
                ctx.request_repaint();
            }
        }

        finished_just_now
    }
}

// Helper to obfuscate sensitive user paths
fn censor_path(path: &str) -> String {
    if path == "No source selected" {
        return path.to_string();
    }

    let mut clean = path.to_string();

    if let Ok(user) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !user.is_empty() {
             clean = clean.replace(&user, "***");
        }
    }

    let path_obj = Path::new(&clean);
    let components: Vec<_> = path_obj.components().collect();
    
    if components.len() > 3 {
        let count = components.len();
        let last_parts: PathBuf = components.iter().skip(count.saturating_sub(3)).collect();
        return format!("...{}{}", std::path::MAIN_SEPARATOR, last_parts.display());
    }

    clean
}

pub fn show(ctx: &egui::Context, state: &mut ImportState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Game Data Management");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 

            let tabs = [(DataTab::Import, "Import"), (DataTab::Export, "Export")];
            
            for (tab, label) in tabs {
                let is_selected = state.active_tab == tab;
                
                // Form Button Style
                let (fill, stroke, text_color) = if is_selected {
                    (
                        egui::Color32::from_rgb(0, 100, 200), 
                        egui::Stroke::new(2.0, egui::Color32::WHITE), 
                        egui::Color32::WHITE
                    )
                } else {
                    (
                        egui::Color32::from_gray(40), 
                        egui::Stroke::new(1.0, egui::Color32::from_gray(100)), 
                        egui::Color32::from_gray(200)
                    )
                };

                let btn = egui::Button::new(egui::RichText::new(label).color(text_color))
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(egui::Rounding::ZERO)
                    .min_size(egui::vec2(80.0, 30.0));

                if ui.add(btn).clicked() {
                    state.active_tab = tab;
                }
            }
        });

        ui.add_space(15.0);

        match state.active_tab {
            DataTab::Import => show_import_ui(ui, state),
            DataTab::Export => show_export_ui(ui, state),
        }

        ui.add_space(5.0);
        ui.separator();
        
        if state.rx.is_some() && !state.status_message.contains("Success") && !state.status_message.contains("Error") && !state.status_message.contains("Aborted") {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&state.status_message);
            });
        } else {
            if state.status_message.contains("Error") || state.status_message.contains("Aborted") {
                ui.colored_label(egui::Color32::RED, &state.status_message);
            } else if state.status_message.contains("Success") {
                ui.colored_label(egui::Color32::GREEN, &state.status_message);
            } else {
                ui.colored_label(egui::Color32::LIGHT_BLUE, &state.status_message);
            }
        }

        ui.separator();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;

                for line in state.log_content.lines() {
                    if line.contains("was found!") {
                        ui.label(egui::RichText::new(line).color(egui::Color32::GREEN).monospace());
                    } else if line.contains("Error") || line.contains("Aborted") {
                        ui.label(egui::RichText::new(line).color(egui::Color32::RED).monospace());
                    } else {
                        ui.label(egui::RichText::new(line).monospace());
                    }
                }
            })
    });
}

fn show_import_ui(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Import files into the local system.").strong());
    ui.add_space(10.0);

    let btn_enabled = state.rx.is_none();

    ui.horizontal(|ui| {
        let btn_folder = egui::Button::new("Select Raw Folder");
        let resp_folder = ui.add_enabled(btn_enabled, btn_folder);
        
        if resp_folder.clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                state.selected_path = path.display().to_string();
                state.import_mode = ImportMode::Folder;
                state.status_message = "Raw Folder selected.".to_string();
                state.log_content.clear();
            }
        }
        resp_folder.on_hover_text("Select a folder containing raw decrypted game data so it can be sorted into the system!");

        ui.add_space(0.0);

        let btn_zip = egui::Button::new("Select game.zip");
        let resp_zip = ui.add_enabled(btn_enabled, btn_zip);

        if resp_zip.clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Archive", &["zip"])
                .pick_file() 
            {
                state.selected_path = path.display().to_string();
                state.import_mode = ImportMode::Zip;
                state.status_message = "Zip Archive selected.".to_string();
                state.log_content.clear();
            }
        }
        resp_zip.on_hover_text("Select a 'game.zip' file provided by the community that contains only essential pre-sorted files!");
    });

    ui.add_space(5.0);
    ui.monospace(censor_path(&state.selected_path));
    
    ui.add_space(5.0);

    let can_start = state.import_mode != ImportMode::None && state.rx.is_none();
    
    if ui.add_enabled(can_start, egui::Button::new("Start File Sort")).clicked() {
        state.status_message = "Starting worker...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);

        let path = state.selected_path.clone();
        let mode = state.import_mode;

        thread::spawn(move || {
            let result = match mode {
                ImportMode::Folder => game_data::import_from_folder(&path, tx.clone()),
                ImportMode::Zip => game_data::import_from_zip(&path, tx.clone()),
                _ => Err("Invalid mode".to_string()),
            };

            match result {
                Ok(_) => {
                    if mode == ImportMode::Folder {
                        let _ = tx.send("Starting Sort...".to_string());
                        match sort::sort_game_files(tx.clone()) {
                            Ok(_) => { let _ = tx.send("Success! Files imported and sorted.".to_string()); },
                            Err(e) => { let _ = tx.send(format!("Error Sorting: {}", e)); }
                        }
                    } else {
                        let _ = tx.send("Success! Archive extracted.".to_string());
                    }
                },
                Err(e) => { let _ = tx.send(format!("Error: {}", e)); }
            }
        });
    }
}

fn show_export_ui(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Package sorted files into a ZIP archive.").strong());
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Compression Level:");
        ui.add(egui::Slider::new(&mut state.compression_level, 0..=9));
    });
    
    ui.add_space(15.0);

    let can_zip = state.rx.is_none(); 
    
    if ui.add_enabled(can_zip, egui::Button::new("Create game.zip")).clicked() {
        state.status_message = "Preparing to zip...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);
        let level = state.compression_level;

        thread::spawn(move || {
            match game_data::create_game_zip(tx.clone(), level) {
                Ok(_) => {}, 
                Err(e) => { let _ = tx.send(format!("Error Zipping: {}", e)); }
            }
        });
    }
}