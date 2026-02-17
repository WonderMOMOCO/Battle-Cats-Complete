use eframe::egui;
use crate::core::import::{ImportState, AdbImportType, AdbRegion};
use crate::core::addons::adb::bridge;
use crate::core::addons::toolpaths::{self, Presence};
use crate::core::settings::Settings;
use std::sync::mpsc;
use std::path::PathBuf;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &Settings) {
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
                .selected_text(match state.adb_import_type {
                    AdbImportType::All => "All Content",
                    AdbImportType::Update => "Update Only",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.adb_import_type, AdbImportType::All, "All Content")
                        .on_hover_text("Downloads all game content\nRecommended for first-time import\nRequires root access");
                    ui.selectable_value(&mut state.adb_import_type, AdbImportType::Update, "Update Only")
                        .on_hover_text("Downloads content from the last 3-or-so game updates\nRecommended for database upkeep\nRoot access optional");
                });
        });
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Game Region:");
            egui::ComboBox::from_id_salt("adb_region_combo")
                .selected_text(match state.adb_region {
                    AdbRegion::English => "Global",
                    AdbRegion::Japanese => "Japanese",
                    AdbRegion::Taiwan => "Taiwan",
                    AdbRegion::Korean => "Korean",
                    AdbRegion::All => "All Regions",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.adb_region, AdbRegion::English, "Global");
                    ui.selectable_value(&mut state.adb_region, AdbRegion::Japanese, "Japanese");
                    ui.selectable_value(&mut state.adb_region, AdbRegion::Taiwan, "Taiwan");
                    ui.selectable_value(&mut state.adb_region, AdbRegion::Korean, "Korean");
                    ui.separator(); 
                    ui.selectable_value(&mut state.adb_region, AdbRegion::All, "All Regions")
                       .on_hover_text("Attempts to download content for ALL 4 versions sequentially.");
                });
        });
    });

    ui.add_space(15.0);

    let button_text = if is_present { "Start Import" } else { "ADB Missing" };
    if ui.add_enabled(controls_enabled, egui::Button::new(button_text)).clicked() {
        state.log_content.clear(); 
        start_adb_import(state, settings.emulator_config());
    }
}

fn start_adb_import(state: &mut ImportState, config: crate::core::settings::handle::EmulatorConfig) {
    state.is_adb_busy = true;
    state.status_message = "Initializing ADB...".to_string(); 
    
    let (tx, rx) = mpsc::channel();
    state.adb_rx = Some(rx);
    
    let output = PathBuf::from("game/app");
    let mode = state.adb_import_type;
    let region = state.adb_region;

    bridge::spawn_full_import(tx, output, mode, region, config);
}