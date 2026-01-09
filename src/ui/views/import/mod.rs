use eframe::egui;
use crate::core::import::{ImportState, DataTab};

pub mod import_view;
pub mod export_view;

#[cfg(feature = "dev")]
pub mod decrypt_view;

pub fn show(ctx: &egui::Context, state: &mut ImportState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Game Data Management");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            
            let mut tabs = Vec::new();
            #[cfg(feature = "dev")]
            tabs.push((DataTab::Decrypt, "Decrypt"));
            tabs.push((DataTab::Import, "Import"));
            tabs.push((DataTab::Export, "Export"));

            for (tab, label) in tabs {
                let is_selected = state.active_tab == tab;
                let (fill, stroke, text_color) = if is_selected {
                    (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                } else {
                    (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                };
                if ui.add(egui::Button::new(egui::RichText::new(label).color(text_color)).fill(fill).stroke(stroke).rounding(egui::Rounding::ZERO).min_size(egui::vec2(80.0, 30.0))).clicked() {
                    state.active_tab = tab;
                }
            }
        });

        ui.add_space(15.0);

        match state.active_tab {
            DataTab::Import => import_view::show(ui, state),
            DataTab::Export => export_view::show(ui, state),
            #[cfg(feature = "dev")]
            DataTab::Decrypt => decrypt_view::show(ui, state),
        }

        ui.add_space(15.0);
        ui.separator(); 

        if state.rx.is_some() && !state.status_message.contains("Success") && !state.status_message.contains("Error") {
            ui.horizontal(|ui| { ui.spinner(); ui.label(&state.status_message); });
        } else {
            let color = if state.status_message.contains("Error") { egui::Color32::RED } 
                       else if state.status_message.contains("Success") { egui::Color32::GREEN } 
                       else { egui::Color32::LIGHT_BLUE };
            ui.colored_label(color, &state.status_message);
        }
        
        ui.separator();
        
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false]) 
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut state.log_content.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                );
            });
    });
}