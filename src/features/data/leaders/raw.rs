use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use rayon::prelude::*;

use crate::features::data::utilities::{audit, router, manifest};

pub fn run(
    source_path_string: &str, 
    status_sender: Sender<String>, 
    abort_flag: Arc<AtomicBool>, 
    progress_current: Arc<AtomicUsize>, 
    progress_maximum: Arc<AtomicUsize>
) -> Result<(), String> {
    
    let source_path = Path::new(source_path_string);
    let game_root_path = Path::new("game");
    let raw_directory_path = game_root_path.join("raw");

    if !raw_directory_path.exists() { 
        let _ = fs::create_dir_all(&raw_directory_path); 
    }

    if let (Ok(source_canonical), Ok(raw_canonical)) = (source_path.canonicalize(), raw_directory_path.canonicalize()) {
        if source_canonical == raw_canonical {
            let _ = status_sender.send("Organizing recognized raw data.".to_string());
            return sort_raw_folder(&raw_directory_path, game_root_path, &status_sender, &abort_flag, &progress_current, &progress_maximum);
        }
    }

    if let (Ok(source_canonical), Ok(game_canonical)) = (source_path.canonicalize(), game_root_path.canonicalize()) {
        if source_canonical == game_canonical {
            let _ = status_sender.send("Beginning database restructure...".to_string());
            flatten_to_raw(game_root_path, &raw_directory_path, &status_sender, &abort_flag, &progress_current, &progress_maximum)?;
            return sort_raw_folder(&raw_directory_path, game_root_path, &status_sender, &abort_flag, &progress_current, &progress_maximum);
        }
    }

    let _ = status_sender.send("Importing standard raw files...".to_string());
    
    let mut files_to_import = Vec::new();
    collect_files_recursive(source_path, &mut files_to_import);
    
    if files_to_import.is_empty() {
        let _ = status_sender.send("No files found in source directory.".to_string());
        return Ok(());
    }
    
    let update_interval = (files_to_import.len() / 100).max(10);
    progress_maximum.store(files_to_import.len(), Ordering::Relaxed);
    progress_current.store(0, Ordering::Relaxed);
    
    let count = std::sync::atomic::AtomicUsize::new(0);
    
    files_to_import.par_iter().for_each(|path| {
        if abort_flag.load(Ordering::Relaxed) { return; }
        if let Some(filename) = path.file_name() {
            let _ = fs::copy(path, raw_directory_path.join(filename));
            
            let c = count.fetch_add(1, Ordering::Relaxed) + 1;
            progress_current.store(c, Ordering::Relaxed);
            if c % update_interval == 0 {
                let _ = status_sender.send(format!("Copied {} files to raw...", c));
            }
        }
    });

    sort_raw_folder(&raw_directory_path, game_root_path, &status_sender, &abort_flag, &progress_current, &progress_maximum)
}

