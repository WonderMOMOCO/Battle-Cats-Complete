use std::fs;
use std::path::{Path, PathBuf};
use rayon::prelude::*;
use zip::ZipArchive;

pub fn find_files(
    search_directory: &Path, 
    list_paths: &mut Vec<PathBuf>, 
    apk_paths: &mut Vec<PathBuf>,
    audio_paths: &mut Vec<PathBuf>
) -> std::io::Result<()> {
    if !search_directory.is_dir() { return Ok(()); }
    let directory_entries = fs::read_dir(search_directory)?;
    
    for entry_result in directory_entries.flatten() {
        let item_path = entry_result.path();
        if item_path.is_dir() {
            find_files(&item_path, list_paths, apk_paths, audio_paths)?;
            continue;
        }
        
        let Some(file_extension) = item_path.extension() else { continue; };
        let extension_string = file_extension.to_string_lossy().to_lowercase();
        
        if extension_string == "list" { 
            list_paths.push(item_path); 
        } else if extension_string == "apk" || extension_string == "xapk" { 
            apk_paths.push(item_path); 
        } else if extension_string == "caf" || extension_string == "ogg" { 
            audio_paths.push(item_path); 
        }
    }
    Ok(())
}

pub fn extract_all(apk_paths: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>) {
    if apk_paths.is_empty() { return (Vec::new(), Vec::new()); }
    
    let parallel_results: Vec<(Vec<PathBuf>, PathBuf)> = apk_paths.par_iter().filter_map(|apk_file_path| {
        let parent_directory = apk_file_path.parent().unwrap_or(Path::new(""));
        let apk_stem_name = apk_file_path.file_stem().unwrap_or_default().to_string_lossy();
        let extraction_directory = parent_directory.join(apk_stem_name.to_string());
        
        if !extraction_directory.exists() { 
            let _ = fs::create_dir_all(&extraction_directory); 
        }
        
        let mut extracted_lists = Vec::new();
        if let Ok(input_zip) = fs::File::open(apk_file_path) {
            if let Ok(mut archive) = ZipArchive::new(input_zip) {
                for index in 0..archive.len() {
                    let Ok(mut file) = archive.by_index(index) else { continue; };
                    let file_name = file.name().to_string();
                    
                    if file_name.ends_with(".list") || file_name.ends_with(".pack") {
                        let safe_name = Path::new(&file_name).file_name().unwrap();
                        let destination = extraction_directory.join(safe_name);
                        if let Ok(mut output) = fs::File::create(&destination) { 
                            let _ = std::io::copy(&mut file, &mut output); 
                        }
                        if file_name.ends_with(".list") { 
                            extracted_lists.push(destination); 
                        }
                    }
                }
            }
        }
        Some((extracted_lists, extraction_directory))
    }).collect();

    let mut final_list_paths = Vec::new();
    let mut final_temp_dirs = Vec::new();
    for (lists, temp_dir) in parallel_results {
        final_list_paths.extend(lists);
        final_temp_dirs.push(temp_dir);
    }
    
    (final_list_paths, final_temp_dirs)
}