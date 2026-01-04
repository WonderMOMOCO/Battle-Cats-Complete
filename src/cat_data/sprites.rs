use eframe::egui;
use std::path::{Path};
use std::fs;
use std::thread;
use std::sync::mpsc::{self, Receiver};

#[derive(Clone)]
pub struct SpriteCut {
    pub uv: egui::Rect,
    pub original_size: egui::Vec2,
    pub line_num: usize,
}

pub struct SpriteSheet {
    pub texture: Option<egui::TextureHandle>,
    pub cuts: Vec<SpriteCut>, 
    loading: bool,
    rx: Option<Receiver<(egui::ColorImage, Vec<SpriteCut>)>>,
}

impl Default for SpriteSheet {
    fn default() -> Self {
        Self {
            texture: None,
            cuts: Vec::new(),
            loading: false,
            rx: None,
        }
    }
}

impl SpriteSheet {
    pub fn load(&mut self, ctx: &egui::Context, image_path: &Path, cut_path: &Path) {
        if let Some(rx) = &self.rx {
            if let Ok((img, cuts)) = rx.try_recv() {
                self.texture = Some(ctx.load_texture(
                    "img015_atlas",
                    img,
                    egui::TextureOptions::LINEAR
                ));
                self.cuts = cuts;
                self.rx = None;
                self.loading = false;
            }
        }

        if self.texture.is_some() || self.loading { return; }

        self.loading = true;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        
        let img_p = image_path.to_path_buf();
        let cut_p = cut_path.to_path_buf();
        let ctx_clone = ctx.clone();

        thread::spawn(move || {
            // Image Decode
            let image = match image::open(&img_p) {
                Ok(img) => img.to_rgba8(),
                Err(_) => return, 
            };

            let egui_img = egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_flat_samples().as_slice()
            );

            // Text Parse
            let w_img = image.width() as f32;
            let h_img = image.height() as f32;
            let mut cuts = Vec::new();
            
            if let Ok(content) = fs::read_to_string(&cut_p) {
                for (i, line) in content.lines().enumerate() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 4 {
                        if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                            parts[0].trim().parse::<f32>(),
                            parts[1].trim().parse::<f32>(),
                            parts[2].trim().parse::<f32>(), 
                            parts[3].trim().parse::<f32>(), 
                        ) {
                            let min = egui::pos2(x / w_img, y / h_img);
                            let max = egui::pos2((x + w) / w_img, (y + h) / h_img);
                            
                            cuts.push(SpriteCut {
                                uv: egui::Rect::from_min_max(min, max),
                                original_size: egui::vec2(w, h),
                                line_num: i + 1,
                            });
                        }
                    }
                }
            }
            
            let _ = tx.send((egui_img, cuts));
            ctx_clone.request_repaint();
        });
    }

    pub fn get_sprite_by_line(&self, target_line: usize) -> Option<egui::Image<'_>> {
        if let Some(tex) = &self.texture {
            if let Some(cut) = self.cuts.iter().find(|c| c.line_num == target_line) {
                return Some(
                    egui::Image::new(tex)
                        .uv(cut.uv)
                        .maintain_aspect_ratio(false)
                        .fit_to_exact_size(cut.original_size)
                );
            }
        }
        None
    }
}