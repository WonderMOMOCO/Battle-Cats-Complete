use eframe::egui;
use std::path::PathBuf;
use crate::features::mods::logic::state::{ModState, ModPackType};
use crate::features::settings::logic::Settings;
use crate::features::import::logic::ImportSubTab;
use crate::features::addons::toolpaths::{self, Presence};
use crate::features::mods::logic::manager;

const PACKAGE_INPUT_PADDING: f32 = 5.0;

pub fn show(ctx: &egui::Context, state: &mut ModState, settings: &Settings) {
    let mut is_open = state.import.is_open;
    let window_id = egui::Id::new("import_mod_window");

    let is_busy = manager::process_events(state);
    if is_busy {
        ctx.request_repaint();
    }

    let (allow_drag, fixed_pos) = state.drag_guard.assign_bounds(ctx, window_id);

    let mut window = egui::Window::new("Import Mod")
        .id(window_id)
        .open(&mut is_open)
        .resizable(true)
        .default_size(egui::vec2(500.0, 400.0))
        .collapsible(false)
        .constrain(false)
        .movable(allow_drag);

    if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

    window.show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);
            let tabs = [
                (ImportSubTab::Emulator, "Android"),
                (ImportSubTab::Decrypt, "Pack"),
                (ImportSubTab::Sort, "Raw"),
            ];
            for (tab_enum, label) in tabs {
                let is_active = state.import.tab == tab_enum;
                let btn = egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE).size(14.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(egui::vec2(80.0, 30.0));
                if ui.add(btn).clicked() { state.import.tab = tab_enum; }
            }
        });

        ui.add_space(15.0);

        match state.import.tab {
            ImportSubTab::Emulator => show_adb_view(ui, state, settings),
            ImportSubTab::Decrypt => show_pack_view(ui, state),
            ImportSubTab::Sort => show_raw_view(ui, state),
        }

        ui.add_space(15.0);
        ui.separator();

        let status = &state.import.status_message;

        if is_busy && !status.contains("Success") && !status.contains("Error") {
            ui.horizontal(|ui| { ui.spinner(); ui.label(status); });
        } else {
            let color = if status.contains("Error") || status.contains("Failed") { egui::Color32::LIGHT_RED } 
            else if status.contains("Success") || status.contains("Complete") { egui::Color32::LIGHT_GREEN } 
            else { egui::Color32::LIGHT_BLUE };
            ui.colored_label(color, status);
        }
        
        ui.separator();
        
        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(egui::RichText::new(&state.import.log_content).monospace().size(12.0));
        });
    });

    state.import.is_open = is_open;
}

fn show_adb_view(ui: &mut egui::Ui, state: &mut ModState, _settings: &Settings) {
    let is_present = toolpaths::adb_status() == Presence::Installed;
    
    if is_present {
        ui.label("Import mod package using Android/Emulator");
    } else {
        ui.label(egui::RichText::new("Android Bridge is required. Download it in Settings > Add-Ons").color(egui::Color32::from_rgb(200, 150, 50)));
    }
    
    ui.add_space(10.0);

    ui.add_enabled_ui(!state.import.is_busy && is_present, |ui| {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = PACKAGE_INPUT_PADDING;
        ui.label(egui::RichText::new("Package:"));
        ui.add(egui::TextEdit::singleline(&mut state.import.package_suffix)
            .hint_text(egui::RichText::new("en").weak())
            .desired_width(40.0));
        });
    });

    ui.add_space(15.0);

    let btn_text = if is_present { "Start Import" } else { "ADB Missing" };
    if ui.add_enabled(!state.import.is_busy && is_present, egui::Button::new(btn_text)).clicked() {
        manager::start_adb_import(state);
    }
}

