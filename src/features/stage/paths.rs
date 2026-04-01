#![allow(dead_code)]
use std::path::{Path, PathBuf};

pub const DIR_STAGES: &str = "game/stages";

pub fn normalize_prefix(prefix: &str) -> String {
    let upper = prefix.to_uppercase();
    if upper == "SPACE" { return "Space".to_string(); }
    if upper.starts_with('R') && upper.len() > 1 { return upper[1..].to_string(); }
    upper
}

pub fn map_folder(root: &Path, prefix: &str, map_id: u32) -> PathBuf {
    root.join(normalize_prefix(prefix)).join(format!("{:03}", map_id))
}

pub fn stage_folder(root: &Path, prefix: &str, map_id: u32, stage_id: u32) -> PathBuf {
    map_folder(root, prefix, map_id).join(format!("{:02}", stage_id))
}

pub fn stage_name_image(root: &Path, prefix: &str, map_id: u32, stage_id: u32, lang: &str) -> PathBuf {
    let norm = normalize_prefix(prefix);
    let lang_suffix = if lang.is_empty() { String::new() } else { format!("_{}", lang) };

    // The Main Chapter Reuse Rule
    if matches!(norm.as_str(), "EC" | "W" | "Space" | "PT") || (norm == "M" && stage_id == 50 && map_id == 0) {
        let file_prefix = match norm.as_str() {
            "W" => "wc",
            "Space" => "sc",
            _ => "ec",
        };

        // Handle the offset for the Moon image sharing
        let mut img_id = stage_id;
        if norm == "M" && stage_id == 50 { img_id = 48; } // Old Challenge Battle Offset
        if norm == "PT" { img_id = stage_id - 2; }

        // Force the path to look in Map 000
        return root.join(&norm)
                   .join("000")
                   .join(format!("{:02}", stage_id))
                   .join(format!("{}{:03}_n{}.png", file_prefix, img_id, lang_suffix));
    }

    // Modern Maps
    // They have unique images inside their specific map folders
    root.join(&norm)
        .join(format!("{:03}", map_id))
        .join(format!("{:02}", stage_id))
        .join(format!("mapsn{:03}_{:02}_{}{}.png", map_id, stage_id, norm.to_lowercase(), lang_suffix))
}