use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::features::cat::patterns as cat_patterns; 
use crate::global::io::patterns as global_patterns;
use super::{cat, global, enemy};

pub fn count_lines(path: &Path) -> usize {
    if let Some(ext) = path.extension() {
        if ext.to_string_lossy() == "png" { return 0; } 
    }

    let Ok(f) = fs::File::open(path) else { return 0; };
    io::BufReader::new(f).lines().count()
}

fn ensure_parent_dir(dest: &Path) {
    if let Some(parent) = dest.parent() {
        if !parent.exists() { let _ = fs::create_dir_all(parent); }
    }
}

pub fn move_if_bigger(src: &Path, dest: &Path) -> std::io::Result<bool> {
    ensure_parent_dir(dest);

    if !dest.exists() {
        fs::rename(src, dest)?;
        return Ok(true);
    }

    let src_lines = count_lines(src);
    let dest_lines = count_lines(dest);

    if src_lines > dest_lines {
        let _ = fs::remove_file(dest);
        fs::rename(src, dest)?;
        Ok(true)
    } else {
        let _ = fs::remove_file(src)?;
        Ok(false)
    }
}

// --- FIX: The Binary Byte-Size Gatekeeper ---
pub fn move_if_heavier(src: &Path, dest: &Path) -> std::io::Result<bool> {
    ensure_parent_dir(dest);

    if !dest.exists() {
        fs::rename(src, dest)?;
        return Ok(true);
    }
    
    let src_size = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    let dest_size = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    
    // Check byte size instead of line counts
    if src_size >= dest_size {
        let _ = fs::remove_file(dest);
        fs::rename(src, dest)?;
        Ok(true)
    } else {
        let _ = fs::remove_file(src)?;
        Ok(false)
    }
}

fn is_cat_base_banner(name: &str, clean_name: &str) -> bool {
    // Only target udi files for the 10 basic cats
    if !name.starts_with("udi") || name.len() < 6 { return false; }
    
    let Ok(id) = name[3..6].parse::<u32>() else { return false; };
    if id > 9 { return false; }

    // If the physical name has a region code (udi001_s_tw.png != udi001_s.png)
    // then it's a Cat Base Upgrade asset
    name != clean_name
}

fn clean_base_name(stem: &str, ext: &str) -> String {
    for &(code, _) in global_patterns::APP_LANGUAGES {
        let suffix = format!("_{}", code);
        if stem.len() > suffix.len() && stem.ends_with(&suffix) {
            let clean_stem = &stem[..stem.len() - suffix.len()];
            return if ext.is_empty() { clean_stem.to_string() } else { format!("{}.{}", clean_stem, ext) };
        }
    }
    if ext.is_empty() { stem.to_string() } else { format!("{}.{}", stem, ext) }
}

pub fn sort_game_files(tx: Sender<String>) -> Result<(), String> {
    let raw_dir = Path::new("game/raw");
    let cats_dir = Path::new("game/cats");
    let assets_dir = Path::new("game/assets");
    let enemy_dir = Path::new("game/enemies");

    if !raw_dir.exists() { return Err("Raw directory not found.".to_string()); }

    let cat_matcher = cat::CatMatcher::new();
    let global_matcher = global::GlobalMatcher::new();
    let enemy_matcher = enemy::EnemyMatcher::new();

    let mut valid_tasks: Vec<(PathBuf, String, String, PathBuf)> = Vec::new();

    for entry in fs::read_dir(raw_dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.is_dir() { continue; }

        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(|n| n.to_string()) else { continue; };
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();

        let base_name = clean_base_name(&stem, &ext);

        let dest_folder = if cat_patterns::CAT_UNIVERSAL_FILES.contains(&base_name.as_str()) {
            Some(cats_dir.to_path_buf())
        } else if is_cat_base_banner(&name, &base_name) {
            Some(cats_dir.join("CatBase"))
        } else {
            global_matcher.get_dest(&base_name, assets_dir)
                .or_else(|| cat_matcher.get_dest(&base_name, cats_dir))
                .or_else(|| enemy_matcher.get_dest(&base_name, enemy_dir))
        };

        if let Some(folder) = dest_folder {
            valid_tasks.push((path, name, base_name, folder));
        }
    }

    let files_to_sort = valid_tasks.len();
    if files_to_sort == 0 {
        let _ = tx.send("No new files to sort.".to_string());
        return Ok(());
    }

    let update_interval = (files_to_sort / 100).max(10);
    let _ = tx.send(format!("Sorting {} recognized game files...", files_to_sort));

    let mut count = 0;
    
    for (path, name, base_name, folder) in valid_tasks {
        if !folder.exists() { let _ = fs::create_dir_all(&folder); }
        let dest = folder.join(&name);
        
        let is_line_file = global_patterns::CHECK_LINE_FILES.contains(&base_name.as_str());
        
        let processed = if is_line_file {
            move_if_bigger(&path, &dest).unwrap_or(false)
        } else {
            move_if_heavier(&path, &dest).unwrap_or(false)
        };

        if processed {
            count += 1;
            if count % update_interval == 0 {
                let _ = tx.send(format!("Sorted {} files | Current: {}", count, name));
            }
        }
    }

    let _ = tx.send("Success! Files sorted.".to_string());
    Ok(())
}