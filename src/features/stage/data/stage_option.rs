use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StageOption {
    pub map_id: u32,
    pub target_crowns: i8, // -1 means All Crowns
    pub target_stage: i32, // -1 means All Stages
    pub rarity_mask: u8,
    pub deploy_limit: u32,
    pub allowed_rows: u8,
    pub min_cost: u32,
    pub max_cost: u32,
    pub charagroup_id: u32,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, Vec<StageOption>> {
    let mut map: HashMap<u32, Vec<StageOption>> = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);
    
    let Some(path) = paths.first() else { return map; };
    let Ok(content) = fs::read_to_string(path) else { return map; };
    let sep = detect_csv_separator(&content);

    for line in content.lines().skip(1) {
        let clean = line.split("//").next().unwrap_or("").trim();
        if clean.is_empty() { continue; }
        
        let parts: Vec<&str> = clean.split(sep).collect();
        if parts.len() < 9 { continue; }

        let Ok(map_id) = parts[0].trim().parse::<u32>() else { continue; };

        let opt = StageOption {
            map_id,
            target_crowns: parts[1].trim().parse().unwrap_or(-1),
            target_stage: parts[2].trim().parse().unwrap_or(-1),
            rarity_mask: parts[3].trim().parse().unwrap_or(0),
            deploy_limit: parts[4].trim().parse().unwrap_or(0),
            allowed_rows: parts[5].trim().parse().unwrap_or(0),
            min_cost: parts[6].trim().parse().unwrap_or(0),
            max_cost: parts[7].trim().parse().unwrap_or(0),
            charagroup_id: parts[8].trim().parse().unwrap_or(0),
        };

        map.entry(map_id).or_default().push(opt);
    }
    
    map
}