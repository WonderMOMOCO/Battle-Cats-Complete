use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use rayon::prelude::*;
use zip::ZipArchive;

use crate::features::import::logic::keys; 
use crate::global::io::patterns;

#[derive(Clone)]
struct PackEntry {
    pack_path: PathBuf,
    original_name: String,
    offset: u64,
    size: usize,
}

pub fn run(folder_path: &str, region_code: &str, tx: Sender<String>) -> Result<(), String> {
    let source_dir = Path::new(folder_path);
    let raw_dir = Path::new("game/raw");
    let game_dir = Path::new("game");
    
    if !raw_dir.exists() { let _ = fs::create_dir_all(raw_dir); }

    let _ = tx.send("Indexing existing workspace files...".to_string());
    let shared_index = Arc::new(build_index(game_dir));

    let _ = tx.send("Scanning for game files...".to_string());
    
    let mut list_paths = Vec::new();
    let mut apk_paths = Vec::new();
    let _ = find_game_files(source_dir, &mut list_paths, &mut apk_paths);

    let mut dynamic_temp_dirs = Vec::new();

    // Extract APK lists to dynamic sibling directories
    if !apk_paths.is_empty() {
        let _ = tx.send("Extracting base data from APK...".to_string());
        for apk in apk_paths {
            let parent = apk.parent().unwrap_or(Path::new(""));
            let stem = apk.file_stem().unwrap_or_default().to_string_lossy();
            let apk_temp_dir = parent.join(stem.to_string());
            
            if !apk_temp_dir.exists() { 
                let _ = fs::create_dir_all(&apk_temp_dir); 
            }
            
            let mut extracted = extract_apk_data(&apk, &apk_temp_dir);
            list_paths.append(&mut extracted);
            dynamic_temp_dirs.push(apk_temp_dir);
        }
    }

    // Chronological Sort with God Mode
    let _ = tx.send("Sorting patch history chronologically...".to_string());
    list_paths.sort_by_key(|p| calculate_order(p, &dynamic_temp_dirs));

    // Build the "Last One Wins" In-Memory Map
    let mut master_map: HashMap<String, PackEntry> = HashMap::new();
    
    for list_path in list_paths {
        let pack_path = list_path.with_extension("pack");
        if !pack_path.exists() { continue; }

        if let Ok(list_data) = fs::read(&list_path) {
            if let Ok(content) = decrypt_list_content(&list_data) {
                let pack_name = pack_path.file_name().unwrap_or_default().to_string_lossy();
                let current_code = determine_code(&pack_name, region_code);

                for line in content.lines() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() < 3 { continue; }
                    
                    let asset_name = parts[0];
                    let offset: u64 = parts[1].parse().unwrap_or(0);
                    let size: usize = parts[2].parse().unwrap_or(0);

                    // --- LANGUAGE SENSITIVE LOGIC ---
                    let is_lang_sensitive = patterns::LANGUAGE_SENSITIVE_FILES.iter()
                        .any(|&x| asset_name.ends_with(x) || asset_name.starts_with(x));

                    let mut final_filename = asset_name.to_string();
                    if is_lang_sensitive {
                        let path_obj = Path::new(asset_name);
                        let stem = path_obj.file_stem().unwrap().to_string_lossy();
                        let ext = path_obj.extension().unwrap().to_string_lossy();
                        
                        let mut clean_stem = stem.to_string();
                        for code in ["en", "ja", "tw", "kr", "th", "it", "fr", "de", "es"] {
                            let suffix = format!("_{}", code);
                            if clean_stem.ends_with(&suffix) {
                                clean_stem = clean_stem.trim_end_matches(&suffix).to_string();
                                break;
                            }
                        }

                        if current_code.is_empty() {
                            final_filename = format!("{}.{}", clean_stem, ext);
                        } else {
                            final_filename = format!("{}_{}.{}", clean_stem, current_code, ext);
                        }
                    }

                    // Insert into map. Newer files overwrite older files naturally
                    master_map.insert(final_filename, PackEntry {
                        pack_path: pack_path.clone(),
                        original_name: asset_name.to_string(),
                        offset,
                        size,
                    });
                }
            }
        }
    }

    let total_files = master_map.len();
    let _ = tx.send(format!("Found {} total files. Starting extraction...", total_files));

    let update_interval = (total_files / 100).max(10) as i32;

    // Group tasks by pack file to minimize disk IO overhead
    let mut pack_tasks: HashMap<PathBuf, Vec<(String, PackEntry)>> = HashMap::new();
    for (final_name, entry) in master_map {
        pack_tasks.entry(entry.pack_path.clone()).or_default().push((final_name, entry));
    }

    let count = AtomicI32::new(0);

    // High-Speed Parallel Extraction
    pack_tasks.into_par_iter().for_each(|(pack_path, entries)| {
        if let Ok(mut file) = fs::File::open(&pack_path) {
            for (final_name, entry) in entries {
                let target_path = raw_dir.join(&final_name);

                let mut should_extract = true; 
                let name_lower = final_name.to_lowercase();

                let size_tolerance = 128;

                // Check if it exists anywhere in the game folder
                if let Some(existing_paths) = shared_index.get(&name_lower) {
                    for path in existing_paths {
                        if let Ok(meta) = fs::metadata(path) {
                            if meta.len() as usize + size_tolerance >= entry.size {
                                should_extract = false;
                                break;
                            }
                        }
                    }
                }
                
                // Check the raw_dir just in case it was freshly written or not indexed
                if should_extract && target_path.exists() {
                     if let Ok(meta) = fs::metadata(&target_path) {
                         if meta.len() as usize + size_tolerance >= entry.size {
                             should_extract = false;
                         }
                     }
                }

                if !should_extract {
                    continue; 
                }

                // If it doesn't exist (or the incoming file is strictly larger), extract it
                let aligned_size = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };
                if file.seek(SeekFrom::Start(entry.offset)).is_err() { continue; }
                
                let mut buffer = vec![0u8; aligned_size];
                if file.read_exact(&mut buffer).is_err() { continue; }

                if let Ok((decrypted_bytes, _)) = keys::decrypt_pack_chunk(&buffer, &entry.original_name) {
                    let final_data = &decrypted_bytes[..std::cmp::min(entry.size, decrypted_bytes.len())];

                    if let Some(parent_dir) = target_path.parent() {
                        let _ = fs::create_dir_all(parent_dir);
                    }
                    let _ = fs::write(&target_path, final_data);

                    let c = count.fetch_add(1, Ordering::Relaxed) + 1;
                    if c % update_interval == 0 { 
                        let _ = tx.send(format!("Extracted {} files | Current: {}", c, final_name)); 
                    }
                }
            }
        }
    });

    // Cleanup Phase
    for dir in &dynamic_temp_dirs {
        let _ = fs::remove_dir_all(dir);
    }

    let _ = tx.send(format!("Decryption complete. Extracted {} new or updated files.", count.load(Ordering::Relaxed)));
    Ok(())
}

