use std::fs;
use std::path::Path;
use crate::core::utils;
use crate::paths::cat;

pub fn load(cats_directory: &Path, language_code: &str) -> Vec<String> {
    let mut codes_to_try = Vec::new();
    
    if !language_code.is_empty() {
        codes_to_try.push(language_code);
    }
    
    for &code in utils::LANGUAGE_PRIORITY {
        if code != language_code {
            codes_to_try.push(code);
        }
    }

    let base_dir = cats_directory.join(cat::DIR_SKILL_DESCRIPTIONS);

    for code in codes_to_try {
        let filename = if code.is_empty() {
            "SkillDescriptions.csv".to_string()
        } else {
            format!("SkillDescriptions_{}.csv", code)
        };

        let file_path = base_dir.join(&filename);

        if let Ok(content) = fs::read_to_string(&file_path) {
            let mut descriptions = Vec::new();
            let separator = utils::detect_csv_separator(&content);

            for line in content.lines() {
                if line.trim().is_empty() {
                    descriptions.push(String::new());
                    continue;
                }

                let raw_text = if let Some((_id, text_part)) = line.split_once(separator) {
                    text_part
                } else {
                    line 
                };

                let clean_line = raw_text.replace("<br>", "\n").trim().to_string();
                descriptions.push(clean_line);
            }

            if !descriptions.is_empty() {
                return descriptions;
            }
        }
    }

    Vec::new()
}