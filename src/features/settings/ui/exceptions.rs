use std::fs;
use std::path::Path;
use eframe::egui;

use crate::features::settings::logic::exceptions::{ExceptionRule, ExceptionList, RuleHandling};
use crate::global::ui::shared::DragGuard;
use super::tabs::toggle_ui;

const COL_PATTERN_WIDTH: f32 = 210.0;
const COL_EXT_WIDTH: f32 = 100.0;
const COL_HANDLING_WIDTH: f32 = 110.0;
const COL_LANG_WIDTH: f32 = 85.0;
const COL_LOCK_WIDTH: f32 = 40.0;
const COL_ACTION_WIDTH: f32 = 45.0;
const WINDOW_MAX_HEIGHT: f32 = 600.0;

#[derive(Clone, Default)]
struct ManageExceptionsState {
    is_open: bool,
    rules: Vec<ExceptionRule>,
}

#[derive(Clone, Default)]
struct ResetConfirmState {
    is_open: bool,
}

pub fn open(ctx: &egui::Context) {
    let state_id = egui::Id::new("manage_exceptions_state");
    let mut state = ctx.data(|d| d.get_temp::<ManageExceptionsState>(state_id)).unwrap_or_else(|| {
        ManageExceptionsState { 
            is_open: false, 
            rules: ExceptionList::load_or_default().rules
        }
    });
    state.is_open = true;
    ctx.data_mut(|d| d.insert_temp(state_id, state));
}

