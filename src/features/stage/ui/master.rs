use eframe::egui;
use crate::features::stage::logic::state::StageListState;
use crate::features::settings::logic::Settings;
use super::{list, view};

const ANIM_SPEED: f32 = 0.15; // Open/close speed multiplier (duration in seconds)
const TOGGLE_BTN_GAP: f32 = 5.0; // Left padding for the toggle button (and distance from list)
const LIST_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(20, 20, 20);

pub fn show(ctx: &egui::Context, state: &mut StageListState, _settings: &mut Settings) {
    let screen_rect = ctx.screen_rect();

    let mut inner_target_width = 180.0; 
    if state.selected_category.is_some() { inner_target_width += list::BTN_SPACING_X + 200.0; } 
    if state.selected_map.is_some() { inner_target_width += list::BTN_SPACING_X + 200.0; } 

    let frame_margin = 15.0;
    let total_target_width = inner_target_width + (frame_margin * 2.0);

    let target_inner_height = screen_rect.height() - (frame_margin * 2.0);

    let target_open = if state.is_list_open { 1.0 } else { 0.0 };
    let app_velocity = 180.0 / ANIM_SPEED; 
    let anim_duration = total_target_width / app_velocity;
    let open_factor = ctx.animate_value_with_time(egui::Id::new("stage_list_anim"), target_open, anim_duration);

    if open_factor > 0.0 && open_factor < 1.0 {
        ctx.request_repaint();
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        if state.scan_receiver.is_some() {
            ui.centered_and_justified(|ui| {
                ui.spinner();
                ui.label("Parsing Stages...");
            });
            return;
        }
        view::draw(ctx, ui, state);
    });

    let hidden_x = -total_target_width - 30.0; 
    let sidebar_x = egui::lerp(hidden_x..=0.0, open_factor);
    let sidebar_right_edge = sidebar_x + total_target_width;
    
    let btn_x = (sidebar_right_edge + TOGGLE_BTN_GAP).max(TOGGLE_BTN_GAP);

    egui::Area::new("stage_sidebar_area".into())
        .constrain(false) 
        .fixed_pos(egui::pos2(sidebar_x, 0.0)) 
        .order(egui::Order::Background) 
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(LIST_BG_COLOR) 
                .inner_margin(frame_margin)
                .rounding(egui::Rounding { nw: 0.0, sw: 0.0, ne: 10.0, se: 10.0 })
                .show(ui, |ui| {
                    ui.set_min_width(inner_target_width);
                    ui.set_max_width(inner_target_width);
                    ui.set_min_size(egui::vec2(inner_target_width, target_inner_height)); 
                    
                    ui.horizontal(|ui| {
                        ui.set_min_height(target_inner_height); 
                        list::draw(ui, state);
                    });
                });
        });

    egui::Area::new("stage_list_toggle_btn".into())
        .constrain(false) 
        .fixed_pos(egui::pos2(btn_x, TOGGLE_BTN_GAP)) 
        .order(egui::Order::Background) 
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
            
            let mut bg_color = egui::Color32::from_gray(220);
            if response.hovered() { bg_color = egui::Color32::from_gray(240); }
            if response.is_pointer_button_down_on() { bg_color = egui::Color32::from_gray(200); }

            ui.painter().rect_filled(rect, 5.0, bg_color);

            let icon = if state.is_list_open { "◀" } else { "▶" };
            let text_color = egui::Color32::DARK_GRAY;
            let text_galley = ui.fonts(|f| f.layout(
                icon.to_owned(),
                egui::FontId::proportional(16.0),
                text_color,
                rect.width(),
            ));
            
            let text_pos = egui::pos2(
                rect.center().x - text_galley.size().x / 2.0,
                rect.center().y - text_galley.size().y / 2.0,
            );
            
            ui.painter().galley(text_pos, text_galley, text_color);
            
            if response.clicked() {
                state.is_list_open = !state.is_list_open;
            }
        });
}