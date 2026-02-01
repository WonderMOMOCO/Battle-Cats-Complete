use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use std::sync::mpsc::Sender;
use std::fs;
use crate::core::adb::driver;
use crate::core::import::{AdbImportType, AdbRegion};
use crate::core::import::{decrypt, sort}; 
use crate::core::settings::handle::EmulatorConfig;

pub enum AdbEvent {
    Status(String),
    Success(String),
    Error(String),
}

pub fn spawn_full_import(tx: Sender<AdbEvent>, base_output_dir: PathBuf, mode: AdbImportType, region: AdbRegion, config: EmulatorConfig) {
    thread::spawn(move || {
        // Initialize
        let _ = tx.send(AdbEvent::Status("Starting ADB Server...".to_string()));
        let _ = driver::run_command(&["kill-server"]);
        thread::sleep(Duration::from_millis(500));
        let _ = driver::run_command(&["start-server"]);

        let _ = tx.send(AdbEvent::Status("Connecting to emulator...".to_string()));
        if let Err(connection_error) = driver::connect_to_emulator() {
            let _ = tx.send(AdbEvent::Error(format!("Connection failed: {}", connection_error)));
            return;
        }

        let serial = match get_first_active_device() {
            Some(device_serial) => device_serial,
            None => {
                let _ = tx.send(AdbEvent::Error("No active device found.".to_string()));
                return;
            }
        };
        
        let regions_to_process = match region {
            AdbRegion::All => vec![
                AdbRegion::English, 
                AdbRegion::Japanese, 
                AdbRegion::Taiwan, 
                AdbRegion::Korean
            ],
            _ => vec![region],
        };

        for (index, current_region) in regions_to_process.iter().enumerate() {
            let suffix = current_region.suffix();
            let pkg = format!("jp.co.ponos.battlecats{}", suffix);
            
            if region == AdbRegion::All {
                let _ = tx.send(AdbEvent::Status(format!("Processing Region {}/4: {}", index + 1, pkg)));
            } else {
                let _ = tx.send(AdbEvent::Status(format!("Processing: {}", pkg)));
            }

            let target_dir = base_output_dir.join(&pkg);
            
            // ADB Pull
            let res = process_single_region_adb(&tx, &serial, &pkg, &target_dir, mode);
            
            if let Err(adb_error) = res {
                let _ = tx.send(AdbEvent::Status(format!("Skipping {} due to ADB error: {}", pkg, adb_error)));
                continue; 
            }

            // Decryption
            if mode == AdbImportType::All {
                let _ = tx.send(AdbEvent::Status("Starting Decryption...".to_string()));
                
                let region_code = match suffix {
                    "" => "ja",
                    "kr" => "ko",
                    other => other,
                };
                
                let (d_tx, d_rx) = std::sync::mpsc::channel();
                let tx_clone = tx.clone();
                thread::spawn(move || {
                    while let Ok(msg) = d_rx.recv() {
                        let _ = tx_clone.send(AdbEvent::Status(msg));
                    }
                });

                if let Err(decrypt_error) = decrypt::run(target_dir.to_str().unwrap(), region_code, d_tx) {
                     let _ = tx.send(AdbEvent::Status(format!("Decryption Failed: {}", decrypt_error)));
                     continue;
                }

                // Clean-up
                if config.keep_app_folder { 
                    let _ = tx.send(AdbEvent::Status("Skipping app folder cleanup (Persistence On)...".to_string()));
                } else {
                    let _ = tx.send(AdbEvent::Status("Cleaning up temporary app files...".to_string()));
                    if base_output_dir.exists() {
                         let _ = fs::remove_dir_all(&base_output_dir);
                    }
                }

                // Sort
                let _ = tx.send(AdbEvent::Status("Starting Sort...".to_string()));
                let (s_tx, s_rx) = std::sync::mpsc::channel();
                let tx_clone_2 = tx.clone();
                thread::spawn(move || {
                    while let Ok(msg) = s_rx.recv() {
                        let _ = tx_clone_2.send(AdbEvent::Status(msg));
                    }
                });

                if let Err(sort_error) = sort::sort_game_files(s_tx) {
                    let _ = tx.send(AdbEvent::Status(format!("Sort Failed: {}", sort_error)));
                } else {
                    let _ = tx.send(AdbEvent::Status("Region processed successfully.".to_string()));
                }
            }

            thread::sleep(Duration::from_secs(1));
        }

        let _ = tx.send(AdbEvent::Success("All Operations Complete!".to_string()));
    });
}

