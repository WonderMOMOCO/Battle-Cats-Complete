use eframe::egui;
use crate::global::formats::mamodel::Model;
use crate::global::formats::maanim::Animation;
use crate::global::formats::imgcut::SpriteSheet;
use crate::features::animation::export::encoding::{self, ExportConfig, ExportFormat, EncoderMessage, EncoderStatus};
use crate::features::animation::export::state::{ExporterState, ExportMode};
use crate::features::animation::logic::{animator, smooth, transform}; 
use crate::features::animation::logic::canvas::GlowRenderer;
use crate::features::animation::export::leader;
use std::sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, Ordering}};
use std::path::{PathBuf, Path};

pub static STATUS_RX: Mutex<Option<mpsc::Receiver<EncoderStatus>>> = Mutex::new(None);

pub fn start_export(state: &mut ExporterState) {
    if state.is_processing { return; }
    
    state.is_processing = true;
    state.current_progress = 0;
    state.encoded_frames = 0; 
    state.completion_time = None; 
    
    // Initialize the abort signal
    let abort_signal = Arc::new(AtomicBool::new(false));
    state.abort = Some(abort_signal.clone());

    if state.export_mode == ExportMode::Showcase {
        state.frame_start = 0;
        let total_frames = state.showcase_walk_len + state.showcase_idle_len + state.showcase_attack_len + state.showcase_kb_len;
        state.frame_end = if total_frames > 0 { total_frames - 1 } else { 0 }; 
    }

    let (base_name, file_name) = if state.file_name.trim().is_empty() {
        let (display_start, display_end) = if state.export_mode == ExportMode::Showcase {
             let total_frames = state.showcase_walk_len + state.showcase_idle_len + state.showcase_attack_len + state.showcase_kb_len;
             let end_display = if total_frames > 0 { total_frames - 1 } else { 0 };
             (0, end_display)
        } else { 
            (state.frame_start, state.frame_end) 
        };

        let range_part = if display_start == display_end { 
            format!("{}f", display_start) 
        } else { 
            format!("{}f~{}f", display_start, display_end) 
        };
        
        let clean_prefix = state.name_prefix.replace("_0", "").replace("_f", "-1").replace("_c", "-2").replace("_s", "-3");
        
        let prefix_display = if state.export_mode == ExportMode::Showcase {
             let prefix_parts: Vec<&str> = clean_prefix.split('.').collect();
             if !prefix_parts.is_empty() { 
                 format!("{}.showcase", prefix_parts[0]) 
             } else { 
                 "unit.showcase".to_string() 
             }
        } else { 
            clean_prefix.clone() 
        };

        let base = if prefix_display.is_empty() { "animation".to_string() } else { prefix_display };
        let full = format!("{}.{}", base, range_part);
        (base, full)
    } else {
        let path_object = Path::new(&state.file_name);
        let base = path_object.file_stem().unwrap_or(path_object.as_os_str()).to_string_lossy().to_string();
        (base, state.file_name.clone())
    };

    let mut output_path = std::env::current_dir().unwrap_or(PathBuf::from("."));
    output_path.push("exports");
    
    let mut final_file_name = file_name;
    
    let target_extension = match state.format {
        ExportFormat::Gif => Some("gif"), 
        ExportFormat::WebP => Some("webp"), 
        ExportFormat::Avif => Some("avif"), 
        ExportFormat::Png => Some("png"), 
        ExportFormat::Mp4 => Some("mp4"),
        ExportFormat::Mkv => Some("mkv"),
        ExportFormat::Webm => Some("webm"),
        ExportFormat::Zip => Some("zip"),
    };

    if let Some(extension) = target_extension {
        let suffix = format!(".{}", extension);
        if !final_file_name.to_lowercase().ends_with(&suffix) { 
            final_file_name = format!("{}{}", final_file_name, suffix); 
        }
    }
    
    output_path.push(final_file_name);

    let config = ExportConfig {
        width: state.region_w as u32, height: state.region_h as u32,
        camera_x: state.region_x, camera_y: state.region_y, camera_zoom: state.zoom,
        format: state.format.clone(), 
        quality_percent: state.quality_percent as u32,
        compression_percent: state.compression_percent as u32,
        fps: state.fps as u32,
        start_frame: state.frame_start, end_frame: state.frame_end, interpolation: state.interpolation,
        output_path,
        base_name, 
        background: state.background,
    };

    let (sender, receiver) = mpsc::channel();
    let (status_sender, status_receiver) = mpsc::channel();
    
    if let Ok(mut lock) = STATUS_RX.lock() { *lock = Some(status_receiver); }
    
    state.tx = Some(sender);
    leader::start_encoding_thread(config, receiver, status_sender, abort_signal);
}

