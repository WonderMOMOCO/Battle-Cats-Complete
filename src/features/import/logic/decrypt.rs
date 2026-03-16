use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use rayon::prelude::*;
use zip::ZipArchive;
use regex::Regex;

use crate::features::import::logic::keys; 
use crate::global::io::patterns;
use crate::features::settings::logic::exceptions::{ExceptionList, ExceptionRule, get_config_path, RuleHandling};

#[derive(Clone)]
struct PackEntry {
    pack_path: PathBuf,
    original_name: String,
    offset: u64,
    size: usize,
}

fn is_potential_conflict(name: &str) -> bool {
    if !name.starts_with("udi") || !name.ends_with(".png") { return false; }
    
    let stem = Path::new(name).file_stem().unwrap_or_default().to_string_lossy();
    if stem.len() < 6 { return false; }
    
    let Ok(id) = stem[3..6].parse::<u32>() else { return false; };
    id <= 9
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
    extract_apks(&apk_paths, &mut list_paths, &mut dynamic_temp_dirs, &tx);

    let _ = tx.send("Sorting patch history chronologically...".to_string());
    list_paths.sort_by_key(|p| calculate_order(p, &dynamic_temp_dirs));

    let compiled_rules = compile_rules();
    let mut master_map = HashMap::new();
    let mut conflict_list = Vec::new(); 
    
    parse_list_files(&list_paths, region_code, &compiled_rules, &mut master_map, &mut conflict_list);

    let filtered_tasks = filter_existing_tasks(master_map, &shared_index, raw_dir);
    
    let cat_base_dir = Path::new("game/cats/CatBase");
    conflict_list.retain(|(name, entry)| {
        let name_lower = name.to_lowercase();
        if shared_index.contains_key(&name_lower) { return false; } 
        let target_raw = raw_dir.join(name);
        let target_base = cat_base_dir.join(name);
        for p in [&target_raw, &target_base] {
            if p.exists() {
                if let Ok(meta) = fs::metadata(p) {
                    if meta.len() as usize + 128 >= entry.size { return false; }
                }
            }
        }
        true
    });

    let to_extract_count = filtered_tasks.len();
    if to_extract_count == 0 && conflict_list.is_empty() {
        let _ = tx.send("Workspace is already up to date.".to_string());
        cleanup_temp_dirs(&dynamic_temp_dirs);
        return Ok(());
    }

    let count = AtomicI32::new(0);
    if to_extract_count > 0 {
        extract_standard_packs(filtered_tasks, raw_dir, &count, to_extract_count, &tx);
    }

    if !conflict_list.is_empty() {
        resolve_conflicts(conflict_list, raw_dir, &count, &tx);
    }

    cleanup_temp_dirs(&dynamic_temp_dirs);
    let _ = tx.send(format!("Decryption complete. Extracted {} files.", count.load(Ordering::Relaxed)));
    Ok(())
}

fn extract_apks(apk_paths: &[PathBuf], list_paths: &mut Vec<PathBuf>, temp_dirs: &mut Vec<PathBuf>, tx: &Sender<String>) {
    if apk_paths.is_empty() { return; }
    
    let _ = tx.send("Extracting base data from APK...".to_string());
    for apk in apk_paths {
        let parent = apk.parent().unwrap_or(Path::new(""));
        let stem = apk.file_stem().unwrap_or_default().to_string_lossy();
        let apk_temp_dir = parent.join(stem.to_string());
        
        if !apk_temp_dir.exists() { let _ = fs::create_dir_all(&apk_temp_dir); }
        
        let mut extracted = extract_apk_data(apk, &apk_temp_dir);
        list_paths.append(&mut extracted);
        temp_dirs.push(apk_temp_dir);
    }
}

fn compile_rules() -> Vec<(Regex, ExceptionRule)> {
    let exceptions = ExceptionList::load_or_default(&get_config_path());
    let mut compiled_rules = Vec::new();
    
    let lang_codes: Vec<&str> = patterns::APP_LANGUAGES.iter().map(|&(code, _)| code).collect();
    let lang_str = format!(r"(?:_(?:{}))?", lang_codes.join("|"));
    
    for rule in exceptions.rules {
        if rule.pattern.is_empty() && rule.extension.is_empty() { continue; }
        let ext_str = if rule.extension.is_empty() { String::new() } else { format!(r"\.(?:{})", rule.extension) };
        let pattern = format!(r"^(?:{}){}{}$", rule.pattern, lang_str, ext_str);
        if let Ok(re) = Regex::new(&pattern) {
            compiled_rules.push((re, rule));
        }
    }
    compiled_rules
}

