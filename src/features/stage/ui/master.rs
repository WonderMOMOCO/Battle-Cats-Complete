use eframe::egui;
use crate::features::stage::logic::state::StageListState;
use crate::features::settings::logic::Settings;
use super::{list, view};

pub fn show(ctx: &egui::Context, state: &mut StageListState, _settings: &mut Settings) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if state.scan_receiver.is_some() {
            ui.centered_and_justified(|ui| {
                ui.spinner();
                ui.label("Parsing Stages...");
            });
            return;
        }

        let rect = ui.available_rect_before_wrap();

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            view::draw(ui, state);
        });

        let mut target_width = 180.0; // Category Column
        if state.selected_category.is_some() { target_width += 200.0; } // Map Column
        if state.selected_map.is_some() { target_width += 200.0; } // Stage Column

        let anim = ctx.animate_bool(egui::Id::new("stage_list_anim"), state.is_list_open);
        let offset = (anim - 1.0) * target_width; // Slides from -Width (hidden) to 0 (shown)

        if anim > 0.0 {
            let mut overlay_rect = rect;
            overlay_rect.set_width(target_width);
            overlay_rect = overlay_rect.translate(egui::vec2(offset, 0.0));

            let frame = egui::Frame::window(&ctx.style())
                .rounding(egui::Rounding { nw: 0.0, sw: 0.0, ne: 8.0, se: 8.0 })
                .shadow(eframe::epaint::Shadow {
                    offset: egui::vec2(4.0, 0.0),
                    blur: 16.0,
                    spread: 0.0,
                    color: egui::Color32::from_black_alpha(50),
                });

            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(overlay_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Min))
            );
            
            frame.show(&mut child_ui, |ui| {
                list::draw(ui, state);
            });
        }

        let btn_x = (offset + target_width).max(0.0) + 10.0; // Follows the drawer, stops at 10px from left
        let btn_pos = rect.min + egui::vec2(btn_x, 10.0);
        
        egui::Area::new(egui::Id::new("stage_list_toggle_btn"))
            .fixed_pos(btn_pos)
            .show(ctx, |ui| {
                let icon = if state.is_list_open { "◀" } else { "▶" };
                
                let btn = egui::Button::new(egui::RichText::new(icon).size(16.0).color(egui::Color32::DARK_GRAY))
                    .fill(egui::Color32::from_gray(220))
                    .rounding(5.0);
                
                if ui.add_sized([32.0, 32.0], btn).clicked() {
                    state.is_list_open = !state.is_list_open;
                }
            });
    });
}