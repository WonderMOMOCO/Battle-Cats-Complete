use std::fs;
use std::io::{Cursor, Write, BufWriter};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};

use eframe::egui;
use eframe::glow::{self, HasContext};
use image::RgbaImage;
use zip::write::FileOptions;
use webp_animation::Encoder as WebpEncoder;
use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat as GifRepeat, DisposalMethod};

use crate::core::anim::canvas::GlowRenderer;
use crate::data::global::imgcut::SpriteSheet;
use crate::core::anim::transform::WorldTransform;

// SHARED DATA STRUCTURES
#[derive(Clone, Debug)]
pub struct ExportConfig {
    pub width: u32,
    pub height: u32,
    #[allow(dead_code)] pub camera_x: f32,
    #[allow(dead_code)] pub camera_y: f32,
    #[allow(dead_code)] pub camera_zoom: f32,
    pub format: ExportFormat,
    #[allow(dead_code)] pub quality_percent: u32, 
    pub compression_percent: u32,
    pub fps: u32,
    pub start_frame: i32,
    pub end_frame: i32,
    #[allow(dead_code)] pub interpolation: bool,
    pub output_path: PathBuf,
    pub base_name: String, 
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportFormat { 
    Gif, 
    WebP, 
    Avif, 
    Png,
    Mp4, 
    Mkv,
    Webm,
    Zip
}

pub enum EncoderMessage {
    Frame(Vec<u8>, u32, u32, u32),
    Finish,
}

#[derive(Debug, Clone)]
pub enum EncoderStatus {
    #[allow(dead_code)] Encoding, 
    Progress(u32),
    Finished,
}

// NATIVE WORKER
pub fn encode_native(
    config: ExportConfig, 
    rx: mpsc::Receiver<EncoderMessage>, 
    status_tx: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort: Arc<AtomicBool>
) -> bool {
    let mut frames_processed = 0;
    let mut success = false;

    match config.format {
        ExportFormat::Gif => {
            if let Ok(file) = fs::File::create(temp_path) {
                let mut writer = BufWriter::new(file);
                if let Ok(mut encoder) = GifEncoder::new(&mut writer, config.width as u16, config.height as u16, &[]) {
                    let _ = encoder.set_repeat(GifRepeat::Infinite);
                    while let Ok(msg) = rx.recv() {
                        if abort.load(Ordering::Relaxed) { return false; }
                        match msg {
                            EncoderMessage::Frame(raw_pixels, w, h, delay_ms) => {
                                let img = prepare_image(raw_pixels, w, h);
                                let mut ticks = (delay_ms as f32 / 10.0).round() as u16;
                                if ticks < 2 { ticks = 2; } 
                                let mut pixels = img.into_vec();
                                for chunk in pixels.chunks_exact_mut(4) {
                                    if chunk[3] < 127 { chunk[0]=0; chunk[1]=0; chunk[2]=0; chunk[3]=0; } 
                                    else { chunk[3]=255; }
                                }
                                let mut frame = GifFrame::from_rgba(config.width as u16, config.height as u16, &mut pixels);
                                frame.dispose = DisposalMethod::Background;
                                frame.delay = ticks;
                                if encoder.write_frame(&frame).is_err() { break; }
                                frames_processed += 1;
                                let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                            },
                            EncoderMessage::Finish => { success = true; break; }
                        }
                    }
                }
            }
        },
        ExportFormat::WebP => {
            if let Ok(mut encoder) = WebpEncoder::new((config.width, config.height)) {
                let mut timestamp_ms = 0;
                while let Ok(msg) = rx.recv() {
                    if abort.load(Ordering::Relaxed) { return false; }
                    match msg {
                        EncoderMessage::Frame(raw_pixels, w, h, delay_ms) => {
                            let img = prepare_image(raw_pixels, w, h);
                            let _ = encoder.add_frame(&img.into_vec(), timestamp_ms);
                            timestamp_ms += delay_ms as i32;
                            frames_processed += 1;
                            let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                        },
                        EncoderMessage::Finish => { success = true; break; }
                    }
                }
                if success && !abort.load(Ordering::Relaxed) {
                    if let Ok(data) = encoder.finalize(timestamp_ms) {
                        success = fs::write(temp_path, data).is_ok();
                    } else { success = false; }
                }
            }
        },
        ExportFormat::Zip => {
            let mut frame_idx = 0;
            let step = if config.start_frame <= config.end_frame { 1 } else { -1 };
            
            // Native ZIP creation
            if let Ok(file) = fs::File::create(temp_path) {
                let mut zip = zip::ZipWriter::new(BufWriter::new(file));
                let method = if config.compression_percent == 0 { zip::CompressionMethod::Stored } else { zip::CompressionMethod::Deflated };
                let options = FileOptions::default().compression_method(method);
                while let Ok(msg) = rx.recv() {
                    if abort.load(Ordering::Relaxed) { return false; }
                    match msg {
                        EncoderMessage::Frame(raw_pixels, w, h, _) => {
                            let img = prepare_image(raw_pixels, w, h);
                            let current_frame = config.start_frame + (frame_idx as i32 * step);
                            let entry_name = format!("{}.{}f.png", config.base_name, current_frame);
                            let _ = zip.start_file(entry_name, options);
                            let mut buffer = Cursor::new(Vec::new());
                            if img.write_to(&mut buffer, image::ImageFormat::Png).is_ok() {
                                let _ = zip.write_all(buffer.get_ref());
                            }
                            frame_idx += 1; frames_processed += 1;
                            let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                        },
                        EncoderMessage::Finish => { success = true; break; },
                    }
                }
                let _ = zip.finish();
            }
        },
        _ => {}
    }
    success
}

pub fn render_frame(
    renderer: &mut GlowRenderer,
    gl: &glow::Context,
    width: u32,
    height: u32,
    parts: &[WorldTransform],
    sheet: &SpriteSheet,
    pan: egui::Vec2,
    zoom: f32,
    bg_color: [u8; 4],
) -> Vec<u8> {
    unsafe {
        gl.disable(glow::SCISSOR_TEST);
        let fbo = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        let tex = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, width as i32, height as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.viewport(0, 0, width as i32, height as i32);
        let (r, g, b, a) = (bg_color[0] as f32 / 255.0, bg_color[1] as f32 / 255.0, bg_color[2] as f32 / 255.0, bg_color[3] as f32 / 255.0);
        gl.clear_color(r, g, b, a);
        gl.clear(glow::COLOR_BUFFER_BIT);
        renderer.paint(gl, egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width as f32, height as f32)), parts, sheet, pan, zoom, true);
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        gl.read_pixels(0, 0, width as i32, height as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut pixels));
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.delete_framebuffer(fbo);
        gl.delete_texture(tex);
        gl.enable(glow::SCISSOR_TEST);
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 4);
        pixels
    }
}

pub fn prepare_image(mut pixels: Vec<u8>, width: u32, height: u32) -> RgbaImage {
    for chunk in pixels.chunks_exact_mut(4) {
        let alpha = chunk[3];
        if alpha > 0 && alpha < 255 {
            let a = alpha as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 / a).min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 / a).min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 / a).min(255.0) as u8;
        }
    }
    if let Some(img) = RgbaImage::from_raw(width, height, pixels) { image::imageops::flip_vertical(&img) } 
    else { RgbaImage::new(width, height) }
}