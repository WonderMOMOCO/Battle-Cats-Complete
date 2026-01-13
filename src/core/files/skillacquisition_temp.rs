#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::core::files::img015;

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
    let file_path = cats_directory.join("SkillAcquisition.csv");
    if let Ok(content) = fs::read_to_string(&file_path) {
        for line in content.lines() {
            if let Some(data) = parse_line(line) {
                map.insert(data.id, data);
            }
        }
    }
    map
}

fn parse_line(line: &str) -> Option<TalentRaw> {
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 2 { return None; }
    let id = parts[0].trim().parse::<u16>().ok()?;
    let type_id = parts[1].trim().parse::<u16>().unwrap_or(0);
    let mut groups = Vec::new();
    let group_size = 14;
    for i in 0..8 {
        let start = 2 + (i * group_size);
        if start + group_size > parts.len() { break; }
        let p = &parts[start..start+group_size];
        let get = |idx: usize| -> u16 { p[idx].trim().parse::<u16>().unwrap_or(0) };
        let ability_id: u8 = get(0) as u8;
        let name_id: i16 = p[12].trim().parse().unwrap_or(-1);
        if ability_id == 0 && name_id == -1 { continue; } 
        groups.push(TalentGroupRaw {
            ability_id, max_level: get(1) as u8,
            min_1: get(2), max_1: get(3), min_2: get(4), max_2: get(5),
            min_3: get(6), max_3: get(7), min_4: get(8), max_4: get(9),
            text_id: get(10) as u8, cost_id: get(11) as u8, name_id, limit: get(13) as u8,
        });
    }
    Some(TalentRaw { id, type_id, groups })
}

pub fn map_ability_to_icon(ability_id: u8) -> Option<usize> {
    match ability_id {
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
        24 => Some(img015::ICON_RESIST_WARP), 
        23 => Some(img015::ICON_IMMUNE_WAVE),
        29 => Some(img015::ICON_IMMUNE_CURSE),
        30 => Some(img015::ICON_RESIST_CURSE),
        44 => Some(img015::ICON_IMMUNE_WEAKEN),
        45 => Some(img015::ICON_IMMUNE_FREEZE),
        46 => Some(img015::ICON_IMMUNE_SLOW),
        47 => Some(img015::ICON_IMMUNE_KNOCKBACK),
        48 => Some(img015::ICON_IMMUNE_WAVE),
        49 => Some(img015::ICON_IMMUNE_WARP),
        53 => Some(img015::ICON_IMMUNE_TOXIC),
        55 => Some(img015::ICON_IMMUNE_SURGE),
        25 => Some(img015::ICON_COST_DOWN),
        26 => Some(img015::ICON_RECOVER_SPEED_UP),
        27 => Some(img015::ICON_MOVE_SPEED),
        28 => Some(img015::ICON_IMPROVE_KNOCKBACK_COUNT),
        31 => Some(img015::ICON_ATTACK_BUFF),
        32 => Some(img015::ICON_HEALTH_BUFF),
        61 => Some(img015::ICON_TBA_DOWN),
        50 => Some(img015::ICON_SAVAGE_BLOW),
        51 => Some(img015::ICON_DODGE),
        52 => Some(img015::ICON_RESIST_TOXIC),
        54 => Some(img015::ICON_SURGE_RESIST),
        56 => Some(img015::ICON_SURGE),
        58 => Some(img015::ICON_SHIELD_PIERCER),
        59 => Some(img015::ICON_SOULSTRIKE),
        60 => Some(img015::ICON_CURSE),
        62 => Some(img015::ICON_MINI_WAVE),
        63 => Some(img015::ICON_COLOSSUS_SLAYER),
        64 => Some(img015::ICON_BEHEMOTH_SLAYER),
        65 => Some(img015::ICON_MINI_SURGE),
        66 => Some(img015::ICON_SAGE_SLAYER),
        67 => Some(img015::ICON_EXPLOSION),
        33 => Some(img015::ICON_TRAIT_RED),
        34 => Some(img015::ICON_TRAIT_FLOATING),
        35 => Some(img015::ICON_TRAIT_BLACK),
        36 => Some(img015::ICON_TRAIT_METAL),
        37 => Some(img015::ICON_TRAIT_ANGEL),
        38 => Some(img015::ICON_TRAIT_ALIEN),
        39 => Some(img015::ICON_TRAIT_ZOMBIE),
        40 => Some(img015::ICON_TRAIT_RELIC),
        41 => Some(img015::ICON_TRAIT_TRAITLESS),
        57 => Some(img015::ICON_TRAIT_AKU),
        _ => None, 
    }
}