use eframe::egui;
use crate::features::settings::logic::state::EnemyDataSettings;
use super::tabs::toggle_ui;

pub fn show(ui: &mut egui::Ui, settings: &mut EnemyDataSettings) -> bool {
    let mut refresh_needed = false;
    egui::ScrollArea::vertical()
        .id_salt("enemy_data_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {

            ui.heading("Enemy List");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if toggle_ui(ui, &mut settings.show_invalid_enemies).changed() {
                    refresh_needed = true;
                }
                ui.label("Show Invalid Enemies");
            });

        });

    refresh_needed
}