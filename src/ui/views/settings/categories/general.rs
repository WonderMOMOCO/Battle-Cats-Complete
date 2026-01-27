use eframe::egui;
use crate::core::settings::{Settings, lang, upd::UpdateMode};

pub fn show(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut refresh_needed = false;
    
    ui.add_space(5.0);
    ui.heading("Updates");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Update Handling:");
        
        egui::ComboBox::from_id_salt("update_mode_selector")
            .selected_text(settings.update_mode.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut settings.update_mode, UpdateMode::AutoReset, "Auto-Reset")
                    .on_hover_text("Automatically downloads updates and restarts the app on startup");
                    
                ui.selectable_value(&mut settings.update_mode, UpdateMode::AutoLoad, "Auto-Load")
                    .on_hover_text("Automatically downloads updates but waits until the next run to apply them");

                ui.selectable_value(&mut settings.update_mode, UpdateMode::Prompt, "Prompt")
                    .on_hover_text("Ask permission before downloading updates or restarting");

                ui.selectable_value(&mut settings.update_mode, UpdateMode::Ignore, "Ignore")
                    .on_hover_text("Never check for updates on startup");
            });
    });
    
    ui.add_space(5.0);

    if ui.add_sized([180.0, 30.0], egui::Button::new("Check for Update Now")).clicked() {
        settings.manual_check_requested = true;
    }
    
    ui.add_space(20.0);
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