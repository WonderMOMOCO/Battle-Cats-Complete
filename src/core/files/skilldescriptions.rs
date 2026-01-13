use std::fs;
use std::path::Path;

pub fn load(cats_directory: &Path, language_code: &str) -> Vec<String> {
    let code = if language_code.is_empty() { "en" } else { language_code };
    
    let filename = format!("SkillDescriptions_{}.csv", code);
    let file_path = cats_directory
        .join("SkillDescriptions")
        .join(&filename);

    // Japan uses ',' separator, others use '|'
    let separator = if code == "ja" { ',' } else { '|' };

    let mut descriptions = Vec::new();

    if let Ok(content) = fs::read_to_string(&file_path) {
        for line in content.lines() {
            if line.trim().is_empty() {
                descriptions.push(String::new());
                continue;
            }

            // Prune everything before the first separator
            let raw_text = if let Some((_id, text_part)) = line.split_once(separator) {
                text_part
            } else {
                line // Fallback if no separator found
            };

            let clean_line = raw_text.replace("<br>", "\n").trim().to_string();
            descriptions.push(clean_line);
        }
    }

    descriptions
}