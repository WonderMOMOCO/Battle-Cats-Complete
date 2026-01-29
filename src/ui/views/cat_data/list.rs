use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use image::imageops;

use crate::core::cat::scanner::CatEntry; 

struct LoadedImage {
    id: u32,
    img: Option<egui::ColorImage>,
}

struct LoadRequest {
    id: u32,
    path: PathBuf,
    high_banner_quality: bool,
    ctx: egui::Context,
}

pub struct CatList {
    texture_cache: HashMap<u32, egui::TextureHandle>,
    placeholder_texture: Option<egui::TextureHandle>,
    tx_request: Sender<LoadRequest>,
    rx_result: Receiver<LoadedImage>,
    pending_requests: HashSet<u32>,
    invalidated_ids: HashSet<u32>,
    missing_ids: HashSet<u32>,
    hovered_id: Option<egui::Id>, 
    hover_start_time: f64,
    hover_lost_time: Option<f64>,
    scroll_to_top_needed: bool,
    last_search_query: String,
    last_unit_count: usize,
    cached_indices: Vec<usize>,
}

impl Default for CatList {
    fn default() -> Self {
        let (tx_request, rx_request) = mpsc::channel::<LoadRequest>();
        let (tx_result, rx_result) = mpsc::channel::<LoadedImage>();

        thread::spawn(move || {
            let bg_cache = {
                const BG_BYTES: &[u8] = include_bytes!("../../../assets/udi_f.png");
                image::load_from_memory(BG_BYTES).ok().map(|img| img.to_rgba8())
            };
            
            let bg_cache = std::sync::Arc::new(bg_cache);

            while let Ok(req) = rx_request.recv() {
                let tx = tx_result.clone();
                let bg = bg_cache.clone();
                let ctx = req.ctx.clone(); 

                rayon::spawn(move || {
                    let result = process_image_robust(req.id, &req.path, &bg, req.high_banner_quality);
                    let _ = tx.send(LoadedImage { id: req.id, img: result });
                    
                    ctx.request_repaint();
                });
            }
        });

        Self {
            texture_cache: HashMap::new(),
            placeholder_texture: None,
            tx_request,
            rx_result,
            pending_requests: HashSet::new(),
            invalidated_ids: HashSet::new(),
            missing_ids: HashSet::new(),
            hovered_id: None,
            hover_start_time: 0.0,
            hover_lost_time: None,
            scroll_to_top_needed: false,
            last_search_query: String::new(),
            last_unit_count: 0,
            cached_indices: Vec::new(),
        }
    }
}

impl CatList {
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.pending_requests.clear();
        self.invalidated_ids.clear();
        self.missing_ids.clear();
        self.hovered_id = None;
        self.hover_lost_time = None;
        self.last_unit_count = 0; 
    }

    pub fn flush_icon(&mut self, id: u32) {
        self.invalidated_ids.insert(id);
        self.missing_ids.remove(&id);
        self.pending_requests.remove(&id);
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_to_top_needed = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, units: &[CatEntry], selected_id: &mut Option<u32>, search_query: &str, high_banner_quality: bool) {
        if self.placeholder_texture.is_none() {
            const BG_BYTES: &[u8] = include_bytes!("../../../assets/udi_f.png");
            if let Ok(img) = image::load_from_memory(BG_BYTES) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.as_flat_samples();
                self.placeholder_texture = Some(ctx.load_texture(
                    "list_placeholder", 
                    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), 
                    egui::TextureOptions::LINEAR
                ));
            }
        }

        while let Ok(loaded) = self.rx_result.try_recv() {
            if let Some(img) = loaded.img {
                let texture = ctx.load_texture(
                    format!("unit_{}", loaded.id),
                    img,
                    egui::TextureOptions::LINEAR
                );
                self.texture_cache.insert(loaded.id, texture);
                self.missing_ids.remove(&loaded.id);
            } else {
                self.texture_cache.remove(&loaded.id);
                self.missing_ids.insert(loaded.id);
            }
            
            self.pending_requests.remove(&loaded.id);
            self.invalidated_ids.remove(&loaded.id); 
            
        }

        if search_query != self.last_search_query || units.len() != self.last_unit_count {
            self.update_search_cache(units, search_query);
        }

        self.render_scroll_area(ctx, ui, units, selected_id, high_banner_quality);
    }

    fn render_scroll_area(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, units: &[CatEntry], selected_id: &mut Option<u32>, hq: bool) {
        let row_height = 55.0; 
        let total_rows = self.cached_indices.len() + 1; 
        let now = ui.input(|i| i.time);

        let mut scroll_area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if self.scroll_to_top_needed {
            scroll_area = scroll_area.vertical_scroll_offset(0.0);
            self.scroll_to_top_needed = false;
        }

        let scroll_output = scroll_area.show_rows(ui, row_height, total_rows, |ui, row_range| {
            let mut hovered_this_frame = None;

            for index in row_range {
                if let Some(&real_index) = self.cached_indices.get(index) {
                    if let Some(hovered) = self.render_list_row(ui, units, real_index, selected_id, hq, now) {
                        hovered_this_frame = Some(hovered);
                    }
                }
            }
            hovered_this_frame
        });

        if scroll_output.inner.is_some() {
             self.hover_lost_time = None;
             return;
        }
        
        if self.hover_lost_time.is_none() {
            self.hover_lost_time = Some(now);
        }
        
        if let Some(lost_start) = self.hover_lost_time {
            if now - lost_start > 0.1 {
                self.hovered_id = None;
            }
        }
    }

    fn render_list_row(&mut self, ui: &mut egui::Ui, units: &[CatEntry], real_index: usize, selected_id: &mut Option<u32>, hq: bool, now: f64) -> Option<egui::Id> {
        let unit = &units[real_index];

        let is_cached = self.texture_cache.contains_key(&unit.id);
        let is_missing = self.missing_ids.contains(&unit.id);
        let is_invalidated = self.invalidated_ids.contains(&unit.id);

        let needs_load = (!is_cached && !is_missing) || is_invalidated;

        if needs_load && !self.pending_requests.contains(&unit.id) {
            if let Some(path) = &unit.image_path {
                self.pending_requests.insert(unit.id);
                let _ = self.tx_request.send(LoadRequest {
                    id: unit.id,
                    path: path.clone(),
                    high_banner_quality: hq, 
                    ctx: ui.ctx().clone(),
                });
            } else {
                self.missing_ids.insert(unit.id);
            }
        }

        let texture = self.texture_cache.get(&unit.id);
        let tex_to_draw = texture.or(self.placeholder_texture.as_ref());
        let is_selected = Some(unit.id) == *selected_id;

        let response = if let Some(tex) = tex_to_draw {
            let size = tex.size_vec2();
            let scale = 50.0 / size.y;
            let btn_size = size * scale;
            ui.add(egui::ImageButton::new((tex.id(), btn_size)).selected(is_selected))
        } else {
            let r = ui.allocate_response(egui::vec2(100.0, 50.0), egui::Sense::click());
            ui.painter().rect_filled(r.rect, 4.0, egui::Color32::from_gray(30));
            r
        };

        if response.clicked() { *selected_id = Some(unit.id); }

        if !ui.rect_contains_pointer(response.rect) {
            return None;
        }

        let response_id = response.id;

        if self.hovered_id != Some(response_id) {
            self.hovered_id = Some(response_id);
            self.hover_start_time = now;
        }

        if now - self.hover_start_time < 0.5 {
            ui.ctx().request_repaint();
        } else {
            response.on_hover_ui(|ui| render_tooltip(ui, unit));
        }

        Some(response_id)
    }

    fn update_search_cache(&mut self, units: &[CatEntry], query: &str) {
        self.last_search_query = query.to_string();
        self.last_unit_count = units.len();
        self.cached_indices.clear();

        let query_lower = query.to_lowercase();
        let is_empty = query.is_empty();

        for (i, unit) in units.iter().enumerate() {
            if is_empty {
                self.cached_indices.push(i);
                continue;
            }
            if format!("{:03}", unit.id).contains(&query_lower) {
                self.cached_indices.push(i);
                continue;
            }
            if unit.names.iter().any(|name| name.to_lowercase().contains(&query_lower)) {
                self.cached_indices.push(i);
            }
        }
    }
}

