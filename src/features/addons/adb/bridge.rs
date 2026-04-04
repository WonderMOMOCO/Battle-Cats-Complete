use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::fs;
use super::driver; 
use crate::features::import::logic::{AdbImportType, AdbRegion};
use crate::features::import::logic::decrypt;
use crate::features::import::sort;
use crate::features::settings::logic::state::EmulatorConfig;

pub fn spawn_full_import(
    sender: Sender<String>, 
    base_output_dir: PathBuf, 
    mode: AdbImportType, 
    region: AdbRegion, 
    config: EmulatorConfig,
    abort_flag: Arc<AtomicBool>,
    status: Arc<AtomicU8>,
    prog_curr: Arc<AtomicUsize>,
    prog_max: Arc<AtomicUsize>
) {
    thread::spawn(move || {
        let terminate = |s: Arc<AtomicU8>, is_err: bool| { s.store(if is_err { 3 } else { 2 }, Ordering::Relaxed); };

        let _ = sender.send("Starting ADB Server...".to_string());
        let _ = driver::run_command(&["kill-server"]);
        thread::sleep(Duration::from_millis(500));
        let _ = driver::run_command(&["start-server"]);
        
        if abort_flag.load(Ordering::Relaxed) { return terminate(status, true); }

        let mut current_serial: String = String::new();
        let mut fallback_ip: Option<String> = None;
        let mut connection_established = false;

        let _ = sender.send("Detecting device...".to_string());

        let try_usb = || -> Option<String> {
            let serial = driver::find_usb_device()?;
            driver::verify_connection(&serial).ok()?;
            Some(serial)
        };

        if let Some(serial) = try_usb() {
            let _ = sender.send(format!("USB Device Found: {}", serial));
            current_serial = serial.clone();
            fallback_ip = driver::enable_wireless_fallback(&current_serial);
            connection_established = true;
        }
        
        if !connection_established {
            let _ = sender.send("Scanning network for Wireless Debugging...".to_string());
            let try_mdns = || -> Option<String> {
                let mdns_target = driver::find_mdns_device()?;
                let _ = sender.send(format!("Found via mDNS: {}", mdns_target));
                driver::connect_manual_ip(&mdns_target).ok()?;
                let stable_ip = driver::bootstrap_tcpip(&mdns_target)?;
                let _ = driver::run_command(&["disconnect", &mdns_target]);
                let stable_serial = driver::connect_manual_ip(&stable_ip).ok()?;
                driver::verify_connection(&stable_serial).ok()?;
                Some(stable_serial)
            };

            if let Some(stable_serial) = try_mdns() {
                current_serial = stable_serial;
                connection_established = true;
                let _ = sender.send("Auto-Connection Successful!".to_string());
            }
        }

        if !connection_established && !config.manual_ip.is_empty() {
            let _ = sender.send(format!("Trying Manual IP: {}", config.manual_ip));
            let try_manual_ip = || -> Option<String> {
                let initial_ip = driver::connect_manual_ip(&config.manual_ip).ok()?;
                let mut test_serial = initial_ip.clone();
                if initial_ip.contains(':') && !initial_ip.ends_with(":5555") {
                    if let Some(new_target) = driver::bootstrap_tcpip(&initial_ip) {
                        let _ = driver::run_command(&["disconnect", &initial_ip]);
                        if let Ok(stable_ip) = driver::connect_manual_ip(&new_target) { test_serial = stable_ip; }
                    }
                }
                driver::verify_connection(&test_serial).ok()?;
                Some(test_serial)
            };

            if let Some(serial) = try_manual_ip() {
                current_serial = serial;
                connection_established = true;
            } else {
                let _ = sender.send("Manual IP failed verification. Scanning for Emulators...".to_string());
            }
        }

        if !connection_established {
            let _ = sender.send("Scanning for Emulators...".to_string());
            let try_emulator = || -> Option<String> {
                let emulator = driver::find_emulator()?;
                driver::verify_connection(&emulator).ok()?;
                Some(emulator)
            };
            if let Some(emulator) = try_emulator() {
                current_serial = emulator;
                connection_established = true;
            }
        }

        if !connection_established {
            let _ = sender.send("Error: No device found. Ensure Wireless Debugging is ON or Emulator is running.".to_string());
            return terminate(status, true);
        }

        let _ = sender.send("Device Verified.".to_string());
        if abort_flag.load(Ordering::Relaxed) { return terminate(status, true); }

        if mode == AdbImportType::All {
            let _ = sender.send("Checking Root Permissions...".to_string());
            let is_rooted = driver::run_command(&["-s", &current_serial, "shell", "su", "-c", "echo root_test"]).unwrap_or_default();

            if is_rooted.contains("root_test") {
                let _ = sender.send("Root access confirmed via su.".to_string());
            } else {
                let _ = sender.send("Requesting Root Access (ADB Root)...".to_string());
                let _ = driver::run_command(&["-s", &current_serial, "root"]);
                thread::sleep(Duration::from_secs(3));
                if abort_flag.load(Ordering::Relaxed) { return terminate(status, true); }
                
                if current_serial.contains(':') {
                     let _ = driver::connect_wireless(&current_serial);
                } else if !current_serial.starts_with("emulator") {
                     if let Some(new_serial) = driver::find_usb_device() { current_serial = new_serial; }
                }
                
                let _ = sender.send("Waiting for device to reconnect...".to_string());
                let _ = driver::run_command(&["-s", &current_serial, "wait-for-device"]);
            }
        }

        let regions_to_process = match region {
            AdbRegion::All => vec![AdbRegion::English, AdbRegion::Japanese, AdbRegion::Taiwan, AdbRegion::Korean],
            _ => vec![region],
        };

        let mut successful_pulls = Vec::new();

        for current_region in regions_to_process.iter() {
            if abort_flag.load(Ordering::Relaxed) { return terminate(status, true); }

            let suffix = current_region.suffix();
            let package_name = format!("jp.co.ponos.battlecats{}", suffix);
            let check_installed = driver::run_command(&["-s", &current_serial, "shell", "pm", "path", &package_name]).unwrap_or_default();
            
            if check_installed.trim().is_empty() || check_installed.contains("Error") {
                let _ = sender.send(format!("Skipping {}: Not installed.", package_name));
                continue;
            }

            let _ = sender.send(format!("Pulling {}...", package_name));
            let target_dir = base_output_dir.join(&package_name);

            let process_result = process_single_region_adb(&sender, &current_serial, &package_name, &target_dir, mode.clone());
            if process_result.is_ok() {
                successful_pulls.push((current_region.clone(), package_name, target_dir));
                continue;
            }
            
            let process_error = process_result.unwrap_err();
            let is_app_warning = process_error.contains("Root Copy Failed") || process_error.contains("APK Path not found") || process_error.contains("Warning:");
            
            if is_app_warning {
                let _ = sender.send(format!("Skipping {}: {}", package_name, process_error));
                continue;
            }

            let Some(ref rescue_ip) = fallback_ip else {
                let _ = sender.send(format!("Skipping {} due to error: {}", package_name, process_error));
                continue;
            };
            
            let _ = sender.send(format!("Error: {}. Engaging Wireless Rescue...", process_error));
            if driver::connect_wireless(rescue_ip).is_ok() {
                current_serial = rescue_ip.clone(); 
                if process_single_region_adb(&sender, &current_serial, &package_name, &target_dir, mode.clone()).is_ok() {
                    let _ = sender.send("Rescue Successful!".to_string());
                    successful_pulls.push((current_region.clone(), package_name, target_dir));
                }
            }
        }

        let _ = driver::run_command(&["kill-server"]);

        if successful_pulls.is_empty() {
             let _ = sender.send("Error: No regions were successfully pulled.".to_string());
             return terminate(status, true);
        }

        let _ = sender.send("Starting Processing Phase...".to_string());
        let mut master_workspace_index = decrypt::build_index(Path::new("game"));

        for (current_region, pkg_name, target_dir) in &successful_pulls {
            if abort_flag.load(Ordering::Relaxed) { return terminate(status, true); }

            let suffix = current_region.suffix();
            let region_code = match suffix { "" => "ja", "kr" => "ko", other => other };
            
            let _ = sender.send(format!("Decrypting {}...", pkg_name));
            let Some(target_dir_str) = target_dir.to_str() else { continue; };
            
            let (dec_tx, dec_rx) = std::sync::mpsc::channel();
            let s_clone = sender.clone();
            thread::spawn(move || { while let Ok(msg) = dec_rx.recv() { let _ = s_clone.send(msg); } });

            if let Err(e) = decrypt::run(target_dir_str, region_code, &mut master_workspace_index, dec_tx, abort_flag.clone(), prog_curr.clone(), prog_max.clone()) {
                let _ = sender.send(format!("Decryption Failed for {}: {}", pkg_name, e));
                if abort_flag.load(Ordering::Relaxed) { return terminate(status, true); }
                continue;
            }

            if abort_flag.load(Ordering::Relaxed) { return terminate(status, true); }

            let _ = sender.send(format!("Sorting {} assets...", pkg_name));
            let (sort_tx, sort_rx) = std::sync::mpsc::channel();
            let s_clone_sort = sender.clone();
            thread::spawn(move || { while let Ok(msg) = sort_rx.recv() { let _ = s_clone_sort.send(msg); } });

            if let Err(sort_error) = sort::sort_game_files(sort_tx, abort_flag.clone(), prog_curr.clone(), prog_max.clone()) {
                let _ = sender.send(format!("Sort Failed for {}: {}", pkg_name, sort_error));
            }
            thread::sleep(Duration::from_millis(500));
        }

        if !config.keep_app_folder {
            let _ = sender.send("Cleaning up temporary app files...".to_string());
            if base_output_dir.exists() { let _ = fs::remove_dir_all(&base_output_dir); }
        }

        let _ = sender.send("All Operations Complete!".to_string());
        terminate(status, false);
    });
}

