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
        self.check_loading_status(ctx);

        if self.texture.is_none() && !self.loading {
            self.start_loading(ctx, image_path, cut_path);
        }
    }

    fn check_loading_status(&mut self, ctx: &egui::Context) {
        let rx = match &self.rx {
            Some(r) => r,
            None => return,
        };

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

    fn start_loading(&mut self, ctx: &egui::Context, image_path: &Path, cut_path: &Path) {
        self.loading = true;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        
        let img_p = image_path.to_path_buf();
        let cut_p = cut_path.to_path_buf();
        let ctx_clone = ctx.clone();

        thread::spawn(move || {
            if let Some(data) = load_sprite_data(&img_p, &cut_p) {
                let _ = tx.send(data);
                ctx_clone.request_repaint();
            }
        });
    }

    pub fn get_sprite_by_line(&self, target_line: usize) -> Option<egui::Image<'_>> {
        let tex = self.texture.as_ref()?;
        let cut = self.cuts.iter().find(|c| c.line_num == target_line)?;

        Some(
            egui::Image::new(tex)
                .uv(cut.uv)
                .maintain_aspect_ratio(false)
                .fit_to_exact_size(cut.original_size)
        )
    }
}

fn load_sprite_data(img_path: &Path, cut_path: &Path) -> Option<(egui::ColorImage, Vec<SpriteCut>)> {
    // Load Image
    let img_data = fs::read(img_path).ok()?;
    let dynamic_img = image::load_from_memory(&img_data).ok()?;
    let rgba = dynamic_img.to_rgba8(); 
    
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.as_flat_samples();
    let egui_img = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

    // Load Cuts
    let content = fs::read_to_string(cut_path).ok()?;
    let w_img = size[0] as f32;
    let h_img = size[1] as f32;
    let mut cuts = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 4 { continue; }

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

    Some((egui_img, cuts))
}