use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DropItem {
    pub map_id: u32,
    pub raw_data: Vec<String>, 
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, DropItem> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);
    
    let Some(path) = paths.first() else { return map; };
    let Ok(content) = fs::read_to_string(path) else { return map; };
    let sep = detect_csv_separator(&content);

    for line in content.lines().skip(1) {
        let clean = line.split("//").next().unwrap_or("").trim();
        if clean.is_empty() { continue; }
        
        let parts: Vec<&str> = clean.split(sep).collect();
        if parts.is_empty() { continue; }

        let Ok(map_id) = parts[0].trim().parse::<u32>() else { continue; };

        let raw_data: Vec<String> = parts.iter()
            .map(|s| s.trim().to_string())
            .collect();

        map.insert(map_id, DropItem { map_id, raw_data });
    }
    
    map
}