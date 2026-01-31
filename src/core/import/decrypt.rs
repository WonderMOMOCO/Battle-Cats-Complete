use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::io::{Read, Seek, SeekFrom};
use rayon::prelude::*;
use zip::ZipArchive;
use crate::core::import::keys; 
use crate::core::patterns; 

pub fn run(folder_path: &str, region_code: &str, tx: Sender<String>) -> Result<(), String> {
    let source_dir = Path::new(folder_path);
    let raw_dir = Path::new("game/raw");
    let game_dir = Path::new("game");
    
    if !raw_dir.exists() {
        fs::create_dir_all(raw_dir).map_err(|e| e.to_string())?;
    }

    let _ = tx.send("Indexing existing files...".to_string());
    let shared_index = Arc::new(build_index(game_dir));

    let _ = tx.send(format!("Scanning for {} files...", region_code));
    
    let mut tasks = Vec::new();
    find_game_files(source_dir, &mut tasks).map_err(|e| e.to_string())?;

    let _ = tx.send(format!("Found {} decryptable files. Starting...", tasks.len()));

    let count = AtomicI32::new(0);
    let region_ref = region_code.to_string(); 

    tasks.par_iter().for_each(|path| {
        process_task(path, raw_dir, &count, &tx, &shared_index, &region_ref);
    });

    let _ = tx.send(format!("Decryption complete. Processed {} files.", count.load(Ordering::Relaxed)));
    
    Ok(())
}

fn process_task(
    file_path: &Path, 
    output_dir: &Path, 
    counter: &AtomicI32, 
    tx: &Sender<String>, 
    index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>, 
    region: &str
) {
    let ext = file_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    
    if ext == "apk" || ext == "xapk" {
        if let Err(e) = process_apk(file_path, output_dir, counter, tx, index, region) {
             let _ = tx.send(format!("Error processing APK: {}", e));
        }
    } else if ext == "list" {
        let pack_path = file_path.with_extension("pack");
        if pack_path.exists() {
            if let Ok(data) = fs::read(file_path) {
                if let Ok(content) = decrypt_list_content(&data) {
                    let _ = extract_pack(&content, &pack_path, output_dir, counter, tx, index, region);
                }
            }
        }
    }
}

fn determine_code(filename: &str, selected_region: &str) -> String {
    if selected_region != "en" {
        return selected_region.to_string();
    }
    for code in patterns::GLOBAL_CODES {
        if *code == "en" { continue; } 
        if filename.contains(&format!("_{}", code)) {
            return code.to_string();
        }
    }
    "en".to_string()
}

fn extract_pack(
    content: &str, 
    pack_path: &Path, 
    output_dir: &Path, 
    counter: &AtomicI32,
    tx: &Sender<String>,
    index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>, 
    region: &str
) -> Result<(), String> {
    let mut file = fs::File::open(pack_path).map_err(|e| e.to_string())?;
    let pack_name = pack_path.file_name().unwrap_or_default().to_string_lossy();
    
    let current_code = determine_code(&pack_name, region);

    for line in content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }
        
        let asset_name = parts[0];
        let offset: u64 = parts[1].parse().unwrap_or(0);
        let size: usize = parts[2].parse().unwrap_or(0);
        
        if should_skip(asset_name, size, output_dir, index) { continue; }

        let aligned_size = if size % 16 == 0 { size } else { ((size / 16) + 1) * 16 };
        if file.seek(SeekFrom::Start(offset)).is_err() { continue; }
        
        let mut buffer = vec![0u8; aligned_size];
        if file.read_exact(&mut buffer).is_err() { continue; }

        if let Ok((decrypted_bytes, _)) = keys::decrypt_pack_chunk(&buffer, asset_name) {
            let final_data = &decrypted_bytes[..std::cmp::min(size, decrypted_bytes.len())];

            let is_region_sensitive = patterns::REGION_SENSITIVE_FILES.iter()
                .any(|&x| asset_name.ends_with(x) || asset_name.starts_with(x));

            let target_path = if is_region_sensitive {
                 let path_obj = Path::new(asset_name);
                 let stem = path_obj.file_stem().unwrap().to_string_lossy();
                 let ext = path_obj.extension().unwrap().to_string_lossy();
                 output_dir.join(format!("{}_{}.{}", stem, current_code, ext))
             } else {
                 output_dir.join(asset_name)
             };

            if write_smart(&target_path, final_data, asset_name) {
                let c = counter.fetch_add(1, Ordering::Relaxed);
                if c % 50 == 0 { 
                    let _ = tx.send(format!("Decrypted {} files | Current: {}", c, asset_name)); 
                }
            }
        }
    }
    Ok(())
}

