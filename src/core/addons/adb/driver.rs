use std::process::Command;
use std::thread;
use std::time::Duration;
use super::download;

pub fn get_adb_command() -> Result<std::path::PathBuf, String> {
    if let Some(path) = download::get_adb_path() {
        Ok(path)
    } else {
        Err("ADB not found. Please download it in Settings > Add-Ons.".to_string())
    }
}

pub fn run_command(args: &[&str]) -> Result<String, String> {
    let adb_path = get_adb_command()?;
    let mut cmd = Command::new(adb_path);
    cmd.args(args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd.output().map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// --- PRIORITY 1: USB ---
// Returns Serial if found
pub fn find_usb_device() -> Option<String> {
    if let Ok(devices_out) = run_command(&["devices"]) {
        for line in devices_out.lines().skip(1) {
            if line.trim().is_empty() { continue; }
            if let Some((serial, status)) = line.split_once('\t') {
                // USB devices do not have IPs (colon) and are not 'emulator-xxxx'
                if status == "device" && !serial.contains(":") && !serial.starts_with("emulator") {
                    return Some(serial.to_string());
                }
            }
        }
    }
    None
}

// --- PRIORITY 2: MANUAL IP ---
// Returns Connected IP if successful
pub fn connect_manual_ip(ip: &str) -> Result<String, String> {
    let target = if ip.contains(":") { ip.to_string() } else { format!("{}:5555", ip) };
    
    let out = run_command(&["connect", &target])?;
    if out.contains("connected") {
        Ok(target)
    } else {
        Err(out)
    }
}

// --- PRIORITY 3: EMULATOR ---
// Returns Emulator Serial if found
pub fn find_emulator() -> Option<String> {
    let ports = [7555, 5555, 62001, 21503, 16384]; 
    
    // \Check if an emulator is ALREADY connected
    if let Ok(devices_out) = run_command(&["devices"]) {
        for line in devices_out.lines().skip(1) {
            if line.trim().is_empty() { continue; }
            if let Some((serial, status)) = line.split_once('\t') {
                if status == "device" {
                    // "emulator-5554" OR "127.0.0.1:5555" are emulators
                    if serial.starts_with("emulator") || serial.contains("127.0.0.1") || serial.contains("localhost") {
                        return Some(serial.to_string());
                    }
                }
            }
        }
    }

    // Scan ports
    for port in ports {
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(out) = run_command(&["connect", &addr]) {
            if out.contains("connected") {
                return Some(addr);
            }
        }
    }
    None
}

// --- UTILS ---

pub fn get_wlan_ip(serial: &str) -> Option<String> {
    if let Ok(output) = run_command(&["-s", serial, "shell", "ip", "route"]) {
        for line in output.lines() {
            if line.contains("wlan0") && line.contains("src") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pos) = parts.iter().position(|&x| x == "src") {
                    if let Some(ip) = parts.get(pos + 1) {
                        return Some(ip.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn enable_wireless_fallback(serial: &str) -> Option<String> {
    // If it's already wireless/emulator, no fallback needed
    if serial.contains(":") || serial.starts_with("emulator") { return None; }
    
    let ip = get_wlan_ip(serial)?;
    let _ = run_command(&["-s", serial, "tcpip", "5555"]);
    thread::sleep(Duration::from_secs(2)); 
    Some(format!("{}:5555", ip))
}

pub fn connect_wireless(ip: &str) -> Result<(), String> {
    let out = run_command(&["connect", ip])?;
    if out.contains("connected") { Ok(()) } else { Err(out) }
}