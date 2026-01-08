#[cfg(feature = "dev")]
use eframe::egui;
#[cfg(feature = "dev")]
use crate::core::import::{ImportState, GameRegion};
#[cfg(feature = "dev")]
use std::sync::mpsc;

#[cfg(feature = "dev")]
pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Extract and decrypt game files.").strong());
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Select Game Region:");
        let enabled = state.rx.is_none();
        ui.add_enabled_ui(enabled, |ui| {
            ui.radio_value(&mut state.selected_region, GameRegion::Global, "Global");
            ui.radio_value(&mut state.selected_region, GameRegion::Japan, "Japan");
            ui.radio_value(&mut state.selected_region, GameRegion::Taiwan, "Taiwan");
            ui.radio_value(&mut state.selected_region, GameRegion::Korean, "Korea");
        });
    });

    ui.add_space(5.0);

    ui.horizontal(|ui| {
        let btn_enabled = state.rx.is_none();
        if ui.add_enabled(btn_enabled, egui::Button::new("Select Game Folder")).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                // Use new setter
                state.set_extract_folder(path.display().to_string());
                state.status_message = "Folder selected.".to_string();
            }
        }
        ui.monospace(&state.censored_folder);
    });

    ui.add_space(15.0);

    let can_start = state.selected_folder != "No folder selected" && state.rx.is_none();
    
    if ui.add_enabled(can_start, egui::Button::new("Start Extraction")).clicked() {
        state.status_message = "Initializing Decryptor...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);

        let folder = state.selected_folder.clone();
        let region_code = state.selected_region.code().to_string();

        std::thread::spawn(move || {
            if let Err(e) = crate::dev::extract_data::run_extraction(folder, region_code, tx.clone()) {
                let _ = tx.send(format!("Error: {}", e));
            }
        });
    }
}