#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::utils;
use crate::features::cat::paths;

#[derive(Debug, Clone)]
pub struct TalentCost {
    pub costs: Vec<u16>,
}

pub fn load(cats_directory: &Path, priority: &[String]) -> HashMap<u8, TalentCost> {
    let mut map = HashMap::new();
    let Some(file_path) = crate::global::resolver::get(cats_directory, &[paths::SKILL_LEVEL], priority).into_iter().next() else {
        return map;
    };
    
    if let Ok(content) = fs::read_to_string(&file_path) {
        let delimiter = utils::detect_csv_separator(&content);
        for line in content.lines() {
            let parts: Vec<&str> = line.split(delimiter).collect();
            if parts.is_empty() { continue; }
            if let Ok(id) = parts[0].trim().parse::<u8>() {
                let costs: Vec<u16> = parts.iter()
                    .skip(1)
                    .filter_map(|s| s.trim().parse::<u16>().ok())
                    .collect();
                map.insert(id, TalentCost { costs });
            }
        }
    } 
    map
}