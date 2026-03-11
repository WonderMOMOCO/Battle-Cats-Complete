use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::mpsc::Sender;

use crate::features::cat::patterns as cat_patterns; 
use crate::global::io::patterns as global_patterns;
use super::{cat, global, enemy};

pub fn count_lines(path: &Path) -> usize {
    if let Some(ext) = path.extension() {
        let s = ext.to_string_lossy();
        if s == "png" { return 0; } 
    }

    if let Ok(f) = fs::File::open(path) {
        let reader = io::BufReader::new(f);
        reader.lines().count()
    } else {
        0
    }
}

pub fn move_if_bigger(src: &Path, dest: &Path) -> std::io::Result<bool> {
    if dest.exists() {
        let src_lines = count_lines(src);
        let dest_lines = count_lines(dest);

        // Files with more defined parts/lines will replace the dummy files
        if src_lines > dest_lines {
            let _ = fs::remove_file(dest);
            fs::rename(src, dest)?;
            Ok(true)
        } else {
            fs::remove_file(src)?;
            Ok(false)
        }
    } else {
        if let Some(parent) = dest.parent() {
            if !parent.exists() { let _ = fs::create_dir_all(parent); }
        }
        fs::rename(src, dest)?;
        Ok(true)
    }
}

pub fn move_fast(src: &Path, dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        if !parent.exists() { let _ = fs::create_dir_all(parent); }
    }
    if dest.exists() {
        let _ = fs::remove_file(dest);
    }
    fs::rename(src, dest)?;
    Ok(())
}

pub fn sort_game_files(tx: Sender<String>) -> Result<(), String> {
    let raw_dir = Path::new("game/raw");
    let cats_dir = Path::new("game/cats");
    let assets_dir = Path::new("game/assets");
    let enemy_dir = Path::new("game/enemies");

    if !raw_dir.exists() {
        return Err("Raw directory not found.".to_string());
    }

    let _ = tx.send("Sorting files...".to_string());

    let cat_matcher = cat::CatMatcher::new();
    let global_matcher = global::GlobalMatcher::new();
    let enemy_matcher = enemy::EnemyMatcher::new();

    let mut count = 0;
    
    for entry in fs::read_dir(raw_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() { continue; }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if cat_patterns::CAT_UNIVERSAL_FILES.contains(&name) {
            let dest = cats_dir.join(name);
            
            if global_patterns::CHECK_LINE_FILES.contains(&name) {
                if let Ok(moved) = move_if_bigger(&path, &dest) {
                    if moved { count += 1; }
                }
            } else {
                if move_fast(&path, &dest).is_ok() { count += 1; }
            }
            continue; 
        }

        let dest_folder = global_matcher.get_dest(name, assets_dir)
            .or_else(|| cat_matcher.get_dest(name, cats_dir))
            .or_else(|| enemy_matcher.get_dest(name, enemy_dir));

        if let Some(folder) = dest_folder {
            if !folder.exists() { 
                let _ = fs::create_dir_all(&folder); 
            }
            
            let dest = folder.join(name);

            if global_patterns::CHECK_LINE_FILES.contains(&name) {
                if let Ok(moved) = move_if_bigger(&path, &dest) {
                    if moved { count += 1; }
                }
            } else {
                if move_fast(&path, &dest).is_ok() { count += 1; }
            }
            
            if count % 500 == 0 {
                let _ = tx.send(format!("Sorted {} files | Current: {}", count, name));
            }
        }
    }

    let _ = tx.send("Success! Files sorted.".to_string());
    Ok(())
}