use eframe::egui;
use crate::features::stage::logic::state::StageListState;

pub fn draw(ui: &mut egui::Ui, state: &StageListState) {
    let Some(stage_id) = &state.selected_stage else {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Select a stage to view details").color(egui::Color32::DARK_GRAY));
        });
        return;
    };

    let Some(stage) = state.registry.stages.get(stage_id) else { return; };

    ui.vertical(|ui| {
        egui::ScrollArea::vertical()
            .id_salt("view_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(20.0); 
                
                ui.horizontal(|ui| {
                    ui.add_space(40.0); 
                    ui.vertical(|ui| {
                        let display_name = if stage.name == format!("{:02}", stage.stage_id) {
                            stage.id.clone() // Fallback to raw file key
                        } else {
                            stage.name.clone() // Pure localized name
                        };

                        ui.heading(display_name);
                        ui.separator();

                        draw_metadata(ui, stage);
                        ui.add_space(20.0);
                        draw_enemy_table(ui, stage);
                    });
                });
            });
    });
}

fn draw_metadata(ui: &mut egui::Ui, stage: &crate::features::stage::registry::Stage) {
    egui::Grid::new("stage_meta_grid").spacing([20.0, 8.0]).show(ui, |ui| {
        ui.label("Base HP:");
        ui.label(egui::RichText::new(stage.base_hp.to_string()).strong());
        ui.end_row();

        ui.label("Energy:");
        ui.label(egui::RichText::new(stage.energy.to_string()).strong());
        ui.end_row();

        ui.label("Width:");
        ui.label(egui::RichText::new(stage.width.to_string()).strong());
        ui.end_row();
        
        ui.label("No Continues:");
        ui.label(if stage.is_no_continues { "Yes" } else { "No" });
        ui.end_row();
    });
}

fn draw_enemy_table(ui: &mut egui::Ui, stage: &crate::features::stage::registry::Stage) {
    ui.strong("Enemy Layout");
    ui.separator();

    if stage.enemies.is_empty() {
        ui.label("No enemies defined for this stage.");
        return;
    }

    egui::Grid::new("enemy_grid")
        .striped(true)
        .spacing([15.0, 4.0])
        .show(ui, |ui| {
            ui.label("ID");
            ui.label("Count");
            ui.label("HP %");
            ui.label("Atk %");
            ui.label("Spawn");
            ui.end_row();

            for enemy in &stage.enemies {
                ui.label(format!("{:03}", enemy.id));
                ui.label(if enemy.amount == 0 { "∞".into() } else { enemy.amount.to_string() });
                ui.label(format!("{}%", enemy.magnification));
                ui.label(format!("{}%", enemy.atk_magnification));
                ui.label(format!("{}f", enemy.start_frame));
                ui.end_row();
            }
        });
}