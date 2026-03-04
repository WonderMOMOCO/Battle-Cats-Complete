use eframe::egui;
use std::path::{Path};

use crate::features::cat::logic::scanner::CatEntry;
use crate::global_data::imgcut::SpriteSheet;
use crate::global_data::mamodel::Model;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::features::settings::logic::Settings;
use crate::features::cat::paths::{self, AnimType};
use crate::features::animation::ui::controls::{
    IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE
};

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cat_entry: &CatEntry,
    current_form: usize,
    anim_viewer: &mut AnimViewer,
    model_data: &mut Option<Model>,
    anim_sheet: &mut SpriteSheet,
    settings: &mut Settings,
) {
    let root = Path::new(paths::DIR_CATS);
    let egg_ids = cat_entry.egg_ids;
    
    // 1. Gather Available Animations
    let mut available_anims = Vec::new();
    let anim_defs = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];
    
    for idx in anim_defs {
        let path = paths::maanim(root, cat_entry.id, current_form, egg_ids, idx);
        if path.exists() { available_anims.push((idx, path)); }
    }

    // 2. Gather Primary Base Assets
    let std_png = paths::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Png);
    let std_cut = paths::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Imgcut);
    let std_model = paths::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Mamodel);
    
    let primary_assets = if std_png.exists() && std_cut.exists() && std_model.exists() {
        Some((std_png, std_cut, std_model))
    } else {
        None
    };

    // 3. Gather Secondary Entity Logic (Spirit)
    let conjure_id = if let Some(Some(stats)) = cat_entry.stats.get(current_form) {
        if stats.conjure_unit_id > 0 { Some(stats.conjure_unit_id as u32) } else { None }
    } else { None };

    let mut secondary_assets = None;
    let secondary_id = if let Some(s_id) = conjure_id { 
        let s_png = paths::anim(root, s_id, 0, (-1, -1), AnimType::Png);
        let s_cut = paths::anim(root, s_id, 0, (-1, -1), AnimType::Imgcut);
        let s_model = paths::anim(root, s_id, 0, (-1, -1), AnimType::Mamodel);
        let s_atk = paths::maanim(root, s_id, 0, (-1, -1), 2); 

        if s_png.exists() && s_cut.exists() && s_model.exists() && s_atk.exists() {
            secondary_assets = Some((s_png, s_cut, s_model, s_atk));
        }
        format!("spirit_{}_{}", s_id, anim_viewer.texture_version)
    } else { 
        String::new() 
    };
    
    // 4. Formatting IDs
    let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
    let id_str = format!("{:03}", cat_entry.id);
    let primary_id = format!("{}_{}_{}", id_str, form_char, anim_viewer.texture_version);

    // 5. Delegate entirely to the Animation Engine
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