fn parse_list_files(
    list_paths: &[PathBuf], region_code: &str, compiled_rules: &[(Regex, ExceptionRule)], 
    master_map: &mut HashMap<String, PackEntry>, conflict_list: &mut Vec<(String, PackEntry)>
) {
    for list_path in list_paths {
        let pack_path = list_path.with_extension("pack");
        if !pack_path.exists() { continue; }

        let Ok(list_data) = fs::read(list_path) else { continue; };
        let Ok(content) = decrypt_list_content(&list_data) else { continue; };
        
        let pack_name = pack_path.file_name().unwrap_or_default().to_string_lossy();
        let current_code = determine_code(&pack_name, region_code);

        for line in content.lines() {
            process_list_line(line, &pack_path, current_code.as_str(), compiled_rules, master_map, conflict_list);
        }
    }
}

fn process_list_line(
    line: &str, pack_path: &Path, current_code: &str, compiled_rules: &[(Regex, ExceptionRule)], 
    master_map: &mut HashMap<String, PackEntry>, conflict_list: &mut Vec<(String, PackEntry)>
) {
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 3 { return; }
    
    let asset_name = parts[0];
    let offset: u64 = parts[1].parse().unwrap_or(0);
    let size: usize = parts[2].parse().unwrap_or(0);

    let matched_rule = compiled_rules.iter().find(|(re, _)| re.is_match(asset_name)).map(|(_, r)| r);

    if let Some(rule) = matched_rule {
        if rule.handling == RuleHandling::Ignore { return; }
    }

    let entry = PackEntry {
        pack_path: pack_path.to_path_buf(),
        original_name: asset_name.to_string(),
        offset,
        size,
    };

    if matched_rule.is_none() {
        if is_potential_conflict(asset_name) {
            if let Some(pos) = conflict_list.iter().position(|(n, _)| n == asset_name) {
                if conflict_list[pos].1.size > entry.size + 128 { return; }
                conflict_list[pos] = (asset_name.to_string(), entry);
            } else {
                conflict_list.push((asset_name.to_string(), entry));
            }
        } else {
            if let Some(existing) = master_map.get(asset_name) {
                if existing.size > entry.size + 128 { return; }
            }
            master_map.insert(asset_name.to_string(), entry);
        }
        return;
    }

    let rule = matched_rule.unwrap();
    let path_obj = Path::new(asset_name);
    let stem = path_obj.file_stem().unwrap().to_string_lossy();
    let ext = path_obj.extension().unwrap_or_default().to_string_lossy();
    
    let mut clean_stem = stem.to_string();
    for &(code, _) in patterns::APP_LANGUAGES {
        let suffix = format!("_{}", code);
        if clean_stem.ends_with(&suffix) {
            clean_stem = clean_stem.trim_end_matches(&suffix).to_string();
            break;
        }
    }
    
    let is_enabled = rule.languages.get(current_code).copied().unwrap_or(false);

    if rule.handling == RuleHandling::Only && !is_enabled { return; }
    
    let mut final_filename = asset_name.to_string();
    if is_enabled && !current_code.is_empty() {
        final_filename = format!("{}_{}", clean_stem, current_code);
        if !ext.is_empty() { final_filename = format!("{}.{}", final_filename, ext); }
    }

    if is_potential_conflict(asset_name) {
        if let Some(pos) = conflict_list.iter().position(|(n, _)| n == &final_filename) {
            if conflict_list[pos].1.size > entry.size + 128 { return; }
            conflict_list[pos] = (final_filename, entry);
        } else {
            conflict_list.push((final_filename, entry));
        }
    } else {
        if let Some(existing) = master_map.get(&final_filename) {
            if existing.size > entry.size + 128 { return; }
        }
        master_map.insert(final_filename, entry);
    }
}

fn filter_existing_tasks(master_map: HashMap<String, PackEntry>, shared_index: &HashMap<String, Vec<PathBuf>>, raw_dir: &Path) -> Vec<(String, PackEntry)> {
    let size_tolerance = 128;
    master_map.into_iter().filter(|(final_name, entry)| {
        let name_lower = final_name.to_lowercase();
        if let Some(existing_paths) = shared_index.get(&name_lower) {
            for path in existing_paths {
                let Ok(meta) = fs::metadata(path) else { continue; };
                if meta.len() as usize + size_tolerance >= entry.size { return false; }
            }
        }
        let target_path = raw_dir.join(final_name);
        if target_path.exists() {
            let Ok(meta) = fs::metadata(&target_path) else { return true; };
            if meta.len() as usize + size_tolerance >= entry.size { return false; }
        }
        true
    }).collect()
}

