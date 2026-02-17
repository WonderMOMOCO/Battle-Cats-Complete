use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use std::sync::mpsc::Sender;
use std::fs;
use super::driver; 
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
        let _ = tx.send(AdbEvent::Status("Starting ADB Server...".to_string()));
        let _ = driver::run_command(&["kill-server"]);
        thread::sleep(Duration::from_millis(500));
        let _ = driver::run_command(&["start-server"]);
        
        let mut current_serial: String;
        let mut fallback_ip: Option<String> = None;

        let _ = tx.send(AdbEvent::Status("Detecting device...".to_string()));

        // --- PRIORITY 1: USB DEVICE ---
        if let Some(serial) = driver::find_usb_device() {
            let _ = tx.send(AdbEvent::Status(format!("USB Device Found: {}", serial)));
            current_serial = serial;
            
            // Setup Safety Net for USB
            fallback_ip = driver::enable_wireless_fallback(&current_serial);
            if let Some(ref ip) = fallback_ip {
                let _ = tx.send(AdbEvent::Status(format!("Wireless Fallback Prepared: {}", ip)));
            }
        } 
        // --- PRIORITY 2: MANUAL IP (If set) ---
        else if !config.manual_ip.is_empty() {
             let _ = tx.send(AdbEvent::Status(format!("Connecting to Manual IP: {}", config.manual_ip)));
             
             match driver::connect_manual_ip(&config.manual_ip) {
                 Ok(ip) => {
                     let _ = tx.send(AdbEvent::Status("Connected to Manual IP.".to_string()));
                     current_serial = ip;
                 },
                 Err(e) => {
                     let _ = tx.send(AdbEvent::Status(format!("Manual IP Connection Failed ({}). Switching to Auto-Detect...", e)));
                     
                     match driver::find_emulator() {
                        Some(emu) => {
                             let _ = tx.send(AdbEvent::Status(format!("Emulator Found: {}", emu)));
                             current_serial = emu;
                        },
                        None => {
                            let _ = tx.send(AdbEvent::Error("No USB, Manual IP failed, and No Emulator found.".to_string()));
                            return;
                        }
                     }
                 }
             }
        }
        // --- PRIORITY 3: EMULATOR AUTO-DETECT ---
        else {
             let _ = tx.send(AdbEvent::Status("Scanning for Emulators...".to_string()));
             match driver::find_emulator() {
                Some(emu) => {
                     let _ = tx.send(AdbEvent::Status(format!("Emulator Found: {}", emu)));
                     current_serial = emu;
                },
                None => {
                    let _ = tx.send(AdbEvent::Error("No valid Android device found.".to_string()));
                    return;
                }
             }
        }

        if mode == AdbImportType::All {
            let _ = tx.send(AdbEvent::Status("Requesting Root Access...".to_string()));
            let _ = driver::run_command(&["-s", &current_serial, "root"]);
            thread::sleep(Duration::from_secs(2));
            
            // Re-acquire connection if root caused a drop
            if !current_serial.contains(":") {
                 if let Some(new_s) = driver::find_usb_device() { current_serial = new_s; }
            } else {
                 let _ = driver::connect_wireless(&current_serial);
            }
        }

        let regions_to_process = match region {
            AdbRegion::All => vec![AdbRegion::English, AdbRegion::Japanese, AdbRegion::Taiwan, AdbRegion::Korean],
            _ => vec![region],
        };

        for (index, current_region) in regions_to_process.iter().enumerate() {
            let suffix = current_region.suffix();
            let pkg = format!("jp.co.ponos.battlecats{}", suffix);
            
            let status_prefix = if region == AdbRegion::All {
                format!("Region {}/4", index + 1)
            } else {
                "Processing".to_string()
            };
            let _ = tx.send(AdbEvent::Status(format!("{}: {}", status_prefix, pkg)));

            let target_dir = base_output_dir.join(&pkg);

            // --- EXECUTE WITH RESCUE LOGIC ---
            let mut attempt_success = false;
            
            if let Err(e) = process_single_region_adb(&tx, &current_serial, &pkg, &target_dir, mode) {
                
                if let Some(ref rescue_ip) = fallback_ip {
                    let _ = tx.send(AdbEvent::Status(format!("USB Error: {} Engaging Wireless Rescue...", e)));
                    let _ = tx.send(AdbEvent::Status(format!("Connecting to {}...", rescue_ip)));

                    if driver::connect_wireless(rescue_ip).is_ok() {
                        current_serial = rescue_ip.clone(); 
                        match process_single_region_adb(&tx, &current_serial, &pkg, &target_dir, mode) {
                            Ok(_) => {
                                attempt_success = true;
                                let _ = tx.send(AdbEvent::Status("Rescue Successful! Continuing via WiFi.".to_string()));
                            },
                            Err(e2) => { let _ = tx.send(AdbEvent::Status(format!("Rescue Failed: {}", e2))); },
                        }
                    } else {
                        let _ = tx.send(AdbEvent::Status("Could not connect to Wireless Fallback.".to_string()));
                    }
                } else {
                     let _ = tx.send(AdbEvent::Status(format!("Skipping {} due to error: {}", pkg, e)));
                }
            } else {
                attempt_success = true;
            }

            if !attempt_success { continue; }

            // --- DECRYPT & SORT ---
            let _ = tx.send(AdbEvent::Status("Starting Decryption...".to_string()));
            let region_code = match suffix { "" => "ja", "kr" => "ko", other => other };
            let (d_tx, d_rx) = std::sync::mpsc::channel();
            let tx_clone = tx.clone();
            thread::spawn(move || { while let Ok(msg) = d_rx.recv() { let _ = tx_clone.send(AdbEvent::Status(msg)); } });

            if let Err(decrypt_error) = decrypt::run(target_dir.to_str().unwrap(), region_code, d_tx) {
                let _ = tx.send(AdbEvent::Status(format!("Decryption Failed: {}", decrypt_error)));
                continue;
            }

            if !config.keep_app_folder {
                let _ = tx.send(AdbEvent::Status("Cleaning up temporary app files...".to_string()));
                if base_output_dir.exists() { let _ = fs::remove_dir_all(&base_output_dir); }
            }

            let _ = tx.send(AdbEvent::Status("Starting Sort...".to_string()));
            let (s_tx, s_rx) = std::sync::mpsc::channel();
            let tx_clone_2 = tx.clone();
            thread::spawn(move || { while let Ok(msg) = s_rx.recv() { let _ = tx_clone_2.send(AdbEvent::Status(msg)); } });

            if let Err(sort_error) = sort::sort_game_files(s_tx) {
                let _ = tx.send(AdbEvent::Status(format!("Sort Failed: {}", sort_error)));
            } else {
                let _ = tx.send(AdbEvent::Status("Region processed successfully.".to_string()));
            }
            thread::sleep(Duration::from_secs(1));
        }

        let _ = tx.send(AdbEvent::Status("Stopping ADB Server...".to_string()));
        let _ = driver::run_command(&["kill-server"]);

        let _ = tx.send(AdbEvent::Success("All Operations Complete!".to_string()));
    });
}

