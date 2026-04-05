use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, AtomicBool, AtomicUsize, Ordering};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use rayon::prelude::*;
use regex::RegexSet;

use crate::features::data::logic::keys; 
use crate::global::io::patterns;
use crate::features::settings::logic::exceptions::{ExceptionRule, RuleHandling};

use super::{apk, chrono, manifest, rules}; 

#[derive(Clone)]
struct PackEntry {
    pack_path: PathBuf,
    original_name: String,
    offset: u64,
    size: usize,
    is_locked: bool,
}

impl PackEntry {
    fn to_manifest_entry(&self, checksum: u64) -> manifest::ManifestEntry {
        manifest::ManifestEntry {
            pack: self.pack_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            offset: self.offset,
            size: self.size,
            checksum,
        }
    }
}

fn is_potential_conflict(name: &str) -> bool {
    if !name.starts_with("udi") || !name.ends_with(".png") { return false; }
    let stem = Path::new(name).file_stem().unwrap_or_default().to_string_lossy();
    if stem.len() < 6 { return false; }
    let Ok(unit_id) = stem[3..6].parse::<u32>() else { return false; };
    unit_id <= 9
}

pub fn build_index(root_dir: &Path) -> HashSet<String> {
    let mut index = HashSet::new();
    let _ = scan_for_index(root_dir, &mut index);
    index
}

fn scan_for_index(dir: &Path, index: &mut HashSet<String>) -> std::io::Result<()> {
    if !dir.is_dir() { return Ok(()); }
    for entry_result in fs::read_dir(dir)?.flatten() {
        let path = entry_result.path();
        if path.is_dir() {
            let path_string = path.to_string_lossy().replace('\\', "/");
            if path_string == "game/app" || path_string == "game/metadata" || path_string == "game/manifest" { continue; }
            let _ = scan_for_index(&path, index);
            continue;
        }
        if let Some(name) = path.file_name() {
            index.insert(name.to_string_lossy().to_lowercase());
        }
    }
    Ok(())
}

