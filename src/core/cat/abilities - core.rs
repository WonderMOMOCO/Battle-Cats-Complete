use crate::data::global::img015;
use crate::core::settings::Settings;
use super::stats::{self, CatRaw};
use eframe::egui;
use crate::data::cat::skillacquisition::TalentRaw;
use std::collections::HashMap;
use crate::core::registries::cat::{self, DisplayGroup};

pub struct AbilityItem {
    pub icon_id: usize,
    pub text: String,
    pub custom_tex: Option<egui::TextureId>,
    pub border_id: Option<usize>,
}

pub fn collect_ability_data(
    cat_stats: &CatRaw,
    current_level: i32,
    level_curve: Option<&stats::CatLevelCurve>,
    multihit_texture: &Option<egui::TextureHandle>,
    kamikaze_texture: &Option<egui::TextureHandle>,
    boss_wave_texture: &Option<egui::TextureHandle>,
    settings: &Settings, 
    is_conjure_unit: bool,
    talent_data: Option<&TalentRaw>,
    talent_levels: Option<&HashMap<u8, u8>>
) -> (Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>) {
    
    let mut group_trait = Vec::new();
    let mut group_headline_1 = Vec::new();
    let mut group_headline_2 = Vec::new();
    let mut group_body_1 = Vec::new();
    let mut group_body_2 = Vec::new();
    let mut group_footer = Vec::new();

    let get_talent_border = |ability_id: u8| -> Option<usize> {
        if ability_id == 0 { return None; }
        if let (Some(data), Some(levels)) = (talent_data, talent_levels) {
            
            // Check direct ID match
            let check_id = |target_id: u8| -> Option<usize> {
                if let Some((idx, group)) = data.groups.iter().enumerate().find(|(_, g)| g.ability_id == target_id) {
                    let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
                    if lv > 0 {
                        let effective_max = if group.max_level == 0 { 1 } else { group.max_level };
                        return Some(if lv >= effective_max { img015::BORDER_GOLD } else { img015::BORDER_RED });
                    }
                }
                None
            };

            if let Some(border) = check_id(ability_id) { return Some(border); }
            if ability_id == 23 { if let Some(border) = check_id(48) { return Some(border); } }

            // Check for Trait Side-Effects
            if is_trait_id(ability_id) {
                for (idx, group) in data.groups.iter().enumerate() {
                    let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
                    if lv > 0 {
                        // type_id is sourced from the parent TalentRaw
                        if enables_trait(group.name_id, data.type_id, ability_id) {
                            return Some(img015::BORDER_GOLD);
                        }
                    }
                }
            }
        }
        None
    };

    let push_custom = |target_list: &mut Vec<AbilityItem>, texture: &Option<egui::TextureHandle>, text: String| {
        if let Some(tex) = texture {
            target_list.push(AbilityItem { icon_id: 0, text, custom_tex: Some(tex.id()), border_id: None });
        } else {
            target_list.push(AbilityItem { icon_id: img015::ICON_CONJURE, text, custom_tex: None, border_id: None });
        }
    };

    let target_label = if is_conjure_unit { "Enemies" } else { "Target Traits" };

    
    // Multi-Hit
    if cat_stats.attack_2 > 0 {
        let damage_hit_1 = level_curve.map_or(cat_stats.attack_1, |curve| curve.calculate_stat(cat_stats.attack_1, current_level));
        let damage_hit_2 = level_curve.map_or(cat_stats.attack_2, |curve| curve.calculate_stat(cat_stats.attack_2, current_level));
        let damage_hit_3 = level_curve.map_or(cat_stats.attack_3, |curve| curve.calculate_stat(cat_stats.attack_3, current_level));
        
        let ability_flag_1 = if cat_stats.attack_1_abilities > 0 { "True" } else { "False" };
        let ability_flag_2 = if cat_stats.attack_2_abilities > 0 { "True" } else { "False" };
        let ability_flag_3 = if cat_stats.attack_3 > 0 { if cat_stats.attack_3_abilities > 0 { " / True" } else { " / False" } } else { "" };
        
        let damage_string = if cat_stats.attack_3 > 0 { 
            format!("{} / {} / {}", damage_hit_1, damage_hit_2, damage_hit_3) 
        } else { 
            format!("{} / {}", damage_hit_1, damage_hit_2) 
        };
        let multihit_description = format!("Damage split {}\nAbility split {} / {}{}", damage_string, ability_flag_1, ability_flag_2, ability_flag_3);
        let effective_multihit_texture = if settings.game_language == "--" { None } else { multihit_texture.as_ref().map(|t| t.id()) };

        group_body_1.push(AbilityItem { icon_id: img015::ICON_MULTIHIT, text: multihit_description, custom_tex: effective_multihit_texture, border_id: None });
    }

    // Range (Long Distance / Omni Strike)
    range_logic(cat_stats, &mut group_body_1);

    if !is_conjure_unit && cat_stats.conjure_unit_id > 0 {
        push_custom(&mut group_body_1, &None, "Conjures a Spirit to the battlefield when tapped\nThis Cat may only be deployed one at a time".to_string());
    }

    // REGISTRY LOOP
    for def in cat::ABILITY_REGISTRY {
        if is_conjure_unit {
            if def.group == DisplayGroup::Trait || def.group == DisplayGroup::Headline1 { continue; } 
            if def.name == "Dodge" || def.name == "Immune Boss Wave" || def.name == "Conjure" { continue; }
        }

        let val = (def.getter)(cat_stats);
        if val > 0 {
            let dur = if let Some(d_get) = def.duration_getter { d_get(cat_stats) } else { 0 };
            let text = (def.formatter)(val, cat_stats, target_label, dur);
            let border = get_talent_border(def.talent_id);

            let custom_tex = if def.name == "Immune Boss Wave" {
                boss_wave_texture.as_ref().map(|t| t.id())
            } else { None };

            let mut final_icon = def.icon_id;
            if def.name == "Wave Attack" && cat_stats.mini_wave_flag > 0 { final_icon = img015::ICON_MINI_WAVE; }
            else if def.name == "Surge Attack" && cat_stats.mini_surge_flag > 0 { final_icon = img015::ICON_MINI_SURGE; }

            let item = AbilityItem { icon_id: final_icon, text, custom_tex, border_id: border };

            match def.group {
                DisplayGroup::Trait => group_trait.push(item),
                DisplayGroup::Headline1 => group_headline_1.push(item),
                DisplayGroup::Headline2 => group_headline_2.push(item),
                DisplayGroup::Body1 => group_body_1.push(item),
                DisplayGroup::Body2 => group_body_2.push(item),
                DisplayGroup::Footer => group_footer.push(item),
            }
        }
    }

    if !is_conjure_unit && cat_stats.kamikaze == 2 {
        if let Some(tex) = kamikaze_texture {
             let item = AbilityItem { icon_id: 0, text: "Unit disappears after a single attack".into(), custom_tex: Some(tex.id()), border_id: None };
             group_headline_2.push(item);
        }
    }

    if let (Some(t_data), Some(levels)) = (talent_data, talent_levels) {
        let mut talent_headline = Vec::new();

        for (idx, group) in t_data.groups.iter().enumerate() {
            let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
            if lv == 0 { continue; }

            if let Some(def) = cat::get_by_talent_id(group.ability_id) {
                if let Some(desc_gen) = def.talent_desc_func {
                    let v1 = crate::core::cat::talents::calculate_talent_value(group.min_1, group.max_1, lv, group.max_level);
                    let v2 = crate::core::cat::talents::calculate_talent_value(group.min_2, group.max_2, lv, group.max_level);
                    
                    let text = desc_gen(v1, v2, cat_stats, level_curve, current_level, group, lv);
                    
                    match group.ability_id {
                        25 | 26 | 27 | 31 | 32 | 61 | 82 => { // Stats
                            let item = AbilityItem { icon_id: def.icon_id, text, custom_tex: None, border_id: get_talent_border(def.talent_id) };
                            talent_headline.push(item);
                        },
                        18 | 19 | 20 | 21 | 22 | 24 | 30 | 52 | 54 => { // Resists
                            let item = AbilityItem { icon_id: def.icon_id, text, custom_tex: None, border_id: get_talent_border(def.talent_id) };
                            group_footer.push(item);
                        },
                        _ => {}
                    }
                }
            }
        }
        let mut new_h2 = talent_headline;
        new_h2.append(&mut group_headline_2);
        group_headline_2 = new_h2;
    }

    (group_trait, group_headline_1, group_headline_2, group_body_1, group_body_2, group_footer)
}

