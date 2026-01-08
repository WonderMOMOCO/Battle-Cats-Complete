use eframe::egui;
use crate::core::import::ImportState;
use std::sync::mpsc;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState) {
    ui.label(egui::RichText::new("Package sorted files into a ZIP archive.").strong());
    ui.add_space(10.0);

    // File Name Input
    ui.horizontal(|ui| {
        ui.label("Export Name:");
        
        ui.add(egui::TextEdit::singleline(&mut state.export_name)
            .hint_text(egui::RichText::new("battlecats").color(egui::Color32::from_gray(100)))
            .desired_width(120.0)
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
    
    // Determine the effective button text
    let display_name = if state.export_name.trim().is_empty() {
        "battlecats"
    } else {
        &state.export_name
    };
    let button_text = format!("Create {}.game.zip", display_name);

    if ui.add_enabled(can_zip, egui::Button::new(button_text)).clicked() {
        state.status_message = "Preparing to zip...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);
        let level = state.compression_level;
        
        // Fallback to "battlecats" if empty
        let final_name = if state.export_name.trim().is_empty() {
            "battlecats".to_string()
        } else {
            state.export_name.clone()
        };

        std::thread::spawn(move || {
            if let Err(e) = crate::core::import::export_data::to_zip(level, final_name, tx.clone()) {
                let _ = tx.send(format!("Error: {}", e));
            }
        });
    }
}