pub fn run(folder_path: &str, region_code: &str, shared_index: &mut HashSet<String>, status_sender: Sender<String>, abort_flag: Arc<AtomicBool>, prog_curr: Arc<AtomicUsize>, prog_max: Arc<AtomicUsize>) -> Result<(), String> {
    if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".into()); }
    let source_dir = Path::new(folder_path);
    let raw_dir = Path::new("game/raw");
    let base_dir = Path::new("game/cats/CatBase");
    let metadata_dir = Path::new("game/metadata");
    
    if !raw_dir.exists() { let _ = fs::create_dir_all(raw_dir); }
    if !metadata_dir.exists() { let _ = fs::create_dir_all(metadata_dir); }

    let manifest_path = metadata_dir.join(format!("{}_manifest.json", region_code));
    let has_manifest = manifest_path.exists();
    let mut current_manifest = manifest::load(&manifest_path);

    let _ = status_sender.send("Scanning for game files...".to_string());
    let mut list_paths = Vec::new();
    let mut apk_paths = Vec::new();
    let _ = apk::find_files(source_dir, &mut list_paths, &mut apk_paths);

    let mut dynamic_temp_dirs = Vec::new();
    apk::extract_all(&apk_paths, &mut list_paths, &mut dynamic_temp_dirs, &status_sender);

    if abort_flag.load(Ordering::Relaxed) { cleanup_temp_dirs(&dynamic_temp_dirs); return Err("Job Aborted".into()); }

    let _ = status_sender.send("Sorting patch history chronologically...".to_string());
    list_paths.sort_by(|path_a, path_b| {
        let score_a = chrono::calculate(path_a, &dynamic_temp_dirs);
        let score_b = chrono::calculate(path_b, &dynamic_temp_dirs);
        score_a.cmp(&score_b).then_with(|| path_a.cmp(path_b))
    });

    let (regex_set, compiled_rules) = rules::compile();
    let mut master_map: HashMap<String, PackEntry> = HashMap::new();
    let mut conflict_map: HashMap<String, PackEntry> = HashMap::new(); 
    
    parse_list_files(&list_paths, region_code, &regex_set, &compiled_rules, &mut master_map, &mut conflict_map);

    if has_manifest && !current_manifest.is_empty() {
        let _ = status_sender.send("Verifying file checksums...".to_string());
    }

    let mut pack_groups: HashMap<PathBuf, Vec<(String, PackEntry)>> = HashMap::new();
    for (name, entry) in master_map {
        pack_groups.entry(entry.pack_path.clone()).or_default().push((name, entry));
    }

    let verified_tasks: Vec<_> = pack_groups.into_par_iter().flat_map(|(pack_path, entries)| {
        if abort_flag.load(Ordering::Relaxed) { return Vec::new(); }
        let mut results = Vec::new();
        if let Ok(mut file) = fs::File::open(&pack_path) {
            let current_pack = pack_path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let mut file_buffer: Vec<u8> = Vec::new(); 
            for (name, entry) in entries {
                let aligned_size = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };
                file_buffer.resize(aligned_size, 0); 
                let calculated_checksum = if file.seek(SeekFrom::Start(entry.offset)).is_ok() && file.read_exact(&mut file_buffer).is_ok() {
                    manifest::hash(&file_buffer)
                } else { 0 };
                
                let is_placeholder = match current_manifest.get(&name) { Some(p) => p.size > entry.size + 32, None => false };
                let is_changed = match current_manifest.get(&name) {
                    Some(p) => if is_placeholder { false } else { p.size != entry.size || p.pack != current_pack || p.checksum != calculated_checksum },
                    None => true,
                };
                let is_missing = !shared_index.contains(&name.to_lowercase());
                if is_changed || (is_missing && !is_placeholder) { results.push((name, entry, calculated_checksum)); }
            }
        } else {
            for (name, entry) in entries { results.push((name, entry, 0)); }
        }
        results
    }).collect();

    if abort_flag.load(Ordering::Relaxed) { cleanup_temp_dirs(&dynamic_temp_dirs); return Err("Job Aborted".into()); }

    let mut conflict_groups: HashMap<PathBuf, Vec<(String, PackEntry)>> = HashMap::new();
    for (name, entry) in conflict_map { conflict_groups.entry(entry.pack_path.clone()).or_default().push((name, entry)); }

    let verified_conflicts: Vec<_> = conflict_groups.into_par_iter().flat_map(|(pack_path, entries)| {
        if abort_flag.load(Ordering::Relaxed) { return Vec::new(); }
        let mut results = Vec::new();
        if let Ok(mut file) = fs::File::open(&pack_path) {
            let current_pack = pack_path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let mut file_buffer: Vec<u8> = Vec::new(); 
            for (name, entry) in entries {
                let aligned_size = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };
                file_buffer.resize(aligned_size, 0); 
                let calculated_checksum = if file.seek(SeekFrom::Start(entry.offset)).is_ok() && file.read_exact(&mut file_buffer).is_ok() {
                    manifest::hash(&file_buffer)
                } else { 0 };
                let is_placeholder = match current_manifest.get(&name) { Some(p) => p.size > entry.size + 32, None => false };
                let is_changed = match current_manifest.get(&name) {
                    Some(p) => if is_placeholder { false } else { p.size != entry.size || p.pack != current_pack || p.checksum != calculated_checksum },
                    None => true,
                };
                let is_missing = !shared_index.contains(&name.to_lowercase());
                if is_changed || (is_missing && !is_placeholder) { results.push((name, entry, calculated_checksum)); }
            }
        } else {
            for (name, entry) in entries { results.push((name, entry, 0)); }
        }
        results
    }).collect();

    if abort_flag.load(Ordering::Relaxed) { cleanup_temp_dirs(&dynamic_temp_dirs); return Err("Job Aborted".into()); }

    let mut filtered_tasks = Vec::new();
    for (name, entry, checksum) in verified_tasks {
        current_manifest.insert(name.clone(), entry.to_manifest_entry(checksum));
        filtered_tasks.push((name, entry));
    }

    let mut filtered_conflicts = Vec::new();
    for (name, entry, checksum) in verified_conflicts {
        current_manifest.insert(name.clone(), entry.to_manifest_entry(checksum));
        filtered_conflicts.push((name, entry));
    }

    let to_extract_count = filtered_tasks.len();
    if to_extract_count == 0 && filtered_conflicts.is_empty() {
        let _ = status_sender.send("Workspace is already up to date.".to_string());
        cleanup_temp_dirs(&dynamic_temp_dirs);
        prog_max.store(0, Ordering::Relaxed);
        return Ok(());
    }

    prog_max.store(to_extract_count + filtered_conflicts.len(), Ordering::Relaxed);
    prog_curr.store(0, Ordering::Relaxed);

    let extracted_count = AtomicI32::new(0);
    if to_extract_count > 0 {
        extract_standard_packs(filtered_tasks.clone(), raw_dir, &extracted_count, to_extract_count, &status_sender, &abort_flag, &prog_curr);
    }

    if !filtered_conflicts.is_empty() && !abort_flag.load(Ordering::Relaxed) {
        resolve_conflicts(filtered_conflicts.clone(), raw_dir, base_dir, &extracted_count, &status_sender, &abort_flag, &prog_curr);
    }

    if abort_flag.load(Ordering::Relaxed) { cleanup_temp_dirs(&dynamic_temp_dirs); return Err("Job Aborted".into()); }

    manifest::save(&manifest_path, &current_manifest);
    cleanup_temp_dirs(&dynamic_temp_dirs);

    for (name, _) in &filtered_tasks { shared_index.insert(name.to_lowercase()); }
    for (name, _) in &filtered_conflicts { shared_index.insert(name.to_lowercase()); }
    
    let _ = status_sender.send(format!("Decryption complete. Extracted {} files.", extracted_count.load(Ordering::Relaxed)));
    prog_max.store(0, Ordering::Relaxed);
    Ok(())
}

