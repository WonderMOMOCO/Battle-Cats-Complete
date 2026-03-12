use crate::features::cat::data::skillacquisition::{TalentRaw, TalentGroupRaw};
use crate::features::cat::data::unitid::CatRaw;
use crate::features::cat::data::unitlevel::CatLevelCurve;
use crate::features::cat::data::skilllevel::TalentCost;
use std::collections::HashMap;
use crate::features::cat::registry::{self, AttrUnit};

// --- CORE MATH ---
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

// --- DYNAMIC UI TEXT ENGINE ---
pub fn calculate_talent_display(
    group: &TalentGroupRaw, 
    base_stats: &CatRaw, 
    talent_level: u8, 
    curve: Option<&CatLevelCurve>, 
    unit_level: i32
) -> Option<String> {
    let def = registry::get_by_talent_id(group.ability_id)?;

    let leveled_base = crate::features::cat::logic::stats::apply_level(base_stats, curve, unit_level);
    let mut mutated = leveled_base.clone();
    let mut dummy_min = leveled_base.clone();
    let mut dummy_max = leveled_base.clone();

    let val1 = calculate_talent_value(group.min_1, group.max_1, talent_level, group.max_level);
    let val2 = calculate_talent_value(group.min_2, group.max_2, talent_level, group.max_level);

    if let Some(apply) = def.apply_func {
        if talent_level > 0 {
            apply(&mut mutated, val1, val2, group);
        }
        
        let val1_min = calculate_talent_value(group.min_1, group.max_1, 1, group.max_level);
        let val2_min = calculate_talent_value(group.min_2, group.max_2, 1, group.max_level);
        apply(&mut dummy_min, val1_min, val2_min, group);

        let val1_max = calculate_talent_value(group.min_1, group.max_1, group.max_level, group.max_level);
        let val2_max = calculate_talent_value(group.min_2, group.max_2, group.max_level, group.max_level);
        apply(&mut dummy_max, val1_max, val2_max, group);
    }

    let max_attrs = (def.get_attributes)(&dummy_max);

    // 1. GENERIC VECTOR ENGINE 
    if !max_attrs.is_empty() {
        let mut diffs_changed = Vec::new();
        let mut diffs_unchanged = Vec::new();
        let mut handled_keys = std::collections::HashSet::new();

        let old_attrs = (def.get_attributes)(&leveled_base);
        let new_attrs = (def.get_attributes)(&mutated);
        let min_attrs = (def.get_attributes)(&dummy_min);

        let get_val = |k: &str, attrs: &[(&'static str, i32, AttrUnit)]| -> i32 {
            attrs.iter().find(|(key, _, _)| *key == k).map(|(_, v, _)| *v).unwrap_or(0)
        };

        if let Some(&("Active", _, _)) = max_attrs.iter().find(|(k, _, _)| *k == "Active") {
            let old_v = get_val("Active", &old_attrs);
            let new_v = get_val("Active", &new_attrs);
            let s_old = if old_v > 0 { "Active" } else { "Inactive" };
            let s_new = if new_v > 0 { "Active" } else { "Inactive" };
            diffs_changed.push(format!("{} -> {}", s_old, s_new));
            handled_keys.insert("Active");
        }

        for &(key, unit) in def.schema {
            if handled_keys.contains(key) { continue; }

            // Range Merger
            if key.starts_with("Min ") {
                let suffix = &key[4..];
                let max_key_str = format!("Max {}", suffix);
                
                if let Some(&(max_key, _)) = def.schema.iter().find(|(k, _)| *k == max_key_str.as_str()) {
                    handled_keys.insert(key);
                    handled_keys.insert(max_key);
                    
                    let old_min = get_val(key, &old_attrs);
                    let new_min = get_val(key, &new_attrs);
                    let min_min = get_val(key, &min_attrs);
                    let max_min = get_val(key, &max_attrs);
                    
                    let old_max = get_val(max_key, &old_attrs);
                    let new_max = get_val(max_key, &new_attrs);
                    let min_max = get_val(max_key, &min_attrs);
                    let max_max = get_val(max_key, &max_attrs);
                    
                    let is_scalable = min_min != max_min || min_max != max_max;
                    let fmt_r = |min, max| { if min == max { format!("{}", min) } else { format!("{}~{}", min, max) } };
                    
                    if is_scalable {
                        let d_min = new_min - old_min;
                        let d_max = new_max - old_max;
                        
                        let diff_str = if d_min == d_max {
                            let sign = if d_min >= 0 { "+" } else { "" };
                            format!("({}{})", sign, d_min)
                        } else {
                            let sign_min = if d_min >= 0 { "+" } else { "" };
                            let sign_max = if d_max >= 0 { "+" } else { "" };
                            format!("({}{}~{}{})", sign_min, d_min, sign_max, d_max)
                        };

                        diffs_changed.push(format!("{}: {} {} -> {}", suffix, fmt_r(old_min, old_max), diff_str, fmt_r(new_min, new_max)));
                    } else {
                        // Edge case fix: Display the Lv1 value even at Lv0 if it doesn't scale
                        diffs_unchanged.push(format!("{}: {}", suffix, fmt_r(min_min, min_max)));
                    }
                    continue;
                }
            }

            // Standard Single Stats
            let old_v = get_val(key, &old_attrs);
            let new_v = get_val(key, &new_attrs);
            let min_v = get_val(key, &min_attrs);
            let max_v = get_val(key, &max_attrs);

            // Logic: Is scalable if Lv1 != MaxLv. 
            // If it is NOT scalable, we force it into unchanged to hide arrows.
            let is_scalable = min_v != max_v;
            
            let fmt_val = |v| match unit {
                AttrUnit::Percent => format!("{}%", v),
                AttrUnit::Frames => format!("{}f", v),
                AttrUnit::Range | AttrUnit::None => format!("{}", v),
            };

            if is_scalable {
                let diff = new_v - old_v;
                let sign = if diff >= 0 { "+" } else { "" };
                let diff_str = match unit {
                    AttrUnit::Percent => format!("({}{}%)", sign, diff),
                    AttrUnit::Frames => format!("({}{}f)", sign, diff),
                    AttrUnit::Range | AttrUnit::None => format!("({}{})", sign, diff),
                };

                diffs_changed.push(format!("{}: {} {} -> {}", key, fmt_val(old_v), diff_str, fmt_val(new_v)));
            } else {
                // Edge case fix: display min_v (Lv1 value) to ensure it shows at Lv0
                diffs_unchanged.push(format!("{}: {}", key, fmt_val(min_v))); 
            }
            handled_keys.insert(key);
        }

        let mut final_diffs = diffs_changed;
        final_diffs.extend(diffs_unchanged);
        
        if !final_diffs.is_empty() {
            return Some(final_diffs.join("\n"));
        }
    }

    // 2. RESISTANCES 
    if def.name.starts_with("Resist ") {
        if val1 == 0 {
            let val1_min = calculate_talent_value(group.min_1, group.max_1, 1, group.max_level);
            let val1_max = calculate_talent_value(group.min_1, group.max_1, group.max_level, group.max_level);
            if val1_min == val1_max {
                return Some(format!("Resist: {}%", val1_min));
            }
            return Some(format!("Resist: 0% (+{}%) -> 0%", val1));
        } else {
            return Some(format!("Resist: 0% (+{}%) -> {}%", val1, val1));
        }
    }

    // 3. BASE STATS 
    if let Some(stat_def) = registry::CAT_STATS_REGISTRY.iter().find(|s| s.linked_talent_id == Some(group.ability_id)) {
        let old_val = (stat_def.get_value)(&leveled_base, 0); 
        let new_val = (stat_def.get_value)(&mutated, 0);
        
        // Edge case: does it scale?
        let val1_min = calculate_talent_value(group.min_1, group.max_1, 1, group.max_level);
        let val1_max = calculate_talent_value(group.min_1, group.max_1, group.max_level, group.max_level);
        
        if val1_min == val1_max {
            let lv1_stats = (stat_def.get_value)(&dummy_min, 0);
            return Some(format!("{}: {}", stat_def.display_name, (stat_def.formatter)(lv1_stats)));
        }

        let old_str = (stat_def.formatter)(old_val);
        let new_str = (stat_def.formatter)(new_val);
        let mod_str = stat_def.talent_modifier_fmt.map(|f| f(val1, val2)).unwrap_or_default();
        
        return Some(format!("{}: {} {} -> {}", stat_def.display_name, old_str, mod_str, new_str));
    }

    None
}

// --- STATE MUTATION ENGINE ---
fn apply_target_traits(stats: &mut CatRaw, name_id: i16, type_id: u16) {
    let mut apply_bit = |bit: u16| {
        match bit {
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
    };

    if name_id >= 0 && name_id <= 11 {
        apply_bit(name_id as u16);
    }

    if type_id > 0 {
        for bit in 0..=11 {
            if (type_id & (1 << bit)) != 0 {
                apply_bit(bit);
            }
        }
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

        if let Some(def) = registry::get_by_talent_id(group.ability_id) {
            if let Some(apply) = def.apply_func {
                apply(&mut stats, val1, val2, group);
            }
        }
    }
    stats
}

// --- COST CALCULATIONS ---
pub fn get_talent_np_cost(cost_id: u8, level: u8, costs_map: &HashMap<u8, TalentCost>) -> i32 {
    if level == 0 { return 0; }
    if let Some(cost_data) = costs_map.get(&cost_id) {
        let limit = (level as usize).min(cost_data.costs.len());
        let mut total = 0;
        for i in 0..limit {
            total += cost_data.costs[i] as i32;
        }
        total
    } else {
        0
    }
}

pub fn get_total_np_cost(
    talent_data: &TalentRaw,
    talent_levels: &HashMap<u8, u8>,
    costs_map: &HashMap<u8, TalentCost>
) -> i32 {
    let mut total = 0;
    for (index, group) in talent_data.groups.iter().enumerate() {
        let level = *talent_levels.get(&(index as u8)).unwrap_or(&0);
        total += get_talent_np_cost(group.cost_id, level, costs_map);
    }
    total
}