fn determine_code(filename: &str, selected_region: &str) -> String {
    if selected_region != "en" { return selected_region.to_string(); }
    for code in patterns::GLOBAL_CODES {
        if *code == "en" { continue; } 
        if filename.contains(&format!("_{}", code)) { return code.to_string(); }
    }
    "en".to_string()
}

fn calculate_order(path: &Path, temp_apk_dirs: &[PathBuf]) -> u64 {
    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let mut score = 5_000; 
    
    let parts: Vec<&str> = name.split('_').collect();
    if parts.len() >= 3 {
        if let (Ok(v1), Ok(v2)) = (parts[1].parse::<u64>(), parts[2].parse::<u64>()) {
            score = 100_000_000 + (v1 * 100) + v2;
        }
    }

    if score == 5_000 && name.ends_with("Server") {
        let chars: Vec<char> = name.chars().collect();
        if chars.len() > 1 && chars[0].is_ascii_uppercase() && chars[1].is_ascii_uppercase() {
            score = 20_000 + (chars[0] as u64); 
        } else {
            score = 10_000; 
        }
    }

    if score == 5_000 && (name == "DataLocal" || name == "UpdateLocal" || name.ends_with("Local")) {
        score = 0; 
    }

    if temp_apk_dirs.iter().any(|dir| path.starts_with(dir)) {
        score += 500_000_000;
    }

    score
}

fn build_index(root_dir: &Path) -> HashMap<String, Vec<PathBuf>> {
    let mut index = HashMap::new();
    let _ = scan_for_index(root_dir, &mut index);
    index
}

fn scan_for_index(dir: &Path, index: &mut HashMap<String, Vec<PathBuf>>) -> std::io::Result<()> {
    if !dir.is_dir() { return Ok(()); }
    for entry_result in fs::read_dir(dir)?.flatten() {
        let path = entry_result.path();
        if path.is_dir() {
            let path_str = path.to_string_lossy().replace('\\', "/");
            if path_str == "game/app" {
                continue;
            }
            let _ = scan_for_index(&path, index);
        } else if let Some(name) = path.file_name() {
            let key = name.to_string_lossy().to_lowercase();
            index.entry(key).or_insert_with(Vec::new).push(path);
        }
    }
    Ok(())
}

fn decrypt_list_content(data: &[u8]) -> Result<String, String> {
    let pack_key = keys::get_md5_key("pack");
    if let Ok(bytes) = keys::decrypt_ecb_with_key(data, &pack_key) {
        if let Ok(s) = String::from_utf8(bytes) { return Ok(s); }
    }
    let bc_key = keys::get_md5_key("battlecats");
    if let Ok(bytes) = keys::decrypt_ecb_with_key(data, &bc_key) {
        if let Ok(s) = String::from_utf8(bytes) { return Ok(s); }
    }
    Err("Decryption failed".into())
}

fn find_game_files(search_dir: &Path, list_paths: &mut Vec<PathBuf>, apk_paths: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !search_dir.is_dir() { return Ok(()); }
    for entry_result in fs::read_dir(search_dir)?.flatten() {
        let path = entry_result.path();
        if path.is_dir() {
            find_game_files(&path, list_paths, apk_paths)?;
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ext_str == "list" {
                list_paths.push(path);
            } else if ext_str == "apk" || ext_str == "xapk" {
                apk_paths.push(path);
            }
        }
    }
    Ok(())
}

fn extract_apk_data(apk_path: &Path, temp_dir: &Path) -> Vec<PathBuf> {
    let mut extracted_lists = Vec::new();
    if let Ok(file) = fs::File::open(apk_path) {
        if let Ok(mut archive) = ZipArchive::new(file) {
            for i in 0..archive.len() {
                if let Ok(mut file_in_zip) = archive.by_index(i) {
                    let name = file_in_zip.name().to_string();
                    if name.ends_with(".list") || name.ends_with(".pack") {
                        let safe_name = Path::new(&name).file_name().unwrap();
                        let out_path = temp_dir.join(safe_name);
                        if let Ok(mut out_file) = fs::File::create(&out_path) {
                            let _ = std::io::copy(&mut file_in_zip, &mut out_file);
                        }
                        if name.ends_with(".list") {
                            extracted_lists.push(out_path);
                        }
                    }
                }
            }
        }
    }
    extracted_lists
}