use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::fs;
use std::thread;
use crate::features::mods::logic::state::{ModState, ModPackType};
use crate::features::mods::logic::{bridge, extract, decrypt};
use crate::features::mods::logic::bridge::ModAdbEvent;

pub fn process_events(state: &mut ModState) -> bool {
    process_adb_events(state);
    process_pack_events(state);
    state.import.is_busy
}

fn process_adb_events(state: &mut ModState) {
    let Some(rx) = &state.import.adb_rx else { return; };
    
    while let Ok(event) = rx.try_recv() {
        match event {
            ModAdbEvent::Status(msg) => {
                state.import.status_message = msg.clone();
                state.import.log_content.push_str(&format!("{}\n", msg));
            }
            ModAdbEvent::Success(msg) => {
                state.import.status_message = msg.clone();
                state.import.log_content.push_str(&format!("SUCCESS: {}\n", msg));
                state.import.is_busy = false;
            }
            ModAdbEvent::Error(msg) => {
                state.import.status_message = format!("Error: {}", msg);
                state.import.log_content.push_str(&format!("ERROR: {}\n", msg));
                state.import.is_busy = false;
            }
        }
    }
}

fn process_pack_events(state: &mut ModState) {
    let Some(rx) = &state.import.pack_rx else { return; };

    while let Ok(msg) = rx.try_recv() {
        state.import.status_message = msg.clone();
        state.import.log_content.push_str(&format!("{}\n", msg));
        
        let lower_msg = msg.to_lowercase();
        if lower_msg.contains("complete!") || lower_msg.contains("error") || lower_msg.contains("failed") {
            state.import.is_busy = false;
        }
    }
}

pub fn start_adb_import(state: &mut ModState) {
    state.import.log_content.clear(); 
    state.import.status_message = "Initializing Mod ADB Pull...".to_string();
    state.import.is_busy = true;
    
    let (tx, rx) = mpsc::channel();
    state.import.adb_rx = Some(rx);
    bridge::spawn_mod_import(tx, state.import.package_suffix.clone());
}

pub fn start_pack_import(state: &mut ModState, path: PathBuf) {
    let pack_type = state.import.pack_type;
    
    state.import.log_content.clear();
    state.import.status_message = format!("Processing {:?}...", path.file_name().unwrap_or_default());
    state.import.is_busy = true;

    let (tx, rx) = std::sync::mpsc::channel();
    state.import.pack_rx = Some(rx);

    std::thread::spawn(move || {
        let pkg_name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let target_dir = PathBuf::from(format!("mods/packages/{}", pkg_name));
        
        let res = match pack_type {
            ModPackType::Apk | ModPackType::Zip => {
                let r = extract::run_archive(&path, &target_dir, tx.clone());
                
                let _ = tx.send("Cleaning up temporary pack files...".to_string());
                
                if let Err(e) = std::fs::remove_dir_all(&target_dir) {
                    let _ = tx.send(format!("Warning: Could not fully delete {}: {}", target_dir.display(), e));
                }
                
                let _ = std::fs::remove_dir("mods/packages"); 
                
                r
            },
            ModPackType::Folder => {
                let _ = tx.send("Folder import coming soon...".to_string());
                Err("Not implemented".to_string())
            },
            ModPackType::Pack => decrypt::run(&path, tx.clone())
        };

        if let Err(e) = res {
            let _ = tx.send(format!("Error: {}", e));
        }
    });
}

pub fn start_raw_import(state: &mut ModState, is_folder: bool, path_opt: Option<PathBuf>, files: Vec<PathBuf>) {
    state.import.log_content.clear();
    state.import.is_busy = true;
    state.import.status_message = "Copying raw files...".to_string();

    let (tx, rx) = std::sync::mpsc::channel();
    state.import.pack_rx = Some(rx);
    
    std::thread::spawn(move || {
         let mods_root = Path::new("mods");
         let mut mod_num = 1;
         while mods_root.join(format!("NewMod{}", mod_num)).exists() {
             mod_num += 1;
         }
         let target_dir = mods_root.join(format!("NewMod{}", mod_num));
         
         let _ = tx.send("Creating new mod workspace...".to_string());
         if std::fs::create_dir_all(&target_dir).is_err() {
             let _ = tx.send("Error: Failed to create target directory".to_string());
             return;
         }

         if is_folder {
             let Some(p) = path_opt else { return; };
             let _ = tx.send(format!("Copying folder {:?}...", p.file_name().unwrap_or_default()));
             if let Err(e) = copy_dir_all(&p, &target_dir) {
                 let _ = tx.send(format!("Error copying folder: {}", e));
             } else {
                 let final_name = apply_metadata_rename(&mods_root, &target_dir, mod_num);
                 let _ = tx.send(format!("Raw Import Complete! Saved as '{}'.", final_name));
             }
             return;
         }

         let _ = tx.send(format!("Copying {} files...", files.len()));
         for file in files {
             let Some(name) = file.file_name() else { continue; };
             if let Err(e) = std::fs::copy(&file, target_dir.join(name)) {
                 let _ = tx.send(format!("Error copying file {:?}: {}", name, e));
             }
         }
         
         let final_name = apply_metadata_rename(&mods_root, &target_dir, mod_num);
         let _ = tx.send(format!("Raw Import Complete! Saved as '{}'.", final_name));
    });
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn apply_metadata_rename(mods_root: &Path, target_dir: &Path, default_num: u32) -> String {
    let mut final_name = format!("NewMod{}", default_num);
    let meta_path = target_dir.join("metadata.json");
    
    if meta_path.exists() {
        let meta = crate::features::mods::logic::metadata::ModMetadata::load(target_dir);
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
                
                if std::fs::rename(target_dir, &new_path).is_ok() {
                    final_name = attempt;
                }
            }
        }
    }
    final_name
}

pub fn add_mod_icon(path: &Path) {
    let Some(image_path) = rfd::FileDialog::new()
        .add_filter("Images", &["png", "ico"])
        .pick_file() else { return; };

    let Some(ext) = image_path.extension() else { return; };
    let ext_str = ext.to_string_lossy().to_lowercase();
    
    if ext_str != "png" && ext_str != "ico" { return; }

    let target_path = path.join(format!("icon.{}", ext_str));
    
    let other_ext = if ext_str == "png" { "ico" } else { "png" };
    let _ = fs::remove_file(path.join(format!("icon.{}", other_ext)));

    let _ = fs::copy(image_path, target_path);
}

pub fn delete_mod_icon(path: &Path) {
    let png_path = path.join("icon.png");
    let ico_path = path.join("icon.ico");

    if png_path.exists() { let _ = fs::remove_file(png_path); }
    if ico_path.exists() { let _ = fs::remove_file(ico_path); }
}

pub fn delete_mod_folder(path: PathBuf) {
    thread::spawn(move || {
        let _ = fs::remove_dir_all(path);
    });
}