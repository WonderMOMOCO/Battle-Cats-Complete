use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::utils; 
use crate::features::cat::paths;

pub fn load(cats_directory: &Path, priority: &[String]) -> HashMap<u32, [Vec<String>; 4]> {
    let mut final_map: HashMap<u32, [Vec<String>; 4]> = HashMap::new();
    let base_dir = cats_directory.join(paths::DIR_UNIT_EVOLVE);

    for file_path in crate::global::get(&base_dir, "unitevolve.csv", priority) {
        let Ok(content) = fs::read_to_string(&file_path) else { continue };
        let delimiter = utils::detect_csv_separator(&content);

        for (line_idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() { continue; }
            
            let parts: Vec<&str> = line.split(delimiter).collect();
            let cat_id = line_idx as u32;

            let get_text = |idx: usize| -> String {
                let raw = parts.get(idx).map(|s| s.trim()).unwrap_or("");
                if raw == "@" || raw == "＠" || raw.is_empty() { return String::new(); }
                raw.replace("<br>", "\n").to_string()
            };

            let tf_new = vec![get_text(0), get_text(1), get_text(2)];
            let uf_new = vec![get_text(4), get_text(5), get_text(6)];
            let has_content = |v: &Vec<String>| v.iter().any(|s| !s.is_empty());

            let entry = final_map.entry(cat_id).or_insert([Vec::new(), Vec::new(), Vec::new(), Vec::new()]);
            
            if !has_content(&entry[2]) && has_content(&tf_new) {
                entry[2] = tf_new;
            }

            if !has_content(&entry[3]) && has_content(&uf_new) {
                entry[3] = uf_new;
            }
        }
    }
    
    final_map
}