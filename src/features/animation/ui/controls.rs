use eframe::egui;
use std::path::PathBuf;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::features::settings::logic::state::Settings;

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
    available_anims: &[(usize, PathBuf)],
    base_assets_available: bool,
    is_loading_new: bool,
    secondary_id: &str,
    primary_id: &str,
    secondary_pack: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    interpolation: bool, 
    native_fps: f32,
    settings: &mut Settings,
) -> bool {
    let mut clip_rect = rect;
    clip_rect = clip_rect.shrink(2.0); 
    ui.set_clip_rect(clip_rect);

    let target_slide = if settings.animation.controls_expanded { 0.0 } else { 1.0 };
    
    let anim_id = egui::Id::new("controls_slide");
    let slide_factor = ui.ctx().animate_value_with_time(anim_id, target_slide, 0.35);
    
    let current_offset = CONTROLS_SLIDE_DISTANCE * slide_factor;
    let bottom_margin = 5.0 + OVERLAY_BOTTOM_OFFSET - current_offset;

    let builder = egui::UiBuilder::new()
        .max_rect(clip_rect)
        .layout(egui::Layout::bottom_up(egui::Align::Min));
    
    let overlay_response = ui.allocate_new_ui(builder, |ui| {
        egui::Frame::window(ui.style())
            .fill(egui::Color32::from_black_alpha(160)) 
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
            .inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 8.0, bottom: 18.0 })
            .outer_margin(egui::Margin { left: 5.0, bottom: bottom_margin, ..Default::default() })
            .rounding(8.0)
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    render_internal_ui(ui, anim_viewer, available_anims, base_assets_available, is_loading_new, secondary_id, primary_id, secondary_pack, interpolation, native_fps, settings);
                    let width_to_use = if anim_viewer.cached_controls_width > 1.0 { anim_viewer.cached_controls_width } else { ui.available_width() };
                    ui.add_sized(egui::vec2(width_to_use, 1.0), egui::Separator::default().horizontal());
                    
                    let icon_text = if settings.animation.controls_expanded { "▼" } else { "▲" };
                    let expand_button = egui::Button::new(egui::RichText::new(icon_text).strong().size(14.0)).fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE);
                    if ui.add_sized(egui::vec2(width_to_use, 18.0), expand_button).clicked() { 
                        settings.animation.controls_expanded = !settings.animation.controls_expanded; 
                    }
                });
            })
    });

    let Some(pointer_position) = ui.ctx().pointer_latest_pos() else { return false; };
    overlay_response.inner.response.rect.contains(pointer_position)
}