fn show_pack_view(ui: &mut egui::Ui, state: &mut ModState) {
    ui.label("Import modded DownloadLocal files manually");
    ui.add_space(10.0);

    let id = egui::Id::new("pack_view_path");
    let mut selected_path = ui.data(|d| d.get_temp::<Option<PathBuf>>(id).unwrap_or_default());

    let err_id = egui::Id::new("pack_error_msg");
    let time_id = egui::Id::new("pack_error_time");

    ui.add_enabled_ui(!state.import.is_busy, |ui| {
        ui.horizontal(|ui| {
            ui.label("Format:");
            egui::ComboBox::from_id_salt("mod_pack_type")
                .selected_text(match state.import.pack_type {
                    ModPackType::Apk => "APK",
                    ModPackType::Zip => "ZIP",
                    ModPackType::Folder => "Folder",
                    ModPackType::Pack => "Pack (.pack/.list)",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut state.import.pack_type, ModPackType::Apk, "APK").clicked() ||
                       ui.selectable_value(&mut state.import.pack_type, ModPackType::Zip, "ZIP").clicked() ||
                       ui.selectable_value(&mut state.import.pack_type, ModPackType::Folder, "Folder").clicked() ||
                       ui.selectable_value(&mut state.import.pack_type, ModPackType::Pack, "Pack (.pack/.list)").clicked() {
                           ui.data_mut(|d| d.insert_temp(id, None::<PathBuf>));
                       }
                });
        });
    });

    ui.add_space(5.0);

    ui.horizontal(|ui| {
        let enabled = !state.import.is_busy;
        let btn_text = if state.import.pack_type == ModPackType::Pack { "Select Pack/List" } else { "Select Source" };
        
        if ui.add_enabled(enabled, egui::Button::new(btn_text)).clicked() {
            handle_pack_selection(ui, state, id, err_id, time_id, &mut selected_path);
        }
        
        render_pack_selection_label(ui, state, time_id, err_id, &selected_path);
    });

    ui.add_space(15.0);

    if ui.add_enabled(!state.import.is_busy && selected_path.is_some(), egui::Button::new("Start Import")).clicked() {
        let Some(path) = selected_path else { return; };
        manager::start_pack_import(state, path);
    }
}

fn show_raw_view(ui: &mut egui::Ui, state: &mut ModState) {
    ui.label("Copy raw modded files into a mod folder");
    ui.add_space(10.0);

    let is_folder_id = egui::Id::new("raw_view_is_folder");
    let mut is_folder = ui.data(|d| d.get_temp::<bool>(is_folder_id).unwrap_or(true));

    let path_id = egui::Id::new("raw_view_path");
    let mut selected_path = ui.data(|d| d.get_temp::<Option<PathBuf>>(path_id).unwrap_or_default());

    let files_id = egui::Id::new("raw_view_files");
    let mut selected_files = ui.data(|d| d.get_temp::<Vec<PathBuf>>(files_id).unwrap_or_default());

    ui.add_enabled_ui(!state.import.is_busy, |ui| {
        ui.horizontal(|ui| {
            ui.label("Format:");
            egui::ComboBox::from_id_salt("raw_format_type")
                .selected_text(if is_folder { "Folder" } else { "Files" })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut is_folder, true, "Folder").clicked() ||
                       ui.selectable_value(&mut is_folder, false, "Files").clicked() {
                        ui.data_mut(|d| d.insert_temp(is_folder_id, is_folder));
                    }
                });
        });
    });

    ui.add_space(5.0);

    ui.horizontal(|ui| {
        let enabled = !state.import.is_busy;
        if is_folder {
            if ui.add_enabled(enabled, egui::Button::new("Select Source")).clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    selected_path = Some(p);
                    ui.data_mut(|d| d.insert_temp(path_id, selected_path.clone()));
                }
            }
            let label_text = if let Some(p) = &selected_path { 
                crate::features::import::logic::censor_path(&p.to_string_lossy()) 
            } else { 
                "No source selected".to_string() 
            };
            ui.label(label_text);
        } else {
            if ui.add_enabled(enabled, egui::Button::new("Select Files")).clicked() {
                if let Some(files) = rfd::FileDialog::new().pick_files() {
                    selected_files = files;
                    ui.data_mut(|d| d.insert_temp(files_id, selected_files.clone()));
                }
            }
            let count = selected_files.len();
            let label_text = match count {
                0 => "No files selected".to_string(),
                1 => "1 File".to_string(),
                _ => format!("{} Files", count),
            };
            ui.label(label_text);
        }
    });

    ui.add_space(15.0);

    let can_import = (is_folder && selected_path.is_some()) || (!is_folder && !selected_files.is_empty());
    
    if ui.add_enabled(!state.import.is_busy && can_import, egui::Button::new("Start Import")).clicked() {
         manager::start_raw_import(state, is_folder, selected_path, selected_files);
    }
}

