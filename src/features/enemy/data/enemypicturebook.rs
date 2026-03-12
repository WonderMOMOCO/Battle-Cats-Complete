use std::fs;
use std::path::Path;
use crate::global::utils;

pub fn load(lang_dir: &Path, target_lang: &str) -> Vec<Vec<String>> {
    let mut descriptions: Vec<Vec<String>> = Vec::new();
    
    // 1. Build our priority queue
    let mut try_langs = vec![target_lang.to_string()];
    for &l in utils::LANGUAGE_PRIORITY {
        if l != target_lang {
            try_langs.push(l.to_string());
        }
    }

    // 2. Iterate through the languages
    for lang in try_langs {
        let file_name = if lang.is_empty() {
            "EnemyPictureBook.csv".to_string()
        } else {
            format!("EnemyPictureBook_{}.csv", lang)
        };
        
        let target_path = lang_dir.join("EnemyPictureBook").join(&file_name);
        
        if let Ok(content) = fs::read_to_string(&target_path) {
            let sep = if content.contains('\t') { '\t' } else { utils::detect_csv_separator(&content) };

            for (i, line) in content.lines().enumerate() {
                let cols: Vec<&str> = line.split(sep).collect();
                let mut desc_lines = Vec::new();
                
                for col in cols.into_iter().skip(1) {
                    let text = col.trim();
                    // Skip empty columns or placeholder text
                    if text.is_empty() || text.starts_with("仮") {
                        continue;
                    }
                    desc_lines.push(text.to_string());
                }
                
                // If this is a new ID we haven't reached yet, push it
                if i >= descriptions.len() {
                    descriptions.push(desc_lines);
                } 
                // If we already have this ID but it was empty, and this language has it, overwrite
                else if descriptions[i].is_empty() && !desc_lines.is_empty() {
                    descriptions[i] = desc_lines;
                }
            }
        }
    }
    
    descriptions
}