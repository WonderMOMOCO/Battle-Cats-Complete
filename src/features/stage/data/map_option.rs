use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResetType {
    None,
    ResetRewards,
    ResetRewardsAndClear,
    ResetMaxClears,
    Unknown(u8),
}

impl From<u8> for ResetType {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::None,
            1 => Self::ResetRewards,
            2 => Self::ResetRewardsAndClear,
            3 => Self::ResetMaxClears,
            _ => Self::Unknown(val),
        }
    }
}

impl Default for ResetType {
    fn default() -> Self { Self::None }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MapOption {
    pub map_id: u32,
    pub max_crowns: u8,
    pub crown_2_mag: Option<u32>,
    pub crown_3_mag: Option<u32>,
    pub crown_4_mag: Option<u32>,
    pub reset_type: ResetType,
    pub max_clears: u32,
    pub cooldown_minutes: u32,
    pub hidden_upon_clear: bool,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, MapOption> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);
    
    let Some(path) = paths.first() else { return map; };
    let Ok(content) = fs::read_to_string(path) else { return map; };
    let sep = detect_csv_separator(&content);

    for line in content.lines().skip(1) {
        let clean = line.split("//").next().unwrap_or("").trim();
        if clean.is_empty() { continue; }
        
        let parts: Vec<&str> = clean.split(sep).collect();
        if parts.len() < 17 { continue; }

        let Ok(map_id) = parts[0].trim().parse::<u32>() else { continue; };
        
        // Pre-15.1 offsets safety logic
        let offset = if parts[2].trim().is_empty() || parts[2].trim().parse::<u32>().is_err() { 1 } else { 0 };

        let max_crowns = parts[1].trim().parse::<u8>().unwrap_or(1);
        let crown_2_mag = (max_crowns >= 2).then(|| parts.get(3 + offset).and_then(|s| s.trim().parse().ok())).flatten();
        let crown_3_mag = (max_crowns >= 3).then(|| parts.get(4 + offset).and_then(|s| s.trim().parse().ok())).flatten();
        let crown_4_mag = (max_crowns >= 4).then(|| parts.get(5 + offset).and_then(|s| s.trim().parse().ok())).flatten();

        let reset_type = parts.get(7 + offset).and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0);
        let max_clears = parts.get(8 + offset).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        let cooldown = parts.get(10 + offset).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        let hidden_upon_clear = parts.get(13 + offset).and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0) == 1;

        map.insert(map_id, MapOption {
            map_id,
            max_crowns,
            crown_2_mag,
            crown_3_mag,
            crown_4_mag,
            reset_type: ResetType::from(reset_type),
            max_clears,
            cooldown_minutes: cooldown,
            hidden_upon_clear,
        });
    }
    
    map
}