fn show_reset_confirm_modal(ctx: &egui::Context, drag_guard: &mut DragGuard) -> bool {
    let state_id = egui::Id::new("reset_rules_modal");
    let mut state = ctx.data(|d| d.get_temp::<ResetConfirmState>(state_id)).unwrap_or_default();
    
    if !state.is_open { return false; }
    
    let mut yes_clicked = false;
    let window_id = egui::Id::new("reset_rules_window");
    let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);
    let mut should_close = false;

    let mut window = egui::Window::new("Confirm Reset")
        .id(window_id)
        .collapsible(false)
        .resizable(false)
        .constrain(false)
        .movable(allow_drag)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center());

    if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

    window.show(ctx, |ui| {
        ui.set_min_width(280.0);
        ui.vertical_centered(|ui| {
            ui.add_space(5.0);
            ui.label("Are you sure you want to reset to default exception rules?\nYour custom rules will be lost");
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

    if should_close { state.is_open = false; }
    ctx.data_mut(|d| d.insert_temp(state_id, state));
    yes_clicked
}

pub fn show(ctx: &egui::Context, drag_guard: &mut DragGuard) {
    let state_id = egui::Id::new("manage_exceptions_state");
    let mut state = ctx.data(|d| d.get_temp::<ManageExceptionsState>(state_id)).unwrap_or_else(|| {
        ManageExceptionsState { 
            is_open: false, 
            rules: ExceptionList::load_or_default().rules
        }
    });

    if !state.is_open { return; }

    let window_id = egui::Id::new("manage_exceptions_window_compact");
    let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);
    let original_rules = state.rules.clone();
    let mut is_open = state.is_open;

    let mut window = egui::Window::new("Manage Exceptions")
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
        let btn_w = 120.0;
        let default_color = egui::Color32::from_rgb(31, 106, 165);
        let success_color = egui::Color32::from_rgb(40, 160, 60);
        let fail_color = egui::Color32::from_rgb(200, 40, 40);
        let danger_color = egui::Color32::from_rgb(180, 50, 50);
        let current_time = ui.input(|i| i.time);

        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                let table_width = COL_PATTERN_WIDTH + COL_EXT_WIDTH + COL_HANDLING_WIDTH + COL_LANG_WIDTH + COL_LOCK_WIDTH + COL_ACTION_WIDTH + (15.0 * 5.0);
                let spacing = ui.spacing().item_spacing.x;
                let total_btn_width = (btn_w * 4.0) + (spacing * 3.0); 
                
                let x_offset = (table_width - total_btn_width) / 2.0;
                ui.add_space(x_offset.max(0.0));

                if ui.add_sized([btn_w, btn_h], egui::Button::new(egui::RichText::new("Add Rule").size(12.0).strong().color(egui::Color32::WHITE)).fill(default_color).rounding(4.0)).clicked() {
                    state.rules.push(ExceptionRule::default());
                }

                let import_time = ctx.data(|d| d.get_temp::<f64>(egui::Id::new("exceptions_import_time"))).unwrap_or(-10.0);
                let import_res = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("exceptions_import_res"))).unwrap_or(false);
                let (import_text, import_color) = if (current_time - import_time) < 2.0 {
                    if import_res { ("Imported!", success_color) } else { ("Failed!", fail_color) }
                } else { ("Load List", default_color) };

                if ui.add_sized([btn_w, btn_h], egui::Button::new(egui::RichText::new(import_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(import_color).rounding(4.0)).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                        let success = match ExceptionList::load_from_file(&path) {
                            Ok(list) => {
                                state.rules = list.rules;
                                ExceptionList { rules: state.rules.clone() }.save();
                                true
                            },
                            Err(_) => false,
                        };
                        ctx.data_mut(|d| {
                            d.insert_temp(egui::Id::new("exceptions_import_time"), current_time);
                            d.insert_temp(egui::Id::new("exceptions_import_res"), success);
                        });
                    }
                }

                let export_time = ctx.data(|d| d.get_temp::<f64>(egui::Id::new("exceptions_export_time"))).unwrap_or(-10.0);
                let export_res = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("exceptions_export_res"))).unwrap_or(false);
                let (export_text, export_color) = if (current_time - export_time) < 2.0 {
                    if export_res { ("Exported!", success_color) } else { ("Failed!", fail_color) }
                } else { ("Export List", default_color) };

                if ui.add_sized([btn_w, btn_h], egui::Button::new(egui::RichText::new(export_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(export_color).rounding(4.0)).clicked() {
                    let export_dir = Path::new("exports");
                    let _ = fs::create_dir_all(export_dir);
                    let success = ExceptionList { rules: state.rules.clone() }.save_to_file(&export_dir.join("exceptions.json")).is_ok();
                    ctx.data_mut(|d| {
                        d.insert_temp(egui::Id::new("exceptions_export_time"), current_time);
                        d.insert_temp(egui::Id::new("exceptions_export_res"), success);
                    });
                }

                if ui.add_sized([btn_w, btn_h], egui::Button::new(egui::RichText::new("Reset to Default").size(12.0).strong().color(egui::Color32::WHITE)).fill(danger_color).rounding(4.0)).clicked() {
                    ctx.data_mut(|d| d.insert_temp(egui::Id::new("reset_rules_modal"), ResetConfirmState { is_open: true }));
                }
            });
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(5.0);

        egui::ScrollArea::vertical().max_height(WINDOW_MAX_HEIGHT).auto_shrink([true, true]).show(ui, |ui| {
            egui::Grid::new("exceptions_grid").striped(true).spacing(egui::vec2(15.0, 10.0)).show(ui, |ui| {
                ui.vertical_centered(|ui| { ui.set_min_width(COL_PATTERN_WIDTH); ui.label(egui::RichText::new("Stem").strong()); });
                ui.vertical_centered(|ui| { ui.set_min_width(COL_EXT_WIDTH); ui.label(egui::RichText::new("Extension").strong()); });
                ui.vertical_centered(|ui| { ui.set_min_width(COL_HANDLING_WIDTH); ui.label(egui::RichText::new("Handling").strong()); });
                ui.vertical_centered(|ui| { ui.set_min_width(COL_LANG_WIDTH); ui.label(egui::RichText::new("Languages").strong()); });
                ui.vertical_centered(|ui| { ui.set_min_width(COL_LOCK_WIDTH); ui.label(egui::RichText::new("Lock").strong()); });
                ui.vertical_centered(|ui| { ui.set_min_width(COL_ACTION_WIDTH); ui.label(egui::RichText::new("Delete").strong()); });
                ui.end_row();

                let mut row_to_delete = None;
                for (i, rule) in state.rules.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut rule.pattern).desired_width(COL_PATTERN_WIDTH));
                    ui.add(egui::TextEdit::singleline(&mut rule.extension).desired_width(COL_EXT_WIDTH));

                    // --- Manual Padding Centering ---
                    ui.horizontal(|ui| {
                        ui.set_min_width(COL_HANDLING_WIDTH);
                        let combo_width = 90.0;
                        let x_offset = (COL_HANDLING_WIDTH - combo_width) / 2.0;
                        ui.add_space(x_offset);
                        
                        egui::ComboBox::from_id_salt(format!("handling_{}", i))
                            .width(combo_width)
                            .selected_text(rule.handling.to_string())
                            .show_ui(ui, |ui| {
                                for option in RuleHandling::all() {
                                    ui.selectable_value(&mut rule.handling, option, option.to_string());
                                }
                            });
                    });

                    ui.vertical_centered(|ui| {
                        let active_count = rule.languages.values().filter(|&&v| v).count();
                        ui.menu_button(format!("Manage ({})", active_count), |ui| {
                            egui::Grid::new(format!("lang_popup_grid_{}", i)).num_columns(2).spacing(egui::vec2(10.0, 5.0)).show(ui, |ui| {
                                for &(lang_code, _) in crate::global::io::patterns::APP_LANGUAGES {
                                    if let Some(enabled) = rule.languages.get_mut(lang_code) {
                                        ui.label(lang_code.to_uppercase());
                                        toggle_ui(ui, enabled); 
                                        ui.end_row(); 
                                    }
                                }
                            });
                        });
                    });

                    ui.vertical_centered(|ui| {
                        ui.set_min_width(COL_LOCK_WIDTH);
                        toggle_ui(ui, &mut rule.locked);
                    });

                    ui.vertical_centered(|ui| {
                        if ui.button("🗑").on_hover_text("Delete Rule").clicked() { row_to_delete = Some(i); }
                    });
                    ui.end_row();
                }

                if let Some(idx) = row_to_delete { state.rules.remove(idx); }
            });
        });
    });

    if show_reset_confirm_modal(ctx, drag_guard) {
        state.rules = ExceptionList::default().rules;
        ExceptionList { rules: state.rules.clone() }.save();
    }

    if state.rules != original_rules || (state.is_open && !is_open) {
        ExceptionList { rules: state.rules.clone() }.save();
    }

    state.is_open = is_open;
    ctx.data_mut(|d| d.insert_temp(state_id, state));
}