fn parse_list_files(
    list_paths: &[PathBuf], region_code: &str, regex_set: &RegexSet, compiled_rules: &[ExceptionRule], 
    master_map: &mut HashMap<String, PackEntry>, conflict_map: &mut HashMap<String, PackEntry>
) {
    for list_path in list_paths {
        let pack_path = list_path.with_extension("pack");
        if !pack_path.exists() { continue; }
        let Ok(list_data) = fs::read(list_path) else { continue; };
        let Ok(content) = decrypt_list_content(&list_data) else { continue; };
        let pack_name = pack_path.file_name().unwrap_or_default().to_string_lossy();
        let current_code = determine_code(&pack_name, region_code);
        for line in content.lines() {
            process_list_line(line, &pack_path, current_code.as_str(), regex_set, compiled_rules, master_map, conflict_map);
        }
    }
}

fn process_list_line(
    line: &str, pack_path: &Path, current_code: &str, regex_set: &RegexSet, compiled_rules: &[ExceptionRule], 
    master_map: &mut HashMap<String, PackEntry>, conflict_map: &mut HashMap<String, PackEntry>
) {
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 3 { return; }
    
    let asset_name = parts[0];
    let offset: u64 = parts[1].parse().unwrap_or(0);
    let size: usize = parts[2].parse().unwrap_or(0);

    let matched_rule = regex_set.matches(asset_name).into_iter().next().map(|index| &compiled_rules[index]);
    let is_rule_active = if let Some(rule) = matched_rule { rule.languages.values().any(|&active| active) } else { false };

    if let Some(rule) = matched_rule { if rule.handling == RuleHandling::Ignore { return; } }

    let entry = PackEntry {
        pack_path: pack_path.to_path_buf(), original_name: asset_name.to_string(), offset, size,
        is_locked: matched_rule.map(|rule| rule.locked).unwrap_or(false),
    };

    if matched_rule.is_none() || !is_rule_active {
        if is_potential_conflict(asset_name) {
            if let Some(existing) = conflict_map.get(asset_name) { if !entry.is_locked && existing.size > entry.size + 32 { return; } }
            conflict_map.insert(asset_name.to_string(), entry); 
        } else {
            if let Some(existing) = master_map.get(asset_name) { if !entry.is_locked && existing.size > entry.size + 32 { return; } }
            master_map.insert(asset_name.to_string(), entry); 
        }
        return;
    }

    let rule = matched_rule.unwrap();
    let path_obj = Path::new(asset_name);
    let stem = path_obj.file_stem().unwrap().to_string_lossy();
    let file_extension = path_obj.extension().unwrap_or_default().to_string_lossy();
    
    let mut clean_stem = stem.to_string();
    for &(code, _) in patterns::APP_LANGUAGES {
        let suffix = format!("_{}", code);
        if clean_stem.ends_with(&suffix) { clean_stem = clean_stem.trim_end_matches(&suffix).to_string(); break; }
    }
    
    let is_enabled = rule.languages.get(current_code).copied().unwrap_or(false);
    if rule.handling == RuleHandling::Only && !is_enabled { return; }
    
    let is_single_lang_only = rule.handling == RuleHandling::Only && rule.languages.values().filter(|&&act| act).count() == 1;
    let mut final_filename = asset_name.to_string();
    
    if is_enabled {
        if is_single_lang_only {
            final_filename = clean_stem;
            if !file_extension.is_empty() { final_filename = format!("{}.{}", final_filename, file_extension); }
        } else if !current_code.is_empty() {
            final_filename = format!("{}_{}", clean_stem, current_code);
            if !file_extension.is_empty() { final_filename = format!("{}.{}", final_filename, file_extension); }
        }
    }

    if is_potential_conflict(asset_name) {
        if let Some(existing) = conflict_map.get(&final_filename) { if !entry.is_locked && existing.size > entry.size + 32 { return; } }
        conflict_map.insert(final_filename, entry);
    } else {
        if let Some(existing) = master_map.get(&final_filename) { if !entry.is_locked && existing.size > entry.size + 32 { return; } }
        master_map.insert(final_filename, entry);
    }
}

