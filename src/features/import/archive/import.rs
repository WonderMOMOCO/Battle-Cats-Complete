use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use rayon::prelude::*;
use zip::ZipArchive;

use crate::features::import::sort::{cat, global, enemy};
use crate::features::cat::patterns as cat_patterns;
use crate::global::io::patterns as global_patterns;
struct FileValidator {
    global_matcher: global::GlobalMatcher,
    cat_matcher: cat::CatMatcher,
    enemy_matcher: enemy::EnemyMatcher,
}

impl FileValidator {
    fn new() -> Self {
        Self {
            global_matcher: global::GlobalMatcher::new(),
            cat_matcher: cat::CatMatcher::new(),
            enemy_matcher: enemy::EnemyMatcher::new(),
        }
    }

    fn is_valid(&self, filename: &str) -> bool {
        let path = Path::new(filename);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension().unwrap_or_default().to_string_lossy().to_string();
        
        let mut base_name = filename.to_string();
        for &(code, _) in global_patterns::APP_LANGUAGES {
            let suffix = format!("_{}", code);
            if stem.len() > suffix.len() && stem.ends_with(&suffix) {
                let clean_stem = &stem[..stem.len() - suffix.len()];
                base_name = if ext.is_empty() { 
                    clean_stem.to_string() 
                } else { 
                    format!("{}.{}", clean_stem, ext) 
                };
                break;
            }
        }

        if cat_patterns::CAT_UNIVERSAL_FILES.contains(&base_name.as_str()) {
            return true;
        }

        let dummy_dir = Path::new("");
        if self.global_matcher.get_dest(&base_name, dummy_dir).is_some() { return true; }
        if self.cat_matcher.get_dest(&base_name, dummy_dir).is_some() { return true; }
        if self.enemy_matcher.get_dest(&base_name, dummy_dir).is_some() { return true; }

        false
    }
}

pub fn import_standard_folder(path_str: &str, tx: Sender<String>) -> Result<bool, String> {
    let source = Path::new(path_str);
    let game_root = Path::new("game");
    let raw_dir = game_root.join("raw");

    if !raw_dir.exists() { 
        fs::create_dir_all(&raw_dir).map_err(|e| e.to_string())?; 
    }

    // Direct game/raw targeting
    if let (Ok(s), Ok(r)) = (source.canonicalize(), raw_dir.canonicalize()) {
        if s == r {
            let _ = tx.send("Targeted game/raw directly.\nBypassing indexer to run Sorter...".to_string());
            
            return Ok(true); 
        }
    }
    
    // Smart Root Detection
    let mut smart_root = None;
    if source.join("assets").join("img015").exists() {
        smart_root = Some(source.to_path_buf());
    } else if let Ok(entries) = fs::read_dir(source) {
        smart_root = entries.flatten()
            .map(|e| e.path())
            .find(|p| p.is_dir() && p.join("assets").join("img015").exists());
    }

    // Path for Smart Import (Existing backups)
    if let Some(root) = smart_root {
        let _ = tx.send("Smart Import: Valid game structure detected.".to_string());
        let mut tasks = Vec::new();
        scan_with_relative_paths(&root, &root, &mut tasks).map_err(|e| e.to_string())?;

        let count = AtomicI32::new(0);
        tasks.par_iter().for_each(|(abs_path, rel_path)| {
            let dest = game_root.join(rel_path);
            if let Some(p) = dest.parent() { if !p.exists() { let _ = fs::create_dir_all(p); } }
            if fs::copy(abs_path, &dest).is_ok() {
                let c = count.fetch_add(1, Ordering::Relaxed) + 1;
                if c % 100 == 0 { let _ = tx.send(format!("Restored {} files...", c)); }
            }
        });
        
        return Ok(true);
    }
    
    // Standard Raw Import
    let mut tasks = Vec::new();
    scan_dir(source, &mut tasks).map_err(|e| e.to_string())?;

    let _ = tx.send("Indexing existing workspace files...".to_string());
    let shared_index = Arc::new(build_index(game_root));
    let validator = FileValidator::new();

    let filtered_tasks: Vec<PathBuf> = tasks.into_par_iter().filter(|path| {
        let Some(name_os) = path.file_name() else { return false; };
        let name = name_os.to_string_lossy();
        if !validator.is_valid(&name) { return false; }

        let name_lower = name.to_lowercase();
        let src_len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        if let Some(existing_paths) = shared_index.get(&name_lower) {
            for ep in existing_paths {
                if fs::metadata(ep).map(|m| m.len()).unwrap_or(0) == src_len { return false; }
            }
        }
        
        let dest = raw_dir.join(name.as_ref());
        if dest.exists() && fs::metadata(&dest).map(|m| m.len()).unwrap_or(0) == src_len { return false; }

        true
    }).collect();

    if filtered_tasks.is_empty() {
        let _ = tx.send("Workspace is already up to date.".to_string());
        
        return Ok(true);
    }

    let _ = tx.send(format!("Found {} new files. Importing...", filtered_tasks.len()));
    let count = AtomicI32::new(0);
    filtered_tasks.par_iter().for_each(|path| {
        let name = path.file_name().unwrap().to_string_lossy();
        if fs::copy(path, raw_dir.join(name.as_ref())).is_ok() {
            let c = count.fetch_add(1, Ordering::Relaxed) + 1;
            if c % 100 == 0 { let _ = tx.send(format!("Imported {} files...", c)); }
        }
    });

    
    Ok(true)
}

