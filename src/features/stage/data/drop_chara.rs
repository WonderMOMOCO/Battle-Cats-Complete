use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

pub fn load(dir_path: &Path, filename: &str, lang_priority: &[String]) -> HashMap<u32, u32> {
    let mut drop_chara_map = HashMap::new();
    let file_paths = resolver::get(dir_path, &[filename], lang_priority);
    
    let Some(first_path) = file_paths.first() else { 
        return drop_chara_map; 
    };
    
    let Ok(file_content) = fs::read_to_string(first_path) else { 
        return drop_chara_map; 
    };

    let csv_separator = detect_csv_separator(&file_content);

    for line_string in file_content.lines().skip(1) {
        let clean_line = line_string.split("//").next().unwrap_or("").trim();
        if clean_line.is_empty() { 
            continue; 
        }

        let line_parts: Vec<&str> = clean_line.split(csv_separator).collect();
        if line_parts.len() < 3 { 
            continue; 
        }

        let Ok(stage_drop_chara_id) = line_parts[0].trim().parse::<i32>() else {
            continue;
        };

        if stage_drop_chara_id < 0 {
            continue;
        }

        let Ok(resolved_chara_id) = line_parts[2].trim().parse::<u32>() else {
            continue;
        };

        drop_chara_map.insert(stage_drop_chara_id as u32, resolved_chara_id);
    }

    drop_chara_map
}