use std::fs;
use std::path::Path;
use crate::core::utils;

#[derive(Clone, Debug, Default)]
pub struct UnitExplanation {
    pub names: [String; 4],
    pub descriptions: [Vec<String>; 4],
}

impl UnitExplanation {
    pub fn load(path: &Path) -> Option<Self> {
        let file_bytes = fs::read(path).ok()?;
        let file_content = String::from_utf8_lossy(&file_bytes);
        let separator_char = utils::detect_csv_separator(&file_content);

        let mut names = [const { String::new() }; 4];
        let mut descriptions = [const { Vec::new() }; 4];

        for (line_index, file_line) in file_content.lines().enumerate().take(4) {
            let parts: Vec<&str> = file_line.split(separator_char).collect();
            
            // Parse Name
            if let Some(name_part) = parts.get(0) {
                let trimmed_name = name_part.trim();
                
                let sanitized_name: String = trimmed_name.chars()
                    .filter(|c| !is_problematic_char(*c))
                    .collect();

                if !sanitized_name.is_empty() && !looks_like_garbage_id(&sanitized_name) {
                    names[line_index] = sanitized_name;
                }
            }

            // Parse Description
            let desc_lines: Vec<String> = parts.iter()
                .skip(1)
                .take(3)
                .map(|s| s.trim().to_string())
                .collect();
            
            descriptions[line_index] = desc_lines;
        }

        Some(Self { names, descriptions })
    }
}

fn is_problematic_char(c: char) -> bool {
    let u = c as u32;
    if (0xE0100..=0xE01EF).contains(&u) { return true; }
    if (0xFE00..=0xFE0F).contains(&u) { return true; }
    false
}

fn looks_like_garbage_id(text: &str) -> bool {
    text.chars().all(|char_check| char_check.is_ascii_digit() || char_check == '-' || char_check == '_')
}