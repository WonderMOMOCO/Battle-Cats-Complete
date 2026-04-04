use eframe::egui;
use std::collections::HashMap;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::DetailTab;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::game::img015;
use crate::global::game::img022; 
use crate::features::settings::logic::Settings;
use crate::global::formats::mamodel::Model;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::features::cat::data::skilllevel::TalentCost;
use crate::global::assets::CustomAssets;
use crate::features::statblock::logic::builder::{generate_and_copy, generate_and_save};
use super::{header, stats, abilities, talents, details, viewer};
use super::header::ExportAction;
use crate::features::cat::logic::statblock::build_cat_statblock;

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
    img015_sheets: &mut Vec<SpriteSheet>,   
    img022_sheets: &mut Vec<SpriteSheet>, 
    anim_sheet: &mut SpriteSheet,   
    model_data: &mut Option<Model>,
    anim_viewer: &mut AnimViewer,
    assets: &CustomAssets, 
    talent_name_cache: &mut HashMap<String, egui::TextureHandle>,
    gatya_item_textures: &mut HashMap<i32, Option<egui::TextureHandle>>,
    skill_descriptions: Option<&Vec<String>>, 
    settings: &mut Settings,
    talent_levels: &mut HashMap<u8, u8>,
    talent_costs: &HashMap<u8, TalentCost>,
    cache_version: u64,
) {
    img015::ensure_loaded(ctx, img015_sheets, settings);
    img022::ensure_loaded(ctx, img022_sheets, settings);

    let export_action = header::render(
        ctx, ui, cat_entry, current_form, current_tab, current_level, level_input, texture_cache, current_key, settings, talent_levels, talent_costs, img022_sheets
    );

    // Bypass scanner cache and fetch modded stats dynamically
    let dynamic_stats = crate::features::cat::logic::stats::load_from_id(cat_entry.id as i32, &settings.general.language_priority);
    let base_stats = dynamic_stats.as_ref().and_then(|v| v.get(*current_form));
    let form_allows_talents = *current_form >= 2;

    let final_stats_owned = if let Some(base) = base_stats {
        Some(crate::features::cat::logic::stats::get_final_stats(
            base,
            cat_entry.curve.as_ref(),
            *current_level,
            if form_allows_talents { cat_entry.talent_data.as_ref() } else { None },
            if form_allows_talents { Some(&*talent_levels) } else { None }
        ))
    } else { None };

    match export_action {
        ExportAction::Copy | ExportAction::Save => {
            if let (Some(final_s), Some(base_s)) = (final_stats_owned.as_ref(), base_stats) {
                
                let expand_id = egui::Id::new(format!("conjure_expand_{}", cat_entry.id));
                let is_conjure_expanded = ctx.data(|d| d.get_temp::<bool>(expand_id).unwrap_or(settings.cat_data.expand_spirit_details));

                let data = build_cat_statblock(
                    cat_entry,
                    *current_form,
                    final_s,
                    base_s,
                    *current_level,
                    level_input.clone(),
                    settings,
                    talent_levels,
                    is_conjure_expanded,
                );

                let priority_clone = settings.general.language_priority.clone();
                let mut cuts_clone = std::collections::HashMap::new();
                for sheet in img015_sheets.iter().rev() {
                    cuts_clone.extend(sheet.cuts_map.clone());
                }

                if export_action == ExportAction::Copy {
                    generate_and_copy(ctx.clone(), priority_clone, data, cuts_clone);
                } else {
                    generate_and_save(ctx.clone(), priority_clone, data, cuts_clone);
                }
            }
        },
        ExportAction::None => {}
    }

    ui.separator(); 
    ui.add_space(0.0);

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
            if let (Some(final_s), Some(base_s)) = (final_stats_owned.as_ref(), base_stats) {
                stats::render(ui, cat_entry, final_s, *current_form);
                ui.spacing_mut().item_spacing.y = 7.0;
                ui.separator(); 
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false]) 
                    .show(ui, |ui| {
                        abilities::render(
                            ui, final_s, base_s, cat_entry, *current_level, img015_sheets, 
                            assets,
                            settings, 
                            if form_allows_talents { cat_entry.talent_data.as_ref() } else { None },
                            if form_allows_talents { Some(&*talent_levels) } else { None }
                        );
                    });
            }
        },
        DetailTab::Talents => {
             if let Some(raw) = &cat_entry.talent_data {
                talents::render(ui, raw, img015_sheets, img022_sheets, talent_name_cache, skill_descriptions, settings, base_stats, cat_entry.curve.as_ref(), *current_level, talent_levels, cat_entry.id, talent_costs, assets);
             }
        },
        DetailTab::Details => {
             let fallback = Vec::new();
             let desc = cat_entry.description.get(*current_form).unwrap_or(&fallback);
             details::render(ui, desc);
             let text_fallback = Vec::new();
             let ev_text = cat_entry.evolve_text.get(*current_form).unwrap_or(&text_fallback);
             details::render_evolve(
                ui, 
                ctx, 
                &cat_entry.unit_buy, 
                ev_text, 
                *current_form, 
                gatya_item_textures, 
                cache_version, 
                &settings.general.language_priority 
            );
        }
        DetailTab::Animation => {
            viewer::show(ui, ctx, cat_entry, *current_form, anim_viewer, model_data, anim_sheet, settings);
        }
    }
}