pub fn process_frame(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut ExporterState,
    model: &Model,
    anim: Option<&Animation>,
    sheet: &SpriteSheet,
    renderer_ref: Arc<Mutex<Option<GlowRenderer>>>,
    current_time: f32, 
) {
    if state.tx.is_none() { return; }

    // CHECK ABORT SIGNAL BEFORE PROCESSING
    if let Some(abort) = &state.abort {
        if abort.load(Ordering::Relaxed) {
            state.tx = None;
            state.abort = None;
            return;
        }
    }

    let frame_count = (state.frame_end - state.frame_start).abs() + 1;
    if state.current_progress >= frame_count {
        if let Some(sender) = state.tx.take() { 
            let _ = sender.send(EncoderMessage::Finish); 
        }
        return;
    }

    let frame_delay_ms = 1000.0 / state.fps as f32;
    
    let parts = if let Some(animation) = anim {
        let raw_frame = if state.export_mode == ExportMode::Showcase {
            current_time
        } else {
            let start = state.frame_start;
            let step = if state.frame_start < state.frame_end { 1 } else { -1 };
            (start + (state.current_progress * step)) as f32
        };
        
        let frame_to_render = if state.export_mode == ExportMode::Showcase {
            let natively_loops = animation.curves.iter().any(|c| c.loop_count != 1);
            if natively_loops {
                raw_frame
            } else {
                let effective_max = animation.curves.iter()
                    .filter_map(|c| c.keyframes.last().map(|k| k.frame))
                    .max()
                    .unwrap_or(0);
                if effective_max > 0 {
                    raw_frame.rem_euclid(effective_max as f32 + 1.0)
                } else {
                    raw_frame
                }
            }
        } else if state.loop_supported {
            raw_frame
        } else if state.max_frame > 0 { 
            raw_frame.rem_euclid(state.max_frame as f32 + 1.0) 
        } else { 
            raw_frame 
        };

        if state.interpolation { 
            smooth::animate(model, animation, frame_to_render) 
        } else { 
            animator::animate(model, animation, frame_to_render) 
        }
    } else { 
        model.parts.clone() 
    };
    
    let world_parts = transform::solve_hierarchy(&parts, model);
    let pan = egui::vec2(-state.region_x - (state.region_w as f32 / (2.0 * state.zoom)), -state.region_y - (state.region_h as f32 / (2.0 * state.zoom)));
    let bg_color = if state.background { [80, 80, 80, 255] } else { [0, 0, 0, 0] };

    let renderer_arc = renderer_ref.clone();
    let sheet_arc = Arc::new(sheet.clone()); 
    let Some(sender) = state.tx.as_ref().cloned() else { return; };
    let (w, h, z) = (state.region_w, state.region_h, state.zoom);
    
    ui.painter().add(egui::PaintCallback {
        rect, 
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_, painter| {
            let Ok(mut lock) = renderer_arc.lock() else { return; };
            let Some(renderer) = lock.as_mut() else { return; };
            
            let raw_pixels = encoding::render_frame(renderer, painter.gl(), w as u32, h as u32, &world_parts, &sheet_arc, pan, z, bg_color);
            let _ = sender.send(EncoderMessage::Frame(raw_pixels, w as u32, h as u32, frame_delay_ms as u32));
        })),
    });

    state.current_progress += 1;
}