pub fn import_standard_archive(path_str: &str, tx: Sender<String>) -> Result<bool, String> {
    if path_str.to_lowercase().ends_with(".zip") {
        import_legacy_zip(path_str, tx)
    } else {
        import_tar_zst(path_str, tx)
    }
}

fn import_tar_zst(path_str: &str, tx: Sender<String>) -> Result<bool, String> {
    let game_root = Path::new("game");
    let raw_dir = game_root.join("raw");
    let mut extracted = 0;

    let _ = tx.send("Scanning archive...".to_string());
    
    let mut smart_prefix = None;
    {
        let file = fs::File::open(path_str).map_err(|e| e.to_string())?;
        let decoder = zstd::stream::read::Decoder::new(file).map_err(|e| e.to_string())?;
        let mut archive = tar::Archive::new(decoder);
        for entry in archive.entries().map_err(|e| e.to_string())?.flatten() {
            let path = entry.path().map_err(|e| e.to_string())?;
            let p_str = path.to_string_lossy();
            if let Some(idx) = p_str.find("assets/img015") {
                smart_prefix = Some(p_str[..idx].to_string());
                break; 
            }
        }
    }

    // Re-open for actual extraction
    let file = fs::File::open(path_str).map_err(|e| e.to_string())?;
    let decoder = zstd::stream::read::Decoder::new(file).map_err(|e| e.to_string())?;
    let mut archive = tar::Archive::new(decoder);

    if let Some(prefix) = smart_prefix {
        let _ = tx.send("Smart Import: Restoring backup...".to_string());
        for entry_res in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry_res.map_err(|e| e.to_string())?;
            if entry.header().entry_type().is_dir() { continue; }
            let path = entry.path().map_err(|e| e.to_string())?;
            let name = path.to_string_lossy().to_string();
            if !name.starts_with(&prefix) { continue; }

            let rel = name[prefix.len()..].trim_start_matches(|c| c == '/' || c == '\\');
            let dest = game_root.join(rel);
            if let Some(p) = dest.parent() { if !p.exists() { let _ = fs::create_dir_all(p); } }
            if entry.unpack(&dest).is_ok() {
                extracted += 1;
                if extracted % 100 == 0 { let _ = tx.send(format!("Extracted {} files...", extracted)); }
            }
        }
        
        return Ok(true);
    }

    // Standard raw archive path
    let _ = tx.send("Indexing existing workspace...".to_string());
    let shared_index = build_index(game_root);
    let validator = FileValidator::new();

    for entry_res in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry_res.map_err(|e| e.to_string())?;
        if entry.header().entry_type().is_dir() { continue; }
        let path = entry.path().map_err(|e| e.to_string())?;
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !validator.is_valid(&name) { continue; }
        
        let src_len = entry.size();
        let mut skip = false;
        if let Some(eps) = shared_index.get(&name.to_lowercase()) {
            for ep in eps { if fs::metadata(ep).map(|m| m.len()).unwrap_or(0) == src_len { skip = true; break; } }
        }

        let dest = raw_dir.join(&name);
        if !skip && dest.exists() && fs::metadata(&dest).map(|m| m.len()).unwrap_or(0) == src_len { skip = true; }

        if !skip {
            if !raw_dir.exists() { let _ = fs::create_dir_all(&raw_dir); }
            if entry.unpack(&dest).is_ok() {
                extracted += 1;
                if extracted % 100 == 0 { let _ = tx.send(format!("Extracted {} files...", extracted)); }
            }
        }
    }

    if extracted == 0 { let _ = tx.send("Workspace up to date.".to_string()); }
    
    Ok(true)
}

