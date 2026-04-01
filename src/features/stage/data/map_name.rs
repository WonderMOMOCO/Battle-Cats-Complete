use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, String> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, filename, priority);
    
    // Reverse iterate so higher priority languages overwrite lower ones safely
    for path in paths.iter().rev() {
        let Ok(content) = fs::read_to_string(path) else { continue; };
        let sep = detect_csv_separator(&content);
        
        for line in content.lines() {
            let clean = line.split("//").next().unwrap_or("").trim();
            if clean.is_empty() { continue; }
            
            let parts: Vec<&str> = clean.split(sep).collect();
            if parts.len() < 2 { continue; }
            
            let Ok(id) = parts[0].trim().parse::<u32>() else { continue; };
            let name = parts[1].trim();
            
            // Overwrite ONLY if the new name is not empty
            if !name.is_empty() {
                map.insert(id, name.to_string());
            }
        }
    }
    
    map
}

/// The Hardcoded PONOS Category Dictionary
pub fn get_category_name(prefix: &str) -> String {
    let name = match prefix.to_uppercase().as_str() {
        "A" => "Gauntlet Stages",
        "B" => "Catamin Stages",
        "C" => "Collab Stages",
        "CA" => "Collab Gauntlet Stages",
        "D" => "Legend Quest",
        "G" => "Catclaw Championships",
        "H" => "Enigma Stages",
        "L" => "Labyrinth",
        "M" => "Challenge Battle",
        "N" => "Stories of Legend",
        "NA" => "Uncanny Legends",
        "ND" => "Zero Legends",
        "Q" => "Behemoth Culling",
        "RR" | "R" => "Dojo Ranking Events",
        "RE" => "Event Stages",
        "RS" | "S" => "Regular Event Stages",
        "SR" => "Otherworld Colosseum",
        "RT" | "T" => "Dojo Hall of Initiates",
        "RV" | "V" => "Towers & Citadels",
        "EC" => "Empire of Cats",
        "W" => "Into the Future",
        "SPACE" => "Cats of the Cosmos",
        "PT" => "Princess Punt",
        "Z" => "Zombie Outbreaks",
        "EX" => "Continuation Stages",
        "DM" | "U" => "Aku Realms",
        _ => return prefix.to_string(), // Fall back to the raw folder name
    };
    name.to_string()
}

/// Translates a Folder Prefix + Local Map ID into PONOS's Global Map ID
pub fn get_global_map_id(prefix: &str, local_map_id: u32) -> Option<u32> {
    let category_id = match prefix.to_uppercase().as_str() {
        "N" => 0,
        "RS" | "S" => 1,
        "C" => 2,
        "RE" => 4,
        "RT" | "T" => 6,
        "RV" | "V" => 7,
        "RR" | "R" => 11,
        "M" => 12,
        "NA" => 13,
        "B" => 14,
        "D" => 16,
        "A" => 24,
        "H" => 25,
        "CA" => 27,
        "Q" => 31,
        "L" => 33,
        "ND" => 34,
        "SR" => 36,
        "G" => 37,
        _ => return None, 
    };
    
    Some((category_id * 1000) + local_map_id)
}