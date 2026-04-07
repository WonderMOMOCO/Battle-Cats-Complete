use eframe::egui;
use std::sync::atomic::Ordering;
use crate::features::data::state::{ImportState, DataTab};
use crate::features::settings::logic::Settings;
use crate::features::data::ui::{import, export};

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
                
                let button = egui::Button::new(egui::RichText::new(label).color(text_color))
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(egui::Rounding::ZERO)
                    .min_size(egui::vec2(80.0, 30.0));

                if ui.add(button).clicked() {
                    state.active_tab = tab;
                }
            }
        });

        let pad_above_tab_sep = 10.0;
        let pad_below_tab_sep = 15.0;
        let pad_above_console_sep = 15.0;
        let pad_below_console_sep = 0.0;
        let pad_above_progress = 8.0;
        let pad_below_progress = 8.0;

        ui.add_space(pad_above_tab_sep);
        ui.add(egui::Separator::default().spacing(0.0)); 
        ui.add_space(pad_below_tab_sep);

        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            
            match state.active_tab {
                DataTab::Import => import::show(ui, state, settings),
                DataTab::Export => export::show(ui, state, settings),
            }

            ui.add_space(pad_above_console_sep);
            ui.add(egui::Separator::default().spacing(0.0)); 
            ui.add_space(pad_below_console_sep);

            let (is_running, log_content, progress_current, progress_maximum) = match state.active_tab {
                DataTab::Import => (
                    state.import_job_status.load(Ordering::Relaxed) == 1,
                    &state.import_log_content,
                    state.import_progress_current.load(Ordering::Relaxed),
                    state.import_progress_maximum.load(Ordering::Relaxed)
                ),
                DataTab::Export => (
                    state.export_job_status.load(Ordering::Relaxed) == 1,
                    &state.export_log_content,
                    state.export_progress_current.load(Ordering::Relaxed),
                    state.export_progress_maximum.load(Ordering::Relaxed)
                ),
            };

            let progress_fraction = if is_running {
                if progress_maximum > 0 { 
                    progress_current as f32 / progress_maximum as f32 
                } else { 
                    1.0 
                }
            } else {
                1.0
            };

            ui.add_space(pad_above_progress);
            ui.add_sized([ui.available_width(), 16.0], egui::ProgressBar::new(progress_fraction).text(""));
            ui.add_space(pad_below_progress);
            
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