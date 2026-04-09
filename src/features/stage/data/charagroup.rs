use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CharaGroupType {
    OnlyUse,
    CannotUse,
    Unknown(u32),
}

impl From<u32> for CharaGroupType {
    fn from(val: u32) -> Self {
        match val {
            0 => Self::OnlyUse,
            2 => Self::CannotUse,
            _ => Self::Unknown(val),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharaGroup {
    pub group_id: u32,
    pub group_type: CharaGroupType,
    pub units: Vec<u32>,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, CharaGroup> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);
    
    let Some(path) = paths.first() else { return map; };
    let Ok(content) = fs::read_to_string(path) else { return map; };
    let sep = detect_csv_separator(&content);

    for line in content.lines().skip(1) {
        let clean = line.split("//").next().unwrap_or("").trim();
        if clean.is_empty() { continue; }
        
        let parts: Vec<&str> = clean.split(sep).collect();
        if parts.len() < 3 { continue; }

        let Ok(group_id) = parts[0].trim().parse::<u32>() else { continue; };
        let group_type_val = parts[2].trim().parse::<u32>().unwrap_or(0);
        
        let units: Vec<u32> = parts.iter()
            .skip(3)
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .collect();

        map.insert(group_id, CharaGroup {
            group_id,
            group_type: CharaGroupType::from(group_type_val),
            units,
        });
    }
    
    map
}