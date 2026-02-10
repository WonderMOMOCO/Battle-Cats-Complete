use eframe::egui;
use std::path::{Path};
use std::fs;
use std::thread;
use std::sync::{Arc, Mutex, mpsc::{self, Receiver}};
use std::collections::HashMap;
use crate::core::utils;

#[derive(Clone, Debug)]
pub struct SpriteCut {
    pub uv_coordinates: egui::Rect,
    pub original_size: egui::Vec2,
    #[allow(dead_code)] pub name: String,
}

impl Clone for SpriteSheet {
    fn clone(&self) -> Self {
        Self {
            texture_handle: self.texture_handle.clone(),
            image_data: self.image_data.clone(),
            cuts_map: self.cuts_map.clone(),
            is_loading_active: self.is_loading_active,
            data_receiver: None,
            sheet_name: self.sheet_name.clone(),
        }
    }
}

pub struct SpriteSheet {
    pub texture_handle: Option<egui::TextureHandle>,
    // Raw image data for Custom GL Renderer
    pub image_data: Option<Arc<egui::ColorImage>>, 
    pub cuts_map: HashMap<usize, SpriteCut>, 
    pub is_loading_active: bool,
    pub data_receiver: Option<Mutex<Receiver<(String, egui::ColorImage, HashMap<usize, SpriteCut>)>>>,
    pub sheet_name: String, 
}

impl Default for SpriteSheet {
    fn default() -> Self {
        Self {
            texture_handle: None,
            image_data: None,
            cuts_map: HashMap::new(),
            is_loading_active: false,
            data_receiver: None,
            sheet_name: String::new(),
        }
    }
}

#[allow(dead_code)]
impl SpriteSheet {
    pub fn is_ready(&self) -> bool {
        self.texture_handle.is_some()
    }

    pub fn load(&mut self, ctx: &egui::Context, png_path: &Path, imgcut_path: &Path, id_str: String) {
        if self.is_loading_active { return; }
        
        self.is_loading_active = true;
        let ctx_clone = ctx.clone();
        let png_path_buf = png_path.to_path_buf();
        let cut_path_buf = imgcut_path.to_path_buf();
        
        let (sender, receiver) = mpsc::channel();
        self.data_receiver = Some(Mutex::new(receiver));

        thread::spawn(move || {
            if let Some((image, cuts)) = Self::load_internal(&png_path_buf, &cut_path_buf) {
                let _ = sender.send((id_str, image, cuts));
                ctx_clone.request_repaint();
            }
        });
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        if let Some(mutex) = &self.data_receiver {
            if let Ok(receiver) = mutex.try_lock() {
                if let Ok((name, image, cuts)) = receiver.try_recv() {
                    self.sheet_name = name.clone(); 
                    self.texture_handle = Some(ctx.load_texture(&name, image.clone(), Default::default()));
                    self.image_data = Some(Arc::new(image));
                    self.cuts_map = cuts;
                    self.is_loading_active = false;
                }
            }
        }
        
        // Cleanup receiver if done
        if !self.is_loading_active && self.data_receiver.is_some() {
            self.data_receiver = None;
        }
    }

    fn load_internal(png_path: &Path, cut_path: &Path) -> Option<(egui::ColorImage, HashMap<usize, SpriteCut>)> {
        // Load Image
        let image_data = fs::read(png_path).ok()?;
        let image = image::load_from_memory(&image_data).ok()?;
        let size = [image.width() as usize, image.height() as usize];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        let egui_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        // Load ImgCut
        let content = fs::read_to_string(cut_path).ok()?;
        let delimiter = utils::detect_csv_separator(&content);
        let lines: Vec<&str> = content.lines().filter(|line| !line.trim().is_empty()).collect();

        // Find Header
        let mut sprite_count = 0;
        let mut data_start_index = 0;
        let mut found_header = false;

        for (index, line) in lines.iter().enumerate() {
            if !line.contains(',') {
                if let Ok(count_val) = line.trim().parse::<usize>() {
                    // Sanity check: valid sprite counts are usually between 1 and 5000
                    if count_val > 0 && count_val < 5000 {
                        sprite_count = count_val;
                        data_start_index = index + 1;
                        found_header = true;
                    }
                }
            } else if found_header { 
                break; 
            }
        }

        if !found_header || sprite_count == 0 {
            data_start_index = 0;
            sprite_count = lines.len();
        }

        // Parse Cuts
        let w = size[0] as f32;
        let h = size[1] as f32;
        let mut parsed_cuts = HashMap::new();

        for i in 0..sprite_count {
            let line_index = data_start_index + i;
            if line_index >= lines.len() { break; }
            
            let line = lines[line_index];
            let parts: Vec<&str> = line.split(delimiter).collect();
            
            let sprite_id = i; 

            // We strictly need at least 4 columns (x, y, w, h).
            if parts.len() >= 4 {
                if let (Ok(x), Ok(y), Ok(cut_width), Ok(cut_height)) = (
                    parts[0].trim().parse::<f32>(),
                    parts[1].trim().parse::<f32>(),
                    parts[2].trim().parse::<f32>(),
                    parts[3].trim().parse::<f32>(),
                ) {
                    let uv_min = egui::pos2(x / w, y / h);
                    let uv_max = egui::pos2((x + cut_width) / w, (y + cut_height) / h);
                    
                    // Check if 5th column exists
                    let cut_name = if parts.len() > 4 { 
                        parts[4].trim().to_string() 
                    } else { 
                        String::new() 
                    };

                    parsed_cuts.insert(sprite_id, SpriteCut {
                        uv_coordinates: egui::Rect::from_min_max(uv_min, uv_max),
                        original_size: egui::vec2(cut_width, cut_height),
                        name: cut_name,
                    });
                }
            }
        }

        Some((egui_image, parsed_cuts))
    }
}