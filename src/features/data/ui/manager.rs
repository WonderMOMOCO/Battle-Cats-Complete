use eframe::egui;
use std::sync::atomic::Ordering;
use crate::features::data::logic::{ImportState, DataTab};
use crate::features::settings::logic::Settings;
use crate::features::data::ui::{import_view, export_view};

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &mut Settings) {
    ui.vertical(|ui| {

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            
            let tabs = [
                (DataTab::Import, "Import"),
                (DataTab::Export, "Export"),
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

        let padding_above_tab_separator = 10.0;
        let padding_below_tab_separator = 15.0;

        let padding_above_console_separator = 15.0;
        let padding_below_console_separator = 0.0;

        let padding_above_progress_bar = 8.0;
        let padding_below_progress_bar = 8.0;

        ui.add_space(padding_above_tab_separator);
        ui.add(egui::Separator::default().spacing(0.0)); 
        ui.add_space(padding_below_tab_separator);

        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            
            match state.active_tab {
                DataTab::Import => import_view::show(ui, state, settings),
                DataTab::Export => export_view::show(ui, state, settings),
            }

            ui.add_space(padding_above_console_separator);
            ui.add(egui::Separator::default().spacing(0.0)); 
            ui.add_space(padding_below_console_separator);

            let (is_running, log_content, prog_curr, prog_max) = match state.active_tab {
                DataTab::Import => (
                    state.import_job_status.load(Ordering::Relaxed) == 1,
                    &state.import_log_content,
                    state.import_progress_current.load(Ordering::Relaxed),
                    state.import_progress_max.load(Ordering::Relaxed)
                ),
                DataTab::Export => (
                    state.export_job_status.load(Ordering::Relaxed) == 1,
                    &state.export_log_content,
                    state.export_progress_current.load(Ordering::Relaxed),
                    state.export_progress_max.load(Ordering::Relaxed)
                ),
            };

            let fraction = if is_running {
                if prog_max > 0 { prog_curr as f32 / prog_max as f32 } else { 1.0 }
            } else {
                1.0
            };

            ui.add_space(padding_above_progress_bar);
            ui.add_sized([ui.available_width(), 16.0], egui::ProgressBar::new(fraction).text(""));
            ui.add_space(padding_below_progress_bar);
            
            ui.add(egui::Separator::default().spacing(0.0));
            
            ui.add_space(5.0);
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false]) 
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.label(
                            egui::RichText::new(log_content)
                            .monospace()
                            .size(12.0)
                        );
                    });
                });
        });
    });
}