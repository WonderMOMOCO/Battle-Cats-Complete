use std::fs;
use std::path::Path;
use crate::global::utils;
use crate::features::cat::paths;

pub fn load(cats_directory: &Path, priority: &[String]) -> Vec<String> {
    let base_dir = cats_directory.join(paths::DIR_SKILL_DESCRIPTIONS);

    let Some(file_path) = crate::global::get(&base_dir, "SkillDescriptions.csv", priority).into_iter().next() else {
        return Vec::new();
    };

    let Ok(content) = fs::read_to_string(&file_path) else {
        return Vec::new();
    };

    let mut descriptions = Vec::new();
    let separator = utils::detect_csv_separator(&content);

    for line in content.lines() {
        if line.trim().is_empty() {
            descriptions.push(String::new());
            continue;
        }

        let raw_text = match line.split_once(separator) {
            Some((_id, text_part)) => text_part,
            None => line,
        };

        descriptions.push(raw_text.replace("<br>", "\n").trim().to_string());
    }

    descriptions
}