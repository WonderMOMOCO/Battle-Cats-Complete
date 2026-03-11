use std::fs;
use std::path::Path;
use crate::core::utils;

pub fn load(lang_dir: &Path, target_lang: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    
    // 1. Build our priority queue: Target language first, then the global fallbacks
    let mut try_langs = vec![target_lang.to_string()];
    for &l in utils::LANGUAGE_PRIORITY {
        if l != target_lang {
            try_langs.push(l.to_string());
        }
    }

    // 2. Iterate through the languages
    for lang in try_langs {
        let file_name = if lang.is_empty() {
            "Enemyname.tsv".to_string()
        } else {
            format!("Enemyname_{}.tsv", lang)
        };
        
        let target_path = lang_dir.join("Enemyname").join(&file_name);
        
        if let Ok(content) = fs::read_to_string(&target_path) {
            let sep = if content.contains('\t') { '\t' } else { utils::detect_csv_separator(&content) };

            for (i, line) in content.lines().enumerate() {
                let name = line.split(sep).next().unwrap_or("").trim().to_string();
                
                // Treat "ダミー" as invalid/empty
                let is_invalid = name.is_empty() || name == "ダミー";

                // If this is a new ID we haven't reached yet, push the name or an empty string if invalid
                if i >= names.len() {
                    names.push(if is_invalid { String::new() } else { name });
                } 
                // If we already have this ID but it was empty/invalid, and this language has a valid name, overwrite
                else if names[i].is_empty() && !is_invalid {
                    names[i] = name;
                }
            }
        }
    }
    
    names
}