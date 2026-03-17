use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use image::imageops;

use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::ui::filter::EnemyFilterState;
use crate::features::enemy::logic::filter;

struct LoadedImage {
    id: u32,
    img: Option<egui::ColorImage>,
}

struct LoadRequest {
    id: u32,
    path: PathBuf,
    ctx: egui::Context,
}

pub struct EnemyList {
    texture_cache: HashMap<u32, egui::TextureHandle>,
    placeholder_texture: Option<egui::TextureHandle>,
    tx_request: Sender<LoadRequest>,
    rx_result: Receiver<LoadedImage>,
    pending_requests: HashSet<u32>,
    missing_ids: HashSet<u32>,
    scroll_to_top_needed: bool,
    last_search_query: String,
    last_unit_count: usize,
    last_filter_state: EnemyFilterState,
    cached_indices: Vec<usize>,
}

impl Default for EnemyList {
    fn default() -> Self {
        let (tx_request, rx_request) = mpsc::channel::<LoadRequest>();
        let (tx_result, rx_result) = mpsc::channel::<LoadedImage>();

        thread::spawn(move || {
            let bg_cache = image::load_from_memory(crate::global::assets::UDI_F).ok().map(|img| img.to_rgba8());
            let bg_cache = std::sync::Arc::new(bg_cache);

            while let Ok(req) = rx_request.recv() {
                let tx = tx_result.clone();
                let ctx = req.ctx.clone(); 
                let bg = bg_cache.clone();

                rayon::spawn(move || {
                    let result = process_image(&req.path, &*bg);
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
            missing_ids: HashSet::new(),
            scroll_to_top_needed: false,
            last_search_query: String::new(),
            last_unit_count: 0,
            last_filter_state: EnemyFilterState::default(),
            cached_indices: Vec::new(),
        }
    }
}

impl EnemyList {
    pub fn reset_scroll(&mut self) {
        self.scroll_to_top_needed = true;
    }

    pub fn force_search_rebuild(&mut self) {
        self.last_unit_count = usize::MAX;
    }

    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.pending_requests.clear();
        self.missing_ids.clear();
        self.last_unit_count = 0;
    }

    pub fn flush_icon(&mut self, id: u32) {
        self.missing_ids.remove(&id);
        self.pending_requests.remove(&id);
        self.texture_cache.remove(&id);
    }

    pub fn show(
        &mut self, 
        ctx: &egui::Context, 
        ui: &mut egui::Ui, 
        entries: &[EnemyEntry], 
        selected_id: &mut Option<u32>, 
        search_query: &str,
        filter: &EnemyFilterState,
    ) {
        if self.placeholder_texture.is_none() {
            if let Ok(img) = image::load_from_memory(crate::global::assets::UDI_F) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.as_flat_samples();
                self.placeholder_texture = Some(ctx.load_texture(
                    "enemy_list_placeholder", 
                    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), 
                    egui::TextureOptions::LINEAR
                ));
            }
        }

        while let Ok(loaded) = self.rx_result.try_recv() {
            if let Some(img) = loaded.img {
                let texture = ctx.load_texture(
                    format!("enemy_{}", loaded.id),
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
        }

        let filter_changed = *filter != self.last_filter_state;
        if search_query != self.last_search_query || entries.len() != self.last_unit_count || filter_changed {
            self.update_search_cache(entries, search_query, filter);
        }

        let row_height = 55.0; 
        let total_rows = self.cached_indices.len(); 

        let mut scroll_area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if self.scroll_to_top_needed {
            scroll_area = scroll_area.vertical_scroll_offset(0.0);
            self.scroll_to_top_needed = false;
        }

        scroll_area.show_rows(ui, row_height, total_rows, |ui, row_range| {
            for index in row_range {
                if let Some(&real_index) = self.cached_indices.get(index) {
                    self.render_list_row(ui, entries, real_index, selected_id);
                }
            }
        });
    }

    fn render_list_row(&mut self, ui: &mut egui::Ui, entries: &[EnemyEntry], real_index: usize, selected_id: &mut Option<u32>) {
        if real_index == usize::MAX {
            ui.add_space(8.0); 
            return;
        }

        let entry = &entries[real_index];
        let is_cached = self.texture_cache.contains_key(&entry.id);
        let is_missing = self.missing_ids.contains(&entry.id);

        if !is_cached && !is_missing && !self.pending_requests.contains(&entry.id) {
            if let Some(path) = &entry.icon_path {
                self.pending_requests.insert(entry.id);
                let _ = self.tx_request.send(LoadRequest {
                    id: entry.id,
                    path: path.clone(),
                    ctx: ui.ctx().clone(),
                });
            } else {
                self.missing_ids.insert(entry.id);
            }
        }

        let texture = self.texture_cache.get(&entry.id);
        let tex_to_draw = texture.or(self.placeholder_texture.as_ref());
        let is_selected = Some(entry.id) == *selected_id;

        let response = if let Some(tex) = tex_to_draw {
            let size = tex.size_vec2();
            let scale = 50.0 / size.y;
            let btn_size = size * scale;
            ui.add(egui::ImageButton::new(egui::load::SizedTexture::new(tex.id(), btn_size)).selected(is_selected))
        } else {
            let r = ui.allocate_response(egui::vec2(50.0, 50.0), egui::Sense::click());
            ui.painter().rect_filled(r.rect, 4.0, egui::Color32::from_gray(30));
            r
        };

        if response.clicked() { *selected_id = Some(entry.id); }

        response.on_hover_ui(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("[ID]").weak());
                ui.label(entry.id_str());
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("[Name]").weak());
                ui.label(entry.display_name());
            });
        });
    }

    fn update_search_cache(&mut self, entries: &[EnemyEntry], query: &str, filter: &EnemyFilterState) {
        self.last_search_query = query.to_string();
        self.last_unit_count = entries.len();
        self.last_filter_state = filter.clone();
        self.cached_indices.clear();

        let query_lower = query.to_lowercase();
        let is_empty = query.is_empty();

        for (i, entry) in entries.iter().enumerate() {
            if !filter::entity_passes_filter(entry, filter) {
                continue;
            }

            if is_empty {
                self.cached_indices.push(i);
                continue;
            }

            let full_id = entry.id_str().to_lowercase();
            let is_id_search = query_lower.chars().next().map_or(false, |c| c.is_ascii_digit());
            
            if is_id_search && full_id.contains(&query_lower) {
                self.cached_indices.push(i);
                continue;
            }

            if entry.name.to_lowercase().contains(&query_lower) {
                self.cached_indices.push(i);
            }
        }

        // We add usize::MAX as a dummy index at the very end of the list
        if !self.cached_indices.is_empty() {
            self.cached_indices.push(usize::MAX);
        }
    }
}

