use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);
    
    let Some(path) = paths.first() else { return map; };
    let Ok(content) = fs::read_to_string(path) else { return map; };
    let sep = detect_csv_separator(&content);

    for line in content.lines() {
        let clean = line.split("//").next().unwrap_or("").trim();
        if clean.is_empty() { continue; }
        
        let parts: Vec<&str> = clean.split(sep).collect();
        if parts.len() < 2 { continue; }

        let Ok(map_id) = parts[0].trim().parse::<u32>() else { continue; };
        let Ok(ex_map_id) = parts[1].trim().parse::<u32>() else { continue; };

        map.insert(map_id, ex_map_id);
    }
    
    map
}