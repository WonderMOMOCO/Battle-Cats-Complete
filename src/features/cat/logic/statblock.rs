use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::registry::{get_cat_stat, format_cat_stat};
use crate::features::statblock::logic::builder::StatblockData;
use crate::features::cat::logic::abilities::collect_ability_data;
use crate::features::cat::ui::conjure;
use crate::features::cat::logic::context::CatRenderContext;

pub fn build_cat_statblock(
    ctx: &CatRenderContext,
    cat_entry: &CatEntry,
    current_form: usize,
    level_input: String,
    is_conjure_expanded: bool,
) -> StatblockData {
    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(ctx);

    let spirit_data = if is_conjure_expanded {
        conjure::build_spirit_data(ctx)
    } else { 
        None 
    };

    let anim_frames = cat_entry.atk_anim_frames[current_form];
    let cycle = (get_cat_stat("Atk Cycle").get_value)(ctx.final_stats, anim_frames);
    let atk_type = if ctx.final_stats.area_attack == 0 { "Single" } else { "Area" };

    StatblockData {
        is_cat: true,
        id_str: cat_entry.id_str(current_form),
        name: cat_entry.display_name(current_form),
        icon_path: cat_entry.deploy_icon_paths[current_form].clone(),
        top_label: "Level:".to_string(),
        top_value: level_input,
        hp: ctx.final_stats.hitpoints.to_string(),
        kb: ctx.final_stats.knockbacks.to_string(),
        speed: ctx.final_stats.speed.to_string(),
        cd_label: get_cat_stat("Cooldown").display_name.to_string(),
        cd_value: format_cat_stat("Cooldown", ctx.final_stats, anim_frames),
        is_cd_time: true, 
        cd_frames: (get_cat_stat("Cooldown").get_value)(ctx.final_stats, anim_frames),
        cost_label: get_cat_stat("Cost").display_name.to_string(),
        cost_value: format_cat_stat("Cost", ctx.final_stats, anim_frames),
        atk: format_cat_stat("Attack", ctx.final_stats, anim_frames),
        dps: format_cat_stat("Dps", ctx.final_stats, anim_frames),
        range: ctx.final_stats.standing_range.to_string(),
        atk_cycle: cycle,
        atk_type: atk_type.to_string(),
        traits, h1, h2, b1, b2, footer, spirit_data,
    }
}