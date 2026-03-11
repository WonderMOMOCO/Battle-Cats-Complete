use eframe::egui;
use crate::features::settings::logic::state::CatDataSettings;
use super::tabs::toggle_ui;

pub fn show(ui: &mut egui::Ui, settings: &mut CatDataSettings) -> bool {
    let mut refresh_needed = false;
    egui::ScrollArea::vertical()
        .id_salt("cat_data_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {

            ui.heading("Cat List");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Preferred Banner Form");
                
                egui::ComboBox::from_id_salt("pref_banner")
                    .width(80.0)
                    .selected_text(match settings.preferred_banner_form {
                        0 => "Normal",
                        1 => "Evolved",
                        2 => "True",
                        3 => "Ultra",
                        _ => "Normal",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut settings.preferred_banner_form, 0, "Normal").clicked() { refresh_needed = true; }
                        if ui.selectable_value(&mut settings.preferred_banner_form, 1, "Evolved").clicked() { refresh_needed = true; }
                        if ui.selectable_value(&mut settings.preferred_banner_form, 2, "True").clicked() { refresh_needed = true; }
                        if ui.selectable_value(&mut settings.preferred_banner_form, 3, "Ultra").clicked() { refresh_needed = true; }
                    });
            });

            ui.horizontal(|ui| {
                if toggle_ui(ui, &mut settings.high_banner_quality).changed() {
                    refresh_needed = true;
                }
                ui.label("Smooth Banner Scaling");
            });

            ui.horizontal(|ui| {
                toggle_ui(ui, &mut settings.unit_persistence);
                ui.label("Selected Unit Persistence");
            });

            ui.horizontal(|ui| {
                if toggle_ui(ui, &mut settings.show_invalid_units).changed() {
                    refresh_needed = true;
                }
                ui.label("Show Invalid Units");
            });

            ui.add_space(20.0);
            ui.heading("Ability Display");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                toggle_ui(ui, &mut settings.expand_spirit_details);
                ui.label("Expand Spirit Details by Default");
            });

            ui.add_space(20.0);
            ui.heading("Level Display");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.add_enabled_ui(!settings.auto_level_calculations, |ui| {
                    ui.label("Default Level");
                    ui.add(egui::DragValue::new(&mut settings.default_level).speed(1.0).range(1..=150));
                });
            });

            ui.horizontal(|ui| {
                toggle_ui(ui, &mut settings.auto_level_calculations);
                ui.label("Auto Level Calculations").on_hover_text("Automatically calculates the max reasonable level for a unit based on their level caps");
            });

            ui.horizontal(|ui| {
                toggle_ui(ui, &mut settings.bump_ultra_60);
                ui.label("Lv60 For Ultra").on_hover_text("Automatically bumps the level to 60 (if not higher already) when an Ultra Form or Ultra Talent is selected");
            });
        });

    refresh_needed
}