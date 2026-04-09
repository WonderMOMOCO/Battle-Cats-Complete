use eframe::egui;
use crate::features::stage::logic::state::StageListState;

pub fn draw(ctx: &egui::Context, ui: &mut egui::Ui, state: &mut StageListState) {
    let Some(stage_id) = &state.selected_stage else {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Select a stage to view details").color(egui::Color32::DARK_GRAY));
        });
        return;
    };

    let enemy_registry = &state.enemy_registry;
    let texture_cache = &mut state.enemy_texture_cache;
    let Some(stage) = state.registry.stages.get(stage_id) else { return; };

    egui::ScrollArea::vertical()
        .id_salt("view_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(20.0);

            let frame = egui::Frame::none().inner_margin(egui::Margin { left: 40.0, right: 40.0, top: 0.0, bottom: 0.0 });
            
            frame.show(ui, |ui| {
                ui.vertical(|ui| {
                    let display_name = if stage.name == format!("{:02}", stage.stage_id) {
                        stage.id.clone() 
                    } else {
                        stage.name.clone() 
                    };

                    ui.heading(display_name);
                    ui.separator();

                    super::info::draw(ui, stage);
                    ui.add_space(20.0);
                    
                    super::treasure::draw(ui, stage);
                    ui.add_space(20.0);

                    super::battleground::draw(ctx, ui, stage, enemy_registry, texture_cache);
                });
            });
        });
}