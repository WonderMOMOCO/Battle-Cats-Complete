use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use super::scanner::CatEntry;
use image::imageops; 

struct LoadedImage {
    id: u32,
    img: egui::ColorImage,
}

struct LoadRequest {
    id: u32,
    path: PathBuf,
    high_banner_quality: bool,
}

pub struct CatList {
    texture_cache: HashMap<u32, egui::TextureHandle>,
    tx_request: Sender<LoadRequest>,
    rx_result: Receiver<LoadedImage>,
    pending_requests: HashSet<u32>,
    hovered_id: Option<egui::Id>, 
    hover_start_time: f64,
    hover_lost_time: Option<f64>,
    scroll_to_top_needed: bool,
}

impl Default for CatList {
    fn default() -> Self {
        let (tx_request, rx_request) = mpsc::channel::<LoadRequest>();
        let (tx_result, rx_result) = mpsc::channel::<LoadedImage>();

        thread::spawn(move || {
            let bg_cache = {
                const BG_BYTES: &[u8] = include_bytes!("../../assets/udi_bg.png");
                image::load_from_memory(BG_BYTES).ok().map(|img| img.to_rgba8())
            };

            while let Ok(req) = rx_request.recv() {
                if let Some(color_image) = process_image(req.id, &req.path, &bg_cache, req.high_banner_quality) {
                    let _ = tx_result.send(LoadedImage { id: req.id, img: color_image });
                }
            }
        });

        Self {
            texture_cache: HashMap::new(),
            tx_request,
            rx_result,
            pending_requests: HashSet::new(),
            hovered_id: None,
            hover_start_time: 0.0,
            hover_lost_time: None,
            scroll_to_top_needed: false,
        }
    }
}

impl CatList {
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.pending_requests.clear();
        self.hovered_id = None;
        self.hover_lost_time = None;
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_to_top_needed = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, units: &[CatEntry], selected_id: &mut Option<u32>, search_query: &str, high_banner_quality: bool) {
        
        while let Ok(loaded) = self.rx_result.try_recv() {
            let texture = ctx.load_texture(
                format!("unit_{}", loaded.id),
                loaded.img,
                egui::TextureOptions::LINEAR
            );
            self.texture_cache.insert(loaded.id, texture);
            self.pending_requests.remove(&loaded.id);
        }

        let query_lower = search_query.to_lowercase();
        let filtered_units: Vec<&CatEntry> = units.iter()
            .filter(|unit| {
                if search_query.is_empty() { return true; }
                let id_str = format!("{:03}", unit.id);
                if id_str.contains(search_query) { return true; }
                for name in &unit.names {
                    if name.to_lowercase().contains(&query_lower) { return true; }
                }
                false
            })
            .collect();

        let target_height = 50.0; 
        let padding = 5.0;
        let row_height = target_height + padding;
        let total_rows = filtered_units.len() + 1; 

        let now = ui.input(|i| i.time);

        let mut scroll_area = egui::ScrollArea::vertical()
            .auto_shrink([false, false]);

        if self.scroll_to_top_needed {
            scroll_area = scroll_area.vertical_scroll_offset(0.0);
            self.scroll_to_top_needed = false;
        }

        let scroll_output = scroll_area
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                
                let mut hovered_this_frame = None;

                for index in row_range {
                    if let Some(&unit) = filtered_units.get(index) {
                        
                        let texture = self.texture_cache.get(&unit.id);

                        if texture.is_none() && !self.pending_requests.contains(&unit.id) {
                            self.pending_requests.insert(unit.id);
                            let _ = self.tx_request.send(LoadRequest {
                                id: unit.id,
                                path: unit.image_path.clone(),
                                high_banner_quality, 
                            });
                        }

                        if let Some(tex) = texture {
                            let size = tex.size_vec2();
                            let scale = target_height / size.y;
                            let btn_size = size * scale;

                            let is_selected = Some(unit.id) == *selected_id;
                            
                            let btn = egui::ImageButton::new((tex.id(), btn_size))
                                .selected(is_selected);
                            
                            let mut response = ui.add(btn);

                            if ui.rect_contains_pointer(response.rect) {
                                hovered_this_frame = Some(response.id);

                                if self.hovered_id != Some(response.id) {
                                    self.hovered_id = Some(response.id);
                                    self.hover_start_time = now;
                                }

                                if now - self.hover_start_time > 1.0 {
                                    response = response.on_hover_ui(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("[ID]").weak());
                                            ui.label(format!("{:03}", unit.id));
                                        });

                                        let labels = ["Normal", "Evolved", "True", "Ultra"];
                                        let mut previous_name = "";

                                        for i in 0..4 {
                                            if unit.forms[i] {
                                                let raw_name = &unit.names[i];
                                                let display_name = if raw_name.is_empty() {
                                                    format!("{:03}-{}", unit.id, i + 1)
                                                } else {
                                                    raw_name.clone()
                                                };

                                                if i > 0 && raw_name == previous_name && !raw_name.is_empty() {
                                                    continue; 
                                                }

                                                if !raw_name.is_empty() {
                                                    previous_name = raw_name;
                                                }

                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(format!("[{}]", labels[i])).weak());
                                                    ui.label(display_name);
                                                });
                                            }
                                        }
                                    });
                                }
                            }

                            if response.clicked() {
                                *selected_id = Some(unit.id);
                            }

                        } else {
                            let response = ui.allocate_response(
                                egui::vec2(100.0, target_height), 
                                egui::Sense::click()
                            );
                            
                            ui.painter().rect_filled(
                                response.rect, 
                                4.0, 
                                egui::Color32::from_gray(30)
                            );
                            
                            if response.clicked() {
                                *selected_id = Some(unit.id);
                            }
                        }
                    }
                }
                
                hovered_this_frame
            });

        if scroll_output.inner.is_some() {
            self.hover_lost_time = None;
        } else {
            if self.hover_lost_time.is_none() {
                self.hover_lost_time = Some(now);
            }

            if let Some(lost_start) = self.hover_lost_time {
                if now - lost_start > 0.1 {
                    self.hovered_id = None;
                }
            }
        }
    }
}

fn process_image(id: u32, path: &PathBuf, bg_cache: &Option<image::RgbaImage>, high_banner_quality: bool) -> Option<egui::ColorImage> {
    if let Ok(image_buffer) = image::open(path) {
        let mut unit_img = image_buffer.to_rgba8();

        if let Some(bg) = bg_cache {
            let mut final_image = bg.clone();
            let bg_w = final_image.width() as i64;
            let bg_h = final_image.height() as i64;

            let (x, y) = if id <= 25 {
                let fixed_x: i64 = -2; 
                let fixed_y: i64 = 9;  
                (fixed_x, fixed_y)
            } else {
                unit_img = autocrop(unit_img);
                let unit_w = unit_img.width() as i64;
                let unit_h = unit_img.height() as i64;
                let cx = (bg_w - unit_w) / 2;
                let cy = (bg_h - unit_h) / 2;
                (cx, cy)
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
    }
    None
}

fn autocrop(img: image::RgbaImage) -> image::RgbaImage {
    let (width, height) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);
    let mut found = false;

    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] > 0 { 
            if x < min_x { min_x = x; }
            if x > max_x { max_x = x; }
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
            found = true;
        }
    }
    if !found { return img; }
    imageops::crop_imm(&img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}