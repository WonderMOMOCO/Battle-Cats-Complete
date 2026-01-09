use eframe::egui;
use std::sync::mpsc;
use std::thread;
use crate::core::import::{ImportState, game_data};

pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label("Package sorted files into a ZIP archive.");
    ui.add_space(10.0);
    
    ui.horizontal(|ui| {
        ui.label("Filename:");
        
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.add(egui::TextEdit::singleline(&mut state.export_filename)
            .hint_text(egui::RichText::new("battlecats").color(egui::Color32::DARK_GRAY))
            .desired_width(100.0)
        );
        
        ui.label(".game.zip");
    });
    
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.label("Compression Level:");
        ui.add(egui::Slider::new(&mut state.compression_level, 0..=9));
    });
    
    ui.add_space(15.0);
    
    let can_zip = state.rx.is_none(); 
    
    let base_name = if state.export_filename.trim().is_empty() { "battlecats" } else { &state.export_filename };
    let full_name = format!("{}.game.zip", base_name);
    let btn_text = format!("Create {}", full_name);

    if ui.add_enabled(can_zip, egui::Button::new(btn_text)).clicked() {
        state.status_message = "Preparing to zip...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);
        
        let level = state.compression_level;
        let filename_arg = full_name.clone(); 

        thread::spawn(move || {
            if let Err(e) = game_data::create_game_zip(tx.clone(), level, filename_arg) {
                 let _ = tx.send(format!("Error Zipping: {}", e));
            }
        });
    }
}