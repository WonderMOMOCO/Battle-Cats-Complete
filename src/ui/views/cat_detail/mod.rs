use eframe::egui;
use std::collections::HashMap;

use crate::core::cat::scanner::CatEntry;
use crate::core::cat::DetailTab;
use crate::core::files::imgcut::SpriteSheet;
use crate::core::files::img015;
use crate::core::settings::Settings;
use crate::core::cat::talents as talent_logic; 

mod header;
mod stats;
mod abilities;
mod talents;
mod details;

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    current_form: &mut usize,
    current_tab: &mut DetailTab, 
    level_input: &mut String,   
    current_level: &mut i32,    
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    sprite_sheet: &mut SpriteSheet,
    multihit_texture: &mut Option<egui::TextureHandle>,
    kamikaze_texture: &mut Option<egui::TextureHandle>,
    boss_wave_immune_texture: &mut Option<egui::TextureHandle>,
    talent_name_cache: &mut HashMap<String, egui::TextureHandle>,
    gatya_item_textures: &mut HashMap<i32, Option<egui::TextureHandle>>,
    skill_descriptions: Option<&Vec<String>>, 
    settings: &Settings, 
    talent_levels: &mut HashMap<u8, u8>,
    cache_version: u64,
) {
    img015::ensure_loaded(ctx, sprite_sheet, settings);

    header::render(
        ctx, 
        ui, 
        cat, 
        current_form, 
        current_tab, 
        current_level, 
        level_input, 
        texture_cache, 
        current_key,
        settings
    );

    ui.separator(); 
    ui.add_space(0.0);

    // Asset Loading
    if multihit_texture.is_none() {
        const MULTIHIT_BYTES: &[u8] = include_bytes!("../../../assets/multihit.png");
        if let Ok(img) = image::load_from_memory(MULTIHIT_BYTES) {
            let rgba = img.to_rgba8();
            *multihit_texture = Some(ctx.load_texture("multihit_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }
    if kamikaze_texture.is_none() {
        const KAMIKAZE_BYTES: &[u8] = include_bytes!("../../../assets/kamikaze.png");
        if let Ok(img) = image::load_from_memory(KAMIKAZE_BYTES) {
            let rgba = img.to_rgba8();
            *kamikaze_texture = Some(ctx.load_texture("kamikaze_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }
    if boss_wave_immune_texture.is_none() {
        const BOSS_WAVE_BYTES: &[u8] = include_bytes!("../../../assets/boss_wave_immune.png");
        if let Ok(img) = image::load_from_memory(BOSS_WAVE_BYTES) {
            let rgba = img.to_rgba8();
            *boss_wave_immune_texture = Some(ctx.load_texture("boss_wave_immune_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }

    let base_stats = cat.stats.get(*current_form).and_then(|opt| opt.as_ref());
    
    let form_allows_talents = *current_form >= 2;

    let patched_stats_owned = if form_allows_talents {
        if let (Some(base), Some(t_data)) = (base_stats, &cat.talent_data) {
            Some(talent_logic::apply_talent_stats(base, t_data, talent_levels))
        } else {
            None
        }
    } else {
        None
    };

    let current_stats = patched_stats_owned.as_ref().or(base_stats);

    match current_tab {
        DetailTab::Abilities => {
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
                            kamikaze_texture,
                            boss_wave_immune_texture,
                            settings,
                            if form_allows_talents { cat.talent_data.as_ref() } else { None },
                            if form_allows_talents { Some(&*talent_levels) } else { None }
                        );
                        ui.add_space(5.0);
                    }
                });
        },
        DetailTab::Talents => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(raw) = &cat.talent_data {
                        talents::render(
                            ui, 
                            raw, 
                            sprite_sheet, 
                            talent_name_cache, 
                            skill_descriptions,
                            settings,
                            base_stats,
                            cat.curve.as_ref(), 
                            *current_level,
                            talent_levels, 
                            cat.id         
                        );
                    } else {
                        ui.label("Error: Talents expected but not found.");
                    }
                });
        },
        DetailTab::Details => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let fallback = Vec::new();
                    let desc = cat.description.get(*current_form).unwrap_or(&fallback);
                    details::render(ui, desc);
                    
                    let text_fallback = Vec::new();
                    let ev_text = cat.evolve_text.get(*current_form).unwrap_or(&text_fallback);

                    details::render_evolve(ui, ctx, &cat.unit_buy, ev_text, *current_form, gatya_item_textures, cache_version);
                });
        }
    }
}