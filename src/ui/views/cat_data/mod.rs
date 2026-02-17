use eframe::egui;
use std::collections::HashMap;

use crate::core::cat::scanner::CatEntry;
use crate::core::cat::DetailTab;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::img015;
use crate::core::settings::Settings;
use crate::core::cat::talents as talent_logic; 
use crate::data::global::mamodel::Model;
use crate::ui::components::anim::viewer::AnimViewer;

mod header;
mod stats;
mod abilities;
mod talents;
mod details;
pub mod list;
mod catviewer;

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    cat_entry: &CatEntry, 
    current_form: &mut usize,
    current_tab: &mut DetailTab, 
    level_input: &mut String,   
    current_level: &mut i32,    
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    
    icon_sheet: &mut SpriteSheet,   
    anim_sheet: &mut SpriteSheet,   
    
    model_data: &mut Option<Model>,
    anim_viewer: &mut AnimViewer,

    multihit_texture: &mut Option<egui::TextureHandle>,
    kamikaze_texture: &mut Option<egui::TextureHandle>,
    boss_wave_immune_texture: &mut Option<egui::TextureHandle>,
    talent_name_cache: &mut HashMap<String, egui::TextureHandle>,
    gatya_item_textures: &mut HashMap<i32, Option<egui::TextureHandle>>,
    skill_descriptions: Option<&Vec<String>>, 
    settings: &mut Settings,
    talent_levels: &mut HashMap<u8, u8>,
    cache_version: u64,
) {
    img015::ensure_loaded(ctx, icon_sheet, settings);

    header::render(
        ctx, ui, cat_entry, current_form, current_tab, current_level, level_input, texture_cache, current_key, settings
    );

    ui.separator(); 
    ui.add_space(0.0);

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
            *kamikaze_texture = Some(ctx.load_texture("kamkikaze_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
        }
    }

    if boss_wave_immune_texture.is_none() {
        const BOSS_WAVE_BYTES: &[u8] = include_bytes!("../../../assets/boss_wave_immune.png");
        if let Ok(img) = image::load_from_memory(BOSS_WAVE_BYTES) {
          let rgba = img.to_rgba8();
          *boss_wave_immune_texture = Some(ctx.load_texture("boss_wave_icon", egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR));
    }
    }


    let base_stats = cat_entry.stats.get(*current_form).and_then(|opt| opt.as_ref());
    let form_allows_talents = *current_form >= 2;

    let patched_stats_owned = if form_allows_talents {
        if let (Some(base), Some(t_data)) = (base_stats, &cat_entry.talent_data) {
            Some(talent_logic::apply_talent_stats(base, t_data, talent_levels))
        } else { None }
    } else { None };
    let current_stats = patched_stats_owned.as_ref().or(base_stats);
    if *current_tab != DetailTab::Animation {
        if !anim_viewer.loaded_id.is_empty() {
             anim_viewer.held_model = None;
             anim_viewer.held_sheet = None;
             anim_viewer.current_anim = None;
             anim_viewer.loaded_id.clear();
             anim_viewer.staging_model = None;
             anim_viewer.staging_sheet = None;
        }
    }

    match current_tab {
        DetailTab::Abilities => {
            if let Some(s) = current_stats {
                stats::render(ui, cat_entry, s, *current_form, *current_level);
                ui.spacing_mut().item_spacing.y = 7.0;
                ui.separator(); 
            }
             egui::ScrollArea::vertical()
                .auto_shrink([false, false]) 
                .show(ui, |ui| {
                     if let Some(s) = current_stats {
                        abilities::render(
                            ui, s, cat_entry, *current_level, icon_sheet, 
                            multihit_texture, kamikaze_texture, boss_wave_immune_texture, 
                            settings, 
                            if form_allows_talents { cat_entry.talent_data.as_ref() } else { None },
                            if form_allows_talents { Some(&*talent_levels) } else { None }
                        );
                     }
                });
        },
        DetailTab::Talents => {
             if let Some(raw) = &cat_entry.talent_data {
                talents::render(ui, raw, icon_sheet, talent_name_cache, skill_descriptions, settings, base_stats, cat_entry.curve.as_ref(), *current_level, talent_levels, cat_entry.id);
             }
        },
        DetailTab::Details => {
             let fallback = Vec::new();
             let desc = cat_entry.description.get(*current_form).unwrap_or(&fallback);
             details::render(ui, desc);
             let text_fallback = Vec::new();
             let ev_text = cat_entry.evolve_text.get(*current_form).unwrap_or(&text_fallback);
             details::render_evolve(ui, ctx, &cat_entry.unit_buy, ev_text, *current_form, gatya_item_textures, cache_version);
        },
        DetailTab::Animation => {
            catviewer::show(ui, ctx, cat_entry, *current_form, anim_viewer, model_data, anim_sheet, settings);
        }
    }
}