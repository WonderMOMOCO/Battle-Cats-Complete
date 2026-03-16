use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use zip::ZipArchive;
use super::decrypt;

pub fn run_archive(archive_path: &Path, target_dir: &Path, tx: Sender<String>) -> Result<(), String> {
    let _ = tx.send("Opening archive...".to_string());
    
    let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    
    let mut best_list_idx = None;
    let mut best_pack_idx = None;
    
    let mut exact_list_found = false;
    let mut exact_pack_found = false;

    for i in 0..archive.len() {
        let file_in_zip = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file_in_zip.name().to_string();
        
        if name.ends_with("DownloadLocal.list") {
            if name == "assets/DownloadLocal.list" {
                best_list_idx = Some(i);
                exact_list_found = true;
            } else if !exact_list_found {
                best_list_idx = Some(i);
            }
        } else if name.ends_with("DownloadLocal.pack") {
            if name == "assets/DownloadLocal.pack" {
                best_pack_idx = Some(i);
                exact_pack_found = true;
            } else if !exact_pack_found {
                best_pack_idx = Some(i);
            }
        }
    }

    if let (Some(list_idx), Some(pack_idx)) = (best_list_idx, best_pack_idx) {
        if !target_dir.exists() { fs::create_dir_all(target_dir).map_err(|e| e.to_string())?; }

        {
            let mut list_file_zip = archive.by_index(list_idx).map_err(|e| e.to_string())?;
            let _ = tx.send(format!("Extracting {}...", list_file_zip.name()));
            
            let out_path = target_dir.join("DownloadLocal.list");
            let mut out_file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut list_file_zip, &mut out_file).map_err(|e| e.to_string())?;
        }

        {
            let mut pack_file_zip = archive.by_index(pack_idx).map_err(|e| e.to_string())?;
            let _ = tx.send(format!("Extracting {}...", pack_file_zip.name()));
            
            let out_path = target_dir.join("DownloadLocal.pack");
            let mut out_file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut pack_file_zip, &mut out_file).map_err(|e| e.to_string())?;
        }

        let _ = tx.send("Found required files. Starting Decryption...".to_string());
        
        decrypt::run(target_dir, tx)
    } else {
        Err("Could not find both DownloadLocal.list and .pack anywhere in the archive.".to_string())
    }
}