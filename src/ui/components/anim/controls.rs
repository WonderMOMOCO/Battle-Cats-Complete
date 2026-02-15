use eframe::egui;
use std::path::PathBuf;
use crate::ui::components::anim::viewer::AnimViewer;

const TILE_HEIGHT: f32 = 28.0; 
const GAP: f32 = 4.0;
const OVERLAY_BOTTOM_OFFSET: f32 = 35.0; 
pub const CONTROLS_SLIDE_DISTANCE: f32 = 180.0;
const ICON_W: f32 = 60.0;
const COL2_W: f32 = 148.0; 
const NAV_W: f32 = 30.0;
const INPUT_W: f32 = 80.0; 
const COL3_W: f32 = 100.0;

// Animation Indices
pub const IDX_WALK: usize = 0;
pub const IDX_IDLE: usize = 1;
pub const IDX_ATTACK: usize = 2;
pub const IDX_KB: usize = 3;
pub const IDX_SPIRIT: usize = 4;
pub const IDX_BURROW: usize = 5;
pub const IDX_SURFACE: usize = 6;
pub const IDX_MODEL: usize = 99;
pub const IDX_NONE: usize = 999; 

pub fn render_controls_overlay(
    ui: &mut egui::Ui,
    rect: egui::Rect, 
    anim_viewer: &mut AnimViewer,
    available_anims: &Vec<(usize, &str, PathBuf)>,
    spirit_available: bool,
    base_assets_available: bool,
    is_loading_new: bool,
    spirit_sheet_id: &str,
    form_viewer_id: &str,
    spirit_pack: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    interpolation: bool, 
    native_fps: f32,
) -> bool {
    let mut clip_rect = rect;
    clip_rect = clip_rect.shrink(2.0); 
    ui.set_clip_rect(clip_rect);

    let target_slide = if anim_viewer.is_controls_expanded { 0.0 } else { 1.0 };
    
    let anim_id = egui::Id::new("controls_slide");
    let slide_factor = ui.ctx().animate_value_with_time(anim_id, target_slide, 0.35);
    
    let current_offset = CONTROLS_SLIDE_DISTANCE * slide_factor;
    let bottom_margin = 5.0 + OVERLAY_BOTTOM_OFFSET - current_offset;

    let builder = egui::UiBuilder::new()
        .max_rect(clip_rect)
        .layout(egui::Layout::bottom_up(egui::Align::Min));
    
    let res = ui.allocate_new_ui(builder, |ui| {
        egui::Frame::window(ui.style())
            .fill(egui::Color32::from_black_alpha(160)) 
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
            .inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 8.0, bottom: 18.0 })
            .outer_margin(egui::Margin { left: 5.0, bottom: bottom_margin, ..Default::default() })
            .rounding(8.0)
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    render_internal_ui(ui, anim_viewer, available_anims, spirit_available, base_assets_available, is_loading_new, spirit_sheet_id, form_viewer_id, spirit_pack, interpolation, native_fps);
                    let width_to_use = if anim_viewer.cached_controls_width > 1.0 { anim_viewer.cached_controls_width } else { ui.available_width() };
                    ui.add_sized(egui::vec2(width_to_use, 1.0), egui::Separator::default().horizontal());
                    let icon = if anim_viewer.is_controls_expanded { "▼" } else { "▲" };
                    let btn = egui::Button::new(egui::RichText::new(icon).strong().size(14.0)).fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE);
                    if ui.add_sized(egui::vec2(width_to_use, 18.0), btn).clicked() { anim_viewer.is_controls_expanded = !anim_viewer.is_controls_expanded; }
                });
            })
    });

    if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
        if res.inner.response.rect.contains(pointer_pos) { return true; }
    }
    
    false
}

