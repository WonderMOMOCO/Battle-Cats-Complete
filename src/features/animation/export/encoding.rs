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

use crate::features::animation::logic::canvas::GlowRenderer;
use crate::global::formats::imgcut::SpriteSheet;
use crate::features::animation::logic::transform::WorldTransform;

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
    pub background: bool,
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
    receiver: mpsc::Receiver<EncoderMessage>, 
    status_sender: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort_signal: Arc<AtomicBool>
) -> bool {
    let mut frames_processed = 0;
    let mut is_success = false;

    match config.format {
        ExportFormat::Gif => {
            let Ok(file) = fs::File::create(temp_path) else { return false; };
            let mut buffered_writer = BufWriter::new(file);
            
            let Ok(mut gif_encoder) = GifEncoder::new(&mut buffered_writer, config.width as u16, config.height as u16, &[]) else { return false; };
            let _ = gif_encoder.set_repeat(GifRepeat::Infinite);
            
            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) { return false; }
                
                match message {
                    EncoderMessage::Frame(raw_pixels, w, h, delay_ms) => {
                        let image_data = prepare_image(raw_pixels, w, h, config.background);
                        let mut frame_ticks = (delay_ms as f32 / 10.0).round() as u16;
                        if frame_ticks < 2 { frame_ticks = 2; } 
                        
                        let mut pixel_buffer = image_data.into_vec();
                        if !config.background {
                            for chunk in pixel_buffer.chunks_exact_mut(4) {
                                if chunk[3] < 127 { 
                                    chunk[0] = 0; chunk[1] = 0; chunk[2] = 0; chunk[3] = 0; 
                                } else { 
                                    chunk[3] = 255; 
                                }
                            }
                        }
                        
                        let mut gif_frame = GifFrame::from_rgba(config.width as u16, config.height as u16, &mut pixel_buffer);
                        gif_frame.dispose = DisposalMethod::Background;
                        gif_frame.delay = frame_ticks;
                        
                        if gif_encoder.write_frame(&gif_frame).is_err() { break; }
                        frames_processed += 1;
                        let _ = status_sender.send(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => { is_success = true; break; }
                }
            }
        },
        ExportFormat::WebP => {
            let Ok(mut webp_encoder) = WebpEncoder::new((config.width, config.height)) else { return false; };
            let mut timestamp_ms = 0;
            
            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) { return false; }
                
                match message {
                    EncoderMessage::Frame(raw_pixels, w, h, delay_ms) => {
                        let image_data = prepare_image(raw_pixels, w, h, config.background);
                        let _ = webp_encoder.add_frame(&image_data.into_vec(), timestamp_ms);
                        timestamp_ms += delay_ms as i32;
                        frames_processed += 1;
                        let _ = status_sender.send(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => { is_success = true; break; }
                }
            }
            
            if is_success && !abort_signal.load(Ordering::Relaxed) {
                let Ok(final_data) = webp_encoder.finalize(timestamp_ms) else { return false; };
                is_success = fs::write(temp_path, final_data).is_ok();
            } else {
                is_success = false; 
            }
        },
        ExportFormat::Zip => {
            let mut frame_index = 0;
            let step_direction = if config.start_frame <= config.end_frame { 1 } else { -1 };
            
            // Native ZIP creation
            let Ok(file) = fs::File::create(temp_path) else { return false; };
            let mut zip_writer = zip::ZipWriter::new(BufWriter::new(file));
            
            let compression_method = if config.compression_percent == 0 { 
                zip::CompressionMethod::Stored 
            } else { 
                zip::CompressionMethod::Deflated 
            };
            
            let zip_options = FileOptions::default().compression_method(compression_method);
            
            while let Ok(message) = receiver.recv() {
                if abort_signal.load(Ordering::Relaxed) { return false; }
                
                match message {
                    EncoderMessage::Frame(raw_pixels, w, h, _) => {
                        let image_data = prepare_image(raw_pixels, w, h, config.background);
                        let current_frame = config.start_frame + (frame_index as i32 * step_direction);
                        let entry_name = format!("{}.{}f.png", config.base_name, current_frame);
                        
                        let _ = zip_writer.start_file(entry_name, zip_options);
                        let mut memory_buffer = Cursor::new(Vec::new());
                        
                        if image_data.write_to(&mut memory_buffer, image::ImageFormat::Png).is_ok() {
                            let _ = zip_writer.write_all(memory_buffer.get_ref());
                        }
                        
                        frame_index += 1; 
                        frames_processed += 1;
                        let _ = status_sender.send(EncoderStatus::Progress(frames_processed));
                    },
                    EncoderMessage::Finish => { is_success = true; break; },
                }
            }
            let _ = zip_writer.finish();
        },
        _ => {}
    }
    is_success
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
        
        let framebuffer = gl.create_framebuffer().expect("Failed to create OpenGL framebuffer");
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        
        let texture = gl.create_texture().expect("Failed to create OpenGL texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, width as i32, height as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(texture), 0);
        gl.bind_texture(glow::TEXTURE_2D, None);
        
        gl.viewport(0, 0, width as i32, height as i32);
        
        let (r, g, b, a) = (
            bg_color[0] as f32 / 255.0, 
            bg_color[1] as f32 / 255.0, 
            bg_color[2] as f32 / 255.0, 
            bg_color[3] as f32 / 255.0
        );
        gl.clear_color(r, g, b, a);
        gl.clear(glow::COLOR_BUFFER_BIT);
        
        renderer.paint(gl, egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width as f32, height as f32)), parts, sheet, pan, zoom, true);
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        
        let mut pixel_buffer = vec![0u8; (width * height * 4) as usize];
        gl.read_pixels(0, 0, width as i32, height as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut pixel_buffer));
        
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.delete_framebuffer(framebuffer);
        gl.delete_texture(texture);
        
        gl.enable(glow::SCISSOR_TEST);
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 4);
        
        pixel_buffer
    }
}

pub fn prepare_image(mut pixel_buffer: Vec<u8>, width: u32, height: u32, is_opaque_bg: bool) -> RgbaImage {
    for chunk in pixel_buffer.chunks_exact_mut(4) {
        if is_opaque_bg {
            // Force fully opaque so encoders don't apply 1-bit transparency logic to anti-aliased edges
            chunk[3] = 255;
        } else {
            let alpha_value = chunk[3];
            if alpha_value > 0 && alpha_value < 255 {
                let float_alpha = alpha_value as f32 / 255.0;
                chunk[0] = (chunk[0] as f32 / float_alpha).min(255.0) as u8;
                chunk[1] = (chunk[1] as f32 / float_alpha).min(255.0) as u8;
                chunk[2] = (chunk[2] as f32 / float_alpha).min(255.0) as u8;
            }
        }
    }
    
    let Some(image_buffer) = RgbaImage::from_raw(width, height, pixel_buffer) else { 
        return RgbaImage::new(width, height); 
    };
    
    image::imageops::flip_vertical(&image_buffer)
}