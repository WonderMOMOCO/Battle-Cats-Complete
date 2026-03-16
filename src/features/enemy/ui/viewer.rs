use eframe::egui;
use std::path::{Path, PathBuf};

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
    let priority = &settings.general.language_priority;
    let mut available_anims = Vec::new();

    let resolve = |p: PathBuf| {
        let parent = p.parent()?;
        let name = p.file_name()?.to_str()?;
        crate::global::get(parent, name, priority).into_iter().next()
    };

    // Standard anims
    for idx in [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB] {
        let Some(path) = resolve(paths::maanim(root, enemy_entry.id, idx)) else { continue; };
        available_anims.push((idx, path));
    }

    // Zombie anims
    if let Some(p) = resolve(paths::zombie_maanim(root, enemy_entry.id, 0)) { available_anims.push((IDX_BURROW, p)); }
    if let Some(p) = resolve(paths::zombie_maanim(root, enemy_entry.id, 1)) { available_anims.push((7, p)); }
    if let Some(p) = resolve(paths::zombie_maanim(root, enemy_entry.id, 2)) { available_anims.push((IDX_SURFACE, p)); }

    // Primary Assets
    let primary_assets = (|| {
        let png = resolve(paths::anim(root, enemy_entry.id, AnimType::Png))?;
        let cut = resolve(paths::anim(root, enemy_entry.id, AnimType::Imgcut))?;
        let model = resolve(paths::anim(root, enemy_entry.id, AnimType::Mamodel))?;
        Some((png, cut, model))
    })();

    let primary_id = format!("{}_{}", enemy_entry.id_str(), anim_viewer.texture_version);
    anim_viewer.show(ui, ctx, &primary_id, &String::new(), &available_anims, primary_assets, None, model_data, anim_sheet, settings);
}