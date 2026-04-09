#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Debug, Clone)]
pub struct GatyaItemName {
    pub name: String,
    pub description: Vec<String>,
}

pub fn load(dir_path: &Path, filename: &str, lang_priority: &[String]) -> HashMap<usize, GatyaItemName> {
    let mut item_name_map = HashMap::new();
    let file_paths = resolver::get(dir_path, &[filename], lang_priority);
    
    let Some(first_path) = file_paths.first() else { 
        return item_name_map; 
    };
    
    let Ok(file_content) = fs::read_to_string(first_path) else { 
        return item_name_map; 
    };

    let csv_separator = detect_csv_separator(&file_content);

    for (current_row_index, line_string) in file_content.lines().enumerate() {
        let clean_line = line_string.split("//").next().unwrap_or("").trim();
        if clean_line.is_empty() { 
            continue; 
        }

        let line_parts: Vec<&str> = clean_line.split(csv_separator).collect();
        if line_parts.is_empty() {
            continue;
        }

        let item_name_string = line_parts[0].trim().to_string();
        
        let description_lines_array: Vec<String> = line_parts.iter()
            .skip(1)
            .map(|description_part| description_part.trim().to_string())
            .filter(|description_part| !description_part.is_empty() && description_part != "＠")
            .collect();

        let item_name_data = GatyaItemName {
            name: item_name_string,
            description: description_lines_array,
        };

        item_name_map.insert(current_row_index, item_name_data);
    }

    item_name_map
}