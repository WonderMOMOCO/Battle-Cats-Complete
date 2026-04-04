use std::fs;
use std::path::Path;
use crate::global::utils;

pub fn load(lang_dir: &Path, priority: &[String]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let base_dir = lang_dir.join("Enemyname");

    for file_path in crate::global::get(&base_dir, &["Enemyname.tsv"], priority) {
        let Ok(content) = fs::read_to_string(&file_path) else { continue };
        let sep = if content.contains('\t') { '\t' } else { utils::detect_csv_separator(&content) };

        for (i, line) in content.lines().enumerate() {
            let name = line.split(sep).next().unwrap_or("").trim().to_string();
            let is_invalid = name.is_empty() || name == "ダミー";

            if i >= names.len() {
                names.push(if is_invalid { String::new() } else { name });
                continue;
            } 
            
            if names[i].is_empty() && !is_invalid {
                names[i] = name;
            }
        }
    }
    
    names
}