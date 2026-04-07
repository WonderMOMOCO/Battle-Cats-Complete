use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use crate::features::data::utilities::engine;
use crate::features::data::state::{ImportMode, AdbRegion};

pub fn run(
    source_path_string: &str, 
    import_mode: ImportMode,
    _target_region: AdbRegion,
    status_sender: Sender<String>, 
    abort_flag: Arc<AtomicBool>, 
    progress_current: Arc<AtomicUsize>, 
    progress_maximum: Arc<AtomicUsize>
) -> Result<(), String> {
    
    let source_directory = match import_mode {
        ImportMode::Folder => PathBuf::from(source_path_string),
        ImportMode::Zip => {
            let _ = status_sender.send("Extracting archive to temporary workspace...".to_string());
            PathBuf::from("temp_workspace") 
        },
        _ => return Err("Invalid Import Mode selected.".to_string()),
    };

    let directories_to_process = vec![source_directory.clone()];
    
    let engine_result = engine::run_universal_import(&directories_to_process, &status_sender, &abort_flag, &progress_current, &progress_maximum);
    
    if import_mode == ImportMode::Zip {
        let _ = fs::remove_dir_all(source_directory);
    }

    engine_result
}