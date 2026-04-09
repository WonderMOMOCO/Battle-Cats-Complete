use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, Vec<String>> {
    let mut map: HashMap<u32, Vec<String>> = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);
    
    for path in paths.iter().rev() {
        let Ok(content) = fs::read_to_string(path) else { continue; };
        let sep = detect_csv_separator(&content);
        
        for (map_id, line) in content.lines().enumerate() {
            let clean_line = line.split("//").next().unwrap_or("").trim();
            if clean_line.is_empty() { continue; }
            
            let parts: Vec<String> = clean_line.split(sep)
                .map(|s| s.trim().to_string())
                .collect();
            
            let entry = map.entry(map_id as u32).or_insert_with(Vec::new);
            
            if entry.len() < parts.len() {
                entry.resize(parts.len(), String::new());
            }
            
            for (i, part) in parts.into_iter().enumerate() {
                if !part.is_empty() {
                    entry[i] = part;
                }
            }
        }
    }
    
    map
}