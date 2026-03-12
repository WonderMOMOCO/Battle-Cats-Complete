#![allow(dead_code)]
use eframe::egui;
use crate::features::settings::logic::Settings;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::io::paths;

pub const ICON_NP_COST: usize = 97; 

pub fn ensure_loaded(ctx: &egui::Context, sheet: &mut SpriteSheet, settings: &Settings) {
    sheet.update(ctx);

    if settings.general.game_language == "--" {
        return; 
    }

    if sheet.texture_handle.is_some() || sheet.is_loading_active {
        return;
    }

    let base_dir = paths::img022_folder(std::path::Path::new(""));
    let current_language = &settings.general.game_language;
    
    let mut codes_to_try = Vec::new();
    if !current_language.is_empty() {
        codes_to_try.push(current_language.clone());
    }
    
    for code in crate::global::utils::LANGUAGE_PRIORITY {
        codes_to_try.push(code.to_string());
    }

    for code in codes_to_try {
        let (png_filename, imgcut_filename) = if code == "--" || code.is_empty() {
            ("img022.png".to_string(), "img022.imgcut".to_string())
        } else {
            (format!("img022_{}.png", code), format!("img022_{}.imgcut", code))
        };

        let png_path = base_dir.join(&png_filename);
        let imgcut_path = base_dir.join(&imgcut_filename);

        if png_path.exists() && imgcut_path.exists() {
            sheet.load(ctx, &png_path, &imgcut_path, format!("img022_{}", code));
            return;
        }
    }
}