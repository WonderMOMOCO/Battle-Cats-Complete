use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use super::driver; 
use crate::features::data::state::{AdbImportType, AdbRegion};
use crate::features::settings::logic::state::EmulatorConfig;

pub fn execute_pull(
    base_output_directory: &PathBuf, 
    import_mode: AdbImportType, 
    target_region: AdbRegion, 
    emulator_config: &EmulatorConfig,
    status_sender: &Sender<String>,
    abort_flag: &AtomicBool
) -> Result<Vec<PathBuf>, String> {
    
    let _ = status_sender.send("Starting ADB Server...".to_string());
    let _ = driver::run_command(&["kill-server"]);
    thread::sleep(Duration::from_millis(500));
    let _ = driver::run_command(&["start-server"]);
    
    if abort_flag.load(Ordering::Relaxed) { return Err("Aborted".into()); }

    let mut current_serial: String = String::new();
    let mut fallback_ip_address: Option<String> = None;
    let mut is_connection_established = false;

    let _ = status_sender.send("Detecting device...".to_string());

    if let Some(usb_serial) = driver::find_usb_device() {
        if driver::verify_connection(&usb_serial).is_ok() {
            let _ = status_sender.send(format!("USB Device Found: {}", usb_serial));
            current_serial = usb_serial.clone();
            fallback_ip_address = driver::enable_wireless_fallback(&current_serial);
            is_connection_established = true;
        }
    }
    
    if !is_connection_established {
        let _ = status_sender.send("Scanning network for Wireless Debugging...".to_string());
        if let Some(mdns_target) = driver::find_mdns_device() {
            let _ = status_sender.send(format!("Found via mDNS: {}", mdns_target));
            if driver::connect_manual_ip(&mdns_target).is_ok() {
                if let Some(stable_ip) = driver::bootstrap_tcpip(&mdns_target) {
                    let _ = driver::run_command(&["disconnect", &mdns_target]);
                    if let Ok(stable_serial) = driver::connect_manual_ip(&stable_ip) {
                        if driver::verify_connection(&stable_serial).is_ok() {
                            current_serial = stable_serial;
                            is_connection_established = true;
                            let _ = status_sender.send("Auto-Connection Successful!".to_string());
                        }
                    }
                }
            }
        }
    }

    if !is_connection_established && !emulator_config.manual_ip.is_empty() {
        let _ = status_sender.send(format!("Trying Manual IP: {}", emulator_config.manual_ip));
        if let Ok(initial_ip) = driver::connect_manual_ip(&emulator_config.manual_ip) {
            let mut test_serial = initial_ip.clone();
            
            if initial_ip.contains(':') && !initial_ip.ends_with(":5555") {
                if let Some(new_target) = driver::bootstrap_tcpip(&initial_ip) {
                    let _ = driver::run_command(&["disconnect", &initial_ip]);
                    if let Ok(stable_ip) = driver::connect_manual_ip(&new_target) { 
                        test_serial = stable_ip; 
                    }
                }
            }
            
            if driver::verify_connection(&test_serial).is_ok() {
                current_serial = test_serial;
                is_connection_established = true;
            } else {
                let _ = status_sender.send("Manual IP failed verification. Scanning for Emulators...".to_string());
            }
        }
    }

    if !is_connection_established {
        let _ = status_sender.send("Scanning for Emulators...".to_string());
        if let Some(emulator_serial) = driver::find_emulator() {
            if driver::verify_connection(&emulator_serial).is_ok() {
                current_serial = emulator_serial;
                is_connection_established = true;
            }
        }
    }

    if !is_connection_established {
        return Err("No device found. Ensure Wireless Debugging is ON or Emulator is running.".to_string());
    }

    let _ = status_sender.send("Device Verified.".to_string());
    if abort_flag.load(Ordering::Relaxed) { return Err("Aborted".into()); }

    if import_mode == AdbImportType::All {
        let _ = status_sender.send("Checking Root Permissions...".to_string());
        let root_test_output = driver::run_command(&["-s", &current_serial, "shell", "su", "-c", "echo root_test"]).unwrap_or_default();

        if root_test_output.contains("root_test") {
            let _ = status_sender.send("Root access confirmed via su.".to_string());
        } else {
            let _ = status_sender.send("Requesting Root Access...".to_string());
            let _ = driver::run_command(&["-s", &current_serial, "root"]);
            thread::sleep(Duration::from_secs(3));
            
            if abort_flag.load(Ordering::Relaxed) { return Err("Aborted".into()); }
            
            if current_serial.contains(':') {
                let _ = driver::connect_wireless(&current_serial);
            } else if !current_serial.starts_with("emulator") {
                if let Some(new_serial) = driver::find_usb_device() { 
                    current_serial = new_serial; 
                }
            }
            
            let _ = status_sender.send("Waiting for device to reconnect...".to_string());
            let _ = driver::run_command(&["-s", &current_serial, "wait-for-device"]);
        }
    }

    let regions_to_process = match target_region {
        AdbRegion::All => vec![AdbRegion::English, AdbRegion::Japanese, AdbRegion::Taiwan, AdbRegion::Korean],
        _ => vec![target_region],
    };

    let mut successful_pulls = Vec::new();

    for current_region in regions_to_process.iter() {
        if abort_flag.load(Ordering::Relaxed) { return Err("Aborted".into()); }

        let region_suffix = current_region.suffix();
        let package_name = format!("jp.co.ponos.battlecats{}", region_suffix);
        let check_installed_output = driver::run_command(&["-s", &current_serial, "shell", "pm", "path", &package_name]).unwrap_or_default();
        
        if check_installed_output.trim().is_empty() || check_installed_output.contains("Error") {
            let _ = status_sender.send(format!("Skipping {}: Not installed.", package_name));
            continue;
        }

        let _ = status_sender.send(format!("Pulling {}...", package_name));
        let target_directory = base_output_directory.join(&package_name);

        let process_result = process_single_region_adb(status_sender, &current_serial, &package_name, &target_directory, import_mode.clone());
        
        if process_result.is_ok() {
            successful_pulls.push(target_directory);
            continue;
        }
        
        let process_error = process_result.unwrap_err();
        let is_app_warning = process_error.contains("Root Copy Failed") || process_error.contains("APK Path not found") || process_error.contains("Warning:");
        
        if is_app_warning {
            let _ = status_sender.send(format!("Skipping {}: {}", package_name, process_error));
            continue;
        }

        let Some(ref rescue_ip_address) = fallback_ip_address else {
            let _ = status_sender.send(format!("Skipping {} due to error: {}", package_name, process_error));
            continue;
        };
        
        let _ = status_sender.send(format!("Error: {}. Engaging Wireless Rescue...", process_error));
        if driver::connect_wireless(rescue_ip_address).is_ok() {
            current_serial = rescue_ip_address.clone(); 
            if process_single_region_adb(status_sender, &current_serial, &package_name, &target_directory, import_mode.clone()).is_ok() {
                let _ = status_sender.send("Rescue Successful!".to_string());
                successful_pulls.push(target_directory);
            }
        }
    }

    let _ = driver::run_command(&["kill-server"]);

    if successful_pulls.is_empty() {
        return Err("No regions were successfully pulled.".to_string());
    }

    Ok(successful_pulls)
}

