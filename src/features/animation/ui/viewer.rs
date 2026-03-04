use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crate::global_data::imgcut::SpriteSheet;
use crate::global_data::mamodel::Model;
use crate::global_data::maanim::Animation;
use crate::features::animation::logic::{animator, smooth, canvas, transform, controls, bounds}; 
use crate::features::animation::ui::controls::{self as anim_controls, IDX_NONE, IDX_MODEL, IDX_SPIRIT, IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE};
use crate::features::animation::export::state::{ExporterState, ExportMode};
use crate::features::animation::export::encoding::ExportFormat;
use crate::features::animation::export::process;
use crate::features::animation::ui::export;
use crate::features::settings::logic::Settings;

pub struct AnimViewer {
    pub zoom_level: f32,
    pub target_zoom_level: f32,
    pub pan_offset: egui::Vec2,
    pub current_anim: Option<Animation>,
    pub current_frame: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    pub loop_range: (Option<i32>, Option<i32>),
    pub range_str_cache: (String, String),
    pub single_frame_str: String,
    pub speed_str: String,
    pub hold_timer: f32,
    pub hold_dir: i8, 
    pub loaded_anim_index: usize, 
    pub loaded_id: String,
    pub summoner_id: String,
    last_loaded_id: String,
    pub pending_initial_center: bool,
    pub staging_model: Option<Model>,
    pub staging_sheet: Option<SpriteSheet>,
    pub held_model: Option<Model>,
    pub held_sheet: Option<SpriteSheet>,
    pub renderer: Arc<Mutex<Option<canvas::GlowRenderer>>>,
    pub cached_controls_width: f32,
    pub cached_grid_height: f32, 
    pub is_expanded: bool,          
    pub is_controls_expanded: bool, 
    pub texture_version: u64,
    pub is_pointer_over_controls: bool,
    pub is_viewport_dragging: bool,
    pub is_selecting_export_region: bool,
    pub export_selection_start: Option<egui::Pos2>,
    pub export_state: ExporterState,
    pub show_export_popup: bool,
    pub has_scanned_showcase: bool,
    pub was_export_popup_open: bool, 
    pub auto_set_camera: bool, 
}

impl Default for AnimViewer {
    fn default() -> Self {
        Self { 
            zoom_level: 1.0, 
            target_zoom_level: 1.0,
            pan_offset: egui::vec2(0.0, 0.0),
            current_anim: None,
            current_frame: 0.0,
            is_playing: true,
            playback_speed: 1.0,
            loop_range: (None, None),
            range_str_cache: (String::new(), String::new()),
            single_frame_str: String::new(),
            speed_str: String::new(),
            hold_timer: 0.0,
            hold_dir: 0,
            loaded_anim_index: 0, 
            loaded_id: String::new(),
            summoner_id: String::new(),
            last_loaded_id: "FORCE_INIT".to_string(),
            pending_initial_center: false,
            staging_model: None,
            staging_sheet: None,
            held_model: None,
            held_sheet: None,
            renderer: Arc::new(Mutex::new(None)),
            cached_controls_width: 0.0,
            cached_grid_height: 55.0, 
            is_expanded: false,
            is_controls_expanded: true,
            texture_version: 0,
            is_pointer_over_controls: false,
            is_viewport_dragging: false,
            is_selecting_export_region: false,
            export_selection_start: None,
            export_state: ExporterState::default(),
            show_export_popup: false,
            has_scanned_showcase: false,
            was_export_popup_open: false,
            auto_set_camera: false, 
        }
    }
}

