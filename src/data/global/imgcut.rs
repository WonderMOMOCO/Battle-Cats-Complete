use eframe::egui;
use std::path::{Path};
use std::fs;
use std::thread;
use std::sync::mpsc::{self, Receiver};
use std::collections::HashMap;
use crate::core::utils;

#[derive(Clone)]
pub struct SpriteCut {
    pub uv_coordinates: egui::Rect,
    pub original_size: egui::Vec2,
}

pub struct SpriteSheet {
    pub texture_handle: Option<egui::TextureHandle>,
    pub cuts_map: HashMap<usize, SpriteCut>, 
    pub is_loading_active: bool,
    pub data_receiver: Option<Receiver<(egui::ColorImage, HashMap<usize, SpriteCut>)>>,
}

impl Default for SpriteSheet {
    fn default() -> Self {
        Self {
            texture_handle: None,
            cuts_map: HashMap::new(),
            is_loading_active: false,
            data_receiver: None,
        }
    }
}

impl SpriteSheet {
    pub fn is_loading(&self) -> bool {
        self.is_loading_active
    }

    pub fn update(&mut self, context: &egui::Context) {
        let receiver = match &self.data_receiver {
            Some(r) => r,
            None => return,
        };

        if let Ok((loaded_image, loaded_cuts)) = receiver.try_recv() {
            self.texture_handle = Some(context.load_texture(
                "img015_atlas",
                loaded_image,
                egui::TextureOptions::LINEAR
            ));
            self.cuts_map = loaded_cuts;
            self.data_receiver = None;
            self.is_loading_active = false;
        }
    }

    pub fn load(&mut self, context: &egui::Context, image_path: &Path, cut_path: &Path) {
        self.update(context);

        if self.texture_handle.is_none() && !self.is_loading_active {
            self.start_loading_thread(context, image_path, cut_path);
        }
    }

    fn start_loading_thread(&mut self, context: &egui::Context, image_path: &Path, cut_path: &Path) {
        self.is_loading_active = true;
        let (sender, receiver) = mpsc::channel();
        self.data_receiver = Some(receiver);
        
        let image_path_buf = image_path.to_path_buf();
        let cut_path_buf = cut_path.to_path_buf();
        let context_clone = context.clone();

        thread::spawn(move || {
            if let Some(parsed_data) = parse_imgcut_data(&image_path_buf, &cut_path_buf) {
                let _ = sender.send(parsed_data);
                context_clone.request_repaint();
            }
        });
    }

    pub fn get_sprite_by_line(&self, target_line_index: usize) -> Option<egui::Image<'_>> {
        let texture = self.texture_handle.as_ref()?;
        
        let cut_data = self.cuts_map.get(&target_line_index)?;

        Some(
            egui::Image::new(texture)
                .uv(cut_data.uv_coordinates)
                .maintain_aspect_ratio(false)
                .fit_to_exact_size(cut_data.original_size)
        )
    }
}

fn parse_imgcut_data(image_file_path: &Path, cut_file_path: &Path) -> Option<(egui::ColorImage, HashMap<usize, SpriteCut>)> {
    let image_bytes = fs::read(image_file_path).ok()?;
    let dynamic_image = image::load_from_memory(&image_bytes).ok()?;
    let rgba_image = dynamic_image.to_rgba8(); 
    
    let image_dimensions = [rgba_image.width() as usize, rgba_image.height() as usize];
    let pixel_data = rgba_image.as_flat_samples();
    let egui_color_image = egui::ColorImage::from_rgba_unmultiplied(image_dimensions, pixel_data.as_slice());

    let imgcut_content = fs::read_to_string(cut_file_path).ok()?;
    let atlas_width = image_dimensions[0] as f32;
    let atlas_height = image_dimensions[1] as f32;
    
    let mut parsed_cuts = HashMap::new();

    let delimiter = utils::detect_csv_separator(&imgcut_content);

    for (line_index, file_line) in imgcut_content.lines().enumerate() {
        let line_parts: Vec<&str> = file_line.split(delimiter).collect();
        if line_parts.len() < 4 { continue; }

        if let (Ok(sprite_x), Ok(sprite_y), Ok(sprite_width), Ok(sprite_height)) = (
            line_parts[0].trim().parse::<f32>(),
            line_parts[1].trim().parse::<f32>(),
            line_parts[2].trim().parse::<f32>(), 
            line_parts[3].trim().parse::<f32>(), 
        ) {
            let uv_min = egui::pos2(sprite_x / atlas_width, sprite_y / atlas_height);
            let uv_max = egui::pos2((sprite_x + sprite_width) / atlas_width, (sprite_y + sprite_height) / atlas_height);

            parsed_cuts.insert(line_index + 1, SpriteCut {
                uv_coordinates: egui::Rect::from_min_max(uv_min, uv_max),
                original_size: egui::vec2(sprite_width, sprite_height),
            });
        }
    }

    Some((egui_color_image, parsed_cuts))
}