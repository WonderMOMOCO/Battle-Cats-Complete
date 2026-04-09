use eframe::egui;
use crate::features::stage::logic::{state::StageListState, navigate};

pub const BTN_SPACING_X: f32 = 14.0; // Total horizontal space given to the separator
pub const BTN_SPACING_Y: f32 = 6.0;  // Vertical padding between list buttons

pub fn draw(ui: &mut egui::Ui, state: &mut StageListState) {
    let mut categories = navigate::get_categories(&state.registry);

    categories.sort_by_key(|(prefix, _)| crate::features::stage::data::map_name::get_category_sort_order(prefix));

    if categories.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("No Stages Found").strong().color(egui::Color32::LIGHT_RED));
        });
        return;
    }

    ui.spacing_mut().item_spacing.x = 0.0;

    draw_categories(ui, state, &categories);

    if state.selected_category.is_some() {
        ui.add(egui::Separator::default().vertical().spacing(BTN_SPACING_X));
        draw_maps(ui, state);

        if state.selected_map.is_some() {
            ui.add(egui::Separator::default().vertical().spacing(BTN_SPACING_X));
            draw_stages(ui, state);
        }
    }
}

fn draw_sidebar_btn(ui: &mut egui::Ui, text: &str, is_selected: bool) -> bool {
    let bg_color = if is_selected {
        egui::Color32::from_rgb(31, 106, 165)
    } else {
        egui::Color32::from_rgb(50, 50, 50)
    };

    let btn_text = egui::RichText::new(text).size(13.0);
    let btn = egui::Button::new(btn_text).fill(bg_color).wrap();
    
    ui.add_sized([ui.available_width(), 30.0], btn).clicked()
}

fn draw_categories(ui: &mut egui::Ui, state: &mut StageListState, categories: &[(String, String)]) {
    ui.vertical(|ui| {
        ui.set_min_width(180.0);
        ui.set_max_width(180.0);
        ui.set_min_height(ui.available_height()); 

        egui::ScrollArea::vertical()
            .id_salt("cat_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = BTN_SPACING_Y; 
                ui.add_space(BTN_SPACING_Y);
                
                for (cat_prefix, cat_name) in categories {
                    let is_selected = state.selected_category.as_deref() == Some(cat_prefix);
                    
                    if draw_sidebar_btn(ui, cat_name, is_selected) {
                        state.selected_category = Some(cat_prefix.clone());
                        state.selected_map = None;
                        state.selected_stage = None;
                    }
                }
                
                ui.add_space(BTN_SPACING_Y);
            });
    });
}

fn draw_maps(ui: &mut egui::Ui, state: &mut StageListState) {
    let Some(cat) = &state.selected_category else { return; };
    
    ui.vertical(|ui| {
        ui.set_min_width(200.0);
        ui.set_max_width(200.0);
        ui.set_min_height(ui.available_height());

        egui::ScrollArea::vertical()
            .id_salt("map_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = BTN_SPACING_Y;
                ui.add_space(BTN_SPACING_Y);
                
                let maps = navigate::get_maps(&state.registry, cat);
                for map in maps {
                    let is_selected = state.selected_map.as_ref() == Some(&map.id);
                    
                    if draw_sidebar_btn(ui, &map.name, is_selected) {
                        state.selected_map = Some(map.id);
                        state.selected_stage = None;
                    }
                }
                
                ui.add_space(BTN_SPACING_Y); 
            });
    });
}

fn draw_stages(ui: &mut egui::Ui, state: &mut StageListState) {
    let Some(map_id) = &state.selected_map else { return; };

    ui.vertical(|ui| {
        ui.set_min_width(200.0);
        ui.set_max_width(200.0);
        ui.set_min_height(ui.available_height());

        egui::ScrollArea::vertical()
            .id_salt("stage_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = BTN_SPACING_Y;
                ui.add_space(BTN_SPACING_Y);
                
                let stages = navigate::get_stages(&state.registry, map_id);
                for stage in stages {
                    let is_selected = state.selected_stage.as_ref() == Some(&stage.id);
                    
                    if draw_sidebar_btn(ui, &stage.name, is_selected) {
                        state.selected_stage = Some(stage.id);
                    }
                }
                
                ui.add_space(BTN_SPACING_Y);
            });
    });
}