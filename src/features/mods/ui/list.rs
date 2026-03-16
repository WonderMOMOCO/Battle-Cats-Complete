use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use image::imageops;

use crate::features::settings::logic::Settings;
use crate::features::mods::logic::state::ModState;

const TOP_PANEL_PADDING: f32 = 2.5;
const SEARCH_FILTER_GAP: f32 = 5.0;
const SPACE_BEFORE_SEPARATOR: f32 = 2.0;
const SPACE_AFTER_SEPARATOR: f32 = 2.0;

const OUTLINE_THICKNESS: f32 = 3.0;
const BUTTON_ROUNDING: f32 = 4.0;
const ICON_UI_SIZE: f32 = 46.0;
const ICON_RENDER_SIZE: u32 = 92; 
const ICON_TEXT_PADDING: f32 = 12.0;

const BUTTON_BASE_WIDTH: f32 = 140.0;
const BUTTON_HEIGHT: f32 = 52.0;
const BUTTON_PADDING_RIGHT: f32 = 15.0;

struct LoadedImage {
    folder_name: String,
    img: Option<egui::ColorImage>,
}

struct LoadRequest {
    folder_name: String,
    path: PathBuf,
    ctx: egui::Context,
}

pub struct ModList {
    texture_cache: HashMap<String, egui::TextureHandle>,
    tx_request: Sender<LoadRequest>,
    rx_result: Receiver<LoadedImage>,
    pending_requests: HashSet<String>,
    missing_ids: HashSet<String>,
    cached_indices: Vec<usize>,
    last_search_query: String,
    last_mod_count: usize,
    fallback_texture: Option<egui::TextureHandle>,
}

impl Default for ModList {
    fn default() -> Self {
        let (tx_request, rx_request) = mpsc::channel::<LoadRequest>();
        let (tx_result, rx_result) = mpsc::channel::<LoadedImage>();

        thread::spawn(move || {
            while let Ok(req) = rx_request.recv() {
                let tx = tx_result.clone();
                let ctx = req.ctx.clone(); 

                rayon::spawn(move || {
                    let result = process_image(&req.path);
                    let _ = tx.send(LoadedImage { folder_name: req.folder_name, img: result });
                    ctx.request_repaint();
                });
            }
        });

        Self {
            texture_cache: HashMap::new(),
            tx_request,
            rx_result,
            pending_requests: HashSet::new(),
            missing_ids: HashSet::new(),
            cached_indices: Vec::new(),
            last_search_query: String::new(),
            last_mod_count: 0,
            fallback_texture: None,
        }
    }
}