fn render_internal_ui(
    ui: &mut egui::Ui,
    anim_viewer: &mut AnimViewer,
    available_anims: &Vec<(usize, &str, PathBuf)>,
    spirit_available: bool,
    base_assets_available: bool,
    is_loading_new: bool,
    spirit_sheet_id: &str,
    form_viewer_id: &str,
    spirit_pack: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    interpolation: bool,
    native_fps: f32,
) {
    let mut clicked_index: Option<usize> = None;
    let active_color = egui::Color32::from_rgb(31, 106, 165);
    let inactive_color = egui::Color32::from_gray(60);
    
    let btn_w = 70.0;
    let grid_gap = 5.0;
    let btn_size = egui::vec2(btn_w, 25.0);

    let display_multiplier = if interpolation { native_fps / 30.0 } else { 1.0 };

    let (lcm_result, max_frame_val) = if let Some(anim) = &anim_viewer.current_anim {
        if anim_viewer.loaded_anim_index <= 1 { 
            let res = anim.calculate_true_loop();
            (res, res.unwrap_or(0)) 
        } else { 
            (Some(anim.max_frame), anim.max_frame) 
        }
    } else { 
        (Some(0), 0) 
    };
    
    let display_max_str = match lcm_result {
        Some(v) if v > 999_999 => "???".to_string(), 
        Some(v) => ((v as f32 * display_multiplier).round() as i32).to_string(),
        None => "???".to_string()
    };

    let cur_frame_val = anim_viewer.current_frame;
    let loop_range_0 = anim_viewer.loop_range.0;
    let loop_range_1 = anim_viewer.loop_range.1;
    let is_playing = anim_viewer.is_playing;
    let is_model_mode = anim_viewer.loaded_anim_index == IDX_MODEL;
    
    let cur_display_val = (cur_frame_val * display_multiplier).round() as i32;
    
    let effective_display_max = if is_model_mode {
        "0".to_string()
    } else if let Some(override_end) = loop_range_1 {
        ((override_end as f32 * display_multiplier).round() as i32).to_string()
    } else {
        display_max_str.clone()
    };
    
    let tile_frame = egui::Frame::none()
        .fill(egui::Color32::from_gray(40))
        .rounding(4.0)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
        .inner_margin(0.0);

    let controls_response = ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = GAP;

        // Column 1
        ui.vertical(|ui| {
            let play_icon = if is_playing { "⏸" } else { "▶" };
            let enabled = anim_viewer.loaded_anim_index != IDX_NONE && base_assets_available;
            
            if ui.add_enabled_ui(enabled, |ui| {
                ui.add_sized(egui::vec2(ICON_W, TILE_HEIGHT), egui::Button::new(egui::RichText::new(play_icon).size(16.0)))
            }).inner.clicked() {
                anim_viewer.is_playing = !anim_viewer.is_playing;
            }
            
            ui.add_space(GAP);
            
            if ui.add_enabled_ui(base_assets_available, |ui| {
                ui.add_sized(egui::vec2(ICON_W, TILE_HEIGHT), egui::Button::new("Orient"))
            }).inner.clicked() { 
                anim_viewer.pan_offset = egui::Vec2::ZERO; 
            }
        });

        ui.add_sized(egui::vec2(10.0, (TILE_HEIGHT * 2.0) + GAP), egui::Separator::default().vertical());

        // Column 2
        ui.vertical(|ui| {
            ui.allocate_ui(egui::vec2(COL2_W, TILE_HEIGHT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    
                    let enabled = anim_viewer.loaded_anim_index != IDX_NONE && base_assets_available;

                    if !is_playing {
                        let left = ui.add_enabled_ui(enabled, |ui| {
                            ui.add_sized(egui::vec2(NAV_W, TILE_HEIGHT), egui::Button::new("◀").sense(egui::Sense::click().union(egui::Sense::drag())))
                        }).inner;
                        
                        tile_frame.show(ui, |ui| {
                            ui.set_width(INPUT_W); ui.set_height(TILE_HEIGHT);
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                let re = ui.add_enabled(enabled, egui::TextEdit::singleline(&mut anim_viewer.single_frame_str)
                                    .frame(false).desired_width(INPUT_W).vertical_align(egui::Align::Center).horizontal_align(egui::Align::Center));
                                if re.changed() {
                                    if let Ok(val) = anim_viewer.single_frame_str.parse::<i32>() {
                                        anim_viewer.current_frame = val as f32 / display_multiplier;
                                    }
                                }
                                if !re.has_focus() {
                                    anim_viewer.single_frame_str = format!("{}", cur_display_val);
                                }
                            });
                        });

                        let right = ui.add_enabled_ui(enabled, |ui| {
                            ui.add_sized(egui::vec2(NAV_W, TILE_HEIGHT), egui::Button::new("▶").sense(egui::Sense::click().union(egui::Sense::drag())))
                        }).inner;

                        if left.is_pointer_button_down_on() { anim_viewer.hold_dir = -1; } 
                        else if right.is_pointer_button_down_on() { anim_viewer.hold_dir = 1; } 
                        else { anim_viewer.hold_dir = 0; }
                        
                        if left.clicked() {
                            let f = cur_frame_val - 1.0;
                            let wrap = if lcm_result.is_some() { max_frame_val as f32 } else { 0.0 }; 
                            anim_viewer.current_frame = if f < 0.0 { wrap } else { f };
                        }
                        if right.clicked() {
                            let f = cur_frame_val + 1.0;
                            if let Some(mx) = lcm_result {
                                anim_viewer.current_frame = if f > mx as f32 { 0.0 } else { f };
                            } else {
                                anim_viewer.current_frame = f;
                            }
                        }
                    } else {
                        // Range Controls
                        tile_frame.show(ui, |ui| {
                            ui.set_width(60.0); ui.set_height(TILE_HEIGHT);
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                if loop_range_0.is_some() && anim_viewer.range_str_cache.0.is_empty() {
                                    let v = (loop_range_0.unwrap() as f32 * display_multiplier).round() as i32;
                                    anim_viewer.range_str_cache.0 = v.to_string();
                                }
                                let re = ui.add_enabled(enabled, egui::TextEdit::singleline(&mut anim_viewer.range_str_cache.0)
                                    .hint_text(egui::RichText::new("0").color(egui::Color32::GRAY)).frame(false).desired_width(60.0).vertical_align(egui::Align::Center).horizontal_align(egui::Align::Center));
                                if re.changed() {
                                    if anim_viewer.range_str_cache.0.is_empty() { anim_viewer.loop_range.0 = None; } 
                                    else if let Ok(val) = anim_viewer.range_str_cache.0.parse::<i32>() {
                                        anim_viewer.loop_range.0 = Some((val as f32 / display_multiplier).round() as i32);
                                    }
                                }
                                if re.secondary_clicked() { anim_viewer.loop_range.0 = None; anim_viewer.range_str_cache.0.clear(); }
                            });
                        });
                        tile_frame.show(ui, |ui| { ui.set_width(20.0); ui.set_height(TILE_HEIGHT); ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| { ui.label("~"); }); });
                        tile_frame.show(ui, |ui| {
                            ui.set_width(60.0); ui.set_height(TILE_HEIGHT);
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                if loop_range_1.is_some() && anim_viewer.range_str_cache.1.is_empty() {
                                    let v = (loop_range_1.unwrap() as f32 * display_multiplier).round() as i32;
                                    anim_viewer.range_str_cache.1 = v.to_string();
                                }
                                let re = ui.add_enabled(enabled, egui::TextEdit::singleline(&mut anim_viewer.range_str_cache.1)
                                    .hint_text(egui::RichText::new(&display_max_str).color(egui::Color32::GRAY)).frame(false).desired_width(60.0).vertical_align(egui::Align::Center).horizontal_align(egui::Align::Center));
                                if re.changed() {
                                    if anim_viewer.range_str_cache.1.is_empty() { anim_viewer.loop_range.1 = None; } 
                                    else if let Ok(val) = anim_viewer.range_str_cache.1.parse::<i32>() {
                                        anim_viewer.loop_range.1 = Some((val as f32 / display_multiplier).round() as i32);
                                    }
                                }
                                if re.secondary_clicked() { anim_viewer.loop_range.1 = None; anim_viewer.range_str_cache.1.clear(); }
                            });
                        });
                    }
                });
            });

            ui.add_space(GAP);

            // Info Row
            ui.allocate_ui(egui::vec2(COL2_W, TILE_HEIGHT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    tile_frame.show(ui, |ui| { ui.set_width(60.0); ui.set_height(TILE_HEIGHT); ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| { ui.label(egui::RichText::new(format!("{}", cur_display_val)).color(egui::Color32::WHITE)); }); });
                    tile_frame.show(ui, |ui| { ui.set_width(20.0); ui.set_height(TILE_HEIGHT); ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| { ui.label("/"); }); });
                    tile_frame.show(ui, |ui| { ui.set_width(60.0); ui.set_height(TILE_HEIGHT); ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| { ui.label(egui::RichText::new(&effective_display_max).color(egui::Color32::WHITE)); }); });
                });
            });
        });

        ui.add_sized(egui::vec2(10.0, (TILE_HEIGHT * 2.0) + GAP), egui::Separator::default().vertical());

        // Column 3
        ui.vertical(|ui| {
            // EXPORT BUTTON LOGIC
            let btn_resp = ui.add_enabled_ui(base_assets_available, |ui| {
                ui.add_sized(egui::vec2(COL3_W, TILE_HEIGHT), egui::Button::new("Export"))
            }).inner;
            
            if btn_resp.clicked() { 
                anim_viewer.show_export_popup = true;
            }

            ui.add_space(GAP);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                tile_frame.show(ui, |ui| { ui.set_width(50.0); ui.set_height(TILE_HEIGHT); ui.centered_and_justified(|ui| ui.label("Speed")); });
                tile_frame.show(ui, |ui| {
                    ui.set_width(COL3_W - 50.0 - GAP); ui.set_height(TILE_HEIGHT);
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                        ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                        
                        let re = ui.add_enabled(base_assets_available, egui::TextEdit::singleline(&mut anim_viewer.speed_str)
                            .hint_text(egui::RichText::new("1.0").color(egui::Color32::GRAY))
                            .frame(false)
                            .desired_width(40.0)
                            .vertical_align(egui::Align::Center)
                            .horizontal_align(egui::Align::Center));
                        
                        if re.changed() {
                            if anim_viewer.speed_str.is_empty() {
                                anim_viewer.playback_speed = 1.0;
                            } else if let Ok(val) = anim_viewer.speed_str.parse::<f32>() {
                                anim_viewer.playback_speed = val;
                            }
                        }
                    });
                });
            });
        });
    });

    let actual_width = controls_response.response.rect.width();
    if (anim_viewer.cached_controls_width - actual_width).abs() > 0.1 {
        anim_viewer.cached_controls_width = actual_width;
        ui.ctx().request_repaint();
    }

    ui.add_sized(egui::vec2(actual_width, 1.0), egui::Separator::default().horizontal());

    let top_row_w = (btn_w * 4.0) + (grid_gap * 3.0);
    let left_pad = if actual_width > top_row_w { (actual_width - top_row_w) / 2.0 } else { 0.0 };

    let grid_alloc = ui.allocate_ui(egui::vec2(ui.available_width(), anim_viewer.cached_grid_height), |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(left_pad);
                egui::Grid::new("anim_controls_grid").spacing(egui::vec2(grid_gap, grid_gap)).show(ui, |ui| {
                    let mut draw_anim_btn = |ui: &mut egui::Ui, label: &str, idx: usize, anim_exists: bool| {
                        let effective_enabled = base_assets_available && anim_exists;
                        let is_active = anim_viewer.loaded_anim_index == idx && anim_viewer.loaded_anim_index != IDX_NONE;
                        
                        let fill = if is_active { active_color } else { inactive_color };
                        let btn = egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE).size(13.0)).fill(fill);
                        
                        if ui.add_enabled_ui(effective_enabled, |ui| { ui.add_sized(btn_size, btn) }).inner.clicked() { 
                            clicked_index = Some(idx); 
                        }
                    };

                    let has_walk = available_anims.iter().any(|(i, _, _)| *i == IDX_WALK); draw_anim_btn(ui, "Walk", IDX_WALK, has_walk);
                    let has_idle = available_anims.iter().any(|(i, _, _)| *i == IDX_IDLE); draw_anim_btn(ui, "Idle", IDX_IDLE, has_idle);
                    let has_atk = available_anims.iter().any(|(i, _, _)| *i == IDX_ATTACK); draw_anim_btn(ui, "Attack", IDX_ATTACK, has_atk);
                    let has_kb = available_anims.iter().any(|(i, _, _)| *i == IDX_KB); draw_anim_btn(ui, "Knockback", IDX_KB, has_kb);
                    ui.end_row();

                    let has_burrow = available_anims.iter().any(|(i, _, _)| *i == IDX_BURROW); draw_anim_btn(ui, "Burrow", IDX_BURROW, has_burrow);
                    let has_surface = available_anims.iter().any(|(i, _, _)| *i == IDX_SURFACE); draw_anim_btn(ui, "Surface", IDX_SURFACE, has_surface);
                    draw_anim_btn(ui, "Spirit", IDX_SPIRIT, spirit_available);
                    draw_anim_btn(ui, "Model", IDX_MODEL, base_assets_available);
                    ui.end_row();
                });
            });
        });
    });

    let actual_grid_height = grid_alloc.response.rect.height();
    if (anim_viewer.cached_grid_height - actual_grid_height).abs() > 0.1 {
        anim_viewer.cached_grid_height = actual_grid_height;
        ui.ctx().request_repaint();
    }

    if let Some(target_idx) = clicked_index {
        if !is_loading_new {
            anim_viewer.loaded_anim_index = target_idx;
            let intended_target_id = if target_idx == IDX_SPIRIT { spirit_sheet_id.to_string() } else { form_viewer_id.to_string() };
            if anim_viewer.loaded_id == intended_target_id {
                let anim_path = if target_idx == IDX_SPIRIT { spirit_pack.as_ref().map(|(_, _, _, a)| a) }
                else { available_anims.iter().find(|(i, _, _)| *i == target_idx).map(|(_, _, p)| p) };
                if let Some(a_path) = anim_path { 
                    anim_viewer.load_anim(a_path); 
                } else if target_idx == IDX_MODEL { 
                    anim_viewer.current_anim = None; 
                    anim_viewer.current_frame = 0.0;
                    anim_viewer.single_frame_str = "0".to_string();

                    // FIX: Cleared strings instead of setting them to "0"
                    anim_viewer.export_state.name_prefix = format!("{}.model", form_viewer_id);
                    anim_viewer.export_state.anim_name = "Model".to_string();
                    anim_viewer.export_state.max_frame = 0;
                    anim_viewer.export_state.frame_start = 0;
                    anim_viewer.export_state.frame_start_str = String::new(); // FIXED
                    anim_viewer.export_state.frame_end = 0;
                    anim_viewer.export_state.frame_end_str = String::new(); // FIXED
                }
            }
        }
    }
}