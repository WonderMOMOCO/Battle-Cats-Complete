use std::fs;
use std::io::{Read, Write, BufWriter}; 
use std::path::Path;
use std::sync::mpsc::Sender;
use zip::write::FileOptions;
use crate::core::import::log::Logger;

pub fn to_zip(compression_level: i32, base_file_name: String, status_sender: Sender<String>) -> Result<(), String> {
    let logger = Logger::new(status_sender);
    let source_directory = Path::new("game");
    
    let exports_directory = Path::new("exports");
    if !exports_directory.exists() {
        fs::create_dir_all(exports_directory).map_err(|e| e.to_string())?;
    }

    let final_file_name = format!("{}.game.zip", base_file_name);
    let output_zip_path = exports_directory.join(&final_file_name);
    
    if !source_directory.exists() {
        let msg = "Game directory not found. Nothing to export.";
        logger.error(msg);
        return Err(msg.to_string());
    }

    let output_file_handle = fs::File::create(&output_zip_path).map_err(|e| e.to_string())?;
    let buffered_writer = BufWriter::with_capacity(1_048_576, output_file_handle); 
    let mut zip_writer = zip::ZipWriter::new(buffered_writer);
    
    let zip_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(compression_level)) 
        .unix_permissions(0o755);

    logger.info(format!("Creating {}...", final_file_name));
    logger.info("Scanning files to zip...");

    let mut files_to_zip_list = Vec::new();
    let mut directory_stack = vec![source_directory.to_path_buf()];

    while let Some(current_directory) = directory_stack.pop() {
        if let Ok(entries) = fs::read_dir(&current_directory) {
            for entry_result in entries.flatten() {
                let entry_path = entry_result.path();
                
                if entry_path.to_string_lossy().contains(&format!("game{}raw", std::path::MAIN_SEPARATOR)) {
                    continue;
                }

                if entry_path.is_dir() {
                    directory_stack.push(entry_path);
                } else {
                    files_to_zip_list.push(entry_path);
                }
            }
        }
    }

    let total_file_count = files_to_zip_list.len();
    logger.info(format!("Found {} files. Starting compression...", total_file_count));

    let mut data_buffer = Vec::with_capacity(1_048_576); 

    for (index, file_path) in files_to_zip_list.iter().enumerate() {
        let file_name_in_zip = file_path.to_string_lossy().replace("\\", "/");
        
        if index % 100 == 0 || index == total_file_count - 1 {
            logger.info(format!("Zipped {} files | Current: {}", index, file_name_in_zip));
        }

        if zip_writer.start_file(file_name_in_zip, zip_options).is_ok() {
            if let Ok(mut source_file) = fs::File::open(file_path) {
                data_buffer.clear(); 
                if source_file.read_to_end(&mut data_buffer).is_ok() {
                    let _ = zip_writer.write_all(&data_buffer);
                }
            }
        }
    }

    if zip_writer.finish().is_ok() {
        logger.success(format!("Success! Saved to {}", output_zip_path.display()));
        Ok(())
    } else {
        Err("Failed to finalize zip file.".to_string())
    }
}