impl ModList {
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut ModState, _settings: &mut Settings) {
        self.process_incoming_textures(ui.ctx());
        self.render_top_panel(ui, state);
        
        let query = state.search_query.to_lowercase();
        if query != self.last_search_query || state.loaded_mods.len() != self.last_mod_count {
            self.update_search_cache(state, &query);
        }

        let row_height = 55.0; 
        let total_rows = self.cached_indices.len();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                for index in row_range {
                    let Some(&real_index) = self.cached_indices.get(index) else { continue; };
                    self.render_list_row(ui, state, real_index);
                }
            });
    }

    fn process_incoming_textures(&mut self, ctx: &egui::Context) {
        while let Ok(loaded) = self.rx_result.try_recv() {
            if let Some(img) = loaded.img {
                let texture = ctx.load_texture(
                    format!("mod_{}", loaded.folder_name),
                    img,
                    egui::TextureOptions::LINEAR
                );
                self.texture_cache.insert(loaded.folder_name.clone(), texture);
                self.missing_ids.remove(&loaded.folder_name);
            } else {
                self.texture_cache.remove(&loaded.folder_name);
                self.missing_ids.insert(loaded.folder_name.clone());
            }
            self.pending_requests.remove(&loaded.folder_name);
        }
    }

    fn render_top_panel(&mut self, ui: &mut egui::Ui, state: &mut ModState) {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(TOP_PANEL_PADDING);
            
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                
                let search_response = ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text(egui::RichText::new("Search Mods...").color(egui::Color32::GRAY))
                        .desired_width(140.0)
                );
                
                ui.add_space(SEARCH_FILTER_GAP);
                
                let btn_size = egui::vec2(140.0, search_response.rect.height());
                let import_btn = egui::Button::new("Import Mod");
                
                if ui.add_sized(btn_size, import_btn).clicked() {
                    state.import.is_open = true;
                }
            });
            
            ui.add_space(SPACE_BEFORE_SEPARATOR); 
            ui.separator();
            ui.add_space(SPACE_AFTER_SEPARATOR);
        });
    }

    fn update_search_cache(&mut self, state: &ModState, query: &str) {
        self.last_search_query = query.to_string();
        self.last_mod_count = state.loaded_mods.len();
        self.cached_indices.clear();

        for (i, m) in state.loaded_mods.iter().enumerate() {
            if query.is_empty() || m.folder_name.to_lowercase().contains(query) {
                self.cached_indices.push(i);
            }
        }
    }

    fn render_list_row(&mut self, ui: &mut egui::Ui, state: &mut ModState, real_index: usize) {
        let mod_data = &state.loaded_mods[real_index];
        let is_selected = state.selected_mod.as_deref() == Some(mod_data.folder_name.as_str());
        let mod_path = PathBuf::from(format!("mods/{}", mod_data.folder_name));

        self.request_texture_if_needed(ui.ctx(), &mod_data.folder_name, &mod_path);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            let left_padding = (ui.available_width() - BUTTON_BASE_WIDTH) / 2.0;
            ui.add_space(left_padding.max(0.0));

            let rect_size = egui::vec2(BUTTON_BASE_WIDTH - BUTTON_PADDING_RIGHT, BUTTON_HEIGHT); 
            let (rect, response) = ui.allocate_exact_size(rect_size, egui::Sense::click());
            
            if response.clicked() {
                state.selected_mod = Some(mod_data.folder_name.clone());
                state.rename_buffer = mod_data.folder_name.clone(); 
                self.fallback_texture = None;
            }

            let base_bg = if mod_data.enabled {
                egui::Color32::from_rgb(30, 90, 30) 
            } else {
                egui::Color32::from_gray(30)
            };

            let bg_color = if is_selected {
                ui.visuals().selection.bg_fill
            } else if response.hovered() {
                ui.visuals().widgets.hovered.bg_fill
            } else {
                base_bg
            };
            
            ui.painter().rect_filled(rect, BUTTON_ROUNDING, bg_color);
            ui.painter().rect_stroke(rect, BUTTON_ROUNDING, (OUTLINE_THICKNESS, egui::Color32::BLACK));

            let mut cached_tex = self.texture_cache.get(&mod_data.folder_name).cloned();
            
            if is_selected {
                if let Some(tex) = &cached_tex {
                    self.fallback_texture = Some(tex.clone());
                } else if self.fallback_texture.is_some() {
                    cached_tex = self.fallback_texture.clone();
                }
            }
            
            let has_icon = cached_tex.is_some();
            let mut icon_width = 0.0;
            
            if let Some(tex) = cached_tex {
                let size = tex.size_vec2();
                let scale = ICON_UI_SIZE / size.y;
                let draw_size = size * scale;
                
                let img_rect = egui::Rect::from_min_size(rect.min + egui::vec2(3.0, 3.0), draw_size);
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                
                ui.painter().image(tex.id(), img_rect, uv, egui::Color32::WHITE);
                icon_width = draw_size.x;
            }

            let text_color = if is_selected { egui::Color32::WHITE } else { ui.visuals().text_color() };
            
            let mut job = egui::text::LayoutJob::single_section(
                mod_data.folder_name.clone(),
                egui::TextFormat {
                    font_id: egui::FontId::proportional(12.0),
                    color: text_color,
                    ..Default::default()
                }
            );
            job.wrap.max_rows = 2;
            
            if has_icon {
                let text_x = rect.min.x + icon_width + ICON_TEXT_PADDING;
                let text_rect = egui::Rect::from_min_max(egui::pos2(text_x, rect.min.y + 4.0), rect.max - egui::vec2(4.0, 4.0));
                job.wrap.max_width = text_rect.width();
                
                let galley = ui.fonts(|f| f.layout_job(job));
                let y_pos = rect.min.y + (rect.height() - galley.rect.height()) / 2.0;
                ui.painter().galley(egui::pos2(text_rect.min.x, y_pos), galley, egui::Color32::WHITE); 
            } else {
                job.halign = egui::Align::Center;
                let text_rect = rect.shrink(4.0);
                job.wrap.max_width = text_rect.width();
                
                let galley = ui.fonts(|f| f.layout_job(job));
                
                let text_pos = rect.center() - galley.rect.center().to_vec2();
                ui.painter().galley(text_pos, galley, egui::Color32::WHITE);
            }
        });
    }

    fn request_texture_if_needed(&mut self, ctx: &egui::Context, name: &str, path: &PathBuf) {
        let is_cached = self.texture_cache.contains_key(name);
        let is_missing = self.missing_ids.contains(name);

        if !is_cached && !is_missing && !self.pending_requests.contains(name) {
            self.pending_requests.insert(name.to_string());
            let _ = self.tx_request.send(LoadRequest {
                folder_name: name.to_string(),
                path: path.clone(),
                ctx: ctx.clone(),
            });
        }
    }
}

fn process_image(mod_path: &PathBuf) -> Option<egui::ColorImage> {
    let icon_path = if mod_path.join("icon.png").exists() {
        mod_path.join("icon.png")
    } else if mod_path.join("icon.ico").exists() {
        mod_path.join("icon.ico")
    } else {
        return None;
    };

    let Ok(image_buffer) = image::open(&icon_path) else { return None; };
    let unit_img = image_buffer.to_rgba8();
    
    let final_image = imageops::resize(&unit_img, ICON_RENDER_SIZE, ICON_RENDER_SIZE, imageops::FilterType::Lanczos3);
    
    let size = [final_image.width() as usize, final_image.height() as usize];
    let pixels = final_image.as_flat_samples();
    
    Some(egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}