#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;
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

            let id = match p[0].trim().parse::<u16>() {
                Ok(val) => val,
                Err(_) => continue, 
            };

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