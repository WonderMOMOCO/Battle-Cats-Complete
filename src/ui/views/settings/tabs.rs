use eframe::egui;
use crate::core::settings::Settings;
use crate::core::utils::DragGuard;

pub fn show(ctx: &egui::Context, settings: &mut Settings, drag_guard: &mut DragGuard) -> bool {
    settings.update_language_list();

    let mut refresh_needed = false;
    
    let tabs = ["General", "Cat Data", "Game Data", "Anim View", "Add-Ons"];

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Settings");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 

            for tab_name in tabs {
                let is_selected = settings.active_tab == *tab_name;
                let bg_color = if is_selected {
                    egui::Color32::from_rgb(31, 106, 165)
                } else {
                    egui::Color32::from_gray(60)
                };
                
                let btn = egui::Button::new(
                        egui::RichText::new(tab_name)
                            .color(egui::Color32::WHITE)
                            .size(14.0)
                    )
                    .fill(bg_color)
                    .min_size(egui::vec2(80.0, 30.0));

                if ui.add(btn).clicked() {
                    settings.active_tab = tab_name.to_string();
                    settings.show_ip_field = false;
                }
            }
        });

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let current_tab = settings.active_tab.clone();

            ui.push_id(&current_tab, |ui| {
                let result = match current_tab.as_str() {
                    "General" => super::general::show(ui, settings),
                    "Cat Data" => super::cat_data::show(ui, settings),
                    "Game Data" => super::game_data::show(ui, settings),
                    "Anim View" => super::anim_view::show(ui, settings),
                    "Add-Ons" => super::addons::show(ui, settings, drag_guard), 
                    _ => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(50.0);
                            ui.label(egui::RichText::new("No settings available for this category.").weak().size(16.0));
                        });
                        false
                    }
                };
                
                if result { refresh_needed = true; }
            });
        });
    });

    refresh_needed
}

pub fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let how_on = ui.ctx().animate_bool(response.id, *on);
    let visuals = ui.style().interact_selectable(&response, *on);
    let rect = rect.expand(visuals.expansion);
    let radius = 0.5 * rect.height();
    
    ui.painter().rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
    
    let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
    ui.painter().circle(egui::pos2(circle_x, rect.center().y), 0.75 * radius, visuals.fg_stroke.color, visuals.fg_stroke);

    response
}