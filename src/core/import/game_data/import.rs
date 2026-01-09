use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, Ordering};
use zip::ZipArchive;
use rayon::prelude::*;

pub fn import_from_folder(path_str: &str, tx: Sender<String>) -> Result<bool, String> {
    let source = Path::new(path_str);
    let game_root = Path::new("game");
    let raw_dir = game_root.join("raw");
    
    let mut detected_smart_root = None;
    
    if source.join("assets").join("img015").exists() {
        detected_smart_root = Some(source.to_path_buf());
    } else if let Ok(entries) = fs::read_dir(source) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("assets").join("img015").exists() {
                detected_smart_root = Some(path);
                break;
            }
        }
    }

    if let Some(smart_root) = detected_smart_root {
        let _ = tx.send("Smart Import: Valid game structure detected.".to_string());
        let _ = tx.send("Restoring files directly...".to_string());

        if !game_root.exists() { 
            fs::create_dir_all(game_root).map_err(|e| e.to_string())?; 
        }

        let mut tasks = Vec::new();
        if let Err(e) = scan_with_relative_paths(&smart_root, &smart_root, &mut tasks) {
            return Err(format!("Scan error: {}", e));
        }

        let count = AtomicI32::new(0);
        tasks.par_iter().for_each(|(abs_path, rel_path)| {
            let dest = game_root.join(rel_path);
            
            if let Some(parent) = dest.parent() {
                if !parent.exists() { let _ = fs::create_dir_all(parent); }
            }

            if fs::copy(abs_path, &dest).is_ok() {
                let c = count.fetch_add(1, Ordering::Relaxed);
                if c % 100 == 0 { 
                    let name = abs_path.file_name().unwrap_or_default().to_string_lossy();
                    let _ = tx.send(format!("Coppied {} files | Current: {}", c, name)); 
                }
            }
        });

        let _ = tx.send(format!("Success! Smart Import restored {} files.", count.load(Ordering::Relaxed)));
        return Ok(false);
    }

    if !raw_dir.exists() { 
        fs::create_dir_all(&raw_dir).map_err(|e| e.to_string())?; 
    }
    
    let _ = tx.send("Standard Scan: No game structure found.".to_string());
    
    let mut tasks = Vec::new();
    if let Err(e) = scan_dir(source, &mut tasks) {
        return Err(format!("Failed to scan folder: {}", e));
    }

    let _ = tx.send(format!("Found {} files. Starting raw import...", tasks.len()));
    let count = AtomicI32::new(0);

    tasks.par_iter().for_each(|path| {
        if let Some(name_os) = path.file_name() {
            let name = name_os.to_string_lossy();
            let dest = raw_dir.join(name.as_ref());
            
            if fs::copy(path, &dest).is_ok() {
                let c = count.fetch_add(1, Ordering::Relaxed);
                if c % 100 == 0 { 
                    let _ = tx.send(format!("Imported {} files | Current: {}", c, name)); 
                }
            }
        }
    });

    Ok(true)
}

pub fn import_from_zip(path_str: &str, tx: Sender<String>) -> Result<bool, String> {
    let f = fs::File::open(path_str).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(f).map_err(|e| e.to_string())?;
    let game_root = Path::new("game");
    let raw_dir = game_root.join("raw");
    
    let mut smart_prefix = None;
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            if let Some(idx) = file.name().find("assets/img015") {
                smart_prefix = Some(file.name()[..idx].to_string());
                break;
            }
        }
    }

    if let Some(prefix) = smart_prefix {
        let _ = tx.send("Smart Import: Valid backup detected in ZIP.".to_string());
        
        let total = archive.len();
        let mut extracted = 0;

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
        return Ok(false);
    }

    if !raw_dir.exists() { 
        fs::create_dir_all(&raw_dir).map_err(|e| e.to_string())?; 
    }
    
    let total = archive.len();
    let _ = tx.send(format!("Extracting {} files to raw...", total));

    for i in 0..total {
        let mut file = archive.by_index(i).unwrap();
        if file.is_dir() { continue; }
        
        let name = Path::new(file.name()).file_name().unwrap_or_default().to_string_lossy().to_string();
        let dest = raw_dir.join(&name);
        
        if let Ok(mut out) = fs::File::create(&dest) {
            let _ = std::io::copy(&mut file, &mut out);
        }
        
        if i % 100 == 0 { 
            let _ = tx.send(format!("Extracted {} files | Current: {}", i, name)); 
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