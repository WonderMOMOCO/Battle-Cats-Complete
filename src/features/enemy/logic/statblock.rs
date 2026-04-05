use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::registry::{get_enemy_stat, format_enemy_stat};
use crate::features::statblock::logic::builder::StatblockData;
use crate::features::enemy::logic::abilities::collect_ability_data;
use crate::features::enemy::logic::context::EnemyRenderContext;

pub fn build_enemy_statblock(
    ctx: &EnemyRenderContext,
    enemy_entry: &EnemyEntry,
) -> StatblockData {
    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(ctx);

    let frames = enemy_entry.atk_anim_frames;
    let cycle = (get_enemy_stat("Atk Cycle").get_value)(ctx.stats, frames, ctx.magnification);

    let top_val_str = if ctx.magnification.hitpoints == ctx.magnification.attack {
        format!("{}%", ctx.magnification.hitpoints)
    } else {
        format!("{}%/{}%", ctx.magnification.hitpoints, ctx.magnification.attack)
    };

    StatblockData {
        is_cat: false,
        id_str: enemy_entry.id_str(),
        name: enemy_entry.display_name(),
        icon_path: enemy_entry.icon_path.clone(),
        top_label: "Magnification:".to_string(),
        top_value: top_val_str,
        
        hp: format_enemy_stat("Hitpoints", ctx.stats, frames, ctx.magnification),
        kb: format_enemy_stat("Knockbacks", ctx.stats, frames, ctx.magnification),
        speed: format_enemy_stat("Speed", ctx.stats, frames, ctx.magnification),
        
        cd_label: get_enemy_stat("Endure").display_name.to_string(),
        cd_value: format_enemy_stat("Endure", ctx.stats, frames, ctx.magnification),
        is_cd_time: false, 
        cd_frames: 0,
        
        cost_label: get_enemy_stat("Cash Drop").display_name.to_string(),
        cost_value: format_enemy_stat("Cash Drop", ctx.stats, frames, ctx.magnification),
        
        atk: format_enemy_stat("Attack", ctx.stats, frames, ctx.magnification),
        dps: format_enemy_stat("Dps", ctx.stats, frames, ctx.magnification),
        range: format_enemy_stat("Range", ctx.stats, frames, ctx.magnification),
        atk_cycle: cycle,
        atk_type: format_enemy_stat("Atk Type", ctx.stats, frames, ctx.magnification),
        
        traits, h1, h2, b1, b2, footer, spirit_data: None,
    }
}