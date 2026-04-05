use eframe::egui;
use crate::features::enemy::logic::scanner::{self, EnemyEntry};
use crate::features::enemy::logic::state::EnemyDetailTab;
use crate::features::settings::logic::Settings;
use crate::features::enemy::registry::Magnification;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::game::img015;
use crate::global::formats::mamodel::Model;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::global::assets::CustomAssets;
use crate::global::game::param::Param;
use crate::global::context::GlobalContext;
use crate::features::enemy::logic::context::EnemyRenderContext;
use crate::features::statblock::logic::builder::{generate_and_copy, generate_and_save};
use crate::features::enemy::logic::statblock::build_enemy_statblock;
use super::{header, stats, abilities, details, viewer}; 
use super::header::ExportAction;

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    enemy_entry: &EnemyEntry, 
    current_tab: &mut EnemyDetailTab, 
    mag_input: &mut String,
    magnification: &mut Magnification,
    settings: &mut Settings,
    img015_sheets: &mut Vec<SpriteSheet>,
    anim_sheet: &mut SpriteSheet,
    model_data: &mut Option<Model>,
    anim_viewer: &mut AnimViewer,
    assets: &CustomAssets, 
    detail_texture: &mut Option<egui::TextureHandle>,
    detail_key: &mut String,
    param: &Param,
) {
    img015::ensure_loaded(ctx, img015_sheets, settings);

    let export_action = header::render(
        ctx, ui, enemy_entry, current_tab, mag_input, magnification, detail_texture, detail_key,
    );

    let dynamic_entry = scanner::scan_single(enemy_entry.id, &settings.scanner_config());
    let stats = dynamic_entry.as_ref().map(|e| &e.stats).unwrap_or(&enemy_entry.stats);

    let global_ctx = GlobalContext {
        settings: &*settings,
        param,
        assets,
    };

    let enemy_ctx = EnemyRenderContext {
        global: global_ctx,
        stats,
        magnification: *magnification,
    };

    match export_action {
        ExportAction::Copy | ExportAction::Save => {
            let data = build_enemy_statblock(&enemy_ctx, enemy_entry);

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
                        &enemy_ctx,
                        img015_sheets, 
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