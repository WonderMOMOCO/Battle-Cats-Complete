use std::fs;
use std::path::Path;
use crate::global::utils;

pub fn load(lang_dir: &Path, priority: &[String]) -> Vec<Vec<String>> {
    let mut descriptions: Vec<Vec<String>> = Vec::new();
    let base_dir = lang_dir.join("EnemyPictureBook");
    
    for file_path in crate::global::get(&base_dir, &["EnemyPictureBook.csv"], priority) {
        let Ok(content) = fs::read_to_string(&file_path) else { continue };
        let sep = if content.contains('\t') { '\t' } else { utils::detect_csv_separator(&content) };

        for (i, line) in content.lines().enumerate() {
            let cols: Vec<&str> = line.split(sep).collect();
            let mut desc_lines = Vec::new();
            
            for col in cols.into_iter().skip(1) {
                let text = col.trim();
                if text.is_empty() || text.starts_with("仮") { continue; }
                desc_lines.push(text.to_string());
            }
            
            if i >= descriptions.len() {
                descriptions.push(desc_lines);
                continue;
            } 
            
            if descriptions[i].is_empty() && !desc_lines.is_empty() {
                descriptions[i] = desc_lines;
            }
        }
    }
    
    descriptions
}