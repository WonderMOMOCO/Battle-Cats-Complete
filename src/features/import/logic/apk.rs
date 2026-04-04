use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use zip::ZipArchive;

pub fn find_files(search_dir: &Path, list_paths: &mut Vec<PathBuf>, apk_paths: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !search_dir.is_dir() { return Ok(()); }
    
    for entry_result in fs::read_dir(search_dir)?.flatten() {
        let path = entry_result.path();
        
        if path.is_dir() {
            find_files(&path, list_paths, apk_paths)?;
            continue;
        }
        
        let Some(file_extension) = path.extension() else { continue; };
        let extension_string = file_extension.to_string_lossy().to_lowercase();
        
        if extension_string == "list" { 
            list_paths.push(path); 
        } else if extension_string == "apk" || extension_string == "xapk" { 
            apk_paths.push(path); 
        }
    }
    Ok(())
}

pub fn extract_all(apk_paths: &[PathBuf], list_paths: &mut Vec<PathBuf>, temp_dirs: &mut Vec<PathBuf>, status_sender: &Sender<String>) {
    if apk_paths.is_empty() { return; }
    
    let _ = status_sender.send("Extracting base data from APK...".to_string());
    for apk in apk_paths {
        let parent_directory = apk.parent().unwrap_or(Path::new(""));
        let stem = apk.file_stem().unwrap_or_default().to_string_lossy();
        let apk_temp_dir = parent_directory.join(stem.to_string());
        
        if !apk_temp_dir.exists() { let _ = fs::create_dir_all(&apk_temp_dir); }
        
        let mut extracted = extract_data(apk, &apk_temp_dir);
        list_paths.append(&mut extracted);
        temp_dirs.push(apk_temp_dir);
    }
}

fn extract_data(apk_path: &Path, temp_dir: &Path) -> Vec<PathBuf> {
    let mut extracted_lists = Vec::new();
    let Ok(file) = fs::File::open(apk_path) else { return extracted_lists; };
    let Ok(mut archive) = ZipArchive::new(file) else { return extracted_lists; };
    
    for index in 0..archive.len() {
        let Ok(mut file_in_zip) = archive.by_index(index) else { continue; };
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