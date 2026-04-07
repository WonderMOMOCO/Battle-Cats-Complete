use eframe::egui;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;

use crate::features::data::state::{ImportState, ImportSubTab, AdbImportType, AdbRegion, ImportMode};
use crate::features::settings::logic::Settings;
use crate::features::addons::toolpaths::{self, Presence};
use crate::features::data::leaders::{android, pack, raw};

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &mut Settings) {
    let current_status = state.import_job_status.load(Ordering::Relaxed);
    let is_running = current_status == 1;

    let col_width_reduction = 40.0; 
    let column_min_height = 120.0;  

    let padding_job_details = 10.0; 
    let padding_above_separator = 20.0;
    let padding_below_separator = 15.0;

    ui.add_enabled_ui(!is_running, |ui| {
        let total_width = ui.available_width();
        let spacing = 16.0;
        let col_width = (total_width - (spacing * 2.0) - col_width_reduction) / 3.0;

        ui.horizontal(|ui| {
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);

            // COLUMN 1: ANDROID
            ui.vertical(|ui| {
                ui.set_min_width(col_width);
                ui.set_max_width(col_width);
                ui.set_min_height(column_min_height); 

                let adb_installed = toolpaths::adb_status() == Presence::Installed;

                ui.vertical_centered(|ui| {
                    let header_width = col_width * 0.8;
                    let display_color = if state.selected_job == Some(ImportSubTab::Emulator) { active_color } else { inactive_color };
                    
                    let android_button = egui::Button::new(egui::RichText::new("Android").color(egui::Color32::WHITE).size(16.0))
                        .fill(display_color).rounding(egui::Rounding::same(6.0));

                    if ui.add_sized([header_width, 35.0], android_button).clicked() && adb_installed {
                        state.selected_job = Some(ImportSubTab::Emulator);
                    }
                    
                    ui.add_space(10.0);

                    if adb_installed {
                        ui.label("Import directly via Bridge");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(200, 150, 50), "Requires Android Bridge Add-On");
                    }
                });

                ui.add_space(padding_job_details);

                ui.add_enabled_ui(adb_installed, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(10.0); 
                        ui.label("Type:");
                        
                        let type_text = if settings.game_data.adb_import_type_idx == 1 { "Update Only" } else { "All Content" };
                        
                        egui::ComboBox::from_id_salt("adb_type")
                            .selected_text(type_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut settings.game_data.adb_import_type_idx, 0, "All Content");
                                ui.selectable_value(&mut settings.game_data.adb_import_type_idx, 1, "Update Only");
                            });
                    });
                    
                    ui.add_space(padding_job_details);
                    
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label("Region:");
                        
                        let region_text = match settings.game_data.adb_region_idx { 
                            0 => "Global", 1 => "Japan", 2 => "Taiwan", 3 => "Korea", _ => "All Regions" 
                        };
                        
                        egui::ComboBox::from_id_salt("adb_region")
                            .selected_text(region_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 0, "Global");
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 1, "Japan");
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 2, "Taiwan");
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 3, "Korea");
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 4, "All Regions");
                            });
                    });
                });
            });

            ui.add_space(spacing / 2.0);
            ui.add(egui::Separator::default().vertical().spacing(0.0));
            ui.add_space(spacing / 2.0);

            // COLUMN 2: PACK
            ui.vertical(|ui| {
                ui.set_min_width(col_width);
                ui.set_max_width(col_width);
                ui.set_min_height(column_min_height); 

                ui.vertical_centered(|ui| {
                    let header_width = col_width * 0.8;
                    let display_color = if state.selected_job == Some(ImportSubTab::Decrypt) { active_color } else { inactive_color };
                    
                    let pack_button = egui::Button::new(egui::RichText::new("Pack").color(egui::Color32::WHITE).size(16.0))
                        .fill(display_color).rounding(egui::Rounding::same(6.0));

                    if ui.add_sized([header_width, 35.0], pack_button).clicked() {
                        state.selected_job = Some(ImportSubTab::Decrypt);
                    }
                    
                    ui.add_space(10.0);
                    ui.label("Decrypt external pack files");
                });

                ui.add_space(padding_job_details);
                
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label("Region:");
                    
                    let region_text = match state.adb_region { 
                        AdbRegion::English => "Global", AdbRegion::Japanese => "Japan", AdbRegion::Taiwan => "Taiwan", AdbRegion::Korean => "Korea", AdbRegion::All => "All" 
                    };

                    egui::ComboBox::from_id_salt("dec_region")
                        .selected_text(region_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.adb_region, AdbRegion::English, "Global");
                            ui.selectable_value(&mut state.adb_region, AdbRegion::Japanese, "Japan");
                            ui.selectable_value(&mut state.adb_region, AdbRegion::Taiwan, "Taiwan");
                            ui.selectable_value(&mut state.adb_region, AdbRegion::Korean, "Korea");
                        });
                });
                
                ui.add_space(padding_job_details);
                
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if ui.button("Select Folder").clicked() {
                        if let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
                            state.decrypt_path = folder_path.to_string_lossy().to_string();
                            state.decrypt_censored = crate::features::data::state::censor_path(&state.decrypt_path);
                        }
                    }
                    ui.label(if state.decrypt_censored.is_empty() { "None selected" } else { &state.decrypt_censored });
                });
            });

            ui.add_space(spacing / 2.0);
            ui.add(egui::Separator::default().vertical().spacing(0.0));
            ui.add_space(spacing / 2.0);

            // COLUMN 3: RAW
            ui.vertical(|ui| {
                ui.set_min_width(col_width);
                ui.set_max_width(col_width);
                ui.set_min_height(column_min_height); 

                ui.vertical_centered(|ui| {
                    let header_width = col_width * 0.8;
                    let display_color = if state.selected_job == Some(ImportSubTab::Sort) { active_color } else { inactive_color };
                    
                    let raw_button = egui::Button::new(egui::RichText::new("Raw").color(egui::Color32::WHITE).size(16.0))
                        .fill(display_color).rounding(egui::Rounding::same(6.0));

                    if ui.add_sized([header_width, 35.0], raw_button).clicked() {
                        state.selected_job = Some(ImportSubTab::Sort);
                    }
                    
                    ui.add_space(10.0);
                    ui.label("Sort archive or raw files");
                });

                ui.add_space(padding_job_details);
                
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label("Source:");
                    
                    let source_text = match state.import_mode { ImportMode::Folder => "Folder", _ => "Archive" };

                    egui::ComboBox::from_id_salt("raw_mode")
                        .selected_text(source_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.import_mode, ImportMode::Folder, "Folder");
                            ui.selectable_value(&mut state.import_mode, ImportMode::Zip, "Archive");
                        });
                });
                
                ui.add_space(padding_job_details);
                
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if ui.button("Select Data").clicked() {
                        let dialog_result = match state.import_mode {
                            ImportMode::Zip => rfd::FileDialog::new().add_filter("Archive", &["zst", "tar", "zip"]).pick_file(),
                            ImportMode::Folder => rfd::FileDialog::new().pick_folder(),
                            _ => None,
                        };
                        if let Some(file_path) = dialog_result {
                            state.import_path = file_path.to_string_lossy().to_string();
                            state.import_censored = crate::features::data::state::censor_path(&state.import_path);
                        }
                    }
                    ui.label(if state.import_censored.is_empty() { "None selected" } else { &state.import_censored });
                });
            });
        });
    });

    ui.add_space(padding_above_separator);
    ui.add(egui::Separator::default().spacing(0.0));
    ui.add_space(padding_below_separator);

    ui.horizontal(|ui| {
        let button_width = 300.0;
        ui.add_space((ui.available_width() - button_width) / 2.0);

        let show_success = state.import_job_completed_time.map_or(false, |time| time.elapsed().as_secs() < 2);
        let show_aborted = state.import_job_aborted_time.map_or(false, |time| time.elapsed().as_secs() < 2);
        let is_aborting = is_running && state.import_abort_flag.load(Ordering::Relaxed);

        let (button_text, can_run, active_color) = match state.selected_job {
            Some(ImportSubTab::Emulator) => {
                let is_installed = toolpaths::adb_status() == Presence::Installed;
                (if is_installed { "Start Job" } else { "Bridge Missing" }, is_installed, egui::Color32::from_rgb(31, 106, 165))
            },
            Some(ImportSubTab::Decrypt) => {
                let has_path = !state.decrypt_path.is_empty();
                (if has_path { "Start Job" } else { "Select Source Folder" }, has_path, egui::Color32::from_rgb(31, 106, 165))
            },
            Some(ImportSubTab::Sort) => {
                let has_path = !state.import_path.is_empty();
                (if has_path { "Start Job" } else { "Select Source Data" }, has_path, egui::Color32::from_rgb(31, 106, 165))
            },
            None => ("Select a Job", false, egui::Color32::from_gray(80)),
        };

        if show_success {
            let success_btn = egui::Button::new(egui::RichText::new("Job Complete!").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(50, 180, 50))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));
                
            if ui.add_enabled(can_run, success_btn).clicked() { trigger_import_job(state, settings); }
            return;
        } 
        
        if show_aborted {
            let aborted_btn = egui::Button::new(egui::RichText::new("Job Aborted!").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(200, 50, 50))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));
                
            if ui.add_enabled(can_run, aborted_btn).clicked() { trigger_import_job(state, settings); }
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
                state.import_abort_flag.store(true, Ordering::Relaxed);
                state.import_progress_current.store(0, Ordering::Relaxed);
                state.import_progress_maximum.store(0, Ordering::Relaxed);
            }
            return;
        }

        let final_color = if can_run { active_color } else { egui::Color32::from_gray(80) };
        let action_btn = egui::Button::new(egui::RichText::new(button_text).color(egui::Color32::WHITE).size(18.0).strong())
            .fill(final_color).min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));

        if ui.add_enabled(can_run, action_btn).clicked() {
            trigger_import_job(state, settings);
        }
    });
}