fn render_internal_ui(
    ui: &mut egui::Ui,
    anim_viewer: &mut AnimViewer,
    available_anims: &[(usize, PathBuf)],
    base_assets_available: bool,
    is_loading_new: bool,
    secondary_id: &str,
    primary_id: &str,
    secondary_pack: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    interpolation: bool,
    native_fps: f32,
    settings: &mut Settings,
) {
    let mut clicked_index: Option<usize> = None;
    let active_color = egui::Color32::from_rgb(31, 106, 165);
    let inactive_color = egui::Color32::from_gray(60);
    
    let button_width = 70.0;
    let grid_gap = 5.0;
    let button_size = egui::vec2(button_width, 25.0);
    let is_locked = anim_viewer.export_state.is_processing || anim_viewer.export_state.is_loop_searching;
    let display_multiplier = if interpolation { native_fps / 30.0 } else { 1.0 };

    let (loop_lcm_result, max_frame_value) = if let Some(animation) = &anim_viewer.current_anim {
        if anim_viewer.loaded_anim_index <= 1 { 
            let true_loop = animation.calculate_true_loop();
            (true_loop, true_loop.unwrap_or(0)) 
        } else { 
            (Some(animation.max_frame), animation.max_frame) 
        }
    } else { 
        (Some(0), 0) 
    };
    
    let display_max_string = match loop_lcm_result {
        Some(value) if value > 999_999 => "???".to_string(), 
        Some(value) => ((value as f32 * display_multiplier).round() as i32).to_string(),
        None => "???".to_string()
    };

    let current_frame_value = anim_viewer.current_frame;
    let loop_range_start = anim_viewer.loop_range.0;
    let loop_range_end = anim_viewer.loop_range.1;
    let is_playing = anim_viewer.is_playing;
    let is_model_mode = anim_viewer.loaded_anim_index == IDX_MODEL;
    
    let current_display_value = (current_frame_value * display_multiplier).round() as i32;
    
    let effective_display_max = if is_model_mode {
        "0".to_string()
    } else if let Some(override_end) = loop_range_end {
        ((override_end as f32 * display_multiplier).round() as i32).to_string()
    } else {
        display_max_string.clone()
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
            let is_enabled = anim_viewer.loaded_anim_index != IDX_NONE && base_assets_available && !is_locked;
            
            if ui.add_enabled_ui(is_enabled, |ui| {
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
            ui.add_enabled_ui(!is_locked, |ui| {
                ui.allocate_ui(egui::vec2(COL2_W, TILE_HEIGHT), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = GAP;
                        
                        let is_enabled = anim_viewer.loaded_anim_index != IDX_NONE && base_assets_available;
    
                        if !is_playing {
                            let left_button = ui.add_enabled_ui(is_enabled, |ui| {
                                ui.add_sized(egui::vec2(NAV_W, TILE_HEIGHT), egui::Button::new("◀").sense(egui::Sense::click().union(egui::Sense::drag())))
                            }).inner;
                            
                            tile_frame.show(ui, |ui| {
                                ui.set_width(INPUT_W); ui.set_height(TILE_HEIGHT);
                                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                    let text_response = ui.add_enabled(is_enabled, egui::TextEdit::singleline(&mut anim_viewer.single_frame_str)
                                        .frame(false).desired_width(INPUT_W).vertical_align(egui::Align::Center).horizontal_align(egui::Align::Center));
                                    if text_response.changed() {
                                        if let Ok(parsed_value) = anim_viewer.single_frame_str.parse::<i32>() {
                                            anim_viewer.current_frame = parsed_value as f32 / display_multiplier;
                                        }
                                    }
                                    if !text_response.has_focus() {
                                        anim_viewer.single_frame_str = format!("{}", current_display_value);
                                    }
                                });
                            });
    
                            let right_button = ui.add_enabled_ui(is_enabled, |ui| {
                                ui.add_sized(egui::vec2(NAV_W, TILE_HEIGHT), egui::Button::new("▶").sense(egui::Sense::click().union(egui::Sense::drag())))
                            }).inner;
    
                            if left_button.is_pointer_button_down_on() { anim_viewer.hold_dir = -1; } 
                            else if right_button.is_pointer_button_down_on() { anim_viewer.hold_dir = 1; } 
                            else { anim_viewer.hold_dir = 0; }
                            
                            if left_button.clicked() {
                                let previous_frame = current_frame_value - 1.0;
                                let wrap_value = if loop_lcm_result.is_some() { max_frame_value as f32 } else { 0.0 }; 
                                anim_viewer.current_frame = if previous_frame < 0.0 { wrap_value } else { previous_frame };
                            }
                            if right_button.clicked() {
                                let next_frame = current_frame_value + 1.0;
                                if let Some(max_limit) = loop_lcm_result {
                                    anim_viewer.current_frame = if next_frame > max_limit as f32 { 0.0 } else { next_frame };
                                } else {
                                    anim_viewer.current_frame = next_frame;
                                }
                            }
                        } else {
                            // Range Controls
                            tile_frame.show(ui, |ui| {
                                ui.set_width(60.0); ui.set_height(TILE_HEIGHT);
                                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    if let Some(range_start) = loop_range_start {
                                        if anim_viewer.range_str_cache.0.is_empty() {
                                            let display_value = (range_start as f32 * display_multiplier).round() as i32;
                                            anim_viewer.range_str_cache.0 = display_value.to_string();
                                        }
                                    }
                                    let text_response = ui.add_enabled(is_enabled, egui::TextEdit::singleline(&mut anim_viewer.range_str_cache.0)
                                        .hint_text(egui::RichText::new("0").color(egui::Color32::GRAY)).frame(false).desired_width(60.0).vertical_align(egui::Align::Center).horizontal_align(egui::Align::Center));
                                    if text_response.changed() {
                                        if anim_viewer.range_str_cache.0.is_empty() { anim_viewer.loop_range.0 = None; } 
                                        else if let Ok(parsed_value) = anim_viewer.range_str_cache.0.parse::<i32>() {
                                            anim_viewer.loop_range.0 = Some((parsed_value as f32 / display_multiplier).round() as i32);
                                        }
                                    }
                                    if text_response.secondary_clicked() { anim_viewer.loop_range.0 = None; anim_viewer.range_str_cache.0.clear(); }
                                });
                            });
                            tile_frame.show(ui, |ui| { ui.set_width(20.0); ui.set_height(TILE_HEIGHT); ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| { ui.label("~"); }); });
                            tile_frame.show(ui, |ui| {
                                ui.set_width(60.0); ui.set_height(TILE_HEIGHT);
                                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    if let Some(range_end) = loop_range_end {
                                        if anim_viewer.range_str_cache.1.is_empty() {
                                            let display_value = (range_end as f32 * display_multiplier).round() as i32;
                                            anim_viewer.range_str_cache.1 = display_value.to_string();
                                        }
                                    }
                                    let text_response = ui.add_enabled(is_enabled, egui::TextEdit::singleline(&mut anim_viewer.range_str_cache.1)
                                        .hint_text(egui::RichText::new(&display_max_string).color(egui::Color32::GRAY)).frame(false).desired_width(60.0).vertical_align(egui::Align::Center).horizontal_align(egui::Align::Center));
                                    if text_response.changed() {
                                        if anim_viewer.range_str_cache.1.is_empty() { anim_viewer.loop_range.1 = None; } 
                                        else if let Ok(parsed_value) = anim_viewer.range_str_cache.1.parse::<i32>() {
                                            anim_viewer.loop_range.1 = Some((parsed_value as f32 / display_multiplier).round() as i32);
                                        }
                                    }
                                    if text_response.secondary_clicked() { anim_viewer.loop_range.1 = None; anim_viewer.range_str_cache.1.clear(); }
                                });
                            });
                        }
                    });
                });
            });

            ui.add_space(GAP);

            // Info Row
            ui.allocate_ui(egui::vec2(COL2_W, TILE_HEIGHT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    tile_frame.show(ui, |ui| { ui.set_width(60.0); ui.set_height(TILE_HEIGHT); ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| { ui.label(egui::RichText::new(format!("{}", current_display_value)).color(egui::Color32::WHITE)); }); });
                    tile_frame.show(ui, |ui| { ui.set_width(20.0); ui.set_height(TILE_HEIGHT); ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| { ui.label("/"); }); });
                    tile_frame.show(ui, |ui| { ui.set_width(60.0); ui.set_height(TILE_HEIGHT); ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| { ui.label(egui::RichText::new(&effective_display_max).color(egui::Color32::WHITE)); }); });
                });
            });
        });

        ui.add_sized(egui::vec2(10.0, (TILE_HEIGHT * 2.0) + GAP), egui::Separator::default().vertical());

        // Column 3
        ui.vertical(|ui| {
            // EXPORT BUTTON LOGIC
            let button_response = ui.add_enabled_ui(base_assets_available, |ui| {
                ui.add_sized(egui::vec2(COL3_W, TILE_HEIGHT), egui::Button::new("Export"))
            }).inner;
            
            if button_response.clicked() { 
                // Write directly to settings!
                settings.animation.export_popup_open = true;
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
                        
                        let text_response = ui.add_enabled(base_assets_available, egui::TextEdit::singleline(&mut anim_viewer.speed_str)
                            .hint_text(egui::RichText::new("1.0").color(egui::Color32::GRAY))
                            .frame(false)
                            .desired_width(40.0)
                            .vertical_align(egui::Align::Center)
                            .horizontal_align(egui::Align::Center));
                        
                        if text_response.changed() {
                            if anim_viewer.speed_str.is_empty() {
                                anim_viewer.playback_speed = 1.0;
                            } else if let Ok(parsed_value) = anim_viewer.speed_str.parse::<f32>() {
                                anim_viewer.playback_speed = parsed_value;
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

    let top_row_width = (button_width * 4.0) + (grid_gap * 3.0);
    let left_padding = if actual_width > top_row_width { (actual_width - top_row_width) / 2.0 } else { 0.0 };

    let grid_allocation = ui.allocate_ui(egui::vec2(ui.available_width(), anim_viewer.cached_grid_height), |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(left_padding);
                egui::Grid::new("anim_controls_grid").spacing(egui::vec2(grid_gap, grid_gap)).show(ui, |ui| {
                    
                    let mut draw_anim_button = |ui: &mut egui::Ui, label: &str, index: usize, anim_exists: bool| {
                        let effective_enabled = base_assets_available && anim_exists && !is_locked;
                        let is_active = anim_viewer.loaded_anim_index == index && anim_viewer.loaded_anim_index != IDX_NONE;
                        
                        let background_fill = if is_active { active_color } else { inactive_color };
                        let animation_button = egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE).size(13.0)).fill(background_fill);
                        
                        if ui.add_enabled_ui(effective_enabled, |ui| { ui.add_sized(button_size, animation_button) }).inner.clicked() { 
                            clicked_index = Some(index); 
                        }
                    };

                    // ALL animations are checked against the generic path list supplied by the caller
                    let has_walk = available_anims.iter().any(|(index, _)| *index == IDX_WALK); draw_anim_button(ui, "Walk", IDX_WALK, has_walk);
                    let has_idle = available_anims.iter().any(|(index, _)| *index == IDX_IDLE); draw_anim_button(ui, "Idle", IDX_IDLE, has_idle);
                    let has_atk = available_anims.iter().any(|(index, _)| *index == IDX_ATTACK); draw_anim_button(ui, "Attack", IDX_ATTACK, has_atk);
                    let has_kb = available_anims.iter().any(|(index, _)| *index == IDX_KB); draw_anim_button(ui, "Knockback", IDX_KB, has_kb);
                    ui.end_row();

                    let has_burrow = available_anims.iter().any(|(index, _)| *index == IDX_BURROW); draw_anim_button(ui, "Burrow", IDX_BURROW, has_burrow);
                    let has_surface = available_anims.iter().any(|(index, _)| *index == IDX_SURFACE); draw_anim_button(ui, "Surface", IDX_SURFACE, has_surface);
                    
                    // Spirit / Secondary Pack validation
                    let secondary_available = secondary_pack.is_some();
                    draw_anim_button(ui, "Spirit", IDX_SPIRIT, secondary_available);
                    
                    draw_anim_button(ui, "Model", IDX_MODEL, base_assets_available);
                    ui.end_row();
                });
            });
        });
    });

    let actual_grid_height = grid_allocation.response.rect.height();
    if (anim_viewer.cached_grid_height - actual_grid_height).abs() > 0.1 {
        anim_viewer.cached_grid_height = actual_grid_height;
        ui.ctx().request_repaint();
    }

    let Some(target_index) = clicked_index else { return; };
    if is_loading_new { return; }

    anim_viewer.loaded_anim_index = target_index;
    let intended_target_id = if target_index == IDX_SPIRIT { secondary_id.to_string() } else { primary_id.to_string() };
    if anim_viewer.loaded_id != intended_target_id { return; }

    let animation_path = if target_index == IDX_SPIRIT { 
        secondary_pack.as_ref().map(|(_, _, _, anim_path)| anim_path) 
    } else { 
        available_anims.iter().find(|(index, _)| *index == target_index).map(|(_, path)| path) 
    };
    
    if let Some(valid_path) = animation_path { 
        anim_viewer.load_anim(valid_path, settings);
    } else if target_index == IDX_MODEL { 
        anim_viewer.current_anim = None; 
        anim_viewer.current_frame = 0.0;
        anim_viewer.single_frame_str = "0".to_string();

        anim_viewer.export_state.name_prefix = format!("{}.model", primary_id);
        anim_viewer.export_state.anim_name = "Model".to_string();
        anim_viewer.export_state.max_frame = 0;
        anim_viewer.export_state.frame_start = 0;
        anim_viewer.export_state.frame_start_str = String::new(); 
        anim_viewer.export_state.frame_end = 0;
        anim_viewer.export_state.frame_end_str = String::new(); 
    }
}