fn process_single_region_adb(status_sender: &Sender<String>, serial_number: &str, package_name: &str, output_directory: &Path, import_mode: AdbImportType) -> Result<(), String> {
    if import_mode == AdbImportType::All {
        let user_identity = driver::run_command(&["-s", serial_number, "shell", "whoami"]).unwrap_or_default();
        let remote_source_path = format!("/data/data/{}/files", package_name);
        let remote_staging_directory = "/data/local/tmp";
        let remote_staging_target = "/data/local/tmp/files";

        let _ = driver::run_command(&["-s", serial_number, "shell", "rm", "-rf", remote_staging_target]); 
        let _ = driver::run_command(&["-s", serial_number, "shell", "mkdir", "-p", remote_staging_directory]); 
        
        let mut copy_successful = false;
        
        if user_identity.contains("root") {
            copy_successful = driver::run_command(&["-s", serial_number, "shell", "cp", "-r", &remote_source_path, remote_staging_directory]).is_ok();
        }
        
        if !copy_successful {
            let su_command_string = format!("'cp -r {} {}'", remote_source_path, remote_staging_directory);
            copy_successful = driver::run_command(&["-s", serial_number, "shell", "su", "-c", &su_command_string]).is_ok();
        }
        
        if !copy_successful {
            copy_successful = driver::run_command(&["-s", serial_number, "shell", "su", "0", "cp", "-r", &remote_source_path, remote_staging_directory]).is_ok();
        }
        
        if !copy_successful { 
            return Err("Root Copy Failed. Device might not be rooted.".to_string()); 
        }

        let _ = driver::run_command(&["-s", serial_number, "shell", "chmod", "-R", "777", remote_staging_target]);
        
        if !output_directory.exists() { 
            let _ = std::fs::create_dir_all(output_directory); 
        }
        
        let _ = driver::run_command(&["-s", serial_number, "shell", "find", remote_staging_target, "-name", "*:*", "-delete"]);

        let Some(output_directory_string) = output_directory.to_str() else { 
            return Err("Invalid path on host machine.".to_string()); 
        };
        
        if driver::run_command(&["-s", serial_number, "pull", remote_staging_target, output_directory_string]).is_err() {
            return Err("ADB Pull Failed.".to_string());
        }

        let total_pulled_files = std::fs::read_dir(output_directory.join("files")).map(|iterator| iterator.count()).unwrap_or(0);
        
        if total_pulled_files == 0 { 
            return Err("Pull verification failed: empty directory.".to_string()); 
        }
        
        let _ = driver::run_command(&["-s", serial_number, "shell", "rm", "-rf", remote_staging_target]);
    } 

    let package_manager_output = driver::run_command(&["-s", serial_number, "shell", "pm", "path", package_name]).unwrap_or_default();
    let has_base_apk = package_manager_output.contains("base.apk");

    if pull_target_apk(serial_number, &package_manager_output, "split_InstallPack.apk", output_directory).is_err() {
        if has_base_apk {
            return Err("Warning: File modification suspected, do a clean install on device.".to_string());
        } 
        let _ = status_sender.send("Warning: Update APK missing.".to_string());
    }
    Ok(())
}

