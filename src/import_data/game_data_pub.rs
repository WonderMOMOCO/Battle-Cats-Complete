use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::io::{Read, Write};
use zip::ZipArchive;
use zip::write::FileOptions;
use crate::patterns; 

pub fn import_from_folder(source_folder: &str, tx: Sender<String>) -> Result<(), String> {
    let input_path = Path::new(source_folder);
    let output_dir = Path::new("game/raw");
    
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| format!("Could not create 'game/raw': {}", e))?;
    }

    let _ = tx.send("Scanning folder...".to_string());

    let mut file_count = 0;

    if input_path.is_dir() {
        let mut stack = vec![input_path.to_path_buf()];
        
        while let Some(current_dir) = stack.pop() {
            let entries = fs::read_dir(&current_dir).map_err(|e| e.to_string())?;
            
            for entry in entries {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                
                if path.is_dir() {
                    stack.push(path);
                } else {
                    if let Some(name_os) = path.file_name() {
                        let name_str = name_os.to_string_lossy().to_string();
                        
                        let mut dest_filename = name_str.clone();

                        if patterns::REGION_SENSITIVE_FILES.contains(&name_str.as_str()) {
                            let path_obj = Path::new(&name_str);
                            let stem = path_obj.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
                            let ext = path_obj.extension().map(|s| s.to_string_lossy()).unwrap_or_default();
                            dest_filename = format!("{}_au.{}", stem, ext);
                        }

                        let dest_path = output_dir.join(&dest_filename);
                        
                        if fs::copy(&path, &dest_path).is_ok() {
                            file_count += 1;
                            if file_count % 50 == 0 {
                                let _ = tx.send(format!("Copied {} files | Current: {}", file_count, dest_filename));
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err("Selected path is not a directory.".to_string());
    }

    let _ = tx.send(format!("Copy complete. {} files moved to game/raw.", file_count));
    Ok(())
}

pub fn import_from_zip(zip_path_str: &str, tx: Sender<String>) -> Result<(), String> {
    let file = fs::File::open(zip_path_str).map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {}", e))?;

    let _ = tx.send("Validating Archive...".to_string());

    for (dir, prefix, extensions) in patterns::ESSENTIAL_FILES {
        let mut set_satisfied = false;

        for &code in patterns::GLOBAL_CODES {
            if code == "en" { continue; } 

            let mut all_extensions_found = true;
            let mut found_files = Vec::new();

            for &ext in *extensions {
                let expected_path = format!("{}/{}_{}.{}", dir, prefix, code, ext);
                let expected_filename = format!("{}_{}.{}", prefix, code, ext);
                
                if archive.by_name(&expected_path).is_err() {
                    all_extensions_found = false;
                    break;
                } else {
                    found_files.push(expected_filename);
                }
            }

            if all_extensions_found {
                set_satisfied = true;
                for f in found_files {
                    let _ = tx.send(format!("{} was found!", f));
                }
                break; 
            }
        }

        if !set_satisfied {
            return Err(format!("Import Aborted: ZIP Archive is Missing Essential Files! (Missing set for '{}')", prefix));
        }
    }

    let _ = tx.send("Validation Passed. Extracting...".to_string());

    let len = archive.len();
    for i in 0..len {
        let mut file = archive.by_index(i).unwrap();
        
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        if outpath.starts_with("game") {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    let _ = fs::create_dir_all(p);
                }
            }

            if file.is_dir() {
                if !outpath.exists() {
                    let _ = fs::create_dir_all(&outpath);
                }
            } else {
                let safe_name = outpath.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                if let Ok(mut outfile) = fs::File::create(&outpath) {
                    if std::io::copy(&mut file, &mut outfile).is_ok() {
                        // Lively logging
                        if i % 50 == 0 {
                            let _ = tx.send(format!("Extracted {} files | Current: {}", i + 1, safe_name));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn create_game_zip(tx: Sender<String>, compression_level: i32) -> Result<(), String> {
    let src_dir = Path::new("game");
    let exports_dir = Path::new("exports");
    let zip_path = exports_dir.join("game.zip");

    if !src_dir.exists() {
        return Err("No 'game' folder found to zip.".to_string());
    }

    if !exports_dir.exists() {
        fs::create_dir_all(exports_dir).map_err(|e| e.to_string())?;
    }

    let _ = tx.send(format!("Creating game.zip with Compression Level {}...", compression_level));

    let file = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(compression_level)) 
        .unix_permissions(0o755);

    let mut files_to_zip = Vec::new();
    let mut folders_to_visit = vec![src_dir.to_path_buf()];

    while let Some(current_dir) = folders_to_visit.pop() {
        let entries = fs::read_dir(&current_dir).map_err(|e| e.to_string())?;
        
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.to_string_lossy().contains(&format!("game{}raw", std::path::MAIN_SEPARATOR)) {
                continue;
            }

            if path.is_dir() {
                folders_to_visit.push(path);
            } else {
                files_to_zip.push(path);
            }
        }
    }

    let total_files = files_to_zip.len();
    for (i, path) in files_to_zip.iter().enumerate() {
        let name = path.to_string_lossy().replace("\\", "/");
        
        if i % 50 == 0 || i == total_files - 1 {
            let _ = tx.send(format!("Zipped {} files | Current: {}", i + 1, name));
        }

        zip.start_file(name, options).map_err(|e| e.to_string())?;
        
        if let Ok(mut f) = fs::File::open(path) {
            let mut buffer = Vec::new();
            if f.read_to_end(&mut buffer).is_ok() {
                zip.write_all(&buffer).map_err(|e| e.to_string())?;
            }
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    
    let _ = tx.send(format!("Success! Saved to {}", zip_path.display()));
    Ok(())
}