fn import_legacy_zip(path_str: &str, tx: Sender<String>) -> Result<bool, String> {
    let f = fs::File::open(path_str).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(f).map_err(|e| e.to_string())?;
    let game_root = Path::new("game");
    let raw_dir = game_root.join("raw");
    let mut extracted = 0;

    // --- SMART PREFIX DETECTION ---
    let mut smart_prefix = None;
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            if let Some(idx) = file.name().find("assets/img015") {
                smart_prefix = Some(file.name()[..idx].to_string());
                break;
            }
        }
    }

    // --- SMART IMPORT PATH (Backups) ---
    if let Some(prefix) = smart_prefix {
        let _ = tx.send("Smart Import: Valid backup detected in ZIP.".to_string());
        
        let total = archive.len();
        for i in 0..total {
            let mut file = archive.by_index(i).unwrap();
            if file.is_dir() { continue; }
            
            let name = file.name().to_string();
            if !name.starts_with(&prefix) { continue; }

            let rel_name = &name[prefix.len()..];
            if rel_name.is_empty() { continue; }

            let dest = game_root.join(rel_name);
            
            if let Some(parent) = dest.parent() {
                if !parent.exists() { let _ = fs::create_dir_all(parent); }
            }

            if let Ok(mut out) = fs::File::create(&dest) {
                let _ = std::io::copy(&mut file, &mut out);
                extracted += 1;
                
                if extracted % 100 == 0 {
                    let simple_name = Path::new(&name).file_name().unwrap_or_default().to_string_lossy();
                    let _ = tx.send(format!("Extracted {} files | Current: {}", extracted, simple_name));
                }
            }
        }
        let _ = tx.send("Success! Smart Import complete.".to_string());
        
        return Ok(true);
    }

    // --- STANDARD RAW IMPORT PATH ---
    if !raw_dir.exists() { 
        fs::create_dir_all(&raw_dir).map_err(|e| e.to_string())?; 
    }
    
    let _ = tx.send("Indexing existing workspace files...".to_string());
    let shared_index = build_index(game_root);
    let validator = FileValidator::new();

    let total = archive.len();
    let mut indices_to_extract = Vec::new();
    
    // Identify new/updated files
    for i in 0..total {
        let file = archive.by_index(i).unwrap();
        if file.is_dir() { continue; }
        
        let name = Path::new(file.name()).file_name().unwrap_or_default().to_string_lossy().to_string();
        if !validator.is_valid(&name) { continue; }
        
        let name_lower = name.to_lowercase();
        let src_len = file.size();

        let mut should_extract = true;
        
        // Check workspace index
        if let Some(existing_paths) = shared_index.get(&name_lower) {
            for existing_path in existing_paths {
                if fs::metadata(existing_path).map(|m| m.len()).unwrap_or(0) == src_len {
                    should_extract = false;
                    break;
                }
            }
        }

        // Check raw folder
        let dest = raw_dir.join(&name);
        if should_extract && dest.exists() && fs::metadata(&dest).map(|m| m.len()).unwrap_or(0) == src_len {
            should_extract = false;
        }
        
        if should_extract {
            indices_to_extract.push(i);
        }
    }
    
    // Handle "Up to Date" case
    if indices_to_extract.is_empty() {
        let _ = tx.send("Workspace is already up to date.\nNo new files to extract.".to_string());
        
        return Ok(true);
    }
    
    // Perform extraction
    let _ = tx.send(format!("Found {} new or updated files.\nStarting extraction...", indices_to_extract.len()));

    for i in indices_to_extract {
        let mut file = archive.by_index(i).unwrap();
        let name = Path::new(file.name()).file_name().unwrap_or_default().to_string_lossy().to_string();
        let dest = raw_dir.join(&name);
        
        if let Ok(mut out) = fs::File::create(&dest) {
            let _ = std::io::copy(&mut file, &mut out);
            
            extracted += 1;
            if extracted % 100 == 0 { 
                let _ = tx.send(format!("Extracted {} files | Current: {}", extracted, name)); 
            }
        }
    }
    
    
    Ok(true)
}

fn scan_dir(dir: &Path, list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, list)?;
            } else {
                list.push(path);
            }
        }
    }
    Ok(())
}

fn scan_with_relative_paths(root: &Path, current: &Path, list: &mut Vec<(PathBuf, PathBuf)>) -> std::io::Result<()> {
    if current.is_dir() {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                scan_with_relative_paths(root, &path, list)?;
            } else {
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                list.push((path, relative));
            }
        }
    }
    Ok(())
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
            if path_str == "game/app" || path_str == "game/raw" {
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