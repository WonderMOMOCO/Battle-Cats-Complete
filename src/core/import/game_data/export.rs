use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::Sender;
use zip::write::FileOptions;

pub fn create_game_zip(tx: Sender<String>, compression_level: i32, zip_filename: String) -> Result<(), String> {
    let game_root = Path::new("game");
    let export_dir = Path::new("exports");
    
    let zip_file_path = export_dir.join(zip_filename);
    
    if !game_root.exists() { 
        return Err("No 'game' folder found to export.".to_string()); 
    }
    
    if !export_dir.exists() { 
        fs::create_dir_all(export_dir).map_err(|e| e.to_string())?; 
    }
    
    let _ = tx.send("Scanning for files to zip...".to_string());
    
    let zip_file = fs::File::create(&zip_file_path).map_err(|e| e.to_string())?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    
    let zip_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(compression_level))
        .unix_permissions(0o755);
    
    let mut processed_count = 0;
    let mut directory_stack = vec![game_root.to_path_buf()];
    
    while let Some(current_dir) = directory_stack.pop() {
        let entries = match fs::read_dir(&current_dir) {
            Ok(iter) => iter,
            Err(_) => continue,
        };

        for entry_result in entries.flatten() {
            let path = entry_result.path();
            
            let path_str = path.to_string_lossy();
            if path_str.contains("game/raw") || path_str.contains("game\\raw") { 
                continue; 
            }
            
            if path.is_dir() {
                directory_stack.push(path);
                continue;
            } 

            let relative_name = path.strip_prefix(game_root).unwrap().to_string_lossy().replace("\\", "/");
            let _ = zip_writer.start_file(&relative_name, zip_options);
            
            let mut file_handle = match fs::File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let mut buffer = Vec::new();
            if file_handle.read_to_end(&mut buffer).is_err() {
                continue;
            }

            if zip_writer.write_all(&buffer).is_ok() {
                processed_count += 1;
                
                if processed_count % 50 == 0 {
                    let simple_filename = path.file_name().unwrap_or_default().to_string_lossy();
                    let _ = tx.send(format!("Compressed {} files | Current: {}", processed_count, simple_filename));
                }
            }
        }
    }
    
    let _ = zip_writer.finish();
    let _ = tx.send(format!("Success! Exported {} files to {:?}", processed_count, zip_file_path));
    Ok(())
}