fn extract_standard_packs(filtered_tasks: Vec<(String, PackEntry)>, raw_dir: &Path, extracted_count: &AtomicI32, total_files: usize, status_sender: &Sender<String>, abort_flag: &Arc<AtomicBool>, prog_curr: &Arc<AtomicUsize>) {
    let _ = status_sender.send(format!("Found {} new or updated files.", total_files));
    let _ = status_sender.send("Starting extraction...".to_string());
    let update_interval = (total_files / 100).max(10) as i32;

    let mut pack_tasks: HashMap<PathBuf, Vec<(String, PackEntry)>> = HashMap::new();
    for (final_name, entry) in filtered_tasks { pack_tasks.entry(entry.pack_path.clone()).or_default().push((final_name, entry)); }

    pack_tasks.into_par_iter().for_each(|(pack_path, entries)| {
        if abort_flag.load(Ordering::Relaxed) { return; }
        let Ok(mut file) = fs::File::open(&pack_path) else { return; };
        let mut file_buffer: Vec<u8> = Vec::new(); 
        
        for (final_name, entry) in entries {
            if abort_flag.load(Ordering::Relaxed) { return; }
            let target_path = raw_dir.join(&final_name);
            let aligned_size = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };
            if file.seek(SeekFrom::Start(entry.offset)).is_err() { continue; }
            file_buffer.resize(aligned_size, 0);
            if file.read_exact(&mut file_buffer).is_err() { continue; }
            
            let Ok((decrypted_bytes, _)) = keys::decrypt_pack_chunk(&file_buffer, &entry.original_name) else { continue; };
            let final_data = &decrypted_bytes[..std::cmp::min(entry.size, decrypted_bytes.len())];
            
            if let Some(parent_dir) = target_path.parent() { let _ = fs::create_dir_all(parent_dir); }
            let _ = fs::write(&target_path, final_data);
            
            let current_extracted_count = extracted_count.fetch_add(1, Ordering::Relaxed) + 1;
            prog_curr.fetch_add(1, Ordering::Relaxed);

            if current_extracted_count % update_interval == 0 { let _ = status_sender.send(format!("Extracted {} files | Current: {}", current_extracted_count, final_name)); }
        }
    });
}

fn resolve_conflicts(list: Vec<(String, PackEntry)>, raw: &Path, base_dir: &Path, extracted_count: &AtomicI32, status_sender: &Sender<String>, abort_flag: &Arc<AtomicBool>, prog_curr: &Arc<AtomicUsize>) {
    let _ = status_sender.send(format!("Resolving {} Basic Cat Banner overlaps...", list.len()));
    if !base_dir.exists() { let _ = fs::create_dir_all(base_dir); }

    for (name, entry) in list {
        if abort_flag.load(Ordering::Relaxed) { return; }
        let Ok(mut file) = fs::File::open(&entry.pack_path) else { continue; };
        let aligned_size = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };
        if file.seek(SeekFrom::Start(entry.offset)).is_err() { continue; }
        let mut read_buffer = vec![0u8; aligned_size];
        if file.read_exact(&mut read_buffer).is_err() { continue; }
        if let Ok((decrypted_chunk, _)) = keys::decrypt_pack_chunk(&read_buffer, &entry.original_name) {
            let is_base = entry.original_name != name; 
            let target_path = if is_base { base_dir.join(&name) } else { raw.join(&name) };
            if let Some(parent_dir) = target_path.parent() { let _ = fs::create_dir_all(parent_dir); }
            let _ = fs::write(target_path, &decrypted_chunk[..std::cmp::min(entry.size, decrypted_chunk.len())]);
            extracted_count.fetch_add(1, Ordering::Relaxed);
        }
        prog_curr.fetch_add(1, Ordering::Relaxed);
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

fn decrypt_list_content(data: &[u8]) -> Result<String, String> {
    let pack_key = keys::get_md5_key("pack");
    if let Ok(bytes) = keys::decrypt_ecb_with_key(data, &pack_key) {
        if let Ok(decrypted_string) = String::from_utf8(bytes) { return Ok(decrypted_string); }
    }
    let bc_key = keys::get_md5_key("battlecats");
    if let Ok(bytes) = keys::decrypt_ecb_with_key(data, &bc_key) {
        if let Ok(decrypted_string) = String::from_utf8(bytes) { return Ok(decrypted_string); }
    }
    Err("Decryption failed".into())
}