fn handle_pack_selection(
    ui: &mut egui::Ui, 
    state: &mut ModState, 
    id: egui::Id, err_id: egui::Id, time_id: egui::Id, 
    selected_path: &mut Option<PathBuf>
) {
    if state.import.pack_type != ModPackType::Pack {
        let path_opt = match state.import.pack_type {
            ModPackType::Apk => rfd::FileDialog::new().add_filter("APK", &["apk"]).pick_file(),
            ModPackType::Zip => rfd::FileDialog::new().add_filter("ZIP", &["zip"]).pick_file(),
            ModPackType::Folder => rfd::FileDialog::new().pick_folder(),
            _ => None,
        };
        if let Some(p) = path_opt {
            *selected_path = Some(p.clone());
            ui.data_mut(|d| d.insert_temp(id, Some(p)));
        }
        return;
    }

    let Some(files) = rfd::FileDialog::new().add_filter("Pack/List", &["pack", "list"]).pick_files() else { return; };
    let Some(first) = files.first() else { return; };

    let parent = first.parent().unwrap();
    let stem = first.file_stem().unwrap().to_string_lossy();
    let pack_file = parent.join(format!("{}.pack", stem));
    let list_file = parent.join(format!("{}.list", stem));
    
    if !pack_file.exists() {
        ui.data_mut(|d| {
            d.insert_temp(err_id, "Missing .pack!".to_string());
            d.insert_temp(time_id, ui.ctx().input(|i| i.time));
            d.insert_temp(id, None::<PathBuf>);
        });
        *selected_path = None;
        return;
    }

    if !list_file.exists() {
        ui.data_mut(|d| {
            d.insert_temp(err_id, "Missing .list!".to_string());
            d.insert_temp(time_id, ui.ctx().input(|i| i.time));
            d.insert_temp(id, None::<PathBuf>);
        });
        *selected_path = None;
        return;
    }

    *selected_path = Some(pack_file.clone());
    ui.data_mut(|d| d.insert_temp(id, Some(pack_file)));
}

fn render_pack_selection_label(
    ui: &mut egui::Ui, 
    state: &ModState, 
    time_id: egui::Id, err_id: egui::Id, 
    selected_path: &Option<PathBuf>
) {
    let current_time = ui.ctx().input(|i| i.time);
    let err_time = ui.data(|d| d.get_temp::<f64>(time_id).unwrap_or(0.0));
    let err_msg = ui.data(|d| d.get_temp::<String>(err_id).unwrap_or_default());
    
    if current_time < err_time + 2.0 {
        ui.label(egui::RichText::new(err_msg).color(egui::Color32::RED));
        ui.ctx().request_repaint(); 
        return;
    } 
    
    let Some(p) = selected_path else {
        ui.label("No source selected");
        return;
    };

    if state.import.pack_type == ModPackType::Pack {
        let stem = p.file_stem().unwrap_or_default().to_string_lossy();
        ui.label(egui::RichText::new(format!("{} Found!", stem)).color(egui::Color32::GREEN));
        return;
    } 
    
    ui.label(crate::features::import::logic::censor_path(&p.to_string_lossy()));
}