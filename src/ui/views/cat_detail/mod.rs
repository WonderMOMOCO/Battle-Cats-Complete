use eframe::egui;

use crate::core::cat::scanner::CatEntry;
use crate::core::files::imgcut::SpriteSheet;
use crate::core::files::img015;
use crate::core::settings::Settings;

mod header;
mod stats;
mod abilities;

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    current_form: &mut usize,
    level_input: &mut String,   
    current_level: &mut i32,    
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    sprite_sheet: &mut SpriteSheet,
    multihit_texture: &mut Option<egui::TextureHandle>,
    settings: &Settings,
) {
    img015::ensure_loaded(ctx, sprite_sheet, settings);

    header::render(
        ctx, 
        ui, 
        cat, 
        current_form, 
        current_level, 
        level_input, 
        texture_cache, 
        current_key,
        settings
    );

    ui.separator(); 
    ui.add_space(0.0);

    if multihit_texture.is_none() {
        const MULTIHIT_BYTES: &[u8] = include_bytes!("../../../assets/multihit.png");
        if let Ok(img) = image::load_from_memory(MULTIHIT_BYTES) {
            let rgba = img.to_rgba8();
            *multihit_texture = Some(ctx.load_texture(
                "multihit_icon",
                egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_flat_samples().as_slice()
                ),
                egui::TextureOptions::LINEAR
            ));
        }
    }

    let current_stats = cat.stats.get(*current_form).and_then(|opt| opt.as_ref());

    if let Some(s) = current_stats {
        stats::render(ui, cat, s, *current_form, *current_level);
        ui.spacing_mut().item_spacing.y = 7.0;
        ui.separator(); 
    }

    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false]) 
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if let Some(s) = current_stats {
                abilities::render(
                    ui, 
                    s, 
                    cat, 
                    *current_level, 
                    sprite_sheet, 
                    multihit_texture, 
                    settings
                );
                ui.add_space(5.0);
            }
        });
}