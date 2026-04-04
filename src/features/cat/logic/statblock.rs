use std::collections::HashMap;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::stats::CatRaw;
use crate::features::settings::logic::Settings;
use crate::features::cat::registry::{get_cat_stat, format_cat_stat};
use crate::features::statblock::logic::builder::StatblockData;
use crate::features::cat::logic::abilities::collect_ability_data;
use crate::features::cat::ui::conjure;

pub fn build_cat_statblock(
    cat_entry: &CatEntry,
    current_form: usize,
    final_s: &CatRaw,
    base_s: &CatRaw,
    current_level: i32,
    level_input: String,
    settings: &Settings,
    talent_levels: &HashMap<u8, u8>,
    is_conjure_expanded: bool,
) -> StatblockData {
    let form_allows_talents = current_form >= 2;
    
    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(
        final_s, base_s, current_level, cat_entry.curve.as_ref(), settings, false,
        if form_allows_talents { cat_entry.talent_data.as_ref() } else { None },
        if form_allows_talents { Some(talent_levels) } else { None }
    );

    let spirit_data = if is_conjure_expanded {
        conjure::build_spirit_data(base_s, current_level, cat_entry.curve.as_ref(), settings)
    } else { 
        None 
    };

    let anim_frames = cat_entry.atk_anim_frames[current_form];
    let cycle = (get_cat_stat("Atk Cycle").get_value)(final_s, anim_frames);
    let atk_type = if final_s.area_attack == 0 { "Single" } else { "Area" };

    StatblockData {
        is_cat: true,
        id_str: cat_entry.id_str(current_form),
        name: cat_entry.display_name(current_form),
        icon_path: cat_entry.deploy_icon_paths[current_form].clone(),
        top_label: "Level:".to_string(),
        top_value: level_input,
        hp: final_s.hitpoints.to_string(),
        kb: final_s.knockbacks.to_string(),
        speed: final_s.speed.to_string(),
        cd_label: get_cat_stat("Cooldown").display_name.to_string(),
        cd_value: format_cat_stat("Cooldown", final_s, anim_frames),
        is_cd_time: true, 
        cd_frames: (get_cat_stat("Cooldown").get_value)(final_s, anim_frames),
        cost_label: get_cat_stat("Cost").display_name.to_string(),
        cost_value: format_cat_stat("Cost", final_s, anim_frames),
        atk: format_cat_stat("Attack", final_s, anim_frames),
        dps: format_cat_stat("Dps", final_s, anim_frames),
        range: final_s.standing_range.to_string(),
        atk_cycle: cycle,
        atk_type: atk_type.to_string(),
        traits, h1, h2, b1, b2, footer, spirit_data,
    }
}