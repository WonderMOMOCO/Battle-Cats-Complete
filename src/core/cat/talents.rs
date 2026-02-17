use crate::data::cat::skillacquisition::{TalentRaw, TalentGroupRaw};
use crate::data::cat::unitid::CatRaw;
use crate::data::cat::unitlevel::CatLevelCurve;
use std::collections::HashMap;
use crate::core::registries::cat;

pub fn calculate_talent_value(min: u16, max: u16, level: u8, max_level: u8) -> i32 {
    if level == 0 { return 0; }
    if max_level <= 1 { return min as i32; }
    if level == 1 { return min as i32; }
    if level == max_level { return max as i32; }

    let min_f = min as f32;
    let max_f = max as f32;
    let lvl_f = level as f32;
    let max_lvl_f = max_level as f32;

    let val = min_f + (max_f - min_f) * (lvl_f - 1.0) / (max_lvl_f - 1.0);
    val.round() as i32
}

pub fn calculate_talent_display(
    group: &TalentGroupRaw, 
    stats: &CatRaw, 
    talent_level: u8, 
    curve: Option<&CatLevelCurve>, 
    unit_level: i32
) -> Option<String> {
    
    // Look up in Registry
    let def = cat::get_by_talent_id(group.ability_id)?;
    let formatter = def.talent_desc_func?;

    // Calculate Values
    let val1 = calculate_talent_value(group.min_1, group.max_1, talent_level, group.max_level);
    let val2 = calculate_talent_value(group.min_2, group.max_2, talent_level, group.max_level);

    // Format with FULL context
    Some(formatter(val1, val2, stats, curve, unit_level, group, talent_level))
}

fn apply_target_traits(stats: &mut CatRaw, name_id: i16, type_id: u16) {
    match name_id {
        0 => stats.target_red = 1,
        1 => stats.target_floating = 1,
        2 => stats.target_black = 1,
        3 => stats.target_metal = 1,
        4 => stats.target_angel = 1,
        5 => stats.target_alien = 1,
        6 => stats.target_zombie = 1,
        7 => stats.target_relic = 1,
        8 => stats.target_traitless = 1,
        9 => stats.target_witch = 1,
        10 => stats.target_eva = 1,
        11 => stats.target_aku = 1,
        _ => {}
    }

    if type_id > 0 {
        if (type_id & (1 << 0)) != 0 { stats.target_red = 1; }
        if (type_id & (1 << 1)) != 0 { stats.target_floating = 1; }
        if (type_id & (1 << 2)) != 0 { stats.target_black = 1; }
        if (type_id & (1 << 3)) != 0 { stats.target_metal = 1; }
        if (type_id & (1 << 4)) != 0 { stats.target_angel = 1; }
        if (type_id & (1 << 5)) != 0 { stats.target_alien = 1; }
        if (type_id & (1 << 6)) != 0 { stats.target_zombie = 1; }
        if (type_id & (1 << 7)) != 0 { stats.target_relic = 1; }
        if (type_id & (1 << 8)) != 0 { stats.target_traitless = 1; }
        if (type_id & (1 << 9)) != 0 { stats.target_witch = 1; }
        if (type_id & (1 << 10)) != 0 { stats.target_eva = 1; }
        if (type_id & (1 << 11)) != 0 { stats.target_aku = 1; }
    }
}

pub fn apply_talent_stats(base_stats: &CatRaw, talent_data: &TalentRaw, levels: &HashMap<u8, u8>) -> CatRaw {
    let mut stats = base_stats.clone();
    
    for (index, group) in talent_data.groups.iter().enumerate() {
        let current_level = *levels.get(&(index as u8)).unwrap_or(&0);
        
        if current_level > 0 && group.name_id != -1 {
            apply_target_traits(&mut stats, group.name_id, talent_data.type_id);
        }

        if current_level == 0 { continue; }
        
        let val1 = calculate_talent_value(group.min_1, group.max_1, current_level, group.max_level);
        let val2 = calculate_talent_value(group.min_2, group.max_2, current_level, group.max_level);

        // Registry Lookup
        if let Some(def) = cat::get_by_talent_id(group.ability_id) {
            if let Some(apply) = def.apply_func {
                apply(&mut stats, val1, val2, group);
            }
        }
    }
    stats
}