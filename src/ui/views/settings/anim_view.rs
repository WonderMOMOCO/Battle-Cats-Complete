use eframe::egui;
use crate::core::settings::Settings;
use super::tabs::toggle_ui;

pub fn show(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let refresh_needed = false;

    egui::ScrollArea::vertical()
        .id_salt("anim_view_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            
            ui.heading("Viewer");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Centering Behavior:");
                egui::ComboBox::from_id_salt("centering_behavior")
                    .width(80.0)
                    .selected_text(match settings.centering_behavior {
                        0 => "Unit",
                        1 => "Origin",
                        2 => "None",
                        _ => "Unit",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.centering_behavior, 0, "Unit")
                            .on_hover_text("Automatically center the unit on load (Default)");
                        ui.selectable_value(&mut settings.centering_behavior, 1, "Origin")
                            .on_hover_text("Reset camera to (0,0) on load");
                        ui.selectable_value(&mut settings.centering_behavior, 2, "None")
                            .on_hover_text("Keep camera position/zoom when switching units/forms");
                    });
            });

            ui.horizontal(|ui| {
                let tooltip = "Switches from 30fps to your monitors native refresh rate\nAllows animations to be smooth but quite buggy, so expect them\nWhile this feature is supported, it is of low importance";
                
                if toggle_ui(ui, &mut settings.animation_interpolation).on_hover_text(tooltip).changed() {
                    if settings.animation_interpolation {
                        let dt = ui.input(|i| i.stable_dt);
                        if dt > 0.0 {
                            settings.native_fps = (1.0 / dt).round();
                        }
                    }
                    ui.ctx().request_repaint();
                }
                
                ui.label("Use Native Refresh Rate").on_hover_text(tooltip);
                
                if settings.animation_interpolation {
                    ui.label(egui::RichText::new(format!("({}fps)", settings.native_fps)).weak().size(12.0));
                }
            });

            ui.horizontal(|ui| {
                if toggle_ui(ui, &mut settings.animation_debug).changed() {
                    ui.ctx().request_repaint();
                }
                ui.label("Enable Debug View");
            });

            ui.add_space(20.0);
            ui.heading("Exporter");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let tooltip_cam = "Automatically calculates a Units tight bounding box when exporting\nThis setting may cause lag spikes on some devices";
                toggle_ui(ui, &mut settings.auto_set_camera_region).on_hover_text(tooltip_cam);
                ui.label("Auto-Set Camera Region").on_hover_text(tooltip_cam);
            });
        });

    refresh_needed
}