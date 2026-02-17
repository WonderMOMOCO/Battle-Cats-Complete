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

pub fn connect_to_emulator() -> Result<String, String> {
    let ports = [7555, 5555, 62001, 21503, 16384]; 
    
    let devices_out = run_command(&["devices"])?;

    // 1. Check if we are already connected wirelessly (look for IP format)
    for line in devices_out.lines() {
        if line.contains(":5555") && line.contains("\tdevice") {
            return Ok("Wireless device already connected".to_string());
        }
    }

    // 2. Check for a USB device to bootstrap
    let mut usb_serial = None;
    for line in devices_out.lines().skip(1) {
         if let Some((serial, status)) = line.split_once('\t') {
             // If it doesn't have a colon (IP) and is authorized
             if status == "device" && !serial.contains(":") { 
                 usb_serial = Some(serial.to_string());
                 break;
             }
         }
    }

    // 3. Bootstrap Wireless if USB found
    if let Some(serial) = usb_serial {
        // Try to get the IP address from the device
        if let Some(ip) = get_wlan_ip(&serial) {
            // Switch device to TCP/IP mode
            let _ = run_command(&["-s", &serial, "tcpip", "5555"]);
            
            // Wait a moment for adbd to restart on the phone
            thread::sleep(Duration::from_secs(2)); 
            
            // Connect to the IP we found
            let connect_res = run_command(&["connect", &format!("{}:5555", ip)]);
            if let Ok(res) = connect_res {
                if res.contains("connected") {
                     return Ok(format!("Switched to Wireless: {}", ip));
                }
            }
        }
    }

    // 4. Fallback: Attempt to connect to local emulator ports
    for port in ports {
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(out) = run_command(&["connect", &addr]) {
            if out.contains("connected") {
                return Ok(format!("Connected to {}", addr));
            }
        }
    }
    Err("No devices or emulators found.".to_string())
}

// Helper to find the device's local IP via ADB
fn get_wlan_ip(serial: &str) -> Option<String> {
    // Run 'ip route' to find the src IP for the wlan0 interface
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