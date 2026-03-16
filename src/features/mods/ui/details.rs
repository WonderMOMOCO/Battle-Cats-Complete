use eframe::egui;
use std::path::Path;
use crate::features::settings::logic::Settings;
use crate::features::mods::logic::state::ModState;
use crate::global::ui::shared::DragGuard;
use crate::features::mods::logic::manager;

const HEADER_BOTTOM_PADDING: f32 = 4.0; 
const HEADER_TOP_PADDING: f32 = 3.8;
const MOD_TITLE_SIZE: f32 = 25.0;
const BTN_SIZE: [f32; 2] = [115.0, 28.0]; 

const META_LEFT_HEADER_OFFSET: f32 = 38.0;
const META_LEFT_TOP_PADDING: f32 = 7.0;
const META_LEFT_INNER_PADDING: f32 = 3.0;
const META_LEFT_BOTTOM_PADDING: f32 = 0.0;

#[derive(Clone, Default)]
struct ActionState {
    is_open: bool,
    completion_time: Option<f64>,
}

pub fn render(ui: &mut egui::Ui, state: &mut ModState, _settings: &mut Settings) {
    let Some(selected_id) = state.selected_mod.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Please select or import a Mod").weak());
        });
        return;
    };

    let Some(mod_idx) = state.loaded_mods.iter().position(|m| m.folder_name == selected_id) else { return; };
    
    let tracking_id = egui::Id::new("details_tracking_mod");
    let last_viewed = ui.ctx().data(|d| d.get_temp::<String>(tracking_id)).unwrap_or_default();
    
    if last_viewed != selected_id {
        state.rename_buffer = selected_id.clone();
        ui.ctx().data_mut(|d| d.insert_temp(tracking_id, selected_id.clone()));
    }

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        
        ui.add_space(HEADER_TOP_PADDING);

        let header_response = ui.add(
            egui::TextEdit::singleline(&mut state.rename_buffer)
                .font(egui::FontId::proportional(MOD_TITLE_SIZE))
                .frame(false)
                .horizontal_align(egui::Align::Center)
                .desired_width(ui.available_width())
        );

        if header_response.lost_focus() && state.rename_buffer != state.loaded_mods[mod_idx].folder_name {
            let old_name = state.loaded_mods[mod_idx].folder_name.clone();
            let new_name = state.rename_buffer.clone();
            
            if !new_name.is_empty() {
                let old_path = Path::new("mods").join(&old_name);
                let new_path = Path::new("mods").join(&new_name);
                
                if !new_path.exists() && old_path.exists() && std::fs::rename(&old_path, &new_path).is_ok() {
                    if state.loaded_mods[mod_idx].enabled {
                        crate::global::resolver::set_active_mod(Some(new_name.clone()));
                    }
                    state.loaded_mods[mod_idx].folder_name = new_name.clone();
                    state.selected_mod = Some(new_name.clone());
                    ui.ctx().data_mut(|d| d.insert_temp(tracking_id, new_name.clone())); 
                    
                    state.loaded_mods[mod_idx].metadata.title = new_name.clone();
                    let _ = state.loaded_mods[mod_idx].metadata.save(&new_path);
                } else {
                    state.rename_buffer = old_name;
                }
            } else {
                state.rename_buffer = old_name;
            }
        }

        ui.add_space(HEADER_BOTTOM_PADDING);
        ui.separator();
    });

    ui.add_space(5.0);

    let mod_folder = state.loaded_mods[mod_idx].folder_name.clone();
    let mod_path = Path::new("mods").join(&mod_folder);
    let is_enabled = state.loaded_mods[mod_idx].enabled;
    let has_icon = mod_path.join("icon.png").exists() || mod_path.join("icon.ico").exists();

    let mut toggle_clicked = false;
    render_action_buttons(ui, state, &mod_path, has_icon, is_enabled, &mod_folder, &mut toggle_clicked);

    if state.selected_mod.is_none() {
        return; 
    }

    ui.add_space(5.0);
    ui.separator();
    ui.add_space(5.0);

    let mut metadata_changed = false;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(160.0); 

            ui.horizontal(|ui| {
                ui.add_space(META_LEFT_HEADER_OFFSET);
                ui.label(egui::RichText::new("Information").heading().strong());
            });
            
            ui.add_space(META_LEFT_TOP_PADDING);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Author:").strong());
                if ui.add(egui::TextEdit::singleline(&mut state.loaded_mods[mod_idx].metadata.author)
                    .desired_width(100.0)).lost_focus() {
                    metadata_changed = true;
                }
            });

            ui.add_space(META_LEFT_INNER_PADDING); 

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Version:").strong());
                if ui.add(egui::TextEdit::singleline(&mut state.loaded_mods[mod_idx].metadata.version)
                    .desired_width(100.0)).lost_focus() {
                    metadata_changed = true;
                }
            });
            
            ui.add_space(META_LEFT_BOTTOM_PADDING); 
        });

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);

        ui.vertical(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Description").heading().strong());
            });
            
            ui.add_space(2.0);
            
            let desc_hint = egui::RichText::new("Enter mod description here...").color(egui::Color32::GRAY);
            
            if ui.add(
                egui::TextEdit::multiline(&mut state.loaded_mods[mod_idx].metadata.description)
                    .hint_text(desc_hint)
                    .desired_width(ui.available_width())
                    .desired_rows(4) 
            ).lost_focus() {
                metadata_changed = true;
            }
        });
    });

    ui.add_space(5.0);
    ui.separator();

    if metadata_changed {
        state.loaded_mods[mod_idx].metadata.title = state.loaded_mods[mod_idx].folder_name.clone();
        let _ = state.loaded_mods[mod_idx].metadata.save(&mod_path);
    }

    if toggle_clicked {
        for m in state.loaded_mods.iter_mut() {
            m.enabled = false;
        }
        if !is_enabled {
            if let Some(m) = state.loaded_mods.iter_mut().find(|m| m.folder_name == mod_folder) {
                m.enabled = true;
            }
            crate::global::resolver::set_active_mod(Some(mod_folder));
        } else {
            crate::global::resolver::set_active_mod(None);
        }
        state.needs_rescan = true; 
    }
}

