use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::settings::logic::Settings;
use crate::features::enemy::registry::{get_enemy_stat, format_enemy_stat, Magnification};
use crate::features::statblock::logic::builder::StatblockData;
use crate::features::enemy::logic::abilities::collect_ability_data;
use crate::features::enemy::data::t_unit::EnemyRaw;

pub fn build_enemy_statblock(
    enemy_entry: &EnemyEntry,
    stats: &EnemyRaw,
    settings: &Settings,
    magnification: Magnification, 
) -> StatblockData {
    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(stats, settings, magnification);

    let frames = enemy_entry.atk_anim_frames;
    let cycle = (get_enemy_stat("Atk Cycle").get_value)(stats, frames, magnification);

    let top_val_str = if magnification.hitpoints == magnification.attack {
        format!("{}%", magnification.hitpoints)
    } else {
        format!("{}%/{}%", magnification.hitpoints, magnification.attack)
    };

    StatblockData {
        is_cat: false,
        id_str: enemy_entry.id_str(),
        name: enemy_entry.display_name(),
        icon_path: enemy_entry.icon_path.clone(),
        top_label: "Magnification:".to_string(),
        top_value: top_val_str,
        
        hp: format_enemy_stat("Hitpoints", stats, frames, magnification),
        kb: format_enemy_stat("Knockbacks", stats, frames, magnification),
        speed: format_enemy_stat("Speed", stats, frames, magnification),
        
        cd_label: get_enemy_stat("Endure").display_name.to_string(),
        cd_value: format_enemy_stat("Endure", stats, frames, magnification),
        is_cd_time: false, 
        cd_frames: 0,
        
        cost_label: get_enemy_stat("Cash Drop").display_name.to_string(),
        cost_value: format_enemy_stat("Cash Drop", stats, frames, magnification),
        
        atk: format_enemy_stat("Attack", stats, frames, magnification),
        dps: format_enemy_stat("Dps", stats, frames, magnification),
        range: format_enemy_stat("Range", stats, frames, magnification),
        atk_cycle: cycle,
        atk_type: format_enemy_stat("Atk Type", stats, frames, magnification),
        
        traits, h1, h2, b1, b2, footer, spirit_data: None,
    }
}