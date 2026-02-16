use eframe::egui;
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::data::global::imgcut::SpriteSheet;
use crate::core::anim::export::encoding::{ExportFormat, EncoderStatus};
use crate::core::anim::export::state::{ExporterState, ExportMode, LoopStatus};
use crate::core::anim::export::process::{start_export, STATUS_RX};
use crate::core::anim::export::findloop;
use crate::ui::views::settings::toggle_ui; 
use crate::core::anim::bounds;
use crate::core::addons::toolpaths::{self, Presence};

const EXPORT_MODE_SPACING: f32 = 2.0; 
const CAMERA_COLUMN_WIDTH: f32 = 5.0;

pub fn show_popup(
    ui: &mut egui::Ui,
    state: &mut ExporterState,
    model: Option<&Model>,
    anim: Option<&Animation>,
    sheet: Option<&SpriteSheet>,
    is_open: &mut bool,
    start_region_selection: &mut bool,
) {
    // TOOL VALIDATION CHECK
    let ffmpeg_missing = toolpaths::ffmpeg_status() != Presence::Installed;
    let avif_missing = toolpaths::avifenc_status() != Presence::Installed;

    if state.format == ExportFormat::Avif && avif_missing {
        state.format = ExportFormat::Gif;
    }
    
    match state.format {
        ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm | ExportFormat::Png => {
            if ffmpeg_missing {
                state.format = ExportFormat::Gif;
            }
        },
        _ => {}
    }

    // SETUP
    let attention_latch_id = egui::Id::new("export_needs_critical_attention");

    // EXPORT STATUS POLLING
    if state.is_processing {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
        if let Ok(rx_opt) = STATUS_RX.lock() {
            if let Some(rx) = rx_opt.as_ref() {
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        EncoderStatus::Encoding => { },
                        EncoderStatus::Progress(p) => { state.encoded_frames = p as i32; },
                        EncoderStatus::Finished => { 
                            state.is_processing = false; 
                            state.completion_time = Some(ui.input(|i| i.time));
                            ui.ctx().data_mut(|d| d.insert_temp(attention_latch_id, true));
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("export_done_seen"), false));
                        }
                    }
                }
            }
        }
    }

    // LOOP SEARCH STATUS POLLING
    let mut loop_finished = false;
    
    if state.is_loop_searching {
        ui.ctx().request_repaint_after(Duration::from_millis(50));
        
        if let Some(rx) = &state.loop_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    LoopStatus::Searching(n) => { state.loop_frames_searched = n; },
                    LoopStatus::Found(start, end) => {
                        state.frame_start = start;
                        state.frame_end = end;
                        state.frame_start_str = start.to_string();
                        state.frame_end_str = end.to_string();
                        
                        // Grab Attention on Success
                        ui.ctx().data_mut(|d| d.insert_temp(attention_latch_id, true));
                        
                        loop_finished = true;
                    },
                    LoopStatus::NotFound => {
                        loop_finished = true;
                    },
                    LoopStatus::Error(e) => {
                        if e.contains("Timed out") {
                            state.loop_result_msg = Some("Loop Search Timeout (180s)".to_string());
                            state.completion_time = Some(ui.input(|i| i.time));
                            
                            // Grab Attention on Timeout
                            ui.ctx().data_mut(|d| d.insert_temp(attention_latch_id, true));
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("export_done_seen"), false));
                        }
                        loop_finished = true;
                    }
                }
            }
        }
    }
    
    // Cleanup outside the borrow scope
    if loop_finished {
        state.is_loop_searching = false;
        state.loop_rx = None;
        state.loop_abort = None;
    }

    // LATCH EXECUTION
    let needs_attention = ui.ctx().data(|d| d.get_temp(attention_latch_id).unwrap_or(false));
    if needs_attention {
        if ui.input(|i| i.focused) {
            ui.ctx().data_mut(|d| d.insert_temp(attention_latch_id, false));
        } else {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
    }

    // UI RENDERING
    if !*is_open { return; }

    let ctx = ui.ctx().clone();
    let mut open_local = *is_open;
    let allow_drag = state.drag_guard.update(&ctx);

    let saved_style = ctx.style();
    let mut style = (*saved_style).clone();
    style.interaction.resize_grab_radius_side = 0.0;
    ctx.set_style(style);

    let window_id = egui::Id::new("Export Animation");
    let mut fixed_pos = None;

    if let Some(rect) = ctx.memory(|mem| mem.area_rect(window_id)) {
        let screen_rect = ctx.screen_rect();
        let mut new_pos = rect.min;
        let mut changed = false;
        if new_pos.y < screen_rect.top() { new_pos.y = screen_rect.top(); changed = true; }
        if new_pos.y > screen_rect.bottom() - 30.0 { new_pos.y = screen_rect.bottom() - 30.0; changed = true; }
        if new_pos.x + rect.width() - 50.0 < screen_rect.left() { new_pos.x = screen_rect.left() - rect.width() + 50.0; changed = true; }
        if new_pos.x + 50.0 > screen_rect.right() { new_pos.x = screen_rect.right() - 50.0; changed = true; }
        if changed { fixed_pos = Some(new_pos); }
    }

    let mut window = egui::Window::new("Export Animation")
        .id(window_id).open(&mut open_local).order(egui::Order::Foreground)
        .constrain(false).movable(allow_drag).collapsible(false)
        .resizable(false) 
        .default_pos(ctx.screen_rect().center() - egui::vec2(200.0, 260.0));

    if let Some(pos) = fixed_pos { window = window.current_pos(pos); }
        
    window.show(&ctx, |ui| { 
        egui::Resize::default()
            .id(egui::Id::new("export_resize_area"))
            .default_size([400.0, 560.0])
            .min_size([250.0, 300.0])
            .with_stroke(false) 
            .show(ui, |ui| {
                render_content(ui, state, model, anim, sheet, is_open, start_region_selection); 
            });
    });
    
    if !open_local { *is_open = false; }
}