fn render_action_buttons(
    ui: &mut egui::Ui, 
    state: &mut ModState, 
    path: &Path, 
    has_icon: bool, 
    is_enabled: bool, 
    mod_name: &str, 
    toggle_clicked: &mut bool
) {
    let ctx = ui.ctx().clone();
    
    let icon_id = egui::Id::new("mod_icon_action_state");
    let mut icon_state = ctx.data(|d| d.get_temp::<ActionState>(icon_id)).unwrap_or_default();

    let del_id = egui::Id::new("mod_delete_action_state");
    let mut del_is_open = ctx.data(|d| d.get_temp::<bool>(del_id)).unwrap_or_default();

    let current_time = ctx.input(|i| i.time);

    if let Some(t) = icon_state.completion_time {
        if current_time > t { icon_state.completion_time = None; }
    }

    ui.horizontal(|ui| {
        let spacing = 5.0;
        let btn_w = BTN_SIZE[0];
        let total_w = (btn_w * 4.0) + (spacing * 3.0); 
        let available_w = ui.available_width();
        
        ui.add_space((available_w - total_w) / 2.0);
        ui.spacing_mut().item_spacing.x = spacing;

        let (enable_text, enable_color) = if is_enabled {
            ("Disable Mod", egui::Color32::from_rgb(180, 50, 50))
        } else {
            ("Enable Mod", egui::Color32::from_rgb(40, 160, 40))
        };
        if ui.add_sized(BTN_SIZE, egui::Button::new(enable_text).fill(enable_color)).clicked() {
            *toggle_clicked = true;
        }

        let folder_btn = egui::Button::new("Open Folder").fill(egui::Color32::from_rgb(30, 100, 180));
        if ui.add_sized(BTN_SIZE, folder_btn).clicked() {
            let _ = open::that(path);
        }

        if icon_state.completion_time.is_some() {
            let btn = egui::Button::new("Icon Deleted!").fill(egui::Color32::from_rgb(40, 160, 40));
            let _ = ui.add_sized(BTN_SIZE, btn); 
        } else if has_icon {
            let btn = egui::Button::new("Delete Icon").fill(egui::Color32::from_rgb(180, 50, 50));
            if ui.add_sized(BTN_SIZE, btn).clicked() { icon_state.is_open = true; }
        } else {
            let btn = egui::Button::new("Add Icon").fill(egui::Color32::from_rgb(40, 160, 40));
            if ui.add_sized(BTN_SIZE, btn).clicked() {
                manager::add_mod_icon(path);
            }
        }

        let btn = egui::Button::new("Delete Mod").fill(egui::Color32::from_rgb(180, 50, 50));
        if ui.add_sized(BTN_SIZE, btn).clicked() { del_is_open = true; }
    });

    let icon_msg = format!("Are you sure you want to delete the icon for {}?", mod_name);
    if show_confirmation_modal(&ctx, &mut state.drag_guard, "confirm_icon_delete", &icon_msg, &mut icon_state.is_open) {
        manager::delete_mod_icon(path);
        icon_state.completion_time = Some(current_time + 2.0);
    }

    let mod_msg = format!("Are you sure you want to completely delete {}?", mod_name);
    if show_confirmation_modal(&ctx, &mut state.drag_guard, "confirm_mod_delete", &mod_msg, &mut del_is_open) {
        manager::delete_mod_folder(path.to_path_buf());
        state.selected_mod = None;
        state.needs_rescan = true;
        del_is_open = false; 
    }

    ctx.data_mut(|d| {
        d.insert_temp(icon_id, icon_state);
        d.insert_temp(del_id, del_is_open);
    });
}

fn show_confirmation_modal(
    ctx: &egui::Context,
    drag_guard: &mut DragGuard,
    id_str: &str,
    content: &str,
    is_open: &mut bool,
) -> bool {
    if !*is_open { return false; }

    let mut yes_clicked = false;
    let mut should_close = false;
    
    let window_id = egui::Id::new(format!("{}_window", id_str));
    let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

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

    if should_close { *is_open = false; }
    
    yes_clicked
}