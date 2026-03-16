#![allow(dead_code)]
use eframe::egui;
use crate::features::settings::logic::Settings;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::io::paths;

pub const ICON_NP_COST: usize = 97; 

pub fn ensure_loaded(ctx: &egui::Context, sheet: &mut SpriteSheet, settings: &Settings) {
    sheet.update(ctx);

    if sheet.texture_handle.is_some() || sheet.is_loading_active {
        return;
    }

    let base_dir = paths::img022_folder(std::path::Path::new(""));
    
    let png_paths = crate::global::resolver::get(&base_dir, "img022.png", &settings.general.language_priority);
    let cut_paths = crate::global::resolver::get(&base_dir, "img022.imgcut", &settings.general.language_priority);

    if let (Some(png_path), Some(imgcut_path)) = (png_paths.into_iter().next(), cut_paths.into_iter().next()) {
        
        let key = png_path.file_stem().unwrap().to_string_lossy().into_owned();
        
        sheet.load(ctx, &png_path, &imgcut_path, key);
    }
}