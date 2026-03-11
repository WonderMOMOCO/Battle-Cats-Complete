use eframe::egui;
use std::path::Path;

use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::formats::mamodel::Model;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::features::settings::logic::Settings;
use crate::features::enemy::paths::{self, AnimType};
use crate::features::animation::ui::controls::{
    IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE
};

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    enemy_entry: &EnemyEntry,
    anim_viewer: &mut AnimViewer,
    model_data: &mut Option<Model>,
    anim_sheet: &mut SpriteSheet,
    settings: &mut Settings,
) {
    let root = Path::new(paths::DIR_ENEMIES);
    
    let mut available_anims = Vec::new();
    
    // Standard animations (00 = Walk, 01 = Idle, 02 = Attack, 03 = Knockback)
    let standard_defs = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB];
    for idx in standard_defs {
        let path = paths::maanim(root, enemy_entry.id, idx);
        if path.exists() {
            available_anims.push((idx, path));
        }
    }

    // zombie00 = Burrow Down (IDX_BURROW)
    // zombie01 = Underground Idle (Dummy/Generic)
    // zombie02 = Surface / Revive (IDX_SURFACE)

    let z_burrow = paths::zombie_maanim(root, enemy_entry.id, 0);
    if z_burrow.exists() {
        available_anims.push((IDX_BURROW, z_burrow));
    }

    let z_underground = paths::zombie_maanim(root, enemy_entry.id, 1);
    if z_underground.exists() {
        // Map to 7 to prevent UI button overlap
        available_anims.push((7, z_underground)); 
    }

    let z_surface = paths::zombie_maanim(root, enemy_entry.id, 2);
    if z_surface.exists() {
        available_anims.push((IDX_SURFACE, z_surface));
    }

    // Base Assets (Png, Imgcut, Mamodel)
    let base_png = paths::anim(root, enemy_entry.id, AnimType::Png);
    let base_cut = paths::anim(root, enemy_entry.id, AnimType::Imgcut);
    let base_model = paths::anim(root, enemy_entry.id, AnimType::Mamodel);

    let primary_assets = if base_png.exists() && base_cut.exists() && base_model.exists() {
        Some((base_png, base_cut, base_model))
    } else {
        None
    };

    let secondary_assets = None;
    let secondary_id = String::new();
    
    let primary_id = format!("{}_{}", enemy_entry.id_str(), anim_viewer.texture_version);
    // Hand over to the AnimViewer
    anim_viewer.show(
        ui,
        ctx,
        &primary_id,
        &secondary_id,
        &available_anims,
        primary_assets,
        secondary_assets,
        model_data,
        anim_sheet,
        settings
    );
}