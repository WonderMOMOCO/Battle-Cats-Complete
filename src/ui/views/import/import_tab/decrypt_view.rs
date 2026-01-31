use eframe::egui;
use crate::core::import::{ImportState, decrypt, sort};
use std::sync::mpsc;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label("Decrypt and sort game files into database");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Region:");
        egui::ComboBox::from_id_salt("decrypt_region") 
            .selected_text(match state.adb_region {
                crate::core::import::AdbRegion::English => "Global",
                crate::core::import::AdbRegion::Japanese => "Japan",
                crate::core::import::AdbRegion::Taiwan => "Taiwan",
                crate::core::import::AdbRegion::Korean => "Korea",
                crate::core::import::AdbRegion::All => "All",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.adb_region, crate::core::import::AdbRegion::English, "Global");
                ui.selectable_value(&mut state.adb_region, crate::core::import::AdbRegion::Japanese, "Japan");
                ui.selectable_value(&mut state.adb_region, crate::core::import::AdbRegion::Taiwan, "Taiwan");
                ui.selectable_value(&mut state.adb_region, crate::core::import::AdbRegion::Korean, "Korea");
            });
    });

    ui.add_space(5.0);

    ui.horizontal(|ui| {
        if ui.add_enabled(state.rx.is_none(), egui::Button::new("Select Folder")).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                state.set_decrypt_path(path.display().to_string());
            }
        }
        ui.monospace(&state.decrypt_censored);
    });

    ui.add_space(15.0);

    let can_start = !state.decrypt_path.is_empty() && state.rx.is_none();
    
    if ui.add_enabled(can_start, egui::Button::new("Start Decryption")).clicked() {
        state.status_message = "Initializing...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);

        let folder = state.decrypt_path.clone();
        
        // Map AdbRegion to the code string expected by decrypt.rs
        let region_code = match state.adb_region {
            crate::core::import::AdbRegion::English => "en",
            crate::core::import::AdbRegion::Japanese => "jp", 
            crate::core::import::AdbRegion::Taiwan => "tw",
            crate::core::import::AdbRegion::Korean => "kr",
            _ => "en",
        }.to_string();

        std::thread::spawn(move || {
            if let Err(e) = decrypt::run(&folder, &region_code, tx.clone()) {
                let _ = tx.send(format!("Error: {}", e));
                return; 
            }
            
            let _ = tx.send("Sorting extracted files...".to_string());
            if let Err(e) = sort::sort_game_files(tx.clone()) {
                let _ = tx.send(format!("Error Sorting: {}", e));
            } else {
                let _ = tx.send("Success! Process complete.".to_string());
            }
        });
    }
}