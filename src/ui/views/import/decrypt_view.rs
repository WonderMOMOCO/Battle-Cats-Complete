#[cfg(feature = "dev")]
use eframe::egui;
#[cfg(feature = "dev")]
use crate::core::import::{ImportState, GameRegion, game_data, sort};
#[cfg(feature = "dev")]
use std::sync::mpsc;

#[cfg(feature = "dev")]
pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Decrypt and extract game files.").strong());
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Select Game Type:");
        let enabled = state.rx.is_none();
        ui.add_enabled_ui(enabled, |ui| {
            ui.radio_value(&mut state.selected_region, GameRegion::Global, "Global");
            ui.radio_value(&mut state.selected_region, GameRegion::Japan, "Japan");
            ui.radio_value(&mut state.selected_region, GameRegion::Taiwan, "Taiwan");
            ui.radio_value(&mut state.selected_region, GameRegion::Korean, "Korea");
            ui.radio_value(&mut state.selected_region, GameRegion::Mod, "Mod");
        });
    });

    ui.add_space(5.0);

    ui.horizontal(|ui| {
        let btn_enabled = state.rx.is_none();
        if ui.add_enabled(btn_enabled, egui::Button::new("Select Game Folder")).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                state.set_decrypt_path(path.display().to_string());
                state.status_message = "Folder selected.".to_string();
            }
        }
        ui.monospace(&state.decrypt_censored);
    });

    ui.add_space(15.0);

    let can_start = !state.decrypt_path.is_empty() && state.rx.is_none();
    
    if ui.add_enabled(can_start, egui::Button::new("Start Decryption")).clicked() {
        state.status_message = "Initializing Decryptor...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);

        let folder = state.decrypt_path.clone();
        let region_code = state.selected_region.code().to_string();

        std::thread::spawn(move || {
            if let Err(e) = game_data::run_dev_decryption(&folder, &region_code, tx.clone()) {
                let _ = tx.send(format!("Error: {}", e));
                return; 
            }
            
            let _ = tx.send("Sorting extracted files...".to_string());
            if let Err(e) = sort::sort_game_files(tx.clone()) {
                let _ = tx.send(format!("Error Sorting: {}", e));
            } else {
                let _ = tx.send("Success! Decryption and sort complete.".to_string());
            }
        });
    }
}