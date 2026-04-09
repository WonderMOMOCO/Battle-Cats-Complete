use std::fs;
use std::path::Path;
use eframe::egui;

use crate::features::data::utilities::crypto::UserKeys;
use crate::global::ui::shared::DragGuard;

const COL_REGION_WIDTH: f32 = 40.0;
const COL_INPUT_WIDTH: f32 = 250.0;

#[derive(Clone, Default)]
struct ManageKeysState {
    is_open: bool,
    keys: UserKeys,
}

pub fn open(ctx: &egui::Context) {
    let state_id = egui::Id::new("manage_keys_state");
    let mut state = ctx.data(|d| d.get_temp::<ManageKeysState>(state_id)).unwrap_or_else(|| {
        ManageKeysState { 
            is_open: false, 
            keys: UserKeys::load()
        }
    });
    state.is_open = true;
    ctx.data_mut(|d| d.insert_temp(state_id, state));
}

pub fn show(ctx: &egui::Context, drag_guard: &mut DragGuard) {
    let state_id = egui::Id::new("manage_keys_state");
    let mut state = ctx.data(|d| d.get_temp::<ManageKeysState>(state_id)).unwrap_or_else(|| {
        ManageKeysState { 
            is_open: false, 
            keys: UserKeys::load()
        }
    });

    if !state.is_open { return; }

    let window_id = egui::Id::new("manage_keys_window");
    let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);
    let original_keys = state.keys.clone();
    let mut is_open = state.is_open;

    let mut window = egui::Window::new("Manage Decryption Keys")
        .id(window_id)
        .open(&mut is_open) 
        .collapsible(false)
        .resizable(false)
        .constrain(false)
        .movable(allow_drag)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center());

    if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

    window.show(ctx, |ui| {
        ui.add_space(10.0);

        let btn_h = 24.0;
        let btn_w = 140.0;
        let default_color = egui::Color32::from_rgb(31, 106, 165);
        let success_color = egui::Color32::from_rgb(40, 160, 60);
        let fail_color = egui::Color32::from_rgb(200, 40, 40);
        let danger_color = egui::Color32::from_rgb(180, 50, 50);
        let current_time = ui.input(|i| i.time);

        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                let table_width = COL_REGION_WIDTH + (COL_INPUT_WIDTH * 2.0) + (15.0 * 2.0);
                let spacing = ui.spacing().item_spacing.x;
                let total_btn_width = (btn_w * 3.0) + (spacing * 2.0); 
                
                let x_offset = (table_width - total_btn_width) / 2.0;
                ui.add_space(x_offset.max(0.0));

                // --- Load Keys ---
                let import_time = ctx.data(|d| d.get_temp::<f64>(egui::Id::new("keys_import_time"))).unwrap_or(-10.0);
                let import_res = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("keys_import_res"))).unwrap_or(false);
                let (import_text, import_color) = if (current_time - import_time) < 2.0 {
                    if import_res { ("Loaded!", success_color) } else { ("Failed!", fail_color) }
                } else { ("Load Keys", default_color) };

                if ui.add_sized([btn_w, btn_h], egui::Button::new(egui::RichText::new(import_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(import_color).rounding(4.0)).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let success = match fs::read_to_string(&path) {
                            Ok(data) => {
                                if let Ok(parsed_keys) = serde_json::from_str::<UserKeys>(&data) {
                                    state.keys = parsed_keys;
                                    state.keys.save();
                                    true
                                } else { false }
                            },
                            Err(_) => false,
                        };
                        ctx.data_mut(|d| {
                            d.insert_temp(egui::Id::new("keys_import_time"), current_time);
                            d.insert_temp(egui::Id::new("keys_import_res"), success);
                        });
                    }
                }

                // --- Export Keys ---
                let export_time = ctx.data(|d| d.get_temp::<f64>(egui::Id::new("keys_export_time"))).unwrap_or(-10.0);
                let export_res = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("keys_export_res"))).unwrap_or(false);
                let (export_text, export_color) = if (current_time - export_time) < 2.0 {
                    if export_res { ("Exported!", success_color) } else { ("Failed!", fail_color) }
                } else { ("Export Keys", default_color) };

                if ui.add_sized([btn_w, btn_h], egui::Button::new(egui::RichText::new(export_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(export_color).rounding(4.0)).clicked() {
                    let export_dir = Path::new("exports");
                    let _ = fs::create_dir_all(export_dir);
                    
                    let export_path = export_dir.join("keys");
                    let json_data = serde_json::to_string_pretty(&state.keys).unwrap_or_default();
                    let success = fs::write(&export_path, json_data).is_ok();
                    
                    ctx.data_mut(|d| {
                        d.insert_temp(egui::Id::new("keys_export_time"), current_time);
                        d.insert_temp(egui::Id::new("keys_export_res"), success);
                    });
                }

                // --- Delete Keys ---
                if ui.add_sized([btn_w, btn_h], egui::Button::new(egui::RichText::new("Delete Keys").size(12.0).strong().color(egui::Color32::WHITE)).fill(danger_color).rounding(4.0)).clicked() {
                    state.keys = UserKeys::default();
                    state.keys.save();
                }
            });
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(5.0);

        egui::Grid::new("keys_grid").striped(true).spacing(egui::vec2(15.0, 10.0)).show(ui, |ui| {
            ui.vertical_centered(|ui| { ui.set_min_width(COL_REGION_WIDTH); ui.label(egui::RichText::new("Region").strong()); });
            ui.vertical_centered(|ui| { ui.set_min_width(COL_INPUT_WIDTH); ui.label(egui::RichText::new("Decryption Key").strong()); });
            ui.vertical_centered(|ui| { ui.set_min_width(COL_INPUT_WIDTH); ui.label(egui::RichText::new("Initialization Vector").strong()); });
            ui.end_row();

            let regions = [
                ("JP", &mut state.keys.jp),
                ("EN", &mut state.keys.en),
                ("TW", &mut state.keys.tw),
                ("KR", &mut state.keys.kr),
            ];

            for (name, region_data) in regions {
                ui.vertical_centered(|ui| { ui.label(egui::RichText::new(name).strong()); });
                ui.add(egui::TextEdit::singleline(&mut region_data.key).desired_width(COL_INPUT_WIDTH));
                ui.add(egui::TextEdit::singleline(&mut region_data.iv).desired_width(COL_INPUT_WIDTH));
                ui.end_row();
            }
        });
    });

    if state.keys != original_keys || (state.is_open && !is_open) {
        state.keys.save();
    }

    state.is_open = is_open;
    ctx.data_mut(|d| d.insert_temp(state_id, state));
}