impl AnimViewer {
    fn update_export_state(&mut self) {
        self.export_state.loop_supported = self.loaded_anim_index == anim_controls::IDX_WALK || self.loaded_anim_index == anim_controls::IDX_IDLE;

        if self.export_state.export_mode != ExportMode::Showcase {
            if let Some(anim) = &self.current_anim {
                self.export_state.max_frame = anim.max_frame;
                self.export_state.frame_start = 0;
                self.export_state.frame_end = anim.max_frame;
            } else {
                self.export_state.max_frame = 0;
                self.export_state.frame_start = 0;
                self.export_state.frame_end = 0;
            }
            self.export_state.frame_start_str.clear(); 
            self.export_state.frame_end_str.clear();
        }

        if self.show_export_popup && self.auto_set_camera {
            if let (Some(m), Some(s)) = (&self.held_model, &self.held_sheet) {
                if let Some(bounds) = bounds::calculate_tight_bounds(m, self.current_anim.as_ref(), s) {
                    self.export_state.region_x = bounds.min.x;
                    self.export_state.region_y = bounds.min.y;
                    self.export_state.region_w = bounds.width();
                    self.export_state.region_h = bounds.height();
                    self.export_state.zoom = 1.0;
                }
            }
        }

        let type_str = match self.loaded_anim_index {
            anim_controls::IDX_WALK => "walk",
            anim_controls::IDX_IDLE => "idle",
            anim_controls::IDX_ATTACK => "attack",
            anim_controls::IDX_KB => "kb",
            anim_controls::IDX_BURROW => "burrow",
            anim_controls::IDX_SURFACE => "surface",
            anim_controls::IDX_SPIRIT => "spirit",
            anim_controls::IDX_MODEL => "model",
            _ => "anim",
        };

        let raw_id = if self.loaded_anim_index == anim_controls::IDX_SPIRIT {
            if self.summoner_id.is_empty() { &self.loaded_id } else { &self.summoner_id }
        } else { &self.loaded_id };

        let mut clean_id = raw_id.clone();
        let parts: Vec<&str> = raw_id.split('_').collect();
        if parts.len() >= 2 {
            if parts[0].chars().all(char::is_numeric) {
                let form_num = match parts[1].chars().next() {
                    Some('f') => 1, Some('c') => 2, Some('s') => 3, Some('u') => 4, _ => 0
                };
                if form_num > 0 { clean_id = format!("{}-{}", parts[0], form_num); }
            }
        }
        self.export_state.name_prefix = format!("{}.{}", clean_id, type_str);
    }

