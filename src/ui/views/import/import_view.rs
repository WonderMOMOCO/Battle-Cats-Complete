use eframe::egui;
use crate::core::import::{ImportState, ImportMode};
use std::sync::mpsc;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Import game files from a folder or ZIP archive.").strong());
    ui.add_space(10.0);

    // Mode Selection
    ui.horizontal(|ui| {
        ui.label("Import Mode:");
        ui.radio_value(&mut state.import_mode, ImportMode::Zip, "game.zip");
        ui.radio_value(&mut state.import_mode, ImportMode::Folder, "Folder");
    });

    ui.add_space(10.0);

    // Source Selection
    ui.horizontal(|ui| {
        let can_pick = state.rx.is_none() && state.import_mode != ImportMode::None;
        
        match state.import_mode {
            ImportMode::Folder => {
                if ui.add_enabled(can_pick, egui::Button::new("Select Source Folder")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        state.set_import_folder(path.display().to_string());
                    }
                }
                ui.monospace(&state.censored_import_folder);
            },
            ImportMode::Zip => {
                if ui.add_enabled(can_pick, egui::Button::new("Select game.zip")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("ZIP", &["zip"]).pick_file() {
                        state.set_import_zip(path.display().to_string());
                    }
                }
                ui.monospace(&state.censored_import_zip);
            },
            ImportMode::None => {
                 ui.label("Please select a mode.");
            }
        }
    });

    ui.add_space(15.0);

    // Action Button
    let has_selection = match state.import_mode {
        ImportMode::Folder => state.import_folder != "No folder selected",
        ImportMode::Zip => state.import_zip != "No file selected",
        ImportMode::None => false,
    };

    let can_import = state.rx.is_none() 
        && state.import_mode != ImportMode::None 
        && has_selection;

    if ui.add_enabled(can_import, egui::Button::new("Start Import")).clicked() {
        state.status_message = "Initializing Import...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);

        let mode = state.import_mode;
        let path = match mode {
            ImportMode::Folder => state.import_folder.clone(),
            ImportMode::Zip => state.import_zip.clone(),
            _ => String::new(),
        };

        std::thread::spawn(move || {
            let import_res = match mode {
                ImportMode::Folder => crate::core::import::import_data::from_folder(&path, tx.clone()),
                ImportMode::Zip => crate::core::import::import_data::from_zip(&path, tx.clone()),
                _ => Ok(false),
            };

            match import_res {
                Ok(should_sort) => {
                    if should_sort {
                        let _ = tx.send("Structure not found. Starting Sort...".to_string());
                        if let Err(e) = crate::core::import::sort_data::sort_files(tx.clone()) {
                            let _ = tx.send(format!("Error Sorting: {}", e));
                        }
                    } else {
                        let _ = tx.send("Success! Files imported successfully.".to_string());
                    }
                },
                Err(e) => {
                    let _ = tx.send(format!("Error: {}", e));
                }
            }
        });
    }
}