fn process_apk(
    apk_path: &Path, 
    output_dir: &Path, 
    counter: &AtomicI32, 
    tx: &Sender<String>,
    index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>, 
    region: &str
) -> Result<(), String> {
    let file = fs::File::open(apk_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut list_pack_pairs = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file_in_zip) = archive.by_index(i) {
            let name = file_in_zip.name().to_string();
            if name.ends_with(".list") {
                let pack_name = name.replace(".list", ".pack");
                list_pack_pairs.push((name, pack_name));
            }
        }
    }

    for (list_name, pack_name) in list_pack_pairs {
        let mut list_content_bytes = Vec::new();
        let mut read_success = false;

        if let Ok(mut list_file) = archive.by_name(&list_name) {
            if list_file.read_to_end(&mut list_content_bytes).is_ok() { read_success = true; }
        } 

        if read_success {
            if let Ok(decrypted_content) = decrypt_list_content(&list_content_bytes) {
                if let Ok(mut pack_file) = archive.by_name(&pack_name) {
                    let safe_filename = Path::new(&pack_name).file_name().unwrap().to_string_lossy();
                    let temp_pack_path = output_dir.join(format!("_temp_{}", safe_filename));
                    
                    if let Ok(mut temp_file) = fs::File::create(&temp_pack_path) {
                        if std::io::copy(&mut pack_file, &mut temp_file).is_ok() {
                            let _ = extract_pack(&decrypted_content, &temp_pack_path, output_dir, counter, tx, index, region);
                            let _ = fs::remove_file(temp_pack_path);
                        }
                    }
                }
            }
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

fn find_game_files(search_dir: &Path, path_list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !search_dir.is_dir() { return Ok(()); }
    for entry_result in fs::read_dir(search_dir)?.flatten() {
        let path = entry_result.path();
        if path.is_dir() {
            find_game_files(&path, path_list)?;
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ext_str == "list" || ext_str == "apk" || ext_str == "xapk" {
                path_list.push(path);
            }
        }
    }
    Ok(())
}

fn should_skip(name: &str, size: usize, output_dir: &Path, index: &std::collections::HashMap<String, Vec<PathBuf>>) -> bool {
    if patterns::CHECK_LINE_FILES.contains(&name) { return false; }
    if name.ends_with("img015_th.imgcut") { return true; }
    if patterns::REGION_SENSITIVE_FILES.iter().any(|&x| name.ends_with(x) || name.starts_with(x)) { return false; }

    let name_lower = name.to_lowercase();
    if let Some(existing_paths) = index.get(&name_lower) {
        for path in existing_paths {
            if let Ok(meta) = fs::metadata(path) {
                if meta.len() as usize >= size.saturating_sub(16) { return true; }
            }
        }
    }
    let target_path = output_dir.join(name);
    if target_path.exists() {
        if let Ok(meta) = fs::metadata(&target_path) {
            if meta.len() as usize >= size.saturating_sub(16) { return true; }
        }
    }
    false
}

fn build_index(root_dir: &Path) -> std::collections::HashMap<String, Vec<PathBuf>> {
    let mut index = std::collections::HashMap::new();
    let _ = scan_for_index(root_dir, &mut index);
    index
}

fn scan_for_index(dir: &Path, index: &mut std::collections::HashMap<String, Vec<PathBuf>>) -> std::io::Result<()> {
    if !dir.is_dir() { return Ok(()); }
    for entry_result in fs::read_dir(dir)?.flatten() {
        let path = entry_result.path();
        if path.is_dir() {
            let _ = scan_for_index(&path, index);
        } else if let Some(name) = path.file_name() {
            let key = name.to_string_lossy().to_lowercase();
            index.entry(key).or_insert_with(Vec::new).push(path);
        }
    }
    Ok(())
}

fn count_lines(data: &[u8]) -> usize {
    data.iter().filter(|&&b| b == b'\n').count()
}

fn write_smart(target_path: &Path, data: &[u8], filename: &str) -> bool {
    let new_size = data.len() as u64;
    
    if target_path.exists() {
        if patterns::CHECK_LINE_FILES.contains(&filename) {
            if let Ok(existing_bytes) = fs::read(target_path) {
                let old_line_count = count_lines(&existing_bytes);
                let new_line_count = count_lines(data);
                if new_line_count <= old_line_count { return false; }
            }
        } else {
            if let Ok(meta) = fs::metadata(target_path) {
                if meta.len() >= new_size { return false; }
            }
        }
    }

    if let Some(parent_dir) = target_path.parent() {
        if !parent_dir.exists() { let _ = fs::create_dir_all(parent_dir); }
    }
    let temp_ext = format!("tmp_{:?}", std::thread::current().id()).replace("ThreadId(", "").replace(")", "");
    let temp_path = target_path.with_extension(&temp_ext);

    if fs::write(&temp_path, data).is_err() { return false; }
    let _ = fs::rename(&temp_path, target_path);
    true
}