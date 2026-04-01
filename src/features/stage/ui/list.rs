use eframe::egui;
use crate::features::stage::logic::{state::StageListState, navigate};

pub fn draw(ui: &mut egui::Ui, state: &mut StageListState) {
    let categories = navigate::get_categories(&state.registry);

    if categories.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("No Stages Found").strong().color(egui::Color32::LIGHT_RED));
        });
        return;
    }

    ui.spacing_mut().item_spacing.x = 0.0;

    draw_categories(ui, state, &categories);

    // Conditional Separators
    if state.selected_category.is_some() {
        ui.separator();
        draw_maps(ui, state);

        if state.selected_map.is_some() {
            ui.separator();
            draw_stages(ui, state);
        }
    }
}

fn draw_categories(ui: &mut egui::Ui, state: &mut StageListState, categories: &[(String, String)]) {
    ui.vertical(|ui| {
        ui.set_width(180.0);
        let h = ui.available_height();
        ui.set_min_height(h);

        egui::ScrollArea::vertical()
            .id_salt("cat_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(5.0);
                for (cat_prefix, cat_name) in categories {
                    let is_selected = state.selected_category.as_deref() == Some(cat_prefix);
                    if ui.selectable_label(is_selected, cat_name).clicked() {
                        state.selected_category = Some(cat_prefix.clone());
                        state.selected_map = None;
                        state.selected_stage = None;
                    }
                }
            });
    });
}

fn draw_maps(ui: &mut egui::Ui, state: &mut StageListState) {
    let Some(cat) = &state.selected_category else { return; };
    
    ui.vertical(|ui| {
        ui.set_width(200.0);
        let h = ui.available_height();
        ui.set_min_height(h);

        egui::ScrollArea::vertical()
            .id_salt("map_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(5.0);
                let maps = navigate::get_maps(&state.registry, cat);
                for map in maps {
                    let is_selected = state.selected_map.as_ref() == Some(&map.id);
                    if ui.selectable_label(is_selected, &map.name).clicked() {
                        state.selected_map = Some(map.id);
                        state.selected_stage = None;
                    }
                }
            });
    });
}

fn draw_stages(ui: &mut egui::Ui, state: &mut StageListState) {
    let Some(map_id) = &state.selected_map else { return; };

    ui.vertical(|ui| {
        ui.set_width(200.0);
        let h = ui.available_height();
        ui.set_min_height(h);

        egui::ScrollArea::vertical()
            .id_salt("stage_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(5.0);
                let stages = navigate::get_stages(&state.registry, map_id);
                for stage in stages {
                    let is_selected = state.selected_stage.as_ref() == Some(&stage.id);
                    if ui.selectable_label(is_selected, &stage.name).clicked() {
                        state.selected_stage = Some(stage.id);
                    }
                }
            });
    });
}