fn render_content(
    ui: &mut egui::Ui,
    state: &mut ExporterState,
    model: Option<&Model>,
    anim: Option<&Animation>,
    sheet: Option<&SpriteSheet>,
    is_open: &mut bool,
    start_region_selection: &mut bool,
) {
    if state.anim_name.is_empty() {
        if let Some(a) = anim {
            if state.max_frame == 0 || state.max_frame == 100 {
                state.max_frame = a.max_frame;
            }
            if state.frame_end_str.is_empty() { state.frame_end = a.max_frame; }
        }
        state.anim_name = "Animation".to_string(); 
    }

    let bottom_height = 114.0; 
    let available_height = ui.available_height() - bottom_height;

    let ui_locked = state.is_processing || state.is_loop_searching;

    egui::ScrollArea::vertical().max_height(available_height).auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(5.0);
        ui.heading("Input"); 
        ui.add_space(5.0);

        // Export Mode Dropdown
        ui.add_enabled_ui(!ui_locked, |ui| {
            ui.horizontal(|ui| {
                 ui.label("Mode");
                 let mut mode = state.export_mode.clone();
                 egui::ComboBox::from_id_salt("ex_mode").selected_text(match mode {
                     ExportMode::Manual => "Manual",
                     ExportMode::Loop => "Loop",
                     ExportMode::Showcase => "Showcase",
                 }).show_ui(ui, |ui| {
                     ui.selectable_value(&mut mode, ExportMode::Manual, "Manual");
                     
                     if state.loop_supported {
                         ui.selectable_value(&mut mode, ExportMode::Loop, "Loop");
                     } else {
                         let r = ui.add_enabled(false, egui::SelectableLabel::new(false, "Loop"));
                         r.on_disabled_hover_text("Walk and Idle only");
                     }
                     
                     ui.selectable_value(&mut mode, ExportMode::Showcase, "Showcase");
                 });
                 if mode != state.export_mode {
                     // Mode Switch Logic
                     if mode == ExportMode::Showcase {
                         state.showcase_walk_str.clear();
                         state.showcase_idle_str.clear();
                         state.showcase_attack_str.clear();
                         state.showcase_kb_str.clear();
                         state.frame_start = 0;
                     }
                     if mode == ExportMode::Manual && state.export_mode == ExportMode::Loop {
                        state.frame_start = 0;
                        state.frame_end = 0;
                        state.frame_start_str.clear();
                        state.frame_end_str.clear();
                     }
                     
                     state.completion_time = None; 
                     state.current_progress = 0;
                     state.encoded_frames = 0;
                     state.export_mode = mode;
                 }
            });
        });
        ui.add_space(5.0);

        match state.export_mode {
            ExportMode::Manual => {
                ui.add_enabled_ui(!ui_locked, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;

                        let start_hint = egui::RichText::new("0").color(egui::Color32::GRAY);
                        let r1 = ui.add(egui::TextEdit::singleline(&mut state.frame_start_str).hint_text(start_hint).desired_width(40.0));
                        if state.frame_start_str.trim().is_empty() { state.frame_start = 0; } else if let Ok(val) = state.frame_start_str.trim().parse::<i32>() { state.frame_start = val; }
                        
                        ui.label("f");
                        ui.add_space(5.0); 
                        ui.label("~");
                        ui.add_space(5.0); 
                        
                        let hint_val = anim.map_or(0, |a| a.max_frame);
                        let end_hint = egui::RichText::new(hint_val.to_string()).color(egui::Color32::GRAY);
                        let r2 = ui.add(egui::TextEdit::singleline(&mut state.frame_end_str).hint_text(end_hint).desired_width(40.0));
                        if state.frame_end_str.trim().is_empty() { state.frame_end = hint_val; } else if let Ok(val) = state.frame_end_str.trim().parse::<i32>() { state.frame_end = val; }
                        
                        ui.label("f");
    
                        if r1.changed() || r2.changed() {
                            state.completion_time = None;
                            state.current_progress = 0;
                            state.encoded_frames = 0;
                        }
                    });
                });
            },
            ExportMode::Loop => {
                // Locked when locked
                ui.add_enabled_ui(!ui_locked, |ui| {
                    egui::Grid::new("loop_settings_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                        // Row 1: Tolerance
                        ui.label("Loop Tolerance");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                            let hint = egui::RichText::new("30").color(egui::Color32::GRAY);
                            if ui.add(egui::TextEdit::singleline(&mut state.loop_tolerance_str).hint_text(hint).desired_width(40.0)).changed() {
                                if state.loop_tolerance_str.trim().is_empty() { state.loop_tolerance = 30; }
                                else if let Ok(v) = state.loop_tolerance_str.parse::<i32>() { state.loop_tolerance = v; }
                            }
                            ui.label("%");
                        });
                        ui.end_row();
                        
                        // Row 2: Minimum
                        ui.label("Loop Minimum");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                            let hint = egui::RichText::new("15").color(egui::Color32::GRAY);
                            if ui.add(egui::TextEdit::singleline(&mut state.loop_min_str).hint_text(hint).desired_width(40.0)).changed() {
                                if state.loop_min_str.trim().is_empty() { state.loop_min = 15; }
                                else if let Ok(v) = state.loop_min_str.parse::<i32>() { state.loop_min = v; }
                            }
                            ui.label("f");
                        });
                        ui.end_row();

                        // Row 3: Maximum
                        ui.label("Loop Maximum");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                            let hint = egui::RichText::new("None").color(egui::Color32::GRAY);
                            if ui.add(egui::TextEdit::singleline(&mut state.loop_max_str).hint_text(hint).desired_width(40.0)).changed() {
                                 if state.loop_max_str.trim().is_empty() {
                                     state.loop_max = None;
                                 } else if let Ok(v) = state.loop_max_str.parse::<i32>() { 
                                     state.loop_max = Some(v); 
                                 }
                            }
                            ui.label("f");
                        });
                        ui.end_row();
                    });
                });

                // Locked Frames Fields
                ui.add_space(5.0);
                ui.add_enabled_ui(!ui_locked, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                        let mut start = state.frame_start.to_string();
                        let mut end = state.frame_end.to_string();
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut start).desired_width(40.0));
                        ui.add_enabled(false, egui::Label::new("f"));
                        ui.add_space(5.0);
                        ui.add_enabled(false, egui::Label::new("~"));
                        ui.add_space(5.0);
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut end).desired_width(40.0));
                        ui.add_enabled(false, egui::Label::new("f"));
                    });
                });
                
                ui.add_space(5.0);
                if state.is_loop_searching {
                        let btn = egui::Button::new("Abort Loop").fill(egui::Color32::from_rgb(180, 50, 50));
                        if ui.add_sized(egui::vec2(ui.available_width(), 24.0), btn).clicked() {
                            if let Some(flag) = &state.loop_abort {
                                flag.store(true, Ordering::Relaxed);
                            }
                        }
                } else {
                    ui.add_enabled_ui(!state.is_processing, |ui| {
                        if ui.add_sized(egui::vec2(ui.available_width(), 24.0), egui::Button::new("Find Loop")).clicked() {
                                if let (Some(m), Some(a)) = (model, anim) {
                                    let use_tol = if state.loop_tolerance_str.is_empty() { 30 } else { state.loop_tolerance_str.parse().unwrap_or(30) };
                                    let use_min = if state.loop_min_str.is_empty() { 15 } else { state.loop_min_str.parse().unwrap_or(15) };
                                    let use_max = state.loop_max; 

                                    state.loop_tolerance = use_tol;
                                    state.loop_min = use_min;

                                    let (tx, rx) = std::sync::mpsc::channel();
                                    state.loop_rx = Some(rx);
                                    state.is_loop_searching = true;
                                    
                                    state.loop_frames_searched = 0;
                                    state.loop_search_start_time = Some(ui.input(|i| i.time));
                                    
                                    let abort = Arc::new(AtomicBool::new(false));
                                    state.loop_abort = Some(abort.clone());
                                    
                                    findloop::start_search(m.clone(), a.clone(), use_tol, use_min, use_max, tx, abort);
                                }
                        }
                    });
                }
            },
            ExportMode::Showcase => {
                ui.add_enabled_ui(!ui_locked, |ui| {
                    let hint_90 = egui::RichText::new("90").color(egui::Color32::GRAY);
                    egui::Grid::new("showcase_grid").spacing([10.0, 4.0]).show(ui, |ui| {
                        ui.label("Walk");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                            if ui.add(egui::TextEdit::singleline(&mut state.showcase_walk_str).hint_text(hint_90.clone()).desired_width(50.0)).changed() {
                                state.showcase_walk_len = state.showcase_walk_str.trim().parse().unwrap_or(if state.showcase_walk_str.trim().is_empty() { 90 } else { 0 });
                                state.completion_time = None;
                            }
                            if state.showcase_walk_str.trim().is_empty() { state.showcase_walk_len = 90; }
                            ui.label("f");
                        });
                        ui.end_row();
    
                        ui.label("Idle");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                            if ui.add(egui::TextEdit::singleline(&mut state.showcase_idle_str).hint_text(hint_90.clone()).desired_width(50.0)).changed() {
                                state.showcase_idle_len = state.showcase_idle_str.trim().parse().unwrap_or(if state.showcase_idle_str.trim().is_empty() { 90 } else { 0 });
                                state.completion_time = None;
                            }
                            if state.showcase_idle_str.trim().is_empty() { state.showcase_idle_len = 90; }
                            ui.label("f");
                        });
                        ui.end_row();
    
                        ui.label("Attack");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                            let hint_atk = egui::RichText::new(state.detected_attack_len.to_string()).color(egui::Color32::GRAY);
                            if ui.add(egui::TextEdit::singleline(&mut state.showcase_attack_str).hint_text(hint_atk).desired_width(50.0)).changed() {
                                state.showcase_attack_len = state.showcase_attack_str.trim().parse().unwrap_or(if state.showcase_attack_str.trim().is_empty() { state.detected_attack_len } else { 0 });
                                state.completion_time = None;
                            }
                            if state.showcase_attack_str.trim().is_empty() { state.showcase_attack_len = state.detected_attack_len; }
                            ui.label("f");
                        });
                        ui.end_row();
    
                        ui.label("Knockback");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                            if ui.add(egui::TextEdit::singleline(&mut state.showcase_kb_str).hint_text(hint_90.clone()).desired_width(50.0)).changed() {
                                state.showcase_kb_len = state.showcase_kb_str.trim().parse().unwrap_or(if state.showcase_kb_str.trim().is_empty() { 90 } else { 0 });
                                state.completion_time = None;
                            }
                            if state.showcase_kb_str.trim().is_empty() { state.showcase_kb_len = 90; }
                            ui.label("f");
                        });
                        ui.end_row();
                    });
                });
            }
        }

        ui.add_space(20.0);
        ui.heading("Camera"); 
        ui.add_space(5.0);

        ui.add_enabled_ui(!ui_locked, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Set Camera").on_hover_text("Right-click and drag on the viewport to select area").clicked() { *start_region_selection = true; *is_open = false; }
                
                if ui.button("Use Bounds").on_hover_text("Auto-calculate camera from unit size").clicked() { 
                    let mut calculated = false;
                    if let (Some(m), Some(s)) = (model, sheet) {
                        if let Some(bounds) = bounds::calculate_tight_bounds(m, anim, s) {
                            state.region_x = bounds.min.x;
                            state.region_y = bounds.min.y;
                            state.region_w = bounds.width();
                            state.region_h = bounds.height();
                            state.zoom = 1.0;
                            calculated = true;
                        }
                    }

                    if !calculated {
                        state.region_x = 0.0; 
                        state.region_y = 0.0; 
                        state.region_w = 0.0; 
                        state.region_h = 0.0; 
                        state.zoom = 1.0; 
                    }
                }
            });
            ui.add_space(5.0);
            
            // Camera Grid
            egui::Grid::new("camera_grid")
                .num_columns(4)
                .spacing([10.0, 4.0])
                .min_col_width(CAMERA_COLUMN_WIDTH) 
                .show(ui, |ui| {
                    ui.label("X"); ui.add(egui::DragValue::new(&mut state.region_x).speed(1.0));
                    ui.label("Y"); ui.add(egui::DragValue::new(&mut state.region_y).speed(1.0));
                    ui.end_row();

                    ui.label("W"); ui.add(egui::DragValue::new(&mut state.region_w).range(0.0..=10000.0).speed(1.0));
                    ui.label("H"); ui.add(egui::DragValue::new(&mut state.region_h).range(0.0..=10000.0).speed(1.0));
                    ui.end_row();
                });
        });

        ui.add_space(20.0);
        ui.heading("Output"); 
        ui.add_space(5.0);

        ui.add_enabled_ui(!ui_locked, |ui| {
            egui::Grid::new("out_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    // NAME
                    ui.label("Name");
                    let (disp_start, disp_end) = if state.export_mode == ExportMode::Showcase {
                         let total = state.showcase_walk_len + state.showcase_idle_len + state.showcase_attack_len + state.showcase_kb_len;
                         let end_disp = if total > 0 { total - 1 } else { 0 };
                         (0, end_disp)
                    } else { (state.frame_start, state.frame_end) };
    
                    let range_part = if disp_start == disp_end { format!("{}f", disp_start) } else { format!("{}f~{}f", disp_start, disp_end) };
                    let clean_prefix = state.name_prefix.replace("_0", "").replace("_f", "-1").replace("_c", "-2").replace("_s", "-3");
                    let prefix_display = if state.export_mode == ExportMode::Showcase {
                         let p: Vec<&str> = clean_prefix.split('.').collect();
                         if !p.is_empty() { format!("{}.showcase", p[0]) } else { "unit.showcase".to_string() }
                    } else { clean_prefix.clone() };
    
                    let hint_str = if prefix_display.is_empty() { "animation".to_string() } else { format!("{}.{}", prefix_display, range_part) };
                    ui.add(egui::TextEdit::singleline(&mut state.file_name).hint_text(egui::RichText::new(&hint_str).color(egui::Color32::GRAY)).desired_width(120.0));
                    ui.end_row();

                    // FORMAT
                    ui.label("Format");
                    egui::ComboBox::from_id_salt("fmt_combo")
                        .width(60.0) 
                        .selected_text(match state.format {
                            ExportFormat::Gif => "GIF", 
                            ExportFormat::WebP => "WebP", 
                            ExportFormat::Avif => "AVIF", 
                            ExportFormat::Png => "PNG", 
                            ExportFormat::Mp4 => "MP4",
                            ExportFormat::Mkv => "MKV",
                            ExportFormat::Webm => "WebM",
                            ExportFormat::Zip => "ZIP",
                        }).show_ui(ui, |ui| {
                            // GIF
                            ui.selectable_value(&mut state.format, ExportFormat::Gif, "GIF");
                            // WebP
                            ui.selectable_value(&mut state.format, ExportFormat::WebP, "WebP");
                            // AVIF
                            let avif_installed = toolpaths::avifenc_status() == Presence::Installed;
                            let avif_btn = ui.add_enabled(avif_installed, egui::SelectableLabel::new(state.format == ExportFormat::Avif, "AVIF"));
                            if avif_btn.clicked() { state.format = ExportFormat::Avif; }
                            if !avif_installed { avif_btn.on_disabled_hover_text("Requires AVIFENC Add-On"); }
                            
                            // PNG
                            let ffmpeg_installed = toolpaths::ffmpeg_status() == Presence::Installed;
                            let png_btn = ui.add_enabled(ffmpeg_installed, egui::SelectableLabel::new(state.format == ExportFormat::Png, "PNG"));
                            if png_btn.clicked() { state.format = ExportFormat::Png; }
                            if !ffmpeg_installed { png_btn.on_disabled_hover_text("Requires FFMPEG Add-On"); }

                            // VIDEO
                            let mp4_btn = ui.add_enabled(ffmpeg_installed, egui::SelectableLabel::new(state.format == ExportFormat::Mp4, "MP4"));
                            if mp4_btn.clicked() { state.format = ExportFormat::Mp4; }
                            if !ffmpeg_installed { mp4_btn.on_disabled_hover_text("Requires FFMPEG Add-On"); }

                            let mkv_btn = ui.add_enabled(ffmpeg_installed, egui::SelectableLabel::new(state.format == ExportFormat::Mkv, "MKV"));
                            if mkv_btn.clicked() { state.format = ExportFormat::Mkv; }
                            if !ffmpeg_installed { mkv_btn.on_disabled_hover_text("Requires FFMPEG Add-On"); }

                            let webm_btn = ui.add_enabled(ffmpeg_installed, egui::SelectableLabel::new(state.format == ExportFormat::Webm, "WebM"));
                            if webm_btn.clicked() { state.format = ExportFormat::Webm; }
                            if !ffmpeg_installed { webm_btn.on_disabled_hover_text("Requires FFMPEG Add-On"); }

                            // ZIP
                            ui.selectable_value(&mut state.format, ExportFormat::Zip, "ZIP");
                        });
                    ui.end_row();

                    // CHECK INSTALLED TOOLS
                    let ffmpeg_installed = toolpaths::ffmpeg_status() == Presence::Installed;
                    let avif_installed = toolpaths::avifenc_status() == Presence::Installed;

                    // QUALITY
                    let qual_tip = "Quality percentage dictates image quality, with lower quality correlating with lower file size";
                    let (qual_enabled, qual_reason) = match state.format {
                        ExportFormat::WebP | ExportFormat::Gif | ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm => 
                            (ffmpeg_installed, if !ffmpeg_installed { "Requires FFMPEG (Settings > Add-Ons)" } else { qual_tip }),
                        ExportFormat::Avif => 
                            (avif_installed, if !avif_installed { "Requires AVIFENC (Settings > Add-Ons)" } else { qual_tip }),
                        _ => (false, "Not available for this File Type"),
                    };
                    
                    if qual_enabled {
                        ui.label("Quality").on_hover_text(qual_reason);
                    } else {
                        ui.add_enabled(false, egui::Label::new("Quality")).on_disabled_hover_text(qual_reason);
                    }
                    
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                        if qual_enabled {
                            let hint = egui::RichText::new("100").color(egui::Color32::GRAY);
                            if ui.add(egui::TextEdit::singleline(&mut state.quality_percent_str).hint_text(hint).desired_width(40.0)).on_hover_text(qual_reason).changed() {
                                if state.quality_percent_str.trim().is_empty() {
                                    state.quality_percent = 100;
                                } else if let Ok(v) = state.quality_percent_str.parse::<i32>() { 
                                    state.quality_percent = v.clamp(0, 100); 
                                }
                            }
                            ui.label("%").on_hover_text(qual_reason);
                        } else {
                            let mut na = "N/A".to_string();
                            ui.add_enabled(false, egui::TextEdit::singleline(&mut na).desired_width(40.0)).on_disabled_hover_text(qual_reason);
                        }
                    });
                    ui.end_row();

                    // COMPRESSION
                    let comp_tip = "Compression percentage dictates file size, with higher compression correlating with slower encoding speeds";
                    let (comp_enabled, comp_reason) = match state.format {
                        ExportFormat::WebP | ExportFormat::Gif | ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm => 
                            (ffmpeg_installed, if !ffmpeg_installed { "Requires FFMPEG (Settings > Add-Ons)" } else { comp_tip }),
                        ExportFormat::Avif => 
                            (avif_installed, if !avif_installed { "Requires AVIFENC (Settings > Add-Ons)" } else { comp_tip }),
                        ExportFormat::Zip => (true, comp_tip),
                        _ => (false, "Not available for this File Type"),
                    };

                    if comp_enabled {
                        ui.label("Compression").on_hover_text(comp_reason);
                    } else {
                        ui.add_enabled(false, egui::Label::new("Compression")).on_disabled_hover_text(comp_reason);
                    }
                    
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = EXPORT_MODE_SPACING;
                        if comp_enabled {
                            let hint = egui::RichText::new("0").color(egui::Color32::GRAY);
                            if ui.add(egui::TextEdit::singleline(&mut state.compression_percent_str).hint_text(hint).desired_width(40.0)).on_hover_text(comp_reason).changed() {
                                if state.compression_percent_str.trim().is_empty() {
                                    state.compression_percent = 0;
                                } else if let Ok(v) = state.compression_percent_str.parse::<i32>() { 
                                    state.compression_percent = v.clamp(0, 100); 
                                }
                            }
                            ui.label("%").on_hover_text(comp_reason); 
                        } else {
                            let mut na = "N/A".to_string();
                            ui.add_enabled(false, egui::TextEdit::singleline(&mut na).desired_width(40.0)).on_disabled_hover_text(comp_reason);
                        }
                    });
                    ui.end_row();
            });
            
            // BACKGROUND LOGIC
            ui.horizontal(|ui| { 
                let is_forced_opaque = matches!(state.format, ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm);
                
                if is_forced_opaque {
                    if !state.background { state.background = true; }
                    let mut dummy = true;
                    ui.add_enabled_ui(false, |ui| {
                        toggle_ui(ui, &mut dummy);
                    }).response.on_disabled_hover_text("This video format requires a background");
                } else {
                    if toggle_ui(ui, &mut state.background).changed() {
                        state.user_bg_preference = state.background;
                    }
                    if state.background && !state.user_bg_preference {
                        state.background = false;
                    }
                }
                
                ui.label("Background").on_hover_text("Adds a gray background to the image"); 
            });
            
            // It doesnt do anything rn, honestly interpolation looks buggy, why would you want to export it?
            // ui.horizontal(|ui| { toggle_ui(ui, &mut state.interpolation); ui.label("Interpolation"); });
        });

        ui.add_space(20.0);
        ui.heading("Add-Ons");
        ui.add_space(5.0);
        ui.label("Tools that enhance the Exporters functionality");
        ui.add_space(8.0);

        // FFMPEG Status
        let ffmpeg_installed = toolpaths::ffmpeg_status() == Presence::Installed;
        let ffmpeg_text = if ffmpeg_installed { "FFMPEG Installed" } else { "FFMPEG Missing" };
        let ffmpeg_color = if ffmpeg_installed { egui::Color32::from_rgb(40, 160, 40) } else { egui::Color32::from_rgb(180, 50, 50) };

        let ffmpeg_resp = egui::Frame::none()
            .fill(ffmpeg_color)
            .rounding(egui::Rounding::same(5.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                    ui.set_min_height(24.0);
                    ui.label(egui::RichText::new(ffmpeg_text).color(egui::Color32::WHITE).strong());
                });
            }).response;
            
        if !ffmpeg_installed {
            ffmpeg_resp.on_hover_text("Download at Settings > Add-Ons > FFMPEG");
        }

        ui.add_space(5.0);
        // AVIFENC Status
        let avif_installed = toolpaths::avifenc_status() == Presence::Installed;
        let avif_text = if avif_installed { "AVIFENC Installed" } else { "AVIFENC Missing" };
        let avif_color = if avif_installed { egui::Color32::from_rgb(40, 160, 40) } else { egui::Color32::from_rgb(180, 50, 50) };
        
        let avif_resp = egui::Frame::none()
            .fill(avif_color)
            .rounding(egui::Rounding::same(5.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                    ui.set_min_height(24.0);
                    ui.label(egui::RichText::new(avif_text).color(egui::Color32::WHITE).strong());
                });
            }).response;
            
        if !avif_installed {
            avif_resp.on_hover_text("Download at Settings > Add-Ons > AVIFENC");
        }
        
        ui.add_space(5.0);

    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(5.0); 
        
        let count = (state.frame_end - state.frame_start).abs() + 1;
        
        let (progress_val, label_text) = if state.is_loop_searching {
            let start = state.loop_search_start_time.unwrap_or(0.0);
            let p_anim = ((ui.input(|i| i.time) - start) % 1.0) as f32;
            (p_anim, format!("Searching | {} frames", state.loop_frames_searched))
        } else if state.is_processing {
            if state.current_progress < count {
                let ratio = if count == 0 { 0.0 } else { (state.current_progress as f32 / count as f32).min(1.0) };
                let percent = (ratio * 100.0) as i32;
                (ratio, format!("Rendering | {}f/{}f ({}%)", state.current_progress, count, percent))
            } else {
                let ratio = if count == 0 { 0.0 } else { (state.encoded_frames as f32 / count as f32).min(1.0) };
                let percent = (ratio * 100.0) as i32;
                (ratio, format!("Encoding | {}f/{}f ({}%)", state.encoded_frames, count, percent))
            }
        } else {
            match state.completion_time {
                Some(done_time) => {
                    let is_focused = ui.input(|i| i.focused);
                    let seen_id = egui::Id::new("export_done_seen");
                    let mut has_seen = ui.ctx().data(|d| d.get_temp(seen_id).unwrap_or(false));

                    if is_focused && !has_seen {
                        has_seen = true;
                        ui.ctx().data_mut(|d| d.insert_temp(seen_id, true));
                    }

                    let label = state.loop_result_msg.clone().unwrap_or_else(|| "Done".to_string());

                    if !has_seen && !is_focused {
                        state.completion_time = Some(ui.input(|i| i.time));
                        ui.ctx().request_repaint(); 
                        (1.0, label)
                    } else {
                        let elapsed = ui.input(|i| i.time) - done_time;
                        if elapsed < 3.0 { 
                            ui.ctx().request_repaint(); 
                            (1.0, label) 
                        } 
                        else { 
                            state.completion_time = None; 
                            state.loop_result_msg = None;
                            (1.0, "Ready".to_string()) 
                        }
                    }
                },
                None => {
                    let ratio = if count == 0 { 0.0 } else { (state.current_progress as f32 / count as f32).min(1.0) };
                    if ratio > 0.0 && ratio < 1.0 { 
                         let percent = (ratio * 100.0) as i32;
                        (ratio, format!("Paused | {}f/{}f ({}%)", state.current_progress, count, percent)) 
                    } else { 
                        (1.0, "Ready".to_string()) 
                    }
                }
            }
        };

        ui.add(egui::ProgressBar::new(progress_val));
        ui.label(label_text);
        
        ui.add_space(5.0); 
        
        if state.is_processing {
             let btn = egui::Button::new("Abort Export").fill(egui::Color32::from_rgb(180, 50, 50));
             if ui.add_sized(egui::vec2(ui.available_width(), 30.0), btn).clicked() {
                 // ACTIVATE THE SIGNAL
                 if let Some(abort) = &state.abort {
                     abort.store(true, Ordering::Relaxed);
                 }
                 
                 state.is_processing = false; 
                 state.current_progress = 0; 
                 state.encoded_frames = 0;
                 state.completion_time = None;
             }
        } else {
            let is_valid = state.region_w > 0.1 && state.region_h > 0.1;
            let enabled = !state.is_loop_searching && is_valid;
            
            let btn_text = if is_valid { "Begin Export" } else { "No Camera Set" };
            
            if ui.add_enabled_ui(enabled, |ui| {
                ui.add_sized(egui::vec2(ui.available_width(), 30.0), egui::Button::new(btn_text))
            }).inner.clicked() { 
                start_export(state); 
            }
        }
        
        ui.add_space(5.0); ui.separator(); 
    });
}