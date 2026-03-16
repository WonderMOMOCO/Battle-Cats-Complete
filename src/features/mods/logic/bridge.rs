use std::path::PathBuf;
use std::thread;
use std::sync::mpsc::Sender;
use std::fs;
use crate::features::addons::adb::driver; 
use super::extract;

pub enum ModAdbEvent {
    Status(String),
    Success(String),
    Error(String),
}

pub fn spawn_mod_import(tx: Sender<ModAdbEvent>, suffix: String) {
    thread::spawn(move || {
        let _ = tx.send(ModAdbEvent::Status("Starting ADB Server...".to_string()));
        let _ = driver::run_command(&["start-server"]);
        
        let pkg = format!("jp.co.ponos.battlecats{}", suffix);
        let _ = tx.send(ModAdbEvent::Status(format!("Targeting Package: {}", pkg)));

        // Locate Device
        let serial = match driver::find_usb_device().or_else(|| driver::find_emulator()) {
            Some(s) => s,
            None => {
                let _ = tx.send(ModAdbEvent::Error("No device found.".to_string()));
                return;
            }
        };

        // Create the temporary staging directory
        let target_dir = PathBuf::from(format!("mods/packages/{}", pkg));
        if !target_dir.exists() { let _ = fs::create_dir_all(&target_dir); }

        let _ = tx.send(ModAdbEvent::Status(format!("Pulling base.apk for {}...", pkg)));

        // Look for base.apk on the device
        let pm_path = driver::run_command(&["-s", &serial, "shell", "pm", "path", &pkg]).unwrap_or_default();
        let remote_path = pm_path.lines()
            .find(|line| line.contains("base.apk"))
            .unwrap_or("")
            .trim()
            .strip_prefix("package:")
            .unwrap_or("");

        if remote_path.is_empty() {
            let _ = tx.send(ModAdbEvent::Error(format!("Could not find base.apk for {}", pkg)));
            return;
        }

        // Pull the APK to our temporary folder
        let local_apk_path = target_dir.join("base.apk");
        if driver::run_command(&["-s", &serial, "pull", remote_path, local_apk_path.to_str().unwrap()]).is_err() {
            let _ = tx.send(ModAdbEvent::Error("Failed to pull base.apk from device.".to_string()));
            return;
        }

        let _ = tx.send(ModAdbEvent::Status("Extracting DownloadLocal data...".to_string()));
        
        // Setup a proxy channel to catch the String messages from extract/decrypt
        let (e_tx, e_rx) = std::sync::mpsc::channel();
        let tx_clone = tx.clone();
        thread::spawn(move || {
            while let Ok(msg) = e_rx.recv() {
                let _ = tx_clone.send(ModAdbEvent::Status(msg));
            }
        });
        
        // Run the extraction and decryption pipeline
        if let Err(e) = extract::run_archive(&local_apk_path, &target_dir, e_tx) {
            let _ = tx.send(ModAdbEvent::Error(format!("Extraction/Decryption failed: {}", e)));
            return;
        }
        
        // CLEANUP
        let _ = tx.send(ModAdbEvent::Status("Cleaning up temporary base.apk and pack files...".to_string()));
        let _ = fs::remove_dir_all(&target_dir); // Nukes the APK, the .list, and the .pack
        
        let _ = tx.send(ModAdbEvent::Success("ADB Mod Import Complete!".to_string()));
    });
}