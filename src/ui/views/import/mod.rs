use eframe::egui;
use crate::core::import::{ImportState, DataTab};
use crate::core::import::log;

pub mod import_view;
pub mod export_view;

#[cfg(feature = "dev")]
pub mod extract_view;

pub fn show(ctx: &egui::Context, state: &mut ImportState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Game Data Management");
        ui.add_space(10.0);

        // Tabs
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            
            let tabs = vec![
                #[cfg(feature = "dev")]
                (DataTab::Extract, "Extract"),
                (DataTab::Import, "Import"), 
                (DataTab::Export, "Export")
            ];

            for (tab, label) in tabs {
                let is_selected = state.active_tab == tab;
                let (fill, stroke, text_color) = if is_selected {
                    (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                } else {
                    (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                };
                
                let btn = egui::Button::new(egui::RichText::new(label).color(text_color))
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(egui::Rounding::ZERO)
                    .min_size(egui::vec2(80.0, 30.0));

                if ui.add(btn).clicked() {
                    state.active_tab = tab;
                }
            }
        });

        ui.add_space(15.0);

        // Active View
        match state.active_tab {
            #[cfg(feature = "dev")]
            DataTab::Extract => extract_view::show(ui, state),
            DataTab::Import => import_view::show(ui, state),
            DataTab::Export => export_view::show(ui, state),
        }

        ui.add_space(15.0);
        ui.separator(); 

        // Status Bar
        let status_color = log::resolve_status_color(&state.status_message);

        if state.rx.is_some() && !state.status_message.contains("Success") && !state.status_message.contains("Error") {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&state.status_message);
            });
        } else {
            ui.colored_label(status_color, &state.status_message);
        }
        
        ui.separator();

        // Log Output
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for line in state.log_content.lines() {
                    let color = log::resolve_log_color(line);
                    ui.label(egui::RichText::new(line).color(color).monospace());
                }
            });
    });
}