fn range_logic(cat_stats: &CatRaw, group_body_1: &mut Vec<AbilityItem>) {
    let enemy_base_range = {
        let start_range = cat_stats.long_distance_1_anchor;
        let end_range = cat_stats.long_distance_1_anchor + cat_stats.long_distance_1_span;
        let (min_reach, max_reach) = if start_range < end_range { (start_range, end_range) } else { (end_range, start_range) };
        if min_reach > 0 { min_reach } else { max_reach }
    };

    let mut is_omni_strike = false;
    let mut range_strings = Vec::new();
    let range_checks = [
        (cat_stats.long_distance_1_anchor, cat_stats.long_distance_1_span),
        (cat_stats.long_distance_2_anchor, if cat_stats.long_distance_2_flag == 1 { cat_stats.long_distance_2_span } else { 0 }),
        (cat_stats.long_distance_3_anchor, if cat_stats.long_distance_3_flag == 1 { cat_stats.long_distance_3_span } else { 0 }),
    ];
    
    for (anchor, span) in range_checks {
        if span != 0 {
            let start = anchor;
            let end = anchor + span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            if min <= 0 { is_omni_strike = true; }
            range_strings.push(format!("{}~{}", min, max));
        }
    }

    if range_strings.len() > 1 {
        let first = &range_strings[0];
        if range_strings.iter().all(|s| s == first) {
            range_strings.truncate(1);
        }
    }

    if !range_strings.is_empty() {
        let label_prefix = if range_strings.len() > 1 { "Range split" } else { "Effective Range" };
        let range_description = format!(
            "{} {}\nStands at {} Range relative to Enemy Base", 
            label_prefix,
            range_strings.join(" / "), 
            enemy_base_range
        );
        let icon = if is_omni_strike { img015::ICON_OMNI_STRIKE } else { img015::ICON_LONG_DISTANCE };
        group_body_1.push(AbilityItem { icon_id: icon, text: range_description, custom_tex: None, border_id: None });
    }
}

fn is_trait_id(id: u8) -> bool {
    (33..=41).contains(&id) || id == 57
}

fn enables_trait(name_id: i16, type_id: u16, target_id: u8) -> bool {
    let bit_idx = match target_id {
        33 => 0, 34 => 1, 35 => 2, 36 => 3, 37 => 4, 38 => 5, 39 => 6, 40 => 7, 41 => 8, 57 => 11,
        _ => return false,
    };
    if name_id == bit_idx { return true; }
    if type_id > 0 && (type_id & (1 << bit_idx)) != 0 { return true; }
    false
}