const ENEMY_ICON_SCALE_FACTOR: f32 = 2.6; 
const ENEMY_ICON_OFFSET_X: i64 = 8;       
const ENEMY_SHADOW_MARGIN: u32 = 8;       

fn process_image(path: &PathBuf, bg_cache: &Option<image::RgbaImage>) -> Option<egui::ColorImage> {
    if !path.exists() { return None; }
    
    if let Ok(image_buffer) = image::open(path) {
        let mut unit_img = image_buffer.to_rgba8();
        let scaled_w = (unit_img.width() as f32 * ENEMY_ICON_SCALE_FACTOR).round() as u32;
        let scaled_h = (unit_img.height() as f32 * ENEMY_ICON_SCALE_FACTOR).round() as u32;
        unit_img = imageops::resize(&unit_img, scaled_w, scaled_h, imageops::FilterType::Lanczos3);
        
        if let Some(bg) = bg_cache.as_ref() {
            let mut final_image = bg.clone();
            let bg_w = final_image.width() as i64;
            let bg_h = final_image.height() as i64;
            let h = unit_img.height(); 
            let mut min_y = h; let mut max_y = 0;
            let mut found_solid = false;
            let shadow_cutoff = h.saturating_sub(ENEMY_SHADOW_MARGIN);
            
            for (_x, y, pixel) in unit_img.enumerate_pixels() {
                if y < shadow_cutoff && pixel[3] > 150 { 
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    found_solid = true;
                }
            }

            let center_y = if found_solid { (min_y + max_y) as i64 / 2 } else { h as i64 / 2 };
            let offset_y = (bg_h / 2) - center_y;
            use image::Pixel; 
            for (x, y, pixel) in unit_img.enumerate_pixels() {
                let dest_x = ENEMY_ICON_OFFSET_X + x as i64;
                let dest_y = offset_y + y as i64;
                if dest_x >= 0 && dest_x < bg_w && dest_y >= 0 && dest_y < bg_h {
                    let bg_pixel = final_image.get_pixel_mut(dest_x as u32, dest_y as u32);
                    let is_black_border = bg_pixel[0] < 25 && bg_pixel[1] < 25 && bg_pixel[2] < 25 && bg_pixel[3] > 200;
                    if !is_black_border { bg_pixel.blend(&pixel); }
                }
            }

            let target_h = 50; 
            let ratio = target_h as f32 / final_image.height() as f32;
            let target_w = (final_image.width() as f32 * ratio) as u32;
            let final_image = imageops::resize(&final_image, target_w, target_h, imageops::FilterType::Lanczos3);
            let size = [final_image.width() as usize, final_image.height() as usize];
            let pixels = final_image.as_flat_samples();
            return Some(egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()));
        } else {
            let final_image = imageops::resize(&unit_img, 50, 50, imageops::FilterType::Lanczos3);
            let size = [final_image.width() as usize, final_image.height() as usize];
            let pixels = final_image.as_flat_samples();
            return Some(egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()));
        }
    }
    None
}