use eframe::egui;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::path::{Path, PathBuf};
use crate::features::data::logic::{ImportState, ImportSubTab, AdbImportType, AdbRegion, ImportMode, decrypt};
use crate::features::settings::logic::Settings;
use crate::features::addons::toolpaths::{self, Presence};
use crate::features::addons::adb::bridge;
use crate::features::data::{archive, sort};

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &mut Settings) {
    let current_status = state.import_job_status.load(Ordering::Relaxed);
    let is_running = current_status == 1;

    let col_width_reduction = 40.0; 
    let column_min_height = 120.0;  

    let padding_between_job_details = 10.0; 
    
    let padding_above_action_separator = 20.0;
    let padding_between_separator_and_btn = 15.0;

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
                    let header_w = col_width * 0.8;
                    let color = if state.selected_job == Some(ImportSubTab::Emulator) { active_color } else { inactive_color };
                    
                    let android_btn = egui::Button::new(egui::RichText::new("Android").color(egui::Color32::WHITE).size(16.0))
                        .fill(color).rounding(egui::Rounding::same(6.0));

                    if ui.add_sized([header_w, 35.0], android_btn).clicked() && adb_installed {
                        state.selected_job = Some(ImportSubTab::Emulator);
                    }
                    
                    ui.add_space(10.0);

                    if adb_installed {
                        ui.label("Import directly via Bridge");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(200, 150, 50), "Requires Android Bridge Add-On");
                    }
                });

                ui.add_space(padding_between_job_details);

                ui.add_enabled_ui(adb_installed, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(10.0); 
                        ui.label("Type:");
                        egui::ComboBox::from_id_salt("adb_type")
                            .selected_text(if settings.game_data.adb_import_type_idx == 1 { "Update Only" } else { "All Content" })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut settings.game_data.adb_import_type_idx, 0, "All Content");
                                ui.selectable_value(&mut settings.game_data.adb_import_type_idx, 1, "Update Only");
                            });
                    });
                    
                    ui.add_space(padding_between_job_details);
                    
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label("Region:");
                        egui::ComboBox::from_id_salt("adb_region")
                            .selected_text(match settings.game_data.adb_region_idx { 0 => "Global", 1 => "Japan", 2 => "Taiwan", 3 => "Korea", _ => "All Regions" })
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
                    let header_w = col_width * 0.8;
                    let color = if state.selected_job == Some(ImportSubTab::Decrypt) { active_color } else { inactive_color };
                    if ui.add_sized([header_w, 35.0], egui::Button::new(egui::RichText::new("Pack").color(egui::Color32::WHITE).size(16.0)).fill(color).rounding(egui::Rounding::same(6.0))).clicked() {
                        state.selected_job = Some(ImportSubTab::Decrypt);
                    }
                    
                    ui.add_space(10.0);
                    ui.label("Decrypt external pack files");
                });

                ui.add_space(padding_between_job_details);
                
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label("Region:");
                    egui::ComboBox::from_id_salt("dec_region")
                        .selected_text(match state.adb_region { AdbRegion::English => "Global", AdbRegion::Japanese => "Japan", AdbRegion::Taiwan => "Taiwan", AdbRegion::Korean => "Korea", AdbRegion::All => "All" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.adb_region, AdbRegion::English, "Global");
                            ui.selectable_value(&mut state.adb_region, AdbRegion::Japanese, "Japan");
                            ui.selectable_value(&mut state.adb_region, AdbRegion::Taiwan, "Taiwan");
                            ui.selectable_value(&mut state.adb_region, AdbRegion::Korean, "Korea");
                        });
                });
                
                ui.add_space(padding_between_job_details);
                
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if ui.button("Select Folder").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            state.decrypt_path = path.to_string_lossy().to_string();
                            state.decrypt_censored = crate::features::data::logic::censor_path(&state.decrypt_path);
                        }
                    }
                    ui.label(if state.decrypt_censored.is_empty() { "None selected" } else { &state.decrypt_censored });
                });
            });

            ui.add_space(spacing / 2.0);
            ui.add(egui::Separator::default().spacing(0.0));

            // COLUMN 3: RAW
            ui.vertical(|ui| {
                ui.set_min_width(col_width);
                ui.set_max_width(col_width);
                ui.set_min_height(column_min_height); 

                ui.vertical_centered(|ui| {
                    let header_w = col_width * 0.8;
                    let color = if state.selected_job == Some(ImportSubTab::Sort) { active_color } else { inactive_color };
                    if ui.add_sized([header_w, 35.0], egui::Button::new(egui::RichText::new("Raw").color(egui::Color32::WHITE).size(16.0)).fill(color).rounding(egui::Rounding::same(6.0))).clicked() {
                        state.selected_job = Some(ImportSubTab::Sort);
                    }
                    
                    ui.add_space(10.0);
                    ui.label("Sort archive or raw files");
                });

                ui.add_space(padding_between_job_details);
                
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label("Source:");
                    egui::ComboBox::from_id_salt("raw_mode")
                        .selected_text(match state.import_mode { ImportMode::Folder => "Folder", _ => "Archive" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.import_mode, ImportMode::Folder, "Folder");
                            ui.selectable_value(&mut state.import_mode, ImportMode::Zip, "Archive");
                        });
                });
                
                ui.add_space(padding_between_job_details);
                
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if ui.button("Select Data").clicked() {
                        let res = match state.import_mode {
                            ImportMode::Zip => rfd::FileDialog::new().add_filter("Archive", &["zst", "tar", "zip"]).pick_file(),
                            ImportMode::Folder => rfd::FileDialog::new().pick_folder(),
                            _ => None,
                        };
                        if let Some(path) = res {
                            state.import_path = path.to_string_lossy().to_string();
                            state.import_censored = crate::features::data::logic::censor_path(&state.import_path);
                        }
                    }
                    ui.label(if state.import_censored.is_empty() { "None selected" } else { &state.import_censored });
                });
            });
        });
    });

    ui.add_space(padding_above_action_separator);
    ui.add(egui::Separator::default().spacing(0.0));
    ui.add_space(padding_between_separator_and_btn);

    // ACTION BUTTON
    ui.horizontal(|ui| {
        let btn_w = 300.0;
        ui.add_space((ui.available_width() - btn_w) / 2.0);

        let show_success = state.import_job_completed_time.map_or(false, |t| t.elapsed().as_secs() < 2);
        let show_aborted = state.import_job_aborted_time.map_or(false, |t| t.elapsed().as_secs() < 2);
        let is_aborting = is_running && state.import_abort_flag.load(Ordering::Relaxed);

        // Pre-calculate validation for closures
        let (btn_text, can_run, fill_color) = match state.selected_job {
            Some(ImportSubTab::Emulator) => {
                let can = toolpaths::adb_status() == Presence::Installed;
                (if can { "Start Job" } else { "Bridge Missing" }, can, egui::Color32::from_rgb(31, 106, 165))
            },
            Some(ImportSubTab::Decrypt) => {
                let can = !state.decrypt_path.is_empty();
                (if can { "Start Job" } else { "Select Source Folder" }, can, egui::Color32::from_rgb(31, 106, 165))
            },
            Some(ImportSubTab::Sort) => {
                let can = !state.import_path.is_empty();
                (if can { "Start Job" } else { "Select Source Data" }, can, egui::Color32::from_rgb(31, 106, 165))
            },
            None => ("Select a Job", false, egui::Color32::from_gray(80)),
        };

        let mut start_job = || {
            state.import_job_status.store(1, Ordering::Relaxed);
            state.import_abort_flag.store(false, Ordering::Relaxed);
            state.import_progress_current.store(0, Ordering::Relaxed);
            state.import_progress_max.store(0, Ordering::Relaxed);
            state.import_log_content.clear();
            state.import_job_completed_time = None;
            state.import_job_aborted_time = None;
            
            let (tx, rx) = mpsc::channel();
            state.import_rx = Some(rx);
            let abort = state.import_abort_flag.clone();
            let status = state.import_job_status.clone();
            let prog_curr = state.import_progress_current.clone();
            let prog_max = state.import_progress_max.clone();
            
            match state.selected_job {
                Some(ImportSubTab::Emulator) => {
                    let mode = if settings.game_data.adb_import_type_idx == 1 { AdbImportType::Update } else { AdbImportType::All };
                    let region = match settings.game_data.adb_region_idx { 0 => AdbRegion::English, 1 => AdbRegion::Japanese, 2 => AdbRegion::Taiwan, 3 => AdbRegion::Korean, _ => AdbRegion::All };
                    bridge::spawn_full_import(tx, PathBuf::from("game/app"), mode, region, settings.emulator_config(), abort, status, prog_curr, prog_max);
                },
                Some(ImportSubTab::Decrypt) => {
                    let folder = state.decrypt_path.clone();
                    let r_code = match state.adb_region { AdbRegion::English => "en", AdbRegion::Japanese => "ja", AdbRegion::Taiwan => "tw", AdbRegion::Korean => "ko", _ => "en" }.to_string();
                    thread::spawn(move || {
                        let mut idx = decrypt::build_index(Path::new("game"));
                        if let Err(_) = decrypt::run(&folder, &r_code, &mut idx, tx.clone(), abort.clone(), prog_curr.clone(), prog_max.clone()) {
                            status.store(3, Ordering::Relaxed);
                            return;
                        }
                        if abort.load(Ordering::Relaxed) { status.store(3, Ordering::Relaxed); return; }
                        if let Err(_) = sort::sort_game_files(tx.clone(), abort.clone(), prog_curr.clone(), prog_max.clone()) {
                            status.store(3, Ordering::Relaxed);
                        } else {
                            status.store(2, Ordering::Relaxed);
                        }
                    });
                },
                Some(ImportSubTab::Sort) => {
                    let path = state.import_path.clone();
                    let mode = state.import_mode;
                    thread::spawn(move || {
                        let res = match mode {
                            ImportMode::Folder => archive::import_standard_folder(&path, tx.clone(), abort.clone(), prog_curr.clone(), prog_max.clone()), 
                            ImportMode::Zip => archive::import_standard_archive(&path, tx.clone(), abort.clone(), prog_curr.clone(), prog_max.clone()),
                            _ => Err("Invalid mode".to_string()),
                        };
                        match res {
                            Ok(true) => {
                                if !abort.load(Ordering::Relaxed) {
                                    let _ = sort::sort_game_files(tx.clone(), abort.clone(), prog_curr.clone(), prog_max.clone());
                                }
                                status.store(2, Ordering::Relaxed);
                            },
                            Ok(false) => status.store(2, Ordering::Relaxed),
                            Err(_) => status.store(3, Ordering::Relaxed),
                        }
                    });
                },
                _ => {}
            }
        };

        if show_success {
            let btn = egui::Button::new(egui::RichText::new("Job Complete!").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(50, 180, 50))
                .min_size(egui::vec2(btn_w, 45.0)).rounding(egui::Rounding::same(8.0));
            if ui.add_enabled(can_run, btn).clicked() { start_job(); }
        } else if show_aborted {
            let btn = egui::Button::new(egui::RichText::new("Job Aborted!").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(200, 50, 50))
                .min_size(egui::vec2(btn_w, 45.0)).rounding(egui::Rounding::same(8.0));
            if ui.add_enabled(can_run, btn).clicked() { start_job(); }
        } else if is_aborting {
            ui.add(egui::Button::new(egui::RichText::new("Aborting Job...").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(220, 180, 40)) 
                .min_size(egui::vec2(btn_w, 45.0)).rounding(egui::Rounding::same(8.0)));
        } else if is_running {
            if ui.add(egui::Button::new(egui::RichText::new("Abort Job").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(200, 50, 50))
                .min_size(egui::vec2(btn_w, 45.0)).rounding(egui::Rounding::same(8.0))).clicked() {
                state.import_abort_flag.store(true, Ordering::Relaxed);
                state.import_progress_current.store(0, Ordering::Relaxed);
                state.import_progress_max.store(0, Ordering::Relaxed);
            }
        } else {
            let active_fill = if can_run { fill_color } else { egui::Color32::from_gray(80) };
            let action_btn = egui::Button::new(egui::RichText::new(btn_text).color(egui::Color32::WHITE).size(18.0).strong())
                .fill(active_fill).min_size(egui::vec2(btn_w, 45.0)).rounding(egui::Rounding::same(8.0));

            if ui.add_enabled(can_run, action_btn).clicked() {
                start_job();
            }
        }
    });
}