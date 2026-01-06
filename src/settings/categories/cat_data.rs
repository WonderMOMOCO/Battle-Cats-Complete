use eframe::egui;
use crate::settings::{Settings, toggle_ui};

pub fn show(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut refresh_needed = false;

    ui.add_space(5.0);
    ui.heading("Cat List");
    ui.add_space(10.0);

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