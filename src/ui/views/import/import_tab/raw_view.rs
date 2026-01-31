use eframe::egui;
use crate::core::import::{ImportState, ImportMode, game_data, sort};
use std::thread;
use std::sync::mpsc;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label("Sort game archive or decrypted files into database");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Source Type:");
        
        egui::ComboBox::from_id_salt("import_source_mode")
            .selected_text(match state.import_mode {
                ImportMode::Folder => "Folder",
                _ => "Archive",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.import_mode, ImportMode::Zip, "Archive");
                ui.selectable_value(&mut state.import_mode, ImportMode::Folder, "Folder");
            });
    });
    
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        let enabled = state.rx.is_none();
        if ui.add_enabled(enabled, egui::Button::new("Select Source")).clicked() {
            let res = match state.import_mode {
                ImportMode::Zip => rfd::FileDialog::new()
                    .add_filter("Game Archive", &["zst", "tar", "zip"]) 
                    .pick_file(),
                ImportMode::Folder => rfd::FileDialog::new()
                    .pick_folder(),
                _ => None,
            };

            if let Some(path) = res {
                state.import_path = path.to_string_lossy().to_string();
            }
        }
        ui.label(if state.import_censored.is_empty() { "No source selected" } else { &state.import_censored });
    });

    ui.add_space(15.0);

    if ui.button("Start Sort").clicked() {
        start_manual_sort(state);
    }
}

fn start_manual_sort(state: &mut ImportState) {
    state.status_message = "Starting worker...".to_string();
    state.log_content.clear();
    let (tx, rx) = mpsc::channel();
    state.rx = Some(rx);
    
    let path = state.import_path.clone();
    let mode = state.import_mode;

    thread::spawn(move || {
        let import_result = match mode {
            ImportMode::Folder => game_data::import_standard_folder(&path, tx.clone()),
            ImportMode::Zip => game_data::import_standard_archive(&path, tx.clone()),
            _ => Err("Invalid mode".to_string()),
        };

        match import_result {
            Ok(should_sort) => {
                if should_sort {
                    let _ = tx.send("Starting Sort...".to_string());
                    if let Err(e) = sort::sort_game_files(tx.clone()) {
                        let _ = tx.send(format!("Error Sorting: {}", e));
                    } else {
                        let _ = tx.send("Success! Files processed and sorted.".to_string());
                    }
                }
            },
            Err(e) => {
                let _ = tx.send(format!("Error: {}", e));
            }
        }
    });
}