fn extract_standard_packs(filtered_tasks: Vec<(String, PackEntry)>, raw_dir: &Path, count: &AtomicI32, total: usize, tx: &Sender<String>) {
    let _ = tx.send(format!("Found {} new or updated files. Starting extraction...", total));
    let update_interval = (total / 100).max(10) as i32;

    let mut pack_tasks: HashMap<PathBuf, Vec<(String, PackEntry)>> = HashMap::new();
    for (final_name, entry) in filtered_tasks {
        pack_tasks.entry(entry.pack_path.clone()).or_default().push((final_name, entry));
    }

    pack_tasks.into_par_iter().for_each(|(pack_path, entries)| {
        let Ok(mut file) = fs::File::open(&pack_path) else { return; };
        for (final_name, entry) in entries {
            let target_path = raw_dir.join(&final_name);
            let aligned_size = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };
            
            if file.seek(SeekFrom::Start(entry.offset)).is_err() { continue; }
            let mut buffer = vec![0u8; aligned_size];
            if file.read_exact(&mut buffer).is_err() { continue; }
            
            let Ok((decrypted_bytes, _)) = keys::decrypt_pack_chunk(&buffer, &entry.original_name) else { continue; };
            let final_data = &decrypted_bytes[..std::cmp::min(entry.size, decrypted_bytes.len())];
            
            if let Some(parent_dir) = target_path.parent() { let _ = fs::create_dir_all(parent_dir); }
            let _ = fs::write(&target_path, final_data);
            
            let c = count.fetch_add(1, Ordering::Relaxed) + 1;
            if c % update_interval == 0 { let _ = tx.send(format!("Extracted {} files | Current: {}", c, final_name)); }
        }
    });
}

fn resolve_conflicts(list: Vec<(String, PackEntry)>, raw: &Path, count: &AtomicI32, tx: &Sender<String>) {
    let _ = tx.send(format!("Resolving {} Basic Cat Banner overlaps...", list.len()));
    let base_dir = Path::new("game/cats/CatBase");
    if !base_dir.exists() { let _ = fs::create_dir_all(base_dir); }

    for (name, entry) in list {
        let Ok(mut file) = fs::File::open(&entry.pack_path) else { continue; };
        let aligned = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };
        if file.seek(SeekFrom::Start(entry.offset)).is_err() { continue; }
        let mut buf = vec![0u8; aligned];
        if file.read_exact(&mut buf).is_err() { continue; }
        if let Ok((dec, _)) = keys::decrypt_pack_chunk(&buf, &entry.original_name) {
            let is_base = entry.original_name != name; 
            let target = if is_base { base_dir.join(&name) } else { raw.join(&name) };
            if let Some(p) = target.parent() { let _ = fs::create_dir_all(p); }
            let _ = fs::write(target, &dec[..std::cmp::min(entry.size, dec.len())]);
            count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn cleanup_temp_dirs(dirs: &[PathBuf]) {
    for dir in dirs { let _ = fs::remove_dir_all(dir); }
}

fn determine_code(filename: &str, selected_region: &str) -> String {
    if selected_region != "en" { return selected_region.to_string(); }
    for &(code, _) in patterns::APP_LANGUAGES {
        if code == "en" { continue; } 
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
        score = if chars.len() > 1 && chars[0].is_ascii_uppercase() && chars[1].is_ascii_uppercase() { 
            20_000 + (chars[0] as u64) 
        } else { 
            10_000 
        };
    }
    
    if score == 5_000 && (name == "DataLocal" || name == "UpdateLocal" || name.ends_with("Local")) { score = 0; }
    if temp_apk_dirs.iter().any(|dir| path.starts_with(dir)) { score += 500_000_000; }
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
            if path_str == "game/app" { continue; }
            let _ = scan_for_index(&path, index);
            continue;
        }
        
        if let Some(name) = path.file_name() {
            let key = name.to_string_lossy().to_lowercase();
            index.entry(key).or_default().push(path);
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
            continue;
        }
        
        let Some(ext) = path.extension() else { continue; };
        let ext_str = ext.to_string_lossy().to_lowercase();
        
        if ext_str == "list" { 
            list_paths.push(path); 
        } else if ext_str == "apk" || ext_str == "xapk" { 
            apk_paths.push(path); 
        }
    }
    Ok(())
}

fn extract_apk_data(apk_path: &Path, temp_dir: &Path) -> Vec<PathBuf> {
    let mut extracted_lists = Vec::new();
    let Ok(file) = fs::File::open(apk_path) else { return extracted_lists; };
    let Ok(mut archive) = ZipArchive::new(file) else { return extracted_lists; };
    
    for i in 0..archive.len() {
        let Ok(mut file_in_zip) = archive.by_index(i) else { continue; };
        let name = file_in_zip.name().to_string();
        
        if !name.ends_with(".list") && !name.ends_with(".pack") { continue; }
        
        let safe_name = Path::new(&name).file_name().unwrap();
        let out_path = temp_dir.join(safe_name);
        
        if let Ok(mut out_file) = fs::File::create(&out_path) { 
            let _ = std::io::copy(&mut file_in_zip, &mut out_file); 
        }
        
        if name.ends_with(".list") { extracted_lists.push(out_path); }
    }
    extracted_lists
}