use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;
use crate::features::data::utilities::crypto; 

struct PackEntry {
    name: String,
    offset: usize,
    size: usize,
}

pub fn run(pack_dir: &Path, tx: Sender<String>) -> Result<(), String> {
    let list_path = pack_dir.join("DownloadLocal.list");
    let pack_path = pack_dir.join("DownloadLocal.pack");

    if !list_path.exists() || !pack_path.exists() {
        return Err("DownloadLocal.list or .pack missing from target folder".to_string());
    }

    let mods_root = Path::new("mods");
    let mut mod_num = 1;
    while mods_root.join(format!("NewMod{}", mod_num)).exists() {
        mod_num += 1;
    }
    
    let target_dir = mods_root.join(format!("NewMod{}", mod_num));
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let _ = tx.send("Extracting to new mod workspace...".to_string());

    let list_data = fs::read(&list_path).map_err(|e| e.to_string())?;
    let content = decrypt_list_content(&list_data)?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }
        
        entries.push(PackEntry {
            name: parts[0].to_string(),
            offset: parts[1].parse().unwrap_or(0),
            size: parts[2].parse().unwrap_or(0),
        });
    }

    let pack_data = fs::read(&pack_path).map_err(|e| e.to_string())?;
    let extracted_count = AtomicUsize::new(0);

    entries.into_par_iter().for_each(|entry| {
        let aligned_size = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };
        
        if entry.offset + aligned_size <= pack_data.len() {
            let chunk = &pack_data[entry.offset .. entry.offset + aligned_size];
            if let Ok((decrypted_bytes, _)) = crypto::decrypt_pack_chunk(chunk, &entry.name) {
                let final_data = &decrypted_bytes[..std::cmp::min(entry.size, decrypted_bytes.len())];
                let out_file = target_dir.join(&entry.name);
                
                if let Some(parent) = out_file.parent() { let _ = fs::create_dir_all(parent); }
                let _ = fs::write(out_file, final_data);
                
                extracted_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    let mut final_name = format!("NewMod{}", mod_num);
    let meta_path = target_dir.join("metadata.json");
    if meta_path.exists() {
        let meta = crate::features::mods::logic::metadata::ModMetadata::load(&target_dir);
        let safe_title = meta.title.replace(&['<', '>', ':', '"', '/', '\\', '|', '?', '*'][..], "").trim().to_string();
        
        if !safe_title.is_empty() {
            let mut attempt = safe_title.clone();
            let mut counter = 1;
            let mut new_path = mods_root.join(&attempt);
            
            if new_path != target_dir {
                while new_path.exists() {
                    attempt = format!("{}{}", safe_title, counter);
                    new_path = mods_root.join(&attempt);
                    counter += 1;
                }
                if std::fs::rename(&target_dir, &new_path).is_ok() {
                    final_name = attempt;
                }
            }
        }
    }

    let final_count = extracted_count.load(Ordering::Relaxed);
    let _ = tx.send(format!("Decryption complete! Extracted {} files. Saved as '{}'.", final_count, final_name));
    Ok(())
}

fn decrypt_list_content(data: &[u8]) -> Result<String, String> {
    let pack_key = crypto::get_md5_key("pack");
    if let Ok(bytes) = crypto::decrypt_ecb_with_key(data, &pack_key) {
        if let Ok(s) = String::from_utf8(bytes) { return Ok(s); }
    }
    
    let bc_key = crypto::get_md5_key("battlecats");
    if let Ok(bytes) = crypto::decrypt_ecb_with_key(data, &bc_key) {
        if let Ok(s) = String::from_utf8(bytes) { return Ok(s); }
    }
    
    Err("List decryption failed (Invalid keys or corrupted file)".into())
}