fn process_single_region_adb(_tx: &Sender<AdbEvent>, serial: &str, pkg: &str, output_dir: &PathBuf, mode: AdbImportType) -> Result<(), String> {
    if mode == AdbImportType::All {
        let whoami = driver::run_command(&["-s", serial, "shell", "whoami"]).unwrap_or_default();
        let remote_src = format!("/data/data/{}/files", pkg);
        let remote_stage_target = "/data/local/tmp/files";

        let _ = driver::run_command(&["-s", serial, "shell", "rm", "-rf", remote_stage_target]); 
        
        let mut success = false;
        if whoami.contains("root") {
            success = driver::run_command(&["-s", serial, "shell", "cp", "-r", &remote_src, "/data/local/tmp"]).is_ok();
        }
        if !success {
            let cmd = format!("'cp -r {} /data/local/tmp'", remote_src);
            success = driver::run_command(&["-s", serial, "shell", "su", "-c", &cmd]).is_ok();
        }
        
        if !success { return Err("Root Copy Failed. Device might not be rooted.".to_string()); }

        let _ = driver::run_command(&["-s", serial, "shell", "chmod", "-R", "777", remote_stage_target]);
        if !output_dir.exists() { std::fs::create_dir_all(&output_dir).unwrap(); }

        // Windows cannot handle files with ":" in the name
        // We delete them from the staging area on the device BEFORE pulling
        let _ = driver::run_command(&["-s", serial, "shell", "find", remote_stage_target, "-name", "*:*", "-delete"]);

        // --- PULL & VERIFY ---
        let pull_res = driver::run_command(&["-s", serial, "pull", remote_stage_target, output_dir.to_str().unwrap()]);
        
        if pull_res.is_err() {
            return Err("ADB Pull Failed.".to_string());
        }

        let file_count = std::fs::read_dir(output_dir).map(|iter| iter.count()).unwrap_or(0);
        if file_count == 0 {
             return Err("Pull verification failed: Output directory is empty.".to_string());
        }

        let _ = driver::run_command(&["-s", serial, "shell", "rm", "-rf", remote_stage_target]);
    } 

    if let Err(_) = pull_split_apk(serial, pkg, "split_InstallPack.apk", output_dir) {
        return Err("APK Pull Failed.".to_string());
    }
    
    let apk_path = output_dir.join("split_InstallPack.apk");
    if !apk_path.exists() || apk_path.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
         return Err("APK verification failed: File missing or empty.".to_string());
    }

    Ok(())
}

fn pull_split_apk(serial: &str, pkg: &str, target: &str, out_dir: &Path) -> Result<(), String> {
    let cmd_out = driver::run_command(&["-s", serial, "shell", "pm", "path", pkg])?;
    let remote_path = cmd_out.lines().find(|line| line.contains("base.apk"))
        .ok_or("APK Path not found.")?.trim().strip_prefix("package:").unwrap_or("")
        .replace("base.apk", target);

    let local_path = out_dir.join(target);
    if !out_dir.exists() { std::fs::create_dir_all(&out_dir).unwrap_or_default(); }
    driver::run_command(&["-s", serial, "pull", &remote_path, local_path.to_str().unwrap()])?;
    Ok(())
}