fn sort_raw_folder(
    raw_directory: &Path, 
    game_root_path: &Path, 
    status_sender: &Sender<String>, 
    abort_flag: &Arc<AtomicBool>, 
    progress_current: &Arc<AtomicUsize>, 
    progress_maximum: &Arc<AtomicUsize>
) -> Result<(), String> {
    
    let mut all_discovered_files = Vec::new(); 
    collect_files_recursive(raw_directory, &mut all_discovered_files);
    
    if all_discovered_files.is_empty() { 
        let _ = status_sender.send("Raw folder is empty.".to_string());
        return Ok(()); 
    }

    let asset_router = router::AssetRouter::new(game_root_path);
    let file_manifest_path = game_root_path.join("meta").join("file.json");
    let mut global_file_ledger: HashMap<String, manifest::ManifestEntry> = manifest::load(&file_manifest_path);

    progress_maximum.store(all_discovered_files.len(), Ordering::Relaxed);
    progress_current.store(0, Ordering::Relaxed);
    let update_interval = (all_discovered_files.len() / 100).max(10);
    let extracted_count = std::sync::atomic::AtomicUsize::new(0);

    let updated_manifest_entries: Vec<(String, manifest::ManifestEntry)> = all_discovered_files.into_par_iter().filter_map(|file_path: PathBuf| {
        if abort_flag.load(Ordering::Relaxed) { return None; }
        
        let Some(filename_os) = file_path.file_name() else { return None; };
        let filename_string = filename_os.to_string_lossy().to_string();
        
        let target_destination_path = asset_router.resolve_destination(&filename_string, &filename_string);
        
        if file_path == target_destination_path { return None; }

        let Ok(file_data) = fs::read(&file_path) else { return None; };
        
        let true_calculated_weight = audit::calculate_true_weight(&file_data, &filename_string);
        let clean_file_data = audit::strip_carriage_returns(&file_data, &filename_string);
        
        if let Some(parent_directory) = target_destination_path.parent() { 
            let _ = fs::create_dir_all(parent_directory); 
        }
        
        let _ = fs::write(&target_destination_path, &clean_file_data);
        let _ = fs::remove_file(&file_path); 

        let c = extracted_count.fetch_add(1, Ordering::Relaxed) + 1;
        progress_current.store(c, Ordering::Relaxed);
        
        if c % update_interval == 0 {
            let _ = status_sender.send(format!("Sorted {} files | Current: {}", c, filename_string));
        }

        let manifest_entry = manifest::ManifestEntry {
            winner: "Unknown".to_string(),
            weight: true_calculated_weight,
            size: clean_file_data.len(),
            encrypted: file_data.len(),
            checksum: manifest::hash(&clean_file_data),
        };

        Some((filename_string, manifest_entry))
    }).collect();

    for (filename_key, entry_data) in updated_manifest_entries { 
        global_file_ledger.insert(filename_key, entry_data); 
    }
    
    manifest::save(&file_manifest_path, &global_file_ledger);

    let _ = status_sender.send("Raw files successfully structured.".to_string());
    Ok(())
}

fn flatten_to_raw(
    game_root_path: &Path, 
    raw_directory: &Path, 
    status_sender: &Sender<String>, 
    abort_flag: &Arc<AtomicBool>, 
    progress_current: &Arc<AtomicUsize>, 
    progress_maximum: &Arc<AtomicUsize>
) -> Result<(), String> {
    
    let mut all_files = Vec::new();
    let meta_directories = ["raw", "app", "meta"];
    
    if let Ok(entries) = fs::read_dir(game_root_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if !meta_directories.contains(&dir_name.as_str()) {
                    collect_files_recursive(&path, &mut all_files);
                }
            }
        }
    }

    if all_files.is_empty() {
        let _ = status_sender.send("No valid files to flatten.".to_string());
        return Ok(());
    }

    let _ = status_sender.send(format!("Flattening {} files to raw directory...", all_files.len()));
    progress_maximum.store(all_files.len(), Ordering::Relaxed);
    progress_current.store(0, Ordering::Relaxed);
    
    let update_interval = (all_files.len() / 100).max(10);
    let count = std::sync::atomic::AtomicUsize::new(0);

    all_files.par_iter().for_each(|path| {
        if abort_flag.load(Ordering::Relaxed) { return; }
        
        if let Some(file_name) = path.file_name() {
            let destination = raw_directory.join(file_name);
            
            let source_length = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let destination_length = fs::metadata(&destination).map(|m| m.len()).unwrap_or(0);

            if !destination.exists() || source_length != destination_length {
                if fs::rename(path, &destination).is_err() {
                    let _ = fs::copy(path, &destination);
                    let _ = fs::remove_file(path);
                }
            } else {
                let _ = fs::remove_file(path);
            }
        }

        let current_count = count.fetch_add(1, Ordering::Relaxed) + 1;
        progress_current.store(current_count, Ordering::Relaxed);
        
        if current_count % update_interval == 0 {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let _ = status_sender.send(format!("Moved {} files to raw | Current: {}", current_count, name));
        }
    });

    if let Ok(entries) = fs::read_dir(game_root_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if !meta_directories.contains(&dir_name.as_str()) {
                    remove_empty_directories(&path);
                }
            }
        }
    }

    let _ = status_sender.send("Flattening complete.".to_string());
    Ok(())
}

fn collect_files_recursive(directory: &Path, list: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, list);
            } else {
                list.push(path);
            }
        }
    }
}

fn remove_empty_directories(directory: &Path) {
    if !directory.is_dir() { return; }
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                remove_empty_directories(&path);
            }
        }
    }
    let _ = fs::remove_dir(directory);
}