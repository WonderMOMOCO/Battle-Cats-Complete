use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use rayon::prelude::*;
use zip::ZipArchive;

use crate::features::data::sort::{cat, global, enemy};
use crate::features::cat::patterns as cat_patterns;
use crate::global::io::patterns as global_patterns;

const META_DIRS: &[&str] = &["raw", "app", "metadata"];

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
        if self.global_matcher.get_dest(&base_name, dummy_dir, dummy_dir, dummy_dir).is_some() { return true; }
        if self.cat_matcher.get_dest(&base_name, dummy_dir).is_some() { return true; }
        if self.enemy_matcher.get_dest(&base_name, dummy_dir).is_some() { return true; }

        false
    }
}

pub fn import_standard_folder(path_str: &str, tx: Sender<String>, abort_flag: Arc<AtomicBool>, prog_curr: Arc<AtomicUsize>, prog_max: Arc<AtomicUsize>) -> Result<bool, String> {
    prog_curr.store(0, Ordering::Relaxed);
    prog_max.store(0, Ordering::Relaxed);
    
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
    
    let mut is_restructure = false;
    
    // Check if targeting the "game" folder itself, or an imported folder matching the database structure
    if let (Ok(s), Ok(g)) = (source.canonicalize(), game_root.canonicalize()) {
        if s == g {
            is_restructure = true;
        }
    }

    if !is_restructure {
        if let Ok(game_entries) = fs::read_dir(&game_root) {
            let game_dirs: Vec<_> = game_entries.flatten()
                .filter(|e| e.file_type().map(|f| f.is_dir()).unwrap_or(false))
                .map(|e| e.file_name())
                .collect();

            if let Ok(src_entries) = fs::read_dir(&source) {
                let src_dirs: Vec<_> = src_entries.flatten()
                    .filter(|e| e.file_type().map(|f| f.is_dir()).unwrap_or(false))
                    .map(|e| e.file_name())
                    .collect();

                // ONLY trigger restructure if there are explicitly matching database folders
                let has_overlap = src_dirs.iter().any(|d| game_dirs.contains(d));
                
                if !src_dirs.is_empty() && has_overlap {
                    is_restructure = true;
                }
            }
        }
    }

    // Path for Restructure Import
    if is_restructure {
        let _ = tx.send("Beginning database restructure...".to_string());
        let _ = tx.send("Scanning directories...".to_string());
        
        let mut all_files = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&source) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                    if !META_DIRS.contains(&dir_name.as_str()) {
                        let _ = scan_dir(&path, &mut all_files);
                    }
                } else {
                    all_files.push(path);
                }
            }
        }
        
        let _ = tx.send("Directories scanned.".to_string());
         let _ = tx.send("Filtering file paths...".to_string());

        let raw_canon = raw_dir.canonicalize().unwrap_or_else(|_| raw_dir.to_path_buf());
        let files_to_move: Vec<PathBuf> = all_files.into_par_iter().filter(|p| {
            let p_canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            !p_canon.starts_with(&raw_canon)
        }).collect();

        if files_to_move.is_empty() {
            let _ = tx.send("No valid files to move.".to_string());
            let _ = tx.send("Restructure complete.".to_string());
            return Ok(true);
        }

        let _ = tx.send(format!("Flattening {} files to raw directory...", files_to_move.len()));

        prog_max.store(files_to_move.len(), Ordering::Relaxed);
        let update_interval = (files_to_move.len() / 100).max(10);
        let count = AtomicI32::new(0);

        files_to_move.par_iter().for_each(|path| {
            if abort_flag.load(Ordering::Relaxed) { return; }
            
            if let Some(file_name) = path.file_name() {
                let dest = raw_dir.join(file_name);
                
                // Move file to raw if it doesn't exist or sizes differ
                if !dest.exists() || fs::metadata(path).map(|m| m.len()).unwrap_or(0) != fs::metadata(&dest).map(|m| m.len()).unwrap_or(0) {
                    if fs::rename(path, &dest).is_err() {
                        let _ = fs::copy(path, &dest);
                        let _ = fs::remove_file(path);
                    }
                } else {
                    // It's already in raw with the exact same size, so just clean up the duplicate source file
                    let _ = fs::remove_file(path);
                }
            }

            let c = count.fetch_add(1, Ordering::Relaxed) + 1;
            prog_curr.store(c as usize, Ordering::Relaxed);
            
            if c as usize % update_interval == 0 {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let _ = tx.send(format!("Moved {} files to raw | Current: {}", c, name));
            }
        });
        
        // Clean up empty directories from the targeted source
        if let Ok(entries) = fs::read_dir(&source) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                    if !META_DIRS.contains(&dir_name.as_str()) {
                        remove_empty_directories(&path);
                    }
                }
            }
        }

        let _ = tx.send("Flattening complete.".to_string());
        let _ = tx.send("Sorter will now rebuild the database.".to_string());
        return Ok(true);
    }
    
    // Standard Raw Import
    let mut tasks = Vec::new();
    scan_dir(source, &mut tasks).map_err(|e| e.to_string())?;
    if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".to_string()); }

    let _ = tx.send("Indexing existing workspace files...".to_string());
    let shared_index = Arc::new(build_index(game_root));
    let validator = FileValidator::new();

    let filtered_tasks: Vec<PathBuf> = tasks.into_par_iter().filter(|path| {
        if abort_flag.load(Ordering::Relaxed) { return false; }
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

    if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".to_string()); }

    if filtered_tasks.is_empty() {
        let _ = tx.send("Workspace is already up to date.".to_string());
        return Ok(false);
    }

    prog_max.store(filtered_tasks.len(), Ordering::Relaxed);

    let update_interval = (filtered_tasks.len() / 100).max(10) as i32;
    let _ = tx.send(format!("Found {} new files. Importing...", filtered_tasks.len()));
    let count = AtomicI32::new(0);
    
    filtered_tasks.par_iter().for_each(|path| {
        if abort_flag.load(Ordering::Relaxed) { return; }
        let name = path.file_name().unwrap().to_string_lossy();
        if fs::copy(path, raw_dir.join(name.as_ref())).is_ok() {
            let c = count.fetch_add(1, Ordering::Relaxed) + 1;
            prog_curr.fetch_add(1, Ordering::Relaxed);
            if c % update_interval == 0 { let _ = tx.send(format!("Imported {} files...", c)); }
        }
    });

    if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".to_string()); }
    Ok(true)
}

pub fn import_standard_archive(path_str: &str, tx: Sender<String>, abort_flag: Arc<AtomicBool>, prog_curr: Arc<AtomicUsize>, prog_max: Arc<AtomicUsize>) -> Result<bool, String> {
    if path_str.to_lowercase().ends_with(".zip") {
        import_legacy_zip(path_str, tx, abort_flag, prog_curr, prog_max)
    } else {
        import_tar_zst(path_str, tx, abort_flag, prog_curr, prog_max)
    }
}

fn import_tar_zst(path_str: &str, tx: Sender<String>, abort_flag: Arc<AtomicBool>, prog_curr: Arc<AtomicUsize>, prog_max: Arc<AtomicUsize>) -> Result<bool, String> {
    prog_curr.store(0, Ordering::Relaxed);
    prog_max.store(0, Ordering::Relaxed);

    let game_root = Path::new("game");
    let raw_dir = game_root.join("raw");
    let mut extracted = 0;

    let _ = tx.send("Scanning archive...".to_string());
    
    let mut total_entries = 0;
    {
        let file = fs::File::open(path_str).map_err(|e| e.to_string())?;
        let decoder = zstd::stream::read::Decoder::new(file).map_err(|e| e.to_string())?;
        let mut archive = tar::Archive::new(decoder);
        for entry in archive.entries().map_err(|e| e.to_string())?.flatten() {
            if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".to_string()); }
            
            if !entry.header().entry_type().is_dir() {
                total_entries += 1;
            }
        }
    }
    
    prog_max.store(total_entries, Ordering::Relaxed);
    let update_interval = (total_entries / 100).max(10);

    // Re-open for actual extraction
    let file = fs::File::open(path_str).map_err(|e| e.to_string())?;
    let decoder = zstd::stream::read::Decoder::new(file).map_err(|e| e.to_string())?;
    let mut archive = tar::Archive::new(decoder);

    // Standard raw archive path
    let _ = tx.send("Indexing existing workspace...".to_string());
    let shared_index = build_index(game_root);
    let validator = FileValidator::new();

    for entry_res in archive.entries().map_err(|e| e.to_string())? {
        if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".to_string()); }

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
                prog_curr.store(extracted, Ordering::Relaxed);
                if extracted % update_interval == 0 { let _ = tx.send(format!("Extracted {} files...", extracted)); }
            }
        }
    }

    if extracted == 0 { 
        let _ = tx.send("Workspace up to date.".to_string()); 
        return Ok(false);
    }
    
    Ok(true)
}