fn render_tooltip(ui: &mut egui::Ui, unit: &CatEntry) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("[ID]").weak());
        ui.label(format!("{:03}", unit.id));
    });

    let labels = ["Normal", "Evolved", "True", "Ultra"];
    let mut previous_name = "";

    for i in 0..4 {
        if !unit.forms[i] { continue; }

        let raw_name = &unit.names[i];
        let is_duplicate = i > 0 && raw_name == previous_name && !raw_name.is_empty();

        let display_name = if raw_name.is_empty() || is_duplicate {
            format!("{:03}-{}", unit.id, i + 1)
        } else {
            raw_name.clone()
        };

        if !raw_name.is_empty() { previous_name = raw_name; }

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("[{}]", labels[i])).weak());
            ui.label(display_name);
        });
    }
}

fn process_image_robust(_id: u32, path: &PathBuf, bg_cache: &Option<image::RgbaImage>, high_banner_quality: bool) -> Option<egui::ColorImage> {
    for _ in 0..3 {
        if !path.exists() {
            return None; 
        }

        match image::open(path) {
            Ok(image_buffer) => {
                let mut unit_img = image_buffer.to_rgba8();
                if let Some(bg) = bg_cache.as_ref() {
                    let mut final_image = bg.clone();
                    let bg_w = final_image.width() as i64;
                    let bg_h = final_image.height() as i64;
                    let (w, h) = unit_img.dimensions();
                    
                    let is_transparent_unit = w > 311 && h > 2 && unit_img.get_pixel(311, 2)[3] == 0;

                    let (x, y) = if is_transparent_unit {
                        (-2, 9)
                    } else {
                        unit_img = autocrop(unit_img);
                        let unit_w = unit_img.width() as i64;
                        let unit_h = unit_img.height() as i64;
                        ((bg_w - unit_w) / 2, (bg_h - unit_h) / 2)
                    };

                    imageops::overlay(&mut final_image, &unit_img, x, y);

                    let (target_h, filter) = if high_banner_quality {
                        (100, imageops::FilterType::Lanczos3)
                    } else {
                        (50, imageops::FilterType::Nearest)
                    };

                    let ratio = target_h as f32 / final_image.height() as f32;
                    let target_w = (final_image.width() as f32 * ratio) as u32;
                    
                    let final_image = imageops::resize(&final_image, target_w, target_h, filter);
                    let size = [final_image.width() as usize, final_image.height() as usize];
                    let pixels = final_image.as_flat_samples();
                    
                    return Some(egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()));
                }
                return None;
            },
            Err(_) => {
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    None
}

fn autocrop(img: image::RgbaImage) -> image::RgbaImage {
    let (width, height) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);
    let mut found = false;

    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] > 0 { 
            min_x = min_x.min(x); min_y = min_y.min(y);
            max_x = max_x.max(x); max_y = max_y.max(y);
            found = true;
        }
    }
    if !found { return img; }
    imageops::crop_imm(&img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}