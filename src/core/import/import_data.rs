use std::fs;
use std::path::{Path};
use std::sync::mpsc::Sender;
use zip::ZipArchive;
use crate::core::import::log::Logger;

pub fn from_folder(source_folder_path: &str, status_sender: Sender<String>) -> Result<bool, String> {
    let logger = Logger::new(status_sender.clone()); 
    let source_path = Path::new(source_folder_path);
    let game_root_directory = Path::new("game");
    let raw_file_directory = game_root_directory.join("raw");
    
    if !game_root_directory.exists() {
        fs::create_dir_all(game_root_directory).map_err(|e| e.to_string())?;
    }

    logger.info("Analyzing folder structure...");

    let mut detected_game_root = None;
    
    if source_path.join("assets").join("img015").exists() {
        detected_game_root = Some(source_path.to_path_buf());
    } 
    else if let Ok(directory_entries) = fs::read_dir(source_path) {
        for entry_result in directory_entries.flatten() {
            let entry_path = entry_result.path();
            if entry_path.is_dir() {
                if entry_path.join("assets").join("img015").exists() {
                    detected_game_root = Some(entry_path);
                    break;
                }
            }
        }
    }

    if let Some(game_root) = detected_game_root {
        logger.info("Valid game data structure found! Importing directly...");
        
        let mut files_copied_count = 0;
        let mut directory_stack = vec![game_root.clone()];
        
        while let Some(current_source_dir) = directory_stack.pop() {
            if let Ok(entries) = fs::read_dir(&current_source_dir) {
                for entry_result in entries.flatten() {
                    let source_entry_path = entry_result.path();
                    
                    let relative_path = match source_entry_path.strip_prefix(&game_root) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    if relative_path.starts_with("raw") {
                        continue;
                    }

                    let destination_path = game_root_directory.join(relative_path);

                    if source_entry_path.is_dir() {
                        if !destination_path.exists() {
                            let _ = fs::create_dir_all(&destination_path);
                        }
                        directory_stack.push(source_entry_path);
                    } else {
                        if let Some(parent_directory) = destination_path.parent() {
                            if !parent_directory.exists() { let _ = fs::create_dir_all(parent_directory); }
                        }
                        if fs::copy(&source_entry_path, &destination_path).is_ok() {
                            files_copied_count += 1;
                            if files_copied_count % 100 == 0 {
                                logger.info(format!("Copied {} files | Current: {}", files_copied_count, relative_path.display()));
                            }
                        }
                    }
                }
            }
        }
        logger.success(format!("Import Complete. Restored {} files.", files_copied_count));
        return Ok(false); 
    }

    logger.info("No structure detected. Importing as raw files...");
    
    if !raw_file_directory.exists() {
        fs::create_dir_all(&raw_file_directory).map_err(|e| e.to_string())?;
    }

    let mut raw_files_count = 0;
    let mut directory_stack = vec![source_path.to_path_buf()];
        
    while let Some(current_directory) = directory_stack.pop() {
        let entries = fs::read_dir(&current_directory).map_err(|e| e.to_string())?;
        
        for entry_result in entries {
            let entry = entry_result.map_err(|e| e.to_string())?;
            let entry_path = entry.path();
            
            if entry_path.is_dir() {
                directory_stack.push(entry_path);
            } else {
                if let Some(file_name_os) = entry_path.file_name() {
                    let file_name = file_name_os.to_string_lossy().to_string();
                    let destination_path = raw_file_directory.join(&file_name);
                    
                    if fs::copy(&entry_path, &destination_path).is_ok() {
                        raw_files_count += 1;
                        if raw_files_count % 50 == 0 {
                            logger.info(format!("Copied {} files | Current: {}", raw_files_count, file_name));
                        }
                    }
                }
            }
        }
    }
    
    logger.info(format!("Copied {} raw files.", raw_files_count));
    Ok(true) 
}

pub fn from_zip(zip_file_path: &str, status_sender: Sender<String>) -> Result<bool, String> {
    let logger = Logger::new(status_sender.clone());
    let source_zip_file = fs::File::open(zip_file_path).map_err(|e| e.to_string())?;
    let mut zip_archive = ZipArchive::new(source_zip_file).map_err(|e| e.to_string())?;
    let game_root_directory = Path::new("game");
    let raw_file_directory = game_root_directory.join("raw");

    logger.info("Scanning archive structure...");

    let mut detected_path_prefix = None;
    
    for i in 0..zip_archive.len() {
        if let Ok(file_in_archive) = zip_archive.by_index(i) {
            let file_name = file_in_archive.name();
            if let Some(index) = file_name.find("assets/img015") {
                let prefix = &file_name[..index];
                detected_path_prefix = Some(prefix.to_string());
                break;
            }
        }
    }

    if let Some(prefix) = detected_path_prefix {
        logger.info("Valid game backup found in ZIP! Extracting...");

        let total_files_in_zip = zip_archive.len();
        let mut extracted_count = 0;

        for i in 0..total_files_in_zip {
            let mut file_in_archive = zip_archive.by_index(i).unwrap();
            let file_name = file_in_archive.name().to_string();

            if !file_name.starts_with(&prefix) {
                continue;
            }

            let relative_file_name = &file_name[prefix.len()..];
            
            if relative_file_name.is_empty() || relative_file_name.ends_with('/') {
                continue; 
            }

            if relative_file_name.starts_with("raw/") || relative_file_name.contains("/raw/") {
                continue;
            }

            let destination_path = game_root_directory.join(relative_file_name);

            if let Some(parent_directory) = destination_path.parent() {
                if !parent_directory.exists() { fs::create_dir_all(parent_directory).ok(); }
            }

            let mut output_file = fs::File::create(&destination_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file_in_archive, &mut output_file).ok();
            
            extracted_count += 1;
            if extracted_count % 100 == 0 {
                 logger.info(format!("Extracted {} files | Current: {}", extracted_count, relative_file_name));
            }
        }
        
        logger.success("Success! Archive extracted/restored.");
        return Ok(false); 
    }

    logger.info("No structure found. Extracting to raw...");
    
    if !raw_file_directory.exists() {
        fs::create_dir_all(&raw_file_directory).map_err(|e| e.to_string())?;
    }

    let total_files_in_zip = zip_archive.len();
    for i in 0..total_files_in_zip {
        let mut file_in_archive = zip_archive.by_index(i).unwrap();
        
        let output_path = match file_in_archive.enclosed_name() {
            Some(p) => {
                if let Some(name) = p.file_name() {
                    raw_file_directory.join(name)
                } else {
                    continue;
                }
            },
            None => continue,
        };

        let current_file_name = output_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if !file_in_archive.is_dir() {
            let mut output_file = fs::File::create(&output_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file_in_archive, &mut output_file).ok();
        }
        
        if i % 50 == 0 {
             logger.info(format!("Extracted {} / {} files | Current: {}", i, total_files_in_zip, current_file_name));
        }
    }
    
    logger.info("Extraction complete.");
    Ok(true) 
}