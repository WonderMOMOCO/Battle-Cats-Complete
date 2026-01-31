use eframe::egui;
use std::sync::mpsc;
use std::thread;
use crate::core::import::{ImportState, game_data};
use crate::core::settings::Settings;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &mut Settings) {
    ui.label("Package database into a ZST archive.");
    ui.add_space(10.0);
    
    ui.horizontal(|ui| {
        ui.label("Filename:");
        
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.add(egui::TextEdit::singleline(&mut state.export_filename)
            .hint_text(egui::RichText::new("battlecats").color(egui::Color32::DARK_GRAY))
            .desired_width(100.0)
        );
        
        ui.label(".tar.zst");
    });
    
    ui.add_space(5.0);

    let max_level = if settings.enable_ultra_compression { 21 } else { 15 };
    
    if state.compression_level == 0 {
        state.compression_level = settings.last_compression_level;
    }

    if state.compression_level > max_level {
        state.compression_level = max_level;
    }

    ui.horizontal(|ui| {
        ui.label("Compression Level:");
        
        let response = ui.add(egui::Slider::new(&mut state.compression_level, 1..=max_level));
            
        if response.changed() {
            settings.last_compression_level = state.compression_level;
        }
    });
    
    ui.add_space(0.0);
    
    let (desc_text, desc_color) = match state.compression_level {
        1..=9 => (
            "Best compression balance",
            egui::Color32::from_rgb(120, 210, 120) 
        ),
        10..=15 => (
            "Slow compression for low archive size",
            egui::Color32::from_rgb(240, 200, 80) 
        ),
        _ => (
            "Ultra compression granting minimal returns",
            egui::Color32::from_rgb(240, 100, 100) 
        ),
    };

    ui.label(
        egui::RichText::new(desc_text)
            .color(desc_color)
            .small()
    );
    
    ui.add_space(5.0);
    
    let can_zip = state.rx.is_none(); 
    
    let base_name = if state.export_filename.trim().is_empty() { "battlecats" } else { &state.export_filename };
    let full_name = format!("{}.tar.zst", base_name);
    let btn_text = format!("Create {}", full_name);

    if ui.add_enabled(can_zip, egui::Button::new(btn_text)).clicked() {
        state.status_message = "Preparing to pack...".to_string();
        state.log_content.clear();
        
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);
        
        let level = state.compression_level;
        let filename_arg = full_name.clone(); 

        thread::spawn(move || {
            if let Err(e) = game_data::create_game_archive(tx.clone(), level, filename_arg) {
                 let _ = tx.send(format!("Error Packing: {}", e));
            }
        });
    }
}