fn import_legacy_zip(path_str: &str, tx: Sender<String>, abort_flag: Arc<AtomicBool>, prog_curr: Arc<AtomicUsize>, prog_max: Arc<AtomicUsize>) -> Result<bool, String> {
    prog_curr.store(0, Ordering::Relaxed);
    
    let f = fs::File::open(path_str).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(f).map_err(|e| e.to_string())?;
    
    prog_max.store(archive.len(), Ordering::Relaxed);

    let game_root = Path::new("game");
    let raw_dir = game_root.join("raw");
    let mut extracted = 0;

    // STANDARD RAW IMPORT PATH
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
        if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".to_string()); }

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
        return Ok(false);
    }
    
    // Perform extraction
    prog_max.store(indices_to_extract.len(), Ordering::Relaxed);
    prog_curr.store(0, Ordering::Relaxed);

    let update_interval = (indices_to_extract.len() / 100).max(10);
    let _ = tx.send(format!("Found {} new or updated files.", indices_to_extract.len()));
    let _ = tx.send(format!("Starting extraction..."));

    for i in indices_to_extract {
        if abort_flag.load(Ordering::Relaxed) { return Err("Job Aborted".to_string()); }

        let mut file = archive.by_index(i).unwrap();
        let name = Path::new(file.name()).file_name().unwrap_or_default().to_string_lossy().to_string();
        let dest = raw_dir.join(&name);
        
        if let Ok(mut out) = fs::File::create(&dest) {
            let _ = std::io::copy(&mut file, &mut out);
            
            extracted += 1;
            prog_curr.store(extracted, Ordering::Relaxed);

            if extracted % update_interval == 0 { 
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
            let _ = scan_for_index(&path, index);
        } else if let Some(name) = path.file_name() {
            let key = name.to_string_lossy().to_lowercase();
            index.entry(key).or_insert_with(Vec::new).push(path);
        }
    }
    Ok(())
}

fn remove_empty_directories(dir: &Path) {
    if !dir.is_dir() { return; }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                remove_empty_directories(&path);
            }
        }
    }
    // Attempt removal; this safely fails natively if the folder isn't completely empty.
    let _ = fs::remove_dir(dir);
}