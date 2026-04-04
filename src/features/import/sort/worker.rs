use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use crate::features::cat::patterns as cat_patterns; 
use crate::global::io::patterns as global_patterns;
use super::{cat, global, enemy, stage}; 

fn ensure_parent_dir(dest: &Path) {
    if let Some(parent) = dest.parent() {
        if !parent.exists() { let _ = fs::create_dir_all(parent); }
    }
}

pub fn move_if_heavier(src: &Path, dest: &Path) -> std::io::Result<bool> {
    ensure_parent_dir(dest);
    if !dest.exists() {
        fs::rename(src, dest)?;
        return Ok(true);
    }
    let src_size = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    let dest_size = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    
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
    if !name.starts_with("udi") || name.len() < 6 { return false; }
    let Ok(id) = name[3..6].parse::<u32>() else { return false; };
    if id > 9 { return false; }
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

fn collect_files_recursive(dir: &Path, list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, list)?;
            } else {
                list.push(path);
            }
        }
    }
    Ok(())
}

fn remove_empty_dirs(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                remove_empty_dirs(&path);
                let _ = fs::remove_dir(&path); 
            }
        }
    }
}

pub fn sort_game_files(tx: Sender<String>, abort_flag: Arc<AtomicBool>, prog_curr: Arc<AtomicUsize>, prog_max: Arc<AtomicUsize>) -> Result<(), String> {
    let raw_dir = Path::new("game/raw");
    let cats_dir = Path::new("game/cats");
    let sheets_dir = Path::new("game/sheets");
    let ui_dir = Path::new("game/ui");
    let tables_dir = Path::new("game/tables");
    let enemy_dir = Path::new("game/enemies");
    let stages_dir = Path::new("game/stages"); 

    if !raw_dir.exists() { return Err("Raw directory not found.".to_string()); }

    let cat_matcher = cat::CatMatcher::new();
    let global_matcher = global::GlobalMatcher::new();
    let enemy_matcher = enemy::EnemyMatcher::new();
    let stage_matcher = stage::StageMatcher::new(); 

    let mut valid_tasks: Vec<(PathBuf, String, String, PathBuf)> = Vec::new();

    // Recursively gather all files in raw_dir
    let mut all_files = Vec::new();
    if let Err(e) = collect_files_recursive(raw_dir, &mut all_files) {
        let _ = tx.send(format!("Warning: Issue reading some raw files: {}", e));
    }

    // Identify and route valid files
    for path in all_files {
        if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".into()); }

        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(|n| n.to_string()) else { continue; };
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let base_name = clean_base_name(&stem, &ext);

        let dest_folder = if cat_patterns::CAT_UNIVERSAL_FILES.contains(&base_name.as_str()) {
            Some(cats_dir.to_path_buf())
        } else if is_cat_base_banner(&name, &base_name) {
            Some(cats_dir.join("CatBase"))
        } else {
            global_matcher.get_dest(&base_name, sheets_dir, ui_dir, tables_dir)
                .or_else(|| cat_matcher.get_dest(&base_name, cats_dir))
                .or_else(|| enemy_matcher.get_dest(&base_name, enemy_dir))
                .or_else(|| stage_matcher.get_dest(&base_name, stages_dir))
        };

        if let Some(folder) = dest_folder { valid_tasks.push((path, name, base_name, folder)); }
    }

    let files_to_sort = valid_tasks.len();
    if files_to_sort == 0 {
        let _ = tx.send("No new files to sort.".to_string());
        prog_max.store(0, Ordering::Relaxed);
        remove_empty_dirs(raw_dir);
        return Ok(());
    }

    prog_max.store(files_to_sort, Ordering::Relaxed);
    prog_curr.store(0, Ordering::Relaxed);

    let update_interval = (files_to_sort / 100).max(10);
    let _ = tx.send(format!("Sorting {} recognized game files...", files_to_sort));
    let mut count = 0;
    
    // Perform the moves
    for (path, name, _base_name, folder) in valid_tasks {
        if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".into()); }
        if !folder.exists() { let _ = fs::create_dir_all(&folder); }
        let dest = folder.join(&name);
        
        let processed = move_if_heavier(&path, &dest).unwrap_or(false);

        prog_curr.fetch_add(1, Ordering::Relaxed);

        if processed {
            count += 1;
            if count % update_interval == 0 { let _ = tx.send(format!("Sorted {} files | Current: {}", count, name)); }
        }
    }

    remove_empty_dirs(raw_dir);

    let _ = tx.send("Success! Files sorted.".to_string());
    prog_max.store(0, Ordering::Relaxed);
    Ok(())
}