fn pull_target_apk(serial_number: &str, package_manager_output: &str, target_filename: &str, output_directory: &Path) -> Result<(), String> {
    let mut remote_apk_path = package_manager_output.lines().find(|line| line.contains(target_filename))
        .map(|line| line.trim().strip_prefix("package:").unwrap_or("").to_string());

    if remote_apk_path.is_none() {
        remote_apk_path = package_manager_output.lines().find(|line| line.contains("base.apk"))
            .map(|line| line.trim().strip_prefix("package:").unwrap_or("").replace("base.apk", target_filename));
    }

    let Some(final_remote_path) = remote_apk_path else {
        return Err("APK Path not found on device.".to_string());
    };

    let local_destination_path = output_directory.join(target_filename);
    
    if !output_directory.exists() { 
        let _ = std::fs::create_dir_all(output_directory); 
    }
    
    let Some(local_destination_string) = local_destination_path.to_str() else { 
        return Err("Invalid path on host machine.".to_string()); 
    };
    
    let _ = driver::run_command(&["-s", serial_number, "pull", &final_remote_path, local_destination_string])?;
    
    let downloaded_apk_size = local_destination_path.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    
    if !local_destination_path.exists() || downloaded_apk_size == 0 {
        let _ = std::fs::remove_file(&local_destination_path);
        return Err("APK verification failed after pull.".to_string());
    }
    
    Ok(())
}