use eframe::egui;
use crate::core::settings::Settings;
use crate::ui::views::settings::toggle_ui;

pub fn show(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut refresh_needed = false;

    ui.add_space(5.0);
    ui.heading("Cat List");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Preferred Banner Form");
        
        egui::ComboBox::from_id_salt("pref_banner")
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
    
    ui.add_space(10.0);

    egui::Grid::new("ability_grid").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
        ui.label("Ability Padding X");
        ui.add(egui::DragValue::new(&mut settings.ability_padding_x).speed(0.5).range(0.0..=50.0));
        ui.end_row();

        ui.label("Ability Padding Y");
        ui.add(egui::DragValue::new(&mut settings.ability_padding_y).speed(0.5).range(0.0..=50.0));
        ui.end_row();

        ui.label("Trait Padding Y");
        ui.add(egui::DragValue::new(&mut settings.trait_padding_y).speed(0.5).range(0.0..=50.0));
        ui.end_row();
    });

    refresh_needed
}