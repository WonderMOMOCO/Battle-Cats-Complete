use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::utils; 
use crate::features::cat::paths;

pub fn load(cats_directory: &Path, priority: &[String]) -> HashMap<u32, [Vec<String>; 4]> {
    let mut final_map: HashMap<u32, [Vec<String>; 4]> = HashMap::new();
    let base_directory = cats_directory.join(paths::DIR_UNIT_EVOLVE);

    for file_path in crate::global::get(&base_directory, &["unitevolve.csv"], priority) {
        let Ok(file_content) = fs::read_to_string(&file_path) else { 
            continue; 
        };
        
        let delimiter = utils::detect_csv_separator(&file_content);

        for (line_index, line_content) in file_content.lines().enumerate() {
            if line_content.trim().is_empty() { 
                continue; 
            }
            
            let column_parts: Vec<&str> = line_content.split(delimiter).collect();
            let cat_id = line_index as u32;

            let get_text = |index: usize| -> String {
                let raw_string = column_parts.get(index).map(|string_part| string_part.trim()).unwrap_or("");
                
                if raw_string == "@" || raw_string == "＠" || raw_string.is_empty() { 
                    return String::new(); 
                }
                
                raw_string.replace("<br>", "\n").to_string()
            };

            let true_form_new = vec![get_text(0), get_text(1), get_text(2)];
            let ultra_form_new = vec![get_text(4), get_text(5), get_text(6)];
            
            let has_content = |data_vector: &Vec<String>| data_vector.iter().any(|string_part| !string_part.is_empty());

            let entry = final_map.entry(cat_id).or_insert([Vec::new(), Vec::new(), Vec::new(), Vec::new()]);
            
            // Populate True Form data if existing entry is empty and new data exists
            if !has_content(&entry[2]) && has_content(&true_form_new) {
                entry[2] = true_form_new;
            }

            // Populate Ultra Form data if existing entry is empty and new data exists
            if !has_content(&entry[3]) && has_content(&ultra_form_new) {
                entry[3] = ultra_form_new;
            }
        }
    }
    
    final_map
}