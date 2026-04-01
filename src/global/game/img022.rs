#![allow(dead_code)]
use eframe::egui;
use crate::features::settings::logic::Settings;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::io::paths;

pub const ICON_NP_COST: usize = 97; 

pub fn ensure_loaded(ctx: &egui::Context, sheets: &mut Vec<SpriteSheet>, settings: &Settings) {
    let base_dir = paths::img022_folder(std::path::Path::new(""));
    
    let png_paths = crate::global::resolver::get(&base_dir, "img022.png", &settings.general.language_priority);
    let cut_paths = crate::global::resolver::get(&base_dir, "img022.imgcut", &settings.general.language_priority);

    if sheets.len() != png_paths.len() {
        sheets.resize_with(png_paths.len(), SpriteSheet::default);
    }

    for (i, (png_path, imgcut_path)) in png_paths.into_iter().zip(cut_paths.into_iter()).enumerate() {
        sheets[i].update(ctx);

        if sheets[i].texture_handle.is_none() && !sheets[i].is_loading_active {
            let key = png_path.file_stem().unwrap().to_string_lossy().into_owned();
            sheets[i].load(ctx, &png_path, &imgcut_path, key);
        }
    }
}