    pub fn load_anim(&mut self, path: &Path) {
        if let Some(anim) = Animation::load(path) {
            self.current_frame = 0.0;
            self.loop_range = (None, None);
            self.range_str_cache = (String::new(), String::new());
            self.single_frame_str = "0".to_string();
            
            self.current_anim = Some(anim);
            self.update_export_state();
            
        } else {
            self.current_anim = None;
            self.current_frame = 0.0;
            self.loop_range = (None, None); 
            self.range_str_cache = (String::new(), String::new());
            self.single_frame_str = "0".to_string();
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        primary_id: &str,
        secondary_id: &str,
        available_anims: &[(usize, PathBuf)],
        primary_assets: Option<(PathBuf, PathBuf, PathBuf)>, 
        secondary_assets: Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
        model_data_sync: &mut Option<Model>,
        anim_sheet_sync: &mut SpriteSheet,
        settings: &mut Settings,
    ) {
        let base_assets_available = primary_assets.is_some();
        let secondary_available = secondary_assets.is_some();

        // Validation & Fallback Logic
        let current_idx = self.loaded_anim_index;
        let mut valid_idx = current_idx;

        let is_current_valid = if current_idx == IDX_NONE {
            false 
        } else if current_idx == IDX_SPIRIT {
            secondary_available
        } else if current_idx == IDX_MODEL {
            base_assets_available
        } else {
            base_assets_available && available_anims.iter().any(|(i, _)| *i == current_idx)
        };

        if !is_current_valid {
            valid_idx = IDX_NONE; 

            if base_assets_available {
                let priority_list = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];
                for check_idx in priority_list {
                    if available_anims.iter().any(|(i, _)| *i == check_idx) {
                        valid_idx = check_idx;
                        break;
                    }
                }
            }

            if valid_idx == IDX_NONE && secondary_available { valid_idx = IDX_SPIRIT; }
            if valid_idx == IDX_NONE && base_assets_available { valid_idx = IDX_MODEL; }
        }

        if valid_idx != current_idx {
            self.loaded_anim_index = valid_idx;
            if valid_idx == IDX_NONE {
                self.current_anim = None;
                self.held_model = None;
                self.held_sheet = None; 
                *model_data_sync = None;
                *anim_sheet_sync = SpriteSheet::default();
            }
            if current_idx == IDX_NONE && valid_idx != IDX_NONE {
                self.loaded_id.clear();
            }
        }
        
        // Loading State Calculation
        let target_viewer_id = if self.loaded_anim_index == IDX_SPIRIT {
            secondary_id.to_string()
        } else {
            primary_id.to_string()
        };

        let is_stable = self.loaded_id == target_viewer_id;
        let is_loading_new = !is_stable && (self.staging_model.is_some() || self.staging_sheet.is_some());
        let is_first_launch = self.held_model.is_none() && model_data_sync.is_none();
        let mut just_swapped = false;

        if valid_idx == IDX_NONE && !is_stable {
            self.loaded_id = target_viewer_id.clone();
        }

        if is_stable {
            if let Some(m) = model_data_sync {
                self.held_model = Some(m.clone());
            }
            self.held_sheet = Some((*anim_sheet_sync).clone());
        }

        // Start Transition
        if !is_stable && !is_loading_new && !is_first_launch && valid_idx != IDX_NONE {
            let (resolved_png, resolved_cut, resolved_model, _) = resolve_paths(valid_idx, &primary_assets, &secondary_assets, available_anims);
            
            let mut load_success = false;
            if let (Some(png), Some(cut), Some(model_path)) = (resolved_png, resolved_cut, resolved_model) {
                let mut new_sheet = SpriteSheet::default();
                new_sheet.load(ctx, png, cut, target_viewer_id.clone());
                
                if let Some(loaded_model) = Model::load(model_path) {
                    self.staging_sheet = Some(new_sheet);
                    self.staging_model = Some(loaded_model);
                    load_success = true;
                }
            }
            
            if !load_success {
                self.loaded_id = target_viewer_id.clone();
                self.held_model = None;
                self.held_sheet = None;
            }
        }

        // First Launch
        if is_first_launch && valid_idx != IDX_NONE {
            let (resolved_png, resolved_cut, resolved_model, resolved_anim) = resolve_paths(valid_idx, &primary_assets, &secondary_assets, available_anims);

            let mut load_success = false;
            if let (Some(png), Some(cut), Some(model_path)) = (resolved_png, resolved_cut, resolved_model) {
                 anim_sheet_sync.image_data = None; 
                 anim_sheet_sync.load(ctx, png, cut, target_viewer_id.clone());
                 if let Some(loaded_model) = Model::load(model_path) {
                     self.held_model = Some(loaded_model.clone());
                     self.held_sheet = Some((*anim_sheet_sync).clone());
                     *model_data_sync = Some(loaded_model);
                     
                     self.loaded_id = target_viewer_id.clone();
                     self.pending_initial_center = true; 
                     load_success = true;
                 }
            }
            
            if !load_success {
                self.loaded_id = target_viewer_id.clone();
            } else {
                if let Some(anim_path) = resolved_anim { 
                    self.load_anim(anim_path); 
                } else { 
                    self.current_anim = None; 
                }
            }
        }

        // Completion
        if is_loading_new {
            if let Some(staging_sheet) = &mut self.staging_sheet {
                staging_sheet.update(ctx);

                let texture_is_ready = staging_sheet.sheet_name == target_viewer_id 
                                    && !staging_sheet.is_loading_active 
                                    && staging_sheet.image_data.is_some();

                if texture_is_ready {
                    if let (Some(new_model), Some(new_sheet)) = (self.staging_model.take(), self.staging_sheet.take()) {
                        self.held_model = Some(new_model.clone());
                        self.held_sheet = Some(new_sheet.clone());
                        *model_data_sync = Some(new_model);
                        *anim_sheet_sync = new_sheet; 
                        self.loaded_id = target_viewer_id.clone();
                        
                        let (_, _, _, resolved_anim) = resolve_paths(valid_idx, &primary_assets, &secondary_assets, available_anims);
                        
                        if let Some(anim_path) = resolved_anim { 
                            self.load_anim(anim_path); 
                        } else { 
                            self.current_anim = None; 
                        }
                        
                        self.pending_initial_center = true;
                        just_swapped = true;
                        ctx.request_repaint();
                    }
                }
            }
        } else {
            anim_sheet_sync.update(ctx);
        }

        // UI Layout
        let allow_texture_update = !is_loading_new || just_swapped;

        if self.is_expanded {
            egui::Area::new("expanded_anim_viewer_area".into())
                .fixed_pos(egui::pos2(0.0, 0.0))
                .order(egui::Order::Middle) 
                .show(ctx, |ui| {
                    let screen_rect = ctx.screen_rect();
                    egui::Frame::window(&ctx.style())
                        .inner_margin(0.0)
                        .shadow(egui::epaint::Shadow::NONE)
                        .show(ui, |ui| {
                            ui.set_min_size(screen_rect.size());
                            ui.set_max_size(screen_rect.size());
                            let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                            ui.put(rect, |ui: &mut egui::Ui| {
                                self.draw_viewport(
                                    ui, rect, available_anims, base_assets_available, is_loading_new, 
                                    primary_id, secondary_id, &secondary_assets, allow_texture_update, settings
                                );
                                ui.allocate_rect(rect, egui::Sense::hover())
                            });
                        });
                });

            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("Animation Expanded").size(16.0).weak());
                if ui.button("Restore View").clicked() {
                    self.is_expanded = false;
                }
            });

        } else {
            ui.vertical(|ui| {
                let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                ui.put(rect, |ui: &mut egui::Ui| {
                    self.draw_viewport(
                        ui, rect, available_anims, base_assets_available, is_loading_new, 
                        primary_id, secondary_id, &secondary_assets, allow_texture_update, settings
                    );
                    ui.allocate_rect(rect, egui::Sense::hover())
                });
            });
        }
    }

    fn draw_viewport(
        &mut self, 
        ui: &mut egui::Ui, 
        rect: egui::Rect,
        available_anims: &[(usize, PathBuf)],
        base_assets_available: bool,
        is_loading_new: bool,
        primary_id: &str,
        secondary_id: &str,
        secondary_assets: &Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
        allow_update: bool,
        settings: &mut Settings,
    ) {
        let dt = ui.input(|i| i.stable_dt);
        let interpolation = settings.animation_interpolation;
        let debug_show_info = settings.animation_debug;
        let centering_behavior = settings.centering_behavior;
        let native_fps = settings.native_fps;
        
        self.auto_set_camera = settings.auto_set_camera_region; 
        if !primary_id.is_empty() { self.summoner_id = primary_id.to_string(); }

        if self.loaded_id != self.last_loaded_id {
            self.last_loaded_id = self.loaded_id.clone();
            self.pending_initial_center = true;

            let mut preserved_loop_msg: Option<String> = None;
            let mut preserved_export_msg: Option<String> = None;
            let mut preserved_time: Option<f64> = None;
            
            if self.export_state.is_loop_searching {
                if let Some(abort) = &self.export_state.loop_abort {
                    abort.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                preserved_loop_msg = Some("Loop Terminated!".to_string());
                preserved_time = Some(ui.input(|i| i.time));
            }

            if self.export_state.is_processing {
                if let Some(abort) = &self.export_state.abort {
                    abort.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                preserved_export_msg = Some("Export Terminated!".to_string());
                preserved_time = Some(ui.input(|i| i.time));
            }

            let prev_mode = self.export_state.export_mode.clone();
            
            self.export_state = ExporterState::default();
            self.export_state.export_mode = prev_mode;

            self.export_state.format = match settings.last_export_format {
                1 => ExportFormat::WebP,
                2 => ExportFormat::Avif,
                3 => ExportFormat::Png,
                4 => ExportFormat::Mp4,
                5 => ExportFormat::Mkv,
                6 => ExportFormat::Webm,
                7 => ExportFormat::Zip,
                _ => ExportFormat::Gif,
            };
            
            if let Some(msg) = preserved_loop_msg {
                self.export_state.loop_result_msg = Some(msg);
                self.export_state.completion_time = preserved_time;
            }
            if let Some(msg) = preserved_export_msg {
                self.export_state.export_result_msg = Some(msg);
                self.export_state.completion_time = preserved_time;
            }

            self.update_export_state();

            self.has_scanned_showcase = false; 
        }

        if self.export_state.export_mode == ExportMode::Loop {
            if !self.export_state.loop_supported {
                self.export_state.export_mode = ExportMode::Manual;
                self.export_state.frame_start = 0;
                self.export_state.frame_end = 0;
                self.export_state.frame_start_str.clear();
                self.export_state.frame_end_str.clear();
            }
        }

        let mut new_center: Option<(egui::Vec2, f32)> = None;
        let mut should_clear_pending = false;

        if let (Some(model), Some(sheet)) = (&self.held_model, &self.held_sheet) {
            if self.pending_initial_center {
                if centering_behavior == 0 { 
                    if !model.parts.is_empty() {
                        if let Some((offset, fit_zoom)) = bounds::calculate_initial_view(model, self.current_anim.as_ref(), sheet, ui.available_size()) {
                            new_center = Some((offset, fit_zoom));
                        }
                    }
                } else if centering_behavior == 1 { 
                    new_center = Some((egui::Vec2::ZERO, self.target_zoom_level));
                } else { should_clear_pending = true; }
            }
        } else { should_clear_pending = true; }

        if let (Some(offset), Some(zoom)) = (new_center.map(|x| x.0), new_center.map(|x| x.1)) {
            self.pan_offset = offset;
            if centering_behavior == 0 { self.target_zoom_level = zoom; }
            self.pending_initial_center = false;
        } else if should_clear_pending { self.pending_initial_center = false; }

        let diff = self.target_zoom_level - self.zoom_level;
        if diff.abs() > 0.001 { self.zoom_level += diff * 15.0 * dt; } else { self.zoom_level = self.target_zoom_level; }

        if let Some(anim) = &self.current_anim {
            let lcm_max = if self.loaded_anim_index <= 1 { anim.calculate_true_loop() } else { Some(anim.max_frame) };
            let start = self.loop_range.0.unwrap_or(0);
            let (effective_max, is_infinite, has_user_override) = match (self.loop_range.1, lcm_max) {
                (Some(user_override), _) => (user_override as f32, false, true),
                (None, Some(calc)) => (calc as f32, false, false),
                (None, None) => (0.0, true, false), 
            };
            
            if self.hold_dir != 0 {
                self.hold_timer += dt;
                ui.ctx().request_repaint();
                if self.hold_timer > 0.2 {
                   let speed_factor = if self.playback_speed.abs() < 0.05 { 1.0 } else { self.playback_speed.abs() };
                   let delta = self.hold_dir as f32 * dt * 30.0 * speed_factor;
                   let mut new_frame = self.current_frame + delta;
                   if !is_infinite {
                       if new_frame > effective_max { new_frame = 0.0; } else if new_frame < 0.0 { new_frame = effective_max; }
                   } else { if new_frame < 0.0 { new_frame = 0.0; } }
                   self.current_frame = new_frame;
                }
            } else { self.hold_timer = 0.0; }

            if self.is_playing {
                if !is_infinite && effective_max < 1.0 && !has_user_override { self.current_frame = 0.0; } else {
                    self.current_frame += dt * 30.0 * self.playback_speed;
                    if !is_infinite && self.current_frame > effective_max { self.current_frame = start as f32; }
                }
                ui.ctx().request_repaint();
            }
        }

        let (rect_alloc, response) = ui.allocate_exact_size(rect.size(), egui::Sense::drag());
        let (hover_pos, right_down, left_down) = ui.input(|i| (i.pointer.hover_pos(), i.pointer.secondary_down(), i.pointer.primary_down()));
        
        let block_input = self.is_pointer_over_controls || (self.is_selecting_export_region && right_down);
        if self.is_selecting_export_region && left_down && hover_pos.is_some() {
            controls::handle_viewport_input(ui, &response, &mut self.pan_offset, &mut self.zoom_level, &mut self.target_zoom_level, &mut self.pending_initial_center, false, &mut self.is_viewport_dragging);
        } else {
            controls::handle_viewport_input(ui, &response, &mut self.pan_offset, &mut self.zoom_level, &mut self.target_zoom_level, &mut self.pending_initial_center, block_input, &mut self.is_viewport_dragging);
        }

        if self.is_selecting_export_region {
            ui.painter().rect_filled(rect_alloc, 0.0, egui::Color32::from_black_alpha(50));
            let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("anim_export_tip")));
            let tip_text = "Right click & drag to set camera";
            let font_id = egui::FontId::proportional(13.0);
            let galley = painter.layout_no_wrap(tip_text.to_string(), font_id, egui::Color32::WHITE);
            let bg_margin = 6.0;
            let bg_w = galley.size().x + bg_margin * 2.0;
            let bg_h = galley.size().y + bg_margin * 2.0;
            let top_center = rect_alloc.center_top() + egui::vec2(0.0, 30.0);
            let tip_rect = egui::Rect::from_center_size(top_center, egui::vec2(bg_w, bg_h));
            painter.rect(tip_rect, 4.0, egui::Color32::from_black_alpha(180), egui::Stroke::new(1.0, egui::Color32::from_gray(180)));
            painter.galley(tip_rect.min + egui::vec2(bg_margin, bg_margin), galley, egui::Color32::WHITE);

            if let Some(pos) = hover_pos {
                if right_down {
                    if self.export_selection_start.is_none() { 
                        if rect_alloc.contains(pos) && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
                            self.export_selection_start = Some(pos); 
                        }
                    }
                    if let Some(start) = self.export_selection_start {
                        let r = egui::Rect::from_two_pos(start, pos);
                        ui.painter().with_clip_rect(rect_alloc).rect_stroke(r, 0.0, egui::Stroke::new(2.0, egui::Color32::YELLOW));
                        ui.painter().with_clip_rect(rect_alloc).rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(255, 255, 0, 30));
                    }
                } else if self.export_selection_start.is_some() {
                    let start = self.export_selection_start.take().unwrap();
                    let r = egui::Rect::from_two_pos(start, pos);
                    if r.width() * r.height() > 25.0 {
                        let center_screen = rect_alloc.center();
                        let to_world = |p: egui::Pos2| -> egui::Vec2 { ((p - center_screen) / self.zoom_level) - self.pan_offset };
                        let min_w = to_world(r.min); let max_w = to_world(r.max);
                        self.export_state.region_x = min_w.x; self.export_state.region_y = min_w.y;
                        self.export_state.region_w = (max_w.x - min_w.x).abs(); self.export_state.region_h = (max_w.y - min_w.y).abs();
                        self.export_state.zoom = 1.0; 
                        self.is_selecting_export_region = false; 
                        self.show_export_popup = true;
                        self.was_export_popup_open = true; 
                    } else { 
                        self.is_selecting_export_region = false; 
                        self.show_export_popup = true; 
                        self.was_export_popup_open = true; 
                    }
                }
            }
        }

        if self.show_export_popup && !self.was_export_popup_open {
             if self.export_state.format == ExportFormat::Gif && settings.last_export_format != 0 {
                 self.export_state.format = match settings.last_export_format {
                     1 => ExportFormat::WebP,
                     2 => ExportFormat::Avif,
                     3 => ExportFormat::Png,
                     4 => ExportFormat::Mp4,
                     5 => ExportFormat::Mkv,
                     6 => ExportFormat::Webm,
                     7 => ExportFormat::Zip,
                     _ => ExportFormat::Gif,
                 };
             }

             if self.auto_set_camera {
                 if let (Some(m), Some(s)) = (&self.held_model, &self.held_sheet) {
                    if let Some(bounds) = bounds::calculate_tight_bounds(m, self.current_anim.as_ref(), s) {
                        self.export_state.region_x = bounds.min.x;
                        self.export_state.region_y = bounds.min.y;
                        self.export_state.region_w = bounds.width();
                        self.export_state.region_h = bounds.height();
                        self.export_state.zoom = 1.0;
                    }
                }
             } else {
                self.export_state.region_w = 0.0;
                self.export_state.region_h = 0.0;
                self.export_state.region_x = 0.0;
                self.export_state.region_y = 0.0;
             }
        }
        self.was_export_popup_open = self.show_export_popup;

        let walk_mismatch = self.export_state.last_known_walk_default != settings.default_showcase_walk;
        let idle_mismatch = self.export_state.last_known_idle_default != settings.default_showcase_idle;
        let kb_mismatch = self.export_state.last_known_kb_default != settings.default_showcase_kb;

        if walk_mismatch || idle_mismatch || kb_mismatch {
            self.export_state.last_known_walk_default = settings.default_showcase_walk;
            self.export_state.last_known_idle_default = settings.default_showcase_idle;
            self.export_state.last_known_kb_default = settings.default_showcase_kb;

            if self.export_state.showcase_walk_str.is_empty() {
                self.export_state.showcase_walk_len = settings.default_showcase_walk;
            }
            if self.export_state.showcase_idle_str.is_empty() {
                self.export_state.showcase_idle_len = settings.default_showcase_idle;
            }
            if self.export_state.showcase_kb_str.is_empty() {
                self.export_state.showcase_kb_len = settings.default_showcase_kb;
            }

            self.has_scanned_showcase = false;
        }
        
        if self.show_export_popup && self.export_state.export_mode == ExportMode::Showcase && !self.has_scanned_showcase {
             if let Some((_, path)) = available_anims.iter().find(|(i, _)| *i == anim_controls::IDX_ATTACK) {
                 if let Some(anim) = Animation::load(path) {
                     let total = anim.max_frame + 1;
                     self.export_state.detected_attack_len = total;
                     if self.export_state.showcase_attack_str.is_empty() {
                         self.export_state.showcase_attack_len = total;
                     }
                 }
             }

             if let Some((_, path)) = available_anims.iter().find(|(i, _)| *i == anim_controls::IDX_WALK) {
                 if let Some(anim) = Animation::load(path) {
                     let len = anim.calculate_true_loop().unwrap_or(anim.max_frame);
                     
                     let is_short = len <= 1;
                     let new_len = if is_short { 0 } else { settings.default_showcase_walk };
                     self.export_state.detected_walk_len = new_len;
                     
                     if self.export_state.showcase_walk_str.is_empty() || self.export_state.showcase_walk_len == settings.default_showcase_walk {
                        self.export_state.showcase_walk_len = new_len;
                     }
                 }
             }

             if let Some((_, path)) = available_anims.iter().find(|(i, _)| *i == anim_controls::IDX_IDLE) {
                 if let Some(anim) = Animation::load(path) {
                     let len = anim.calculate_true_loop().unwrap_or(anim.max_frame);
                     
                     let is_short = len <= 1;
                     let new_len = if is_short { 0 } else { settings.default_showcase_idle };
                     
                     self.export_state.detected_idle_len = new_len;

                     if self.export_state.showcase_idle_str.is_empty() || self.export_state.showcase_idle_len == settings.default_showcase_idle {
                        self.export_state.showcase_idle_len = new_len;
                     }
                 }
             }

             self.has_scanned_showcase = true;
        }

        let mut showcase_render_time = 0.0;

        if self.export_state.is_processing && self.export_state.export_mode == ExportMode::Showcase {
            let walk_dur = self.export_state.showcase_walk_len;
            let idle_dur = self.export_state.showcase_idle_len;
            let attack_dur = self.export_state.showcase_attack_len;
            let kb_dur = self.export_state.showcase_kb_len;
            
            let p = self.export_state.current_progress;
            
            let target_index = if p < walk_dur {
                showcase_render_time = (p % (if walk_dur < 1 { 1 } else { walk_dur })) as f32; 
                anim_controls::IDX_WALK
            } else if p < walk_dur + idle_dur {
                showcase_render_time = ((p - walk_dur) % (if idle_dur < 1 { 1 } else { idle_dur })) as f32;
                anim_controls::IDX_IDLE
            } else if p < walk_dur + idle_dur + attack_dur {
                showcase_render_time = (p - (walk_dur + idle_dur)) as f32;
                anim_controls::IDX_ATTACK
            } else {
                let kb_rel = p - (walk_dur + idle_dur + attack_dur);
                showcase_render_time = (kb_rel % (if kb_dur < 1 { 1 } else { kb_dur })) as f32;
                anim_controls::IDX_KB
            };

            if self.loaded_anim_index != target_index {
                if let Some((_, path)) = available_anims.iter().find(|(i, _)| *i == target_index) {
                     self.load_anim(path);
                     self.loaded_anim_index = target_index; 
                }
            }
        }

        if let (Some(model), Some(sheet)) = (&self.held_model, &self.held_sheet) {
            
            if self.export_state.is_processing {
                let time_to_use = if self.export_state.export_mode == ExportMode::Showcase {
                    if let Some(anim) = &self.current_anim {
                         if anim.max_frame == 0 { 0.0 } else { showcase_render_time }
                    } else { 0.0 }
                } else {
                     let start = self.export_state.frame_start;
                     let step = if self.export_state.frame_start < self.export_state.frame_end { 1 } else { -1 };
                     (start + (self.export_state.current_progress * step)) as f32
                };

                process::process_frame(ui, rect_alloc, &mut self.export_state, model, self.current_anim.as_ref(), sheet, self.renderer.clone(), time_to_use);
                
                ui.ctx().request_repaint();
            }

            let parts_to_draw = if let Some(anim) = &self.current_anim {
                let f = self.current_frame;
                let animated_parts = if interpolation { smooth::animate(model, anim, f) } else { animator::animate(model, anim, (f + 0.01).floor()) };
                transform::solve_hierarchy(&animated_parts, model)
            } else { transform::solve_hierarchy(&model.parts, model) };

            let sheet_arc = Arc::new(SpriteSheet { texture_handle: sheet.texture_handle.clone(), image_data: sheet.image_data.clone(), cuts_map: sheet.cuts_map.clone(), is_loading_active: sheet.is_loading_active, data_receiver: None, sheet_name: sheet.sheet_name.clone() });
            canvas::paint(ui, rect_alloc, self.renderer.clone(), sheet_arc, parts_to_draw, self.pan_offset, self.zoom_level, allow_update);
            
            if debug_show_info {
                let center = rect_alloc.center() + self.pan_offset * self.zoom_level;
                let s = 15.0; let c = egui::Color32::GREEN; let stk = egui::Stroke::new(2.0, c);
                ui.painter().line_segment([center - egui::vec2(s, 0.0), center + egui::vec2(s, 0.0)], stk);
                ui.painter().line_segment([center - egui::vec2(0.0, s), center + egui::vec2(0.0, s)], stk);
            }
            if self.show_export_popup {
                 if self.export_state.region_w > 0.1 && self.export_state.region_h > 0.1 {
                     let center_screen = rect_alloc.center();
                     let to_screen = |wx: f32, wy: f32| -> egui::Pos2 { let world_pos = egui::vec2(wx, wy); center_screen + (world_pos + self.pan_offset) * self.zoom_level };
                     let min = to_screen(self.export_state.region_x, self.export_state.region_y);
                     let max = to_screen(self.export_state.region_x + self.export_state.region_w, self.export_state.region_y + self.export_state.region_h);
                     ui.painter().with_clip_rect(rect_alloc).rect_stroke(egui::Rect::from_min_max(min, max), 0.0, egui::Stroke::new(1.0, egui::Color32::YELLOW));
                 }
            }
        } else { ui.painter().rect_filled(rect_alloc, 0.0, egui::Color32::from_rgb(20, 20, 20)); }

        let border_rect = rect_alloc.shrink(2.0);
        let border_color = egui::Color32::from_rgb(31, 106, 165); 
        ui.painter().rect_stroke(border_rect, egui::Rounding::same(5.0), egui::Stroke::new(4.0, border_color));

        let btn_size = egui::vec2(30.0, 30.0);
        let btn_rect = egui::Rect::from_min_size(rect_alloc.min + egui::vec2(8.0, 8.0), btn_size);
        let bg_fill = if self.is_expanded { egui::Color32::from_rgb(31, 106, 165) } else { egui::Color32::from_gray(60) };

        let btn_response = ui.put(btn_rect, |ui: &mut egui::Ui| {
            let btn = egui::Button::new(egui::RichText::new("⛶").size(20.0).color(egui::Color32::WHITE))
                .fill(bg_fill)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .rounding(4.0);
            
            let response = ui.add_sized(btn_size, btn);
            if response.clicked() { 
                self.is_expanded = !self.is_expanded; 
            }
            response
        });

        let controls_hovered = anim_controls::render_controls_overlay(ui, rect_alloc, self, available_anims, base_assets_available, is_loading_new, secondary_id, primary_id, secondary_assets, interpolation, native_fps);
        self.is_pointer_over_controls = controls_hovered || btn_response.hovered();

        // Pass everything to the export popup safely
        let state = &mut self.export_state;
        let show_popup = &mut self.show_export_popup;
        let model = self.held_model.as_ref();
        let anim = self.current_anim.as_ref();
        let sheet = self.held_sheet.as_ref();
        let start_select = &mut self.is_selecting_export_region;
        
        export::show_popup(ui, state, model, anim, sheet, show_popup, start_select, settings);
    }
}

// Safely resolves which assets to load based on the button selected
fn resolve_paths<'a>(
    idx: usize,
    primary_assets: &'a Option<(PathBuf, PathBuf, PathBuf)>,
    secondary_assets: &'a Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    available_anims: &'a [(usize, PathBuf)]
) -> (Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>) {
    if idx == IDX_SPIRIT {
        if let Some((s_png, s_cut, s_model, s_anim)) = secondary_assets {
            return (Some(s_png), Some(s_cut), Some(s_model), Some(s_anim));
        }
    } else {
        let anim_path = available_anims.iter().find(|(i, _)| *i == idx).map(|(_, p)| p);
        if let Some((p_png, p_cut, p_model)) = primary_assets {
            return (Some(p_png), Some(p_cut), Some(p_model), anim_path);
        }
    }
    (None, None, None, None)
}