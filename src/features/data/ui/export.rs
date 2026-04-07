use eframe::egui;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;

use crate::features::data::state::ImportState;
use crate::features::data::leaders::export;
use crate::features::settings::logic::Settings;
use crate::features::settings::ui::tabs::toggle_ui;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &mut Settings) {
    let current_status = state.export_job_status.load(Ordering::Relaxed);
    let is_running = current_status == 1;

    let padding_job_details = 10.0;
    let padding_above_separator = 20.0;
    let padding_below_separator = 15.0;

    ui.add_enabled_ui(!is_running, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("Package database into a ZST archive").size(16.0));
        });
        
        ui.add_space(15.0);
        
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            toggle_ui(ui, &mut state.include_raw);
            ui.label("Include \"raw\" Folder");
        });
        
        ui.add_space(padding_job_details);
        
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label("Filename:");
            
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.add(egui::TextEdit::singleline(&mut state.export_filename)
                .hint_text(egui::RichText::new("battlecats").color(egui::Color32::DARK_GRAY))
                .desired_width(100.0)
            );
            
            ui.label(".tar.zst");
        });
        
        ui.add_space(padding_job_details);

        let max_compression = if settings.game_data.enable_ultra_compression { 21 } else { 15 };
        
        if state.compression_level == 0 {
            state.compression_level = settings.game_data.last_compression_level;
        }

        if state.compression_level > max_compression {
            state.compression_level = max_compression;
        }

        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label("Compression Level:");
            
            let slider_response = ui.add(egui::Slider::new(&mut state.compression_level, 1..=max_compression));
                
            if slider_response.changed() {
                settings.game_data.last_compression_level = state.compression_level;
            }
        });
        
        ui.add_space(5.0);
        
        let (desc_text, desc_color) = match state.compression_level {
            1..=9 => (
                "Best compression balance",
                egui::Color32::from_rgb(120, 210, 120) 
            ),
            10..=15 => (
                "Slow compression for low archive size",
                egui::Color32::from_rgb(240, 200, 80) 
            ),
            _ => (
                "Ultra compression granting minimal returns",
                egui::Color32::from_rgb(240, 100, 100) 
            ),
        };

        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(desc_text)
                    .color(desc_color)
                    .small()
            );
        });
    });
    
    ui.add_space(padding_above_separator);
    ui.add(egui::Separator::default().spacing(0.0));
    ui.add_space(padding_below_separator);
    
    let base_filename = if state.export_filename.trim().is_empty() { "battlecats" } else { &state.export_filename };
    let full_filename = format!("{}.tar.zst", base_filename);
    let button_text = format!("Create {}", full_filename);

    let show_success = state.export_job_completed_time.map_or(false, |time| time.elapsed().as_secs() < 2);
    let show_aborted = state.export_job_aborted_time.map_or(false, |time| time.elapsed().as_secs() < 2);
    let is_aborting = is_running && state.export_abort_flag.load(Ordering::Relaxed);

    ui.horizontal(|ui| {
        let button_width = 300.0;
        ui.add_space((ui.available_width() - button_width) / 2.0); 

        if show_success {
            let success_btn = egui::Button::new(egui::RichText::new("Job Complete!").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(50, 180, 50))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));
                
            if ui.add(success_btn).clicked() { trigger_export_job(state, full_filename); }
            return;
        } 
        
        if show_aborted {
            let aborted_btn = egui::Button::new(egui::RichText::new("Job Aborted!").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(200, 50, 50))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));
                
            if ui.add(aborted_btn).clicked() { trigger_export_job(state, full_filename); }
            return;
        } 
        
        if is_aborting {
            let aborting_btn = egui::Button::new(egui::RichText::new("Aborting Job...").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(220, 180, 40)) 
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));
                
            ui.add(aborting_btn);
            return;
        } 
        
        if is_running {
            let cancel_btn = egui::Button::new(egui::RichText::new("Abort Job").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(200, 50, 50))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));
                
            if ui.add(cancel_btn).clicked() {
                state.export_abort_flag.store(true, Ordering::Relaxed);
                state.export_progress_current.store(0, Ordering::Relaxed);
                state.export_progress_maximum.store(0, Ordering::Relaxed);
            }
            return;
        }

        let standard_btn = egui::Button::new(egui::RichText::new(button_text).color(egui::Color32::WHITE).size(18.0).strong())
            .fill(egui::Color32::from_rgb(31, 106, 165))
            .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));

        if ui.add(standard_btn).clicked() {
            trigger_export_job(state, full_filename);
        }
    });
}

fn trigger_export_job(state: &mut ImportState, filename_argument: String) {
    state.export_job_status.store(1, Ordering::Relaxed);
    state.export_abort_flag.store(false, Ordering::Relaxed);
    state.export_progress_current.store(0, Ordering::Relaxed);
    state.export_progress_maximum.store(0, Ordering::Relaxed);
    state.export_log_content.clear();
    state.export_job_completed_time = None;
    state.export_job_aborted_time = None;
    
    let (sender, receiver) = mpsc::channel();
    state.export_rx = Some(receiver);
    
    let compression_level = state.compression_level;
    let include_raw = state.include_raw;
    let status = state.export_job_status.clone();
    let abort = state.export_abort_flag.clone();
    let progress_current = state.export_progress_current.clone();
    let progress_maximum = state.export_progress_maximum.clone();

    thread::spawn(move || {
        let result = export::create_game_archive(
            sender.clone(), 
            abort.clone(), 
            progress_current, 
            progress_maximum, 
            compression_level, 
            filename_argument, 
            include_raw
        );
        
        if let Err(error) = result {
            let _ = sender.send(format!("Error Packing: {}", error));
            status.store(3, Ordering::Relaxed);
        } else if !abort.load(Ordering::Relaxed) {
            status.store(2, Ordering::Relaxed);
        } else {
            status.store(3, Ordering::Relaxed);
        }
    });
}