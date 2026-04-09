use std::fs;
use std::path::Path;
use eframe::egui;
use crate::features::settings::logic::state::{GameDataSettings, RuntimeState};
use crate::features::settings::logic::delete::FolderDeleter;
use crate::global::ui::shared::DragGuard;
use super::tabs::toggle_ui;

#[derive(Clone, Default)]
struct FolderDeleteState {
    is_open: bool,
    size_str: Option<String>,
}

fn get_folder_size(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    size += get_folder_size(&entry.path());
                } else {
                    size += metadata.len();
                }
            }
        }
    }
    size
}

fn format_size(size: u64) -> String {
    let kb = 1024.0;
    let mb = kb * 1024.0;
    let gb = mb * 1024.0;
    let size_f = size as f64;

    if size_f >= gb {
        format!("{:.2} GB", size_f / gb)
    } else if size_f >= mb {
        format!("{:.2} MB", size_f / mb)
    } else if size_f >= kb {
        format!("{:.2} KB", size_f / kb)
    } else {
        format!("{} B", size)
    }
}

fn show_folder_delete_modal(
    ctx: &egui::Context,
    drag_guard: &mut DragGuard,
    id_str: &str,
    content: &str,
) -> bool {
    let state_id = egui::Id::new(id_str);
    let mut state = ctx.data(|d| d.get_temp::<FolderDeleteState>(state_id)).unwrap_or_default();
    let mut yes_clicked = false;

    if state.is_open {
        let window_id = egui::Id::new(format!("{}_window", id_str));
        let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);
        let mut should_close = false;

        let mut window = egui::Window::new("Confirm Deletion")
            .id(window_id)
            .collapsible(false)
            .resizable(false)
            .constrain(false)
            .movable(allow_drag) 
            .default_pos(ctx.screen_rect().center() - egui::vec2(150.0, 50.0));
            
        if let Some(pos) = fixed_pos { window = window.current_pos(pos); }
            
        window.show(ctx, |ui| {
            ui.set_min_width(280.0);
            ui.vertical_centered(|ui| {
                ui.add_space(5.0);
                ui.label(content); 
                
                if let Some(size) = &state.size_str {
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new(format!("Folder size: {}", size)).color(ui.visuals().weak_text_color()));
                }

                ui.add_space(15.0);
                
                ui.horizontal(|ui| {
                    let total_width = 130.0;
                    let x_offset = (ui.available_width() - total_width) / 2.0;
                    ui.add_space(x_offset);

                    if ui.add_sized([60.0, 30.0], egui::Button::new("Yes")).clicked() {
                        yes_clicked = true;
                        should_close = true;
                    }
                    
                    ui.add_space(10.0);

                    if ui.add_sized([60.0, 30.0], egui::Button::new("No")).clicked() {
                        should_close = true;
                    }
                });
                ui.add_space(5.0);
            });
        });
        
        if should_close {
            state.is_open = false;
        }
        
        ctx.data_mut(|d| d.insert_temp(state_id, state));
    }
    
    yes_clicked
}

