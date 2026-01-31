use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;

pub fn create_game_archive(tx: Sender<String>, compression_level: i32, filename: String) -> Result<(), String> {
    let game_root = Path::new("game");
    let export_dir = Path::new("exports");
    
    let final_filename = if filename.ends_with(".tar.zst") {
        filename
    } else {
        let clean_name = filename
            .trim_end_matches(".zip")
            .trim_end_matches(".game")
            .trim_end_matches(".tar.zst");
            
        format!("{}.tar.zst", clean_name)
    };

    let archive_path = export_dir.join(final_filename);
    
    if !game_root.exists() { 
        return Err("No 'game' folder found to export.".to_string()); 
    }
    
    if !export_dir.exists() { 
        fs::create_dir_all(export_dir).map_err(|e| e.to_string())?; 
    }
    
    let threads = match thread::available_parallelism() {
        Ok(n) => n.get() as u32,
        Err(_) => 4,
    };

    let _ = tx.send(format!("Starting Multi-Threaded Compression ({} threads)...", threads));
    
    let file = fs::File::create(&archive_path).map_err(|e| e.to_string())?;
    let mut encoder = zstd::stream::write::Encoder::new(file, compression_level).map_err(|e| e.to_string())?;
    
    encoder.multithread(threads).map_err(|e| e.to_string())?;
    encoder.include_checksum(true).map_err(|e| e.to_string())?;

    let mut tar_builder = tar::Builder::new(encoder);
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
                directory_stack.push(path.clone());
                let relative_name = path.strip_prefix(game_root).unwrap();
                let _ = tar_builder.append_dir(relative_name, &path);
                continue;
            } 

            let relative_name = path.strip_prefix(game_root).unwrap();
            
            let mut file_handle = match fs::File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            if tar_builder.append_file(relative_name, &mut file_handle).is_ok() {
                processed_count += 1;
                
                if processed_count % 50 == 0 {
                    let simple_filename = path.file_name().unwrap_or_default().to_string_lossy();
                    let _ = tx.send(format!("Packed {} files | Current: {}", processed_count, simple_filename));
                }
            }
        }
    }
    
    let zstd_encoder = tar_builder.into_inner().map_err(|e| e.to_string())?;
    let _ = zstd_encoder.finish().map_err(|e| e.to_string())?;
    
    let _ = tx.send(format!("Success! Exported {} files to {:?}", processed_count, archive_path));
    Ok(())
}