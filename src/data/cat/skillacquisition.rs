#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::data::global::img015;
use crate::core::utils;
use crate::paths::cat;

#[derive(Debug, Clone)]
pub struct TalentRaw {
    pub id: u16,
    pub type_id: u16, 
    pub groups: Vec<TalentGroupRaw>,
}

#[derive(Debug, Clone)]
pub struct TalentGroupRaw {
    pub ability_id: u8,
    pub max_level: u8,
    pub min_1: u16, pub max_1: u16,
    pub min_2: u16, pub max_2: u16,
    pub min_3: u16, pub max_3: u16,
    pub min_4: u16, pub max_4: u16,
    pub text_id: u8,
    pub cost_id: u8,
    pub name_id: i16,
    pub limit: u8, 
}

pub fn load(cats_directory: &Path) -> HashMap<u16, TalentRaw> {
    let mut map = HashMap::new();
    let file_path = cats_directory.join(cat::SKILL_ACQUISITION);
    if let Ok(content) = fs::read_to_string(&file_path) {
        let delimiter = utils::detect_csv_separator(&content);
        for line in content.lines() {
            let p: Vec<&str> = line.split(delimiter).collect();
            if p.len() < 2 { continue; }

            let id = p[0].trim().parse::<u16>().unwrap_or(0);
            let type_id = p[1].trim().parse::<u16>().unwrap_or(0);
            
            let mut groups = Vec::new();
            // Data starts at index 2, blocks of 14
            let mut idx = 2;
            while idx + 13 < p.len() {
                let ability_id = p[idx].trim().parse::<u8>().unwrap_or(0);
                if ability_id == 0 { break; }

                let group = TalentGroupRaw {
                    ability_id,
                    max_level: p[idx+1].trim().parse().unwrap_or(0),
                    min_1: p[idx+2].trim().parse().unwrap_or(0), max_1: p[idx+3].trim().parse().unwrap_or(0),
                    min_2: p[idx+4].trim().parse().unwrap_or(0), max_2: p[idx+5].trim().parse().unwrap_or(0),
                    min_3: p[idx+6].trim().parse().unwrap_or(0), max_3: p[idx+7].trim().parse().unwrap_or(0),
                    min_4: p[idx+8].trim().parse().unwrap_or(0), max_4: p[idx+9].trim().parse().unwrap_or(0),
                    text_id: p[idx+10].trim().parse().unwrap_or(0),
                    cost_id: p[idx+11].trim().parse().unwrap_or(0),
                    name_id: p[idx+12].trim().parse().unwrap_or(-1),
                    limit: p[idx+13].trim().parse().unwrap_or(0),
                };
                groups.push(group);
                idx += 14;
            }
            
            map.insert(id, TalentRaw { id, type_id, groups });
        }
    }
    map
}

pub fn map_ability_to_icon(ability_id: u8) -> Option<usize> {
    match ability_id {
        // Stats
        1 => Some(img015::ICON_WEAKEN),
        2 => Some(img015::ICON_FREEZE),
        3 => Some(img015::ICON_SLOW),
        4 => Some(img015::ICON_ATTACK_ONLY),
        5 => Some(img015::ICON_STRONG_AGAINST),
        6 => Some(img015::ICON_RESIST),
        7 => Some(img015::ICON_MASSIVE_DAMAGE),
        8 => Some(img015::ICON_KNOCKBACK),
        9 => Some(img015::ICON_WARP),
        10 => Some(img015::ICON_STRENGTHEN),
        11 => Some(img015::ICON_SURVIVE),
        12 => Some(img015::ICON_BASE_DESTROYER),
        13 => Some(img015::ICON_CRITICAL_HIT),
        14 => Some(img015::ICON_ZOMBIE_KILLER),
        15 => Some(img015::ICON_BARRIER_BREAKER),
        16 => Some(img015::ICON_DOUBLE_BOUNTY),
        17 => Some(img015::ICON_WAVE),
        18 => Some(img015::ICON_RESIST_WEAKEN),
        19 => Some(img015::ICON_RESIST_FREEZE),
        20 => Some(img015::ICON_RESIST_SLOW),
        21 => Some(img015::ICON_RESIST_KNOCKBACK),
        22 => Some(img015::ICON_RESIST_WAVE),
        23 => Some(img015::ICON_WAVE_BLOCK), 
        24 => Some(img015::ICON_RESIST_WARP),
        25 => Some(img015::ICON_COST_DOWN),
        26 => Some(img015::ICON_RECOVER_SPEED_UP),
        27 => Some(img015::ICON_MOVE_SPEED),
        28 => Some(img015::ICON_IMPROVE_KNOCKBACK_COUNT),
        29 => Some(img015::ICON_IMMUNE_CURSE),
        30 => Some(img015::ICON_RESIST_CURSE),
        31 => Some(img015::ICON_ATTACK_BUFF),
        32 => Some(img015::ICON_HEALTH_BUFF),
        33 => Some(img015::ICON_TRAIT_RED),
        34 => Some(img015::ICON_TRAIT_FLOATING),
        35 => Some(img015::ICON_TRAIT_BLACK),
        36 => Some(img015::ICON_TRAIT_METAL),
        37 => Some(img015::ICON_TRAIT_ANGEL),
        38 => Some(img015::ICON_TRAIT_ALIEN),
        39 => Some(img015::ICON_TRAIT_ZOMBIE),
        40 => Some(img015::ICON_TRAIT_RELIC),
        41 => Some(img015::ICON_TRAIT_TRAITLESS),
        43 => Some(img015::ICON_METAL),
        44 => Some(img015::ICON_IMMUNE_WEAKEN),
        45 => Some(img015::ICON_IMMUNE_FREEZE),
        46 => Some(img015::ICON_IMMUNE_SLOW),
        47 => Some(img015::ICON_IMMUNE_KNOCKBACK),
        48 => Some(img015::ICON_IMMUNE_WAVE),
        49 => Some(img015::ICON_IMMUNE_WARP),
        50 => Some(img015::ICON_SAVAGE_BLOW),
        51 => Some(img015::ICON_DODGE),
        52 => Some(img015::ICON_RESIST_TOXIC),
        53 => Some(img015::ICON_IMMUNE_TOXIC),
        54 => Some(img015::ICON_SURGE_RESIST),
        55 => Some(img015::ICON_IMMUNE_SURGE),
        56 => Some(img015::ICON_SURGE),
        57 => Some(img015::ICON_TRAIT_AKU),
        58 => Some(img015::ICON_SHIELD_PIERCER),
        59 => Some(img015::ICON_SOULSTRIKE),
        60 => Some(img015::ICON_CURSE),
        61 => Some(img015::ICON_TBA_DOWN),
        62 => Some(img015::ICON_MINI_WAVE),
        63 => Some(img015::ICON_COLOSSUS_SLAYER),
        64 => Some(img015::ICON_BEHEMOTH_SLAYER),
        65 => Some(img015::ICON_MINI_SURGE),
        66 => Some(img015::ICON_SAGE_SLAYER),
        67 => Some(img015::ICON_EXPLOSION),
        _ => None
    }
}

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