fn trigger_import_job(state: &mut ImportState, settings: &mut Settings) {
    state.import_job_status.store(1, Ordering::Relaxed);
    state.import_abort_flag.store(false, Ordering::Relaxed);
    state.import_progress_current.store(0, Ordering::Relaxed);
    state.import_progress_maximum.store(0, Ordering::Relaxed);
    state.import_log_content.clear();
    state.import_job_completed_time = None;
    state.import_job_aborted_time = None;
    
    let (sender, receiver) = mpsc::channel();
    state.import_rx = Some(receiver);
    
    let abort = state.import_abort_flag.clone();
    let status = state.import_job_status.clone();
    let progress_current = state.import_progress_current.clone();
    let progress_max = state.import_progress_maximum.clone();
    
    match state.selected_job {
        Some(ImportSubTab::Emulator) => {
            let mode = if settings.game_data.adb_import_type_idx == 1 { AdbImportType::Update } else { AdbImportType::All };
            let region = match settings.game_data.adb_region_idx { 
                0 => AdbRegion::English, 1 => AdbRegion::Japanese, 2 => AdbRegion::Taiwan, 3 => AdbRegion::Korean, _ => AdbRegion::All 
            };
            
            android::run(
                sender, 
                mode, 
                region, 
                settings.emulator_config(), 
                abort, 
                status, 
                progress_current, 
                progress_max
            );
        },
        Some(ImportSubTab::Decrypt) => {
            let folder_path = state.decrypt_path.clone();
            let mode = ImportMode::Folder;
            let region = state.adb_region;
            
            thread::spawn(move || {
                let result = pack::run(
                    &folder_path, 
                    mode, 
                    region, 
                    sender, 
                    abort, 
                    progress_current, 
                    progress_max
                );
                
                if result.is_err() {
                    status.store(3, Ordering::Relaxed);
                } else {
                    status.store(2, Ordering::Relaxed);
                }
            });
        },
        Some(ImportSubTab::Sort) => {
            let data_path = state.import_path.clone();
            
            thread::spawn(move || {
                let result = raw::run(
                    &data_path, 
                    sender, 
                    abort, 
                    progress_current, 
                    progress_max
                );
                
                if result.is_err() {
                    status.store(3, Ordering::Relaxed);
                } else {
                    status.store(2, Ordering::Relaxed);
                }
            });
        },
        None => {}
    }
}