fn process_single_region_adb(
    tx: &Sender<AdbEvent>, 
    serial: &str, 
    pkg: &str, 
    output_dir: &PathBuf, 
    mode: AdbImportType
) -> Result<(), String> {

    if mode == AdbImportType::All {
        // Root Check
        let _ = tx.send(AdbEvent::Status("Requesting Root Access...".to_string()));
        let _ = driver::run_command(&["-s", serial, "root"]);
        thread::sleep(Duration::from_secs(1)); 
        let _ = driver::connect_to_emulator(); 

        let whoami = driver::run_command(&["-s", serial, "shell", "whoami"]).unwrap_or_default();
        if whoami.contains("root") {
            let _ = tx.send(AdbEvent::Status("Root confirmed.".to_string()));
        }

        // Stage Files
        let _ = tx.send(AdbEvent::Status("Pulling files...".to_string()));
        let remote_src = format!("/data/data/{}/files", pkg);
        let remote_stage_dir = "/data/local/tmp";
        let remote_stage_target = "/data/local/tmp/files";

        let _ = driver::run_command(&["-s", serial, "shell", "rm", "-rf", remote_stage_target]); 
        let _ = driver::run_command(&["-s", serial, "shell", "mkdir", "-p", remote_stage_dir]); 

        let mut success = false;
        if whoami.contains("root") {
            success = driver::run_command(&["-s", serial, "shell", "cp", "-r", &remote_src, remote_stage_dir]).is_ok();
        }
        if !success {
            let cmd = format!("'cp -r {} {}'", remote_src, remote_stage_dir);
            if driver::run_command(&["-s", serial, "shell", "su", "-c", &cmd]).is_ok() {
                success = true;
            }
        }
        if !success {
            if driver::run_command(&["-s", serial, "shell", "su", "0", "cp", "-r", &remote_src, remote_stage_dir]).is_ok() {
                success = true;
            }
        }
        if !success { return Err("Root Copy Failed.".to_string()); }

        let _ = tx.send(AdbEvent::Status("Cleaning invalid files...".to_string()));
        let clean_cmd = format!("rm -f {}/*:* {}/*firebase*", remote_stage_target, remote_stage_target);
        let _ = driver::run_command(&["-s", serial, "shell", &clean_cmd]);
        let _ = driver::run_command(&["-s", serial, "shell", "su", "-c", &format!("'{}'", clean_cmd)]);
        
        let _ = driver::run_command(&["-s", serial, "shell", "chmod", "-R", "777", remote_stage_target]);

        let _ = tx.send(AdbEvent::Status("Transferring to PC...".to_string()));
        if !output_dir.exists() { std::fs::create_dir_all(&output_dir).unwrap(); }

        if driver::run_command(&["-s", serial, "pull", remote_stage_target, output_dir.to_str().unwrap()]).is_err() {
            return Err("Failed to pull files.".to_string());
        }
        
        let _ = driver::run_command(&["-s", serial, "shell", "rm", "-rf", remote_stage_target]);
        let count = count_files(output_dir);
        let _ = tx.send(AdbEvent::Status(format!("Imported {} files.", count)));
    } 

    let _ = tx.send(AdbEvent::Status("Locating APK...".to_string()));
    if let Err(apk_error) = pull_split_apk(serial, pkg, "split_InstallPack.apk", output_dir) {
         let _ = tx.send(AdbEvent::Status(format!("APK Pull skipped: {}", apk_error)));
    }

    Ok(())
}

fn count_files(dir: &Path) -> usize {
    let mut count = 0;
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    count += count_files(&entry.path());
                } else {
                    count += 1;
                }
            }
        }
    }
    count
}

fn get_first_active_device() -> Option<String> {
    let output = driver::run_command(&["devices"]).ok()?;
    for line in output.lines().skip(1) {
        if line.trim().is_empty() { continue; }
        if let Some((serial, status)) = line.split_once('\t') {
            if status.contains("device") {
                return Some(serial.to_string());
            }
        }
    }
    None
}

fn pull_split_apk(serial: &str, pkg: &str, target: &str, out_dir: &Path) -> Result<(), String> {
    let cmd_out = driver::run_command(&["-s", serial, "shell", "pm", "path", pkg])?;
    
    if cmd_out.trim().is_empty() || cmd_out.contains("not found") {
        return Err("Game not found on device.".to_string());
    }

    let remote_path = cmd_out.lines()
        .find(|line| line.contains("base.apk"))
        .ok_or("APK Path not found.")?
        .trim().strip_prefix("package:").unwrap_or("")
        .replace("base.apk", target);

    let local_path = out_dir.join(target);
    if !out_dir.exists() { std::fs::create_dir_all(&out_dir).unwrap_or_default(); }

    driver::run_command(&["-s", serial, "pull", &remote_path, local_path.to_str().unwrap()])?;
    Ok(())
}