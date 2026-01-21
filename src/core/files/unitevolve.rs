use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::core::utils; 

pub fn load(cats_directory: &Path, language_code: &str) -> HashMap<u32, [Vec<String>; 4]> {
    let mut final_map: HashMap<u32, [Vec<String>; 4]> = HashMap::new();
    
    let priorities: Vec<&str> = if language_code.is_empty() {
        utils::LANGUAGE_PRIORITY.to_vec()
    } else {
        let mut p = vec![language_code];
        for &code in utils::LANGUAGE_PRIORITY {
            if code != language_code {
                p.push(code);
            }
        }
        p
    };

    let base_dir = cats_directory.join("unitevolve");

    for code in priorities {
        let fname = format!("unitevolve_{}.csv", code);
        let path = base_dir.join(&fname);

        let content = if path.exists() {
            fs::read_to_string(path).ok()
        } else {
            None
        };

        let content = match content {
            Some(c) => c,
            None => continue,
        };

        let delimiter = utils::detect_csv_separator(&content);

        for (line_idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() { continue; }
            
            let cat_id = line_idx as u32;
            let parts: Vec<&str> = line.split(delimiter).collect();

            let get_text = |idx: usize| -> String {
                let raw = parts.get(idx).map(|s| s.trim()).unwrap_or("");
                // Strip common empty placeholder
                if raw == "@" || raw == "＠" || raw.is_empty() {
                    String::new() 
                } else {
                    raw.replace("<br>", "\n").to_string()
                }
            };

            // 0-2 True Form
            // 4-6: Ultra Form
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