fn process_single_region_adb(sender: &Sender<String>, serial: &str, package_name: &str, output_dir: &PathBuf, mode: AdbImportType) -> Result<(), String> {
    if mode == AdbImportType::All {
        let whoami = driver::run_command(&["-s", serial, "shell", "whoami"]).unwrap_or_default();
        let remote_src = format!("/data/data/{}/files", package_name);
        let remote_stage_dir = "/data/local/tmp";
        let remote_stage_target = "/data/local/tmp/files";

        let _ = driver::run_command(&["-s", serial, "shell", "rm", "-rf", remote_stage_target]); 
        let _ = driver::run_command(&["-s", serial, "shell", "mkdir", "-p", remote_stage_dir]); 
        
        let mut success = false;
        if whoami.contains("root") {
            success = driver::run_command(&["-s", serial, "shell", "cp", "-r", &remote_src, remote_stage_dir]).is_ok();
        }
        if !success {
            let command_string = format!("'cp -r {} {}'", remote_src, remote_stage_dir);
            success = driver::run_command(&["-s", serial, "shell", "su", "-c", &command_string]).is_ok();
        }
        if !success {
            success = driver::run_command(&["-s", serial, "shell", "su", "0", "cp", "-r", &remote_src, remote_stage_dir]).is_ok();
        }
        if !success { return Err("Root Copy Failed. Device might not be rooted.".to_string()); }

        let _ = driver::run_command(&["-s", serial, "shell", "chmod", "-R", "777", remote_stage_target]);
        if !output_dir.exists() { let _ = std::fs::create_dir_all(&output_dir); }
        let _ = driver::run_command(&["-s", serial, "shell", "find", remote_stage_target, "-name", "*:*", "-delete"]);

        let Some(output_dir_str) = output_dir.to_str() else { return Err("Invalid path.".to_string()); };
        if driver::run_command(&["-s", serial, "pull", remote_stage_target, output_dir_str]).is_err() {
            return Err("ADB Pull Failed.".to_string());
        }

        let file_count = std::fs::read_dir(output_dir).map(|iter| iter.count()).unwrap_or(0);
        if file_count == 0 { return Err("Pull verification failed: empty directory.".to_string()); }
        let _ = driver::run_command(&["-s", serial, "shell", "rm", "-rf", remote_stage_target]);
    } 

    let pm_path_output = driver::run_command(&["-s", serial, "shell", "pm", "path", package_name]).unwrap_or_default();
    let has_base = pm_path_output.contains("base.apk");

    if pull_target_apk(serial, &pm_path_output, "split_InstallPack.apk", output_dir).is_err() {
        if has_base {
            return Err("Warning: File modification suspected, do a clean install.".to_string());
        } else {
            let _ = sender.send("Warning: Update APK missing.".to_string());
        }
    }
    Ok(())
}

fn pull_target_apk(serial: &str, pm_path_output: &str, target: &str, output_dir: &Path) -> Result<(), String> {
    let mut remote_path = pm_path_output.lines().find(|line| line.contains(target))
        .map(|line| line.trim().strip_prefix("package:").unwrap_or("").to_string());

    if remote_path.is_none() {
        remote_path = pm_path_output.lines().find(|line| line.contains("base.apk"))
            .map(|line| line.trim().strip_prefix("package:").unwrap_or("").replace("base.apk", target));
    }

    let final_remote_path = remote_path.ok_or("APK Path not found.")?;
    let local_path = output_dir.join(target);
    if !output_dir.exists() { let _ = std::fs::create_dir_all(&output_dir); }
    let Some(local_path_str) = local_path.to_str() else { return Err("Invalid path.".to_string()); };
    
    driver::run_command(&["-s", serial, "pull", &final_remote_path, local_path_str])?;
    let apk_size = local_path.metadata().map(|m| m.len()).unwrap_or(0);
    
    if !local_path.exists() || apk_size == 0 {
        let _ = std::fs::remove_file(&local_path);
        return Err("APK verification failed.".to_string());
    }
    Ok(())
}