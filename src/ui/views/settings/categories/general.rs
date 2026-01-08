use eframe::egui;
use crate::core::settings::{Settings, lang};

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
                for (code, label) in lang::LANGUAGE_LIST {
                    let code_str = code.to_string();

                    // Only show languages found in assets
                    if settings.available_languages.contains(&code_str) {
                        if ui.selectable_value(&mut settings.game_language, code_str, *label).clicked() {
                            refresh_needed = true;
                        }
                    }
                }
            });
    });

    refresh_needed
}