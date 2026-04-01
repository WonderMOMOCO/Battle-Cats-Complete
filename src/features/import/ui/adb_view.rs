use eframe::egui;
use crate::features::import::logic::{ImportState, AdbImportType, AdbRegion};
use crate::features::addons::adb::bridge;
use crate::features::addons::toolpaths::{self, Presence};
use crate::features::settings::logic::Settings;
use std::sync::mpsc;
use std::path::PathBuf;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &mut Settings) {
    let is_present = toolpaths::adb_status() == Presence::Installed;
    let busy = state.is_adb_busy;
    
    if is_present {
        ui.label("Import, decrypt, and sort game files from android/emulator into database")
            .on_hover_text("Supported emulators include: MuMu, Nox, and LDPlayer");
    } else {
        ui.label(
            egui::RichText::new("Android Bridge is required to utilize this feature, you can download it through Settings > Add-Ons > Android Bridge")
                .color(egui::Color32::from_rgb(200, 150, 50))
        );
    }
    
    ui.add_space(10.0);

    let controls_enabled = !busy && is_present;

    ui.add_enabled_ui(controls_enabled, |ui| {
        ui.horizontal(|ui| {
            ui.label("Import Type:");
            egui::ComboBox::from_id_salt("adb_type_combo")
                .selected_text(match settings.game_data.adb_import_type_idx {
                    1 => "Update Only",
                    _ => "All Content",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.game_data.adb_import_type_idx, 0, "All Content")
                        .on_hover_text("Downloads all game content\nRecommended for first-time import\nRequires root access");
                    ui.selectable_value(&mut settings.game_data.adb_import_type_idx, 1, "Update Only")
                        .on_hover_text("Downloads content from the last 3-or-so game updates\nRecommended for database upkeep\nRoot access optional");
                });
        });
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Game Region:");
            egui::ComboBox::from_id_salt("adb_region_combo")
                .selected_text(match settings.game_data.adb_region_idx {
                    0 => "Global",
                    1 => "Japanese",
                    2 => "Taiwan",
                    3 => "Korean",
                    _ => "All Regions",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.game_data.adb_region_idx, 0, "Global");
                    ui.selectable_value(&mut settings.game_data.adb_region_idx, 1, "Japanese");
                    ui.selectable_value(&mut settings.game_data.adb_region_idx, 2, "Taiwan");
                    ui.selectable_value(&mut settings.game_data.adb_region_idx, 3, "Korean");
                    ui.separator(); 
                    ui.selectable_value(&mut settings.game_data.adb_region_idx, 4, "All Regions")
                        .on_hover_text("Attempts to download content for ALL 4 versions sequentially.");
                });
        });
    });

    ui.add_space(15.0);

    let button_text = if is_present { "Start Import" } else { "ADB Missing" };
    if ui.add_enabled(controls_enabled, egui::Button::new(button_text)).clicked() {
        state.log_content.clear(); 
        start_adb_import(state, settings);
    }
}

fn start_adb_import(state: &mut ImportState, settings: &Settings) {
    state.is_adb_busy = true;
    state.status_message = "Initializing ADB...".to_string(); 
    
    let (tx, rx) = mpsc::channel();
    state.adb_rx = Some(rx);
    
    let output = PathBuf::from("game/app");
    
    // Map the saved indices to the actual Enums for processing
    let mode = match settings.game_data.adb_import_type_idx {
        1 => AdbImportType::Update,
        _ => AdbImportType::All,
    };
    
    let region = match settings.game_data.adb_region_idx {
        0 => AdbRegion::English,
        1 => AdbRegion::Japanese,
        2 => AdbRegion::Taiwan,
        3 => AdbRegion::Korean,
        _ => AdbRegion::All,
    };

    let config = settings.emulator_config();

    bridge::spawn_full_import(tx, output, mode, region, config);
}