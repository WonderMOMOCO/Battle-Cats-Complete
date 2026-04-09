use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, String> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);
    
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

pub struct CategoryMeta {
    pub name: Option<&'static str>,
    pub base_id: Option<u32>,
    pub sort_order: u32,
}


pub fn get_meta(prefix: &str) -> CategoryMeta {
    let (name, base_id, sort_order) = match prefix.to_uppercase().as_str() {
        // Category Prefix => Display Name, Base ID, and UI Sort Order 
        "N"        => (Some("Stories of Legend"),        Some(0),   0),
        "RS" | "S" => (Some("Regular Event Stages"),     Some(1),   1),
        "C"        => (Some("Collab Stages"),            Some(2),   2),
        "EC"       => (Some("Empire of Cats"),           None,      3),
        "W"        => (Some("Into the Future"),          None,      3),
        "SPACE"    => (Some("Cats of the Cosmos"),       None,      3),
        "RE"       => (Some("Event Stages"),             Some(4),   4),
        "EX"       => (Some("Continuation Stages"),      Some(4),   5),
        "RT" | "T" => (Some("Dojo Hall of Initiates"),   Some(6),   6),
        "RV" | "V" => (Some("Towers & Citadels"),        Some(7),   7),
        "RR" | "R" => (Some("Dojo Ranking Events"),      Some(11),  11),
        "M"        => (Some("Challenge Battle"),         Some(12),  12),
        "NA"       => (Some("Uncanny Legends"),          Some(13),  13),
        "B"        => (Some("Catamin Stages"),           Some(14),  14),
        "D"        => (Some("Legend Quest"),             Some(16),  16),
        "Z"        => (Some("Zombie Outbreaks"),         None,      20),
        "A"        => (Some("Gauntlet Stages"),          Some(24),  24),
        "H"        => (Some("Enigma Stages"),            Some(25),  25),
        "CA"       => (Some("Collab Gauntlet Stages"),   Some(27),  27),
        "DM" | "U" => (Some("Aku Realms"),               Some(30),  30),
        "Q"        => (Some("Behemoth Culling"),         Some(31),  31),
        "L"        => (Some("Labyrinth"),                Some(33),  33),
        "ND"       => (Some("Zero Legends"),             Some(34),  34),
        "SR"       => (Some("Otherworld Colosseum"),     Some(36),  36),
        "G"        => (Some("Catclaw Championships"),    Some(37),  37),
        "PT"       => (Some("Legacy Princess Punt"),     None,      98),
        _          => (None,                             None,      99), // Fallback
    };

    CategoryMeta { name, base_id, sort_order }
}


/// The Hardcoded PONOS Category Dictionary
pub fn get_category_name(prefix: &str) -> String {
    get_meta(prefix).name.unwrap_or(prefix).to_string()
}

/// Translates a Folder Prefix + Local Map ID into PONOS' Global Map ID
pub fn get_global_map_id(prefix: &str, local_map_id: u32) -> Option<u32> {
    get_meta(prefix).base_id.map(|base| (base * 1000) + local_map_id)
}

/// Returns the internal PONOS Category ID for UI sorting
pub fn get_category_sort_order(prefix: &str) -> u32 {
    get_meta(prefix).sort_order
}