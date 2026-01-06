use eframe::egui;
use crate::settings::{Settings, lang};

pub fn show(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut refresh_needed = false;
    
    ui.add_space(5.0);
    ui.heading("Visual");
    ui.add_space(10.0);
    
    ui.horizontal(|ui| {
        ui.label("Game Language:");
        if settings.rx_lang.is_some() { ui.spinner(); }

        egui::ComboBox::from_id_salt("lang_selector")
            .selected_text(lang::get_label_for_code(&settings.game_language))
            .show_ui(ui, |ui| {
                for (code, label) in lang::LANGUAGE_PRIORITY {
                    if !settings.available_languages.contains(&code.to_string()) { 
                        continue; 
                    }
                    
                    if ui.selectable_value(&mut settings.game_language, code.to_string(), *label).clicked() {
                        refresh_needed = true;
                    }
                }
                
                if ui.selectable_value(&mut settings.game_language, "".to_string(), "None").clicked() {
                    refresh_needed = true;
                }
            });
    });

    refresh_needed
}