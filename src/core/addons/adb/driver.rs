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
pub fn find_usb_device() -> Option<String> {
    if let Ok(devices_out) = run_command(&["devices"]) {
        for line in devices_out.lines().skip(1) {
            if line.trim().is_empty() { continue; }
            if let Some((serial, status)) = line.split_once('\t') {
                if status == "device" && !serial.contains(":") && !serial.starts_with("emulator") {
                    return Some(serial.to_string());
                }
            }
        }
    }
    None
}

// --- PRIORITY 2: MDNS AUTO-DISCOVERY ---
// Retries for 3 seconds (6 x 500ms) to allow the ADB daemon to catch the broadcast
pub fn find_mdns_device() -> Option<String> {
    let _ = run_command(&["mdns", "check"]);
    
    for _ in 0..6 {
        if let Ok(output) = run_command(&["mdns", "services"]) {
            for line in output.lines() {
                if line.contains("_adb-tls-connect._tcp") {
                    if let Some(ip_port) = line.split_whitespace().last() {
                        if ip_port.contains(':') && ip_port.contains('.') {
                            return Some(ip_port.to_string());
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    None
}

// --- PRIORITY 3: MANUAL IP ---
pub fn connect_manual_ip(ip: &str) -> Result<String, String> {
    let target = if ip.contains(":") { ip.to_string() } else { format!("{}:5555", ip) };
    
    let out = run_command(&["connect", &target])?;
    if out.contains("connected") {
        Ok(target)
    } else {
        Err(out)
    }
}

// --- PRIORITY 4: EMULATOR ---
pub fn find_emulator() -> Option<String> {
    let ports = [7555, 5555, 62001, 21503, 16384]; 
    if let Ok(devices_out) = run_command(&["devices"]) {
        for line in devices_out.lines().skip(1) {
            if line.trim().is_empty() { continue; }
            if let Some((serial, status)) = line.split_once('\t') {
                if status == "device" {
                    if serial.starts_with("emulator") || serial.contains("127.0.0.1") || serial.contains("localhost") {
                        return Some(serial.to_string());
                    }
                }
            }
        }
    }
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

pub fn bootstrap_tcpip(serial: &str) -> Option<String> {
    let ip = serial.split(':').next()?;
    let _ = run_command(&["-s", serial, "tcpip", "5555"]);
    thread::sleep(Duration::from_secs(2));
    Some(format!("{}:5555", ip))
}

pub fn verify_connection(serial: &str) -> Result<(), String> {
    let state = run_command(&["-s", serial, "get-state"])
        .map_err(|_| "Device is not responding. (Is Wireless Debugging ON?)".to_string())?;

    if state.contains("device") {
        Ok(())
    } else if state.contains("unauthorized") {
        Err("Device is UNAUTHORIZED. Check phone screen.".to_string())
    } else if state.contains("offline") {
        Err("Device is OFFLINE. Toggle Wireless Debugging OFF and ON again.".to_string())
    } else {
        Err(format!("Device state unknown: {}", state))
    }
}