pub fn show(ui: &mut egui::Ui, settings: &mut GameDataSettings, runtime: &mut RuntimeState, drag_guard: &mut DragGuard) -> bool {
    let mut refresh_needed = false;
    let ctx = ui.ctx().clone();

    let mut game_deleter = ctx.data_mut(|d| d.get_temp::<FolderDeleter>(egui::Id::new("game_deleter")).unwrap_or_default());
    let mut raw_deleter = ctx.data_mut(|d| d.get_temp::<FolderDeleter>(egui::Id::new("raw_deleter")).unwrap_or_default());
    let mut cache_deleter = ctx.data_mut(|d| d.get_temp::<FolderDeleter>(egui::Id::new("cache_deleter")).unwrap_or_default());

    game_deleter.update();
    raw_deleter.update();
    cache_deleter.update();

    if game_deleter.is_active() || raw_deleter.is_active() || cache_deleter.is_active() {
        ctx.request_repaint();
    }

    let game_exists = Path::new("game").exists();
    let raw_exists = Path::new("game/raw").exists();
    
    let cache_dir_opt = crate::global::io::cache::get_cache_dir();
    let cache_size = cache_dir_opt.as_ref().map(|path| get_folder_size(path)).unwrap_or(0);
    let cache_exists = cache_size > 0;

    egui::ScrollArea::vertical()
        .id_salt("game_data_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {

            ui.heading("Disk");
            ui.add_space(5.0);

            if game_deleter.is_deleting() {
                let btn = egui::Button::new("Deleting \"game\" Folder...")
                    .fill(egui::Color32::from_rgb(200, 180, 50)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if game_deleter.is_done() {
                let btn = egui::Button::new("Deleted \"game\" Folder!")
                    .fill(egui::Color32::from_rgb(40, 160, 40)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if game_exists {
                let btn = egui::Button::new("Delete \"game\" Folder")
                    .fill(egui::Color32::from_rgb(180, 50, 50)); 
                if ui.add_sized([180.0, 30.0], btn).clicked() {
                    let state_id = egui::Id::new("delete_game_modal");
                    ctx.data_mut(|d| d.insert_temp(state_id, FolderDeleteState { is_open: true, size_str: None }));
                }
            } else {
                let btn = egui::Button::new("No \"game\" Folder")
                    .fill(egui::Color32::from_rgb(60, 60, 60)); 
                ui.add_sized([180.0, 30.0], btn);
            }

            ui.add_space(5.0);

            if raw_deleter.is_deleting() {
                let btn = egui::Button::new("Deleting \"raw\" Folder...")
                    .fill(egui::Color32::from_rgb(200, 180, 50)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if raw_deleter.is_done() {
                let btn = egui::Button::new("Deleted \"raw\" Folder!")
                    .fill(egui::Color32::from_rgb(40, 160, 40)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if raw_exists {
                let btn = egui::Button::new("Delete \"raw\" Folder")
                    .fill(egui::Color32::from_rgb(180, 50, 50)); 
                if ui.add_sized([180.0, 30.0], btn).clicked() {
                    let size = get_folder_size(Path::new("game/raw"));
                    let state_id = egui::Id::new("delete_raw_modal");
                    ctx.data_mut(|d| d.insert_temp(state_id, FolderDeleteState { 
                        is_open: true, 
                        size_str: Some(format_size(size)) 
                    }));
                }
            } else {
                let btn = egui::Button::new("No \"raw\" Folder")
                    .fill(egui::Color32::from_rgb(60, 60, 60)); 
                ui.add_sized([180.0, 30.0], btn);
            }

            ui.add_space(5.0);

            if cache_deleter.is_deleting() {
                let btn = egui::Button::new("Clearing Cache...")
                    .fill(egui::Color32::from_rgb(200, 180, 50)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if cache_deleter.is_done() {
                let btn = egui::Button::new("Cleared Cache!")
                    .fill(egui::Color32::from_rgb(40, 160, 40)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if cache_exists {
                let btn = egui::Button::new("Clear Cache")
                    .fill(egui::Color32::from_rgb(180, 50, 50)); 
                if ui.add_sized([180.0, 30.0], btn).clicked() {
                    let state_id = egui::Id::new("delete_cache_modal");
                    ctx.data_mut(|d| d.insert_temp(state_id, FolderDeleteState { 
                        is_open: true, 
                        size_str: Some(format_size(cache_size)) 
                    }));
                }
            } else {
                let btn = egui::Button::new("Cache Empty")
                    .fill(egui::Color32::from_rgb(60, 60, 60)); 
                ui.add_sized([180.0, 30.0], btn);
            }

            ui.add_space(20.0);
            ui.heading("Android");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let tooltip = "Attempt to connect to this IP Address Wirelessly if not automatically found when using Android import method\nMake sure you have \"Wireless USB Debugging\" enabled in your devices developer settings\nRequires ABD OEM Drivers Add-On to function";
                
                ui.label("Fallback IP Address").on_hover_text(tooltip);
                ui.spacing_mut().item_spacing.x = 4.0; 

                ui.allocate_ui(egui::vec2(100.0, 20.0), |ui| {
                    ui.centered_and_justified(|ui| {
                        if runtime.show_ip_field {
                            let hint = egui::RichText::new("192.168.X.X").color(egui::Color32::GRAY);
                            ui.add(egui::TextEdit::singleline(&mut settings.manual_ip)
                                .hint_text(hint)
                                .vertical_align(egui::Align::Center))
                                .on_hover_text(tooltip); 
                        } else {
                            if ui.button("Click to Reveal").on_hover_text(tooltip).clicked() {
                                runtime.show_ip_field = true;
                            }
                        }
                    });
                });

                ui.add_space(2.0);

                if ui.button("👁").on_hover_text("Toggle Visibility").clicked() {
                    runtime.show_ip_field = !runtime.show_ip_field;
                }
            });
            
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let label_response = ui.label("App Folder Persistence");
                let tooltip_text = "Skip the deletion of the \"game/app\" directory after android import";
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(ui, &mut settings.app_folder_persistence).on_hover_text(tooltip_text);
                if toggle_response.changed() { refresh_needed = true; }
            });

            ui.add_space(20.0);
            ui.heading("Import");
            ui.add_space(5.0);

            let keys_btn = egui::Button::new("Manage Keys")
                    .fill(egui::Color32::from_rgb(40, 90, 160));
                
            if ui.add_sized([180.0, 30.0], keys_btn).clicked() {
                crate::features::settings::ui::keys::open(&ctx);
            }

            let import_btn = egui::Button::new("Manage Exceptions")
                .fill(egui::Color32::from_rgb(40, 90, 160));
            
            if ui.add_sized([180.0, 30.0], import_btn).clicked() {
                crate::features::settings::ui::exceptions::open(&ctx);
            }

            ui.add_space(20.0);
            ui.heading("Export");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let label_response = ui.label("Enable Ultra Compression");
                let tooltip_text = "Allows compression levels up to 21\nWARNING: Levels above 15 require significant RAM and time";
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(ui, &mut settings.enable_ultra_compression).on_hover_text(tooltip_text);

                if toggle_response.changed() {
                    refresh_needed = true;
                    if !settings.enable_ultra_compression && settings.last_compression_level > 15 {
                        settings.last_compression_level = 15;
                    }
                }
            });
    });

    if show_folder_delete_modal(&ctx, drag_guard, "delete_game_modal", "Are you sure you want to delete the \"game\" folder?\nMost app function will be lost.") {
        game_deleter.start("game");
    }

    if show_folder_delete_modal(&ctx, drag_guard, "delete_raw_modal", "Are you sure you want to delete the \"raw\" folder?\nYou may need to import again if an app update requires new game assets.") {
        raw_deleter.start("game/raw");
    }

    if show_folder_delete_modal(&ctx, drag_guard, "delete_cache_modal", "Are you sure you want to clear the Cache?\nIt will automatically rebuild the next time the app loads.") {
        if let Some(cache_directory) = crate::global::io::cache::get_cache_dir() {
            cache_deleter.start(cache_directory);
        }
    }

    crate::features::settings::ui::exceptions::show(&ctx, drag_guard);
    crate::features::settings::ui::keys::show(&ctx, drag_guard);

    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("game_deleter"), game_deleter);
        d.insert_temp(egui::Id::new("raw_deleter"), raw_deleter);
        d.insert_temp(egui::Id::new("cache_deleter"), cache_deleter);
    });

    refresh_needed
}