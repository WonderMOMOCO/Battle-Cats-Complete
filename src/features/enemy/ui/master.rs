use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::logic::state::EnemyDetailTab;
use crate::features::settings::logic::Settings;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::game::img015;
use crate::global::formats::mamodel::Model;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::global::assets::CustomAssets;

use crate::features::statblock::logic::builder::{StatblockData, generate_and_copy, generate_and_save};
use crate::features::enemy::registry::{get_enemy_stat, format_enemy_stat};

use super::{header, stats, abilities, details, viewer}; 
use super::header::ExportAction;

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    enemy_entry: &EnemyEntry, 
    current_tab: &mut EnemyDetailTab, 
    mag_input: &mut String,
    magnification: &mut i32,
    settings: &mut Settings,
    icon_sheet: &mut SpriteSheet,
    anim_sheet: &mut SpriteSheet,
    model_data: &mut Option<Model>,
    anim_viewer: &mut AnimViewer,
    assets: &CustomAssets, 
    detail_texture: &mut Option<egui::TextureHandle>,
    detail_key: &mut String,
) {
    img015::ensure_loaded(ctx, icon_sheet, settings);

    let export_action = header::render(
        ctx, ui, enemy_entry, current_tab, mag_input, magnification, detail_texture, detail_key,
    );

    match export_action {
        ExportAction::Copy | ExportAction::Save => {
            
            let (traits, h1, h2, b1, b2, footer) = crate::features::enemy::logic::abilities::collect_ability_data(
                &enemy_entry.stats, settings, *magnification
            );

            let frames = enemy_entry.atk_anim_frames;
            let cycle = (get_enemy_stat("Atk Cycle").get_value)(&enemy_entry.stats, frames, *magnification);

            let data = StatblockData {
                id_str: enemy_entry.id_str(),
                name: enemy_entry.display_name(),
                icon_path: enemy_entry.icon_path.clone(),
                top_label: "Magnification:".to_string(),
                top_value: format!("{}%", magnification),
                
                hp: format_enemy_stat("Hitpoints", &enemy_entry.stats, frames, *magnification),
                kb: format_enemy_stat("Knockbacks", &enemy_entry.stats, frames, *magnification),
                speed: format_enemy_stat("Speed", &enemy_entry.stats, frames, *magnification),
                
                cd_label: get_enemy_stat("Endure").display_name.to_string(),
                cd_value: format_enemy_stat("Endure", &enemy_entry.stats, frames, *magnification),
                is_cd_time: false, 
                cd_frames: 0,
                
                cost_label: get_enemy_stat("Cash Drop").display_name.to_string(),
                cost_value: format_enemy_stat("Cash Drop", &enemy_entry.stats, frames, *magnification),
                
                atk: format_enemy_stat("Attack", &enemy_entry.stats, frames, *magnification),
                dps: format_enemy_stat("Dps", &enemy_entry.stats, frames, *magnification),
                range: format_enemy_stat("Range", &enemy_entry.stats, frames, *magnification),
                atk_cycle: cycle,
                atk_type: format_enemy_stat("Atk Type", &enemy_entry.stats, frames, *magnification),
                
                traits, h1, h2, b1, b2, footer, spirit_data: None,
            };

            let lang_clone = settings.general.game_language.clone();
            let cuts_clone = icon_sheet.cuts_map.clone(); 

            if export_action == ExportAction::Copy {
                generate_and_copy(ctx.clone(), lang_clone, data, cuts_clone);
            } else {
                generate_and_save(ctx.clone(), lang_clone, data, cuts_clone);
            }
        },
        ExportAction::None => {}
    }

    ui.separator(); 
    ui.add_space(0.0);

    if *current_tab != EnemyDetailTab::Animation {
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
        EnemyDetailTab::Abilities => {
            stats::render(ui, enemy_entry, *magnification);
            ui.spacing_mut().item_spacing.y = 7.0;
            ui.separator();
            
            egui::ScrollArea::vertical()
                .auto_shrink([false, false]) 
                .show(ui, |ui| {
                    abilities::render(
                        ui, 
                        enemy_entry, 
                        icon_sheet, 
                        assets,
                        settings,
                        *magnification
                    );
                });
        },
        EnemyDetailTab::Details => {
            details::render(ui, &enemy_entry.description);
        },
        EnemyDetailTab::Animation => {
            viewer::show(ui, ctx, enemy_entry, anim_viewer, model_data, anim_sheet, settings);
        }
    }
}