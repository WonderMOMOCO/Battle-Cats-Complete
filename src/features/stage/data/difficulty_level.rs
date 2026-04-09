use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;

pub fn load(dir_path: &Path, filename: &str, lang_priority: &[String]) -> HashMap<u32, Vec<u16>> {
    let mut difficulty_map = HashMap::new();
    let file_paths = resolver::get(dir_path, &[filename], lang_priority);
    
    let Some(first_path) = file_paths.first() else { 
        return difficulty_map; 
    };
    
    let Ok(file_content) = fs::read_to_string(first_path) else { 
        return difficulty_map; 
    };

    for line in file_content.lines() {
        let clean_line = line.split("//").next().unwrap_or("").trim();
        if clean_line.is_empty() { 
            continue; 
        }
        
        let line_parts: Vec<&str> = clean_line.split('\t').collect();
        if line_parts.len() < 2 { 
            continue; 
        }

        let Ok(map_id) = line_parts[0].trim().parse::<u32>() else { 
            continue; 
        };

        let difficulties: Vec<u16> = line_parts.iter()
            .skip(1)
            .map(|diff_str| {
                let trimmed_str = diff_str.trim();
                let int_part = trimmed_str.split('.').next().unwrap_or("0");
                int_part.parse::<u16>().unwrap_or(0)
            })
            .collect();

        difficulty_map.insert(map_id, difficulties);
    }
    
    difficulty_map
}