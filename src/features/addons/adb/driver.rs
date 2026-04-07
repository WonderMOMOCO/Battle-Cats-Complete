use std::process::Command;
use std::thread;
use std::time::Duration;
use super::download;

pub fn get_adb_command() -> Result<std::path::PathBuf, String> {
    let Some(adb_path) = download::get_adb_path() else {
        return Err("ADB not found. Please download it in Settings > Add-Ons.".to_string());
    };
    Ok(adb_path)
}

pub fn run_command(arguments: &[&str]) -> Result<String, String> {
    let adb_path = get_adb_command()?;
    let mut command_process = Command::new(adb_path);
    command_process.args(arguments);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command_process.creation_flags(0x08000000);
    }

    let command_output = command_process.output().map_err(|error| error.to_string())?;
    
    if !command_output.status.success() {
        return Err(String::from_utf8_lossy(&command_output.stderr).trim().to_string());
    }
    
    Ok(String::from_utf8_lossy(&command_output.stdout).trim().to_string())
}

pub fn find_usb_device() -> Option<String> {
    let devices_output = run_command(&["devices"]).ok()?;
    
    for line in devices_output.lines().skip(1) {
        if line.trim().is_empty() { continue; }
        
        let Some((serial_number, status)) = line.split_once('\t') else { continue; };
        if status != "device" { continue; }
        if serial_number.contains(':') { continue; }
        if serial_number.starts_with("emulator") { continue; }
        
        return Some(serial_number.to_string());
    }
    
    None
}

pub fn find_mdns_device() -> Option<String> {
    let _ = run_command(&["mdns", "check"]);
    
    for _ in 0..6 {
        if let Ok(services_output) = run_command(&["mdns", "services"]) {
            for line in services_output.lines() {
                if !line.contains("_adb-tls-connect._tcp") { continue; }
                
                let Some(ip_and_port) = line.split_whitespace().last() else { continue; };
                if !ip_and_port.contains(':') || !ip_and_port.contains('.') { continue; }
                
                return Some(ip_and_port.to_string());
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    None
}

pub fn connect_manual_ip(ip_address: &str) -> Result<String, String> {
    let target_address = if ip_address.contains(':') { 
        ip_address.to_string() 
    } else { 
        format!("{}:5555", ip_address) 
    };
    
    let connection_output = run_command(&["connect", &target_address])?;
    
    if !connection_output.contains("connected") {
        return Err(connection_output);
    }
    
    Ok(target_address)
}

pub fn find_emulator() -> Option<String> {
    let default_ports = [7555, 5555, 62001, 21503, 16384]; 
    
    if let Ok(devices_output) = run_command(&["devices"]) {
        for line in devices_output.lines().skip(1) {
            if line.trim().is_empty() { continue; }
            
            let Some((serial_number, status)) = line.split_once('\t') else { continue; };
            if status != "device" { continue; }
            
            let is_emulator_name = serial_number.starts_with("emulator");
            let is_local_ip = serial_number.contains("127.0.0.1") || serial_number.contains("localhost");
            
            if is_emulator_name || is_local_ip {
                return Some(serial_number.to_string());
            }
        }
    }
    
    for port_number in default_ports {
        let address = format!("127.0.0.1:{}", port_number);
        let Ok(connection_output) = run_command(&["connect", &address]) else { continue; };
        
        if connection_output.contains("connected") {
            return Some(address);
        }
    }
    
    None
}

pub fn get_wlan_ip(serial_number: &str) -> Option<String> {
    let route_output = run_command(&["-s", serial_number, "shell", "ip", "route"]).ok()?;
    
    for line in route_output.lines() {
        if !line.contains("wlan0") || !line.contains("src") { continue; }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some(source_position) = parts.iter().position(|&word| word == "src") else { continue; };
        let Some(ip_address) = parts.get(source_position + 1) else { continue; };
        
        return Some(ip_address.to_string());
    }
    
    None
}

pub fn enable_wireless_fallback(serial_number: &str) -> Option<String> {
    if serial_number.contains(':') || serial_number.starts_with("emulator") { return None; }
    
    let ip_address = get_wlan_ip(serial_number)?;
    let _ = run_command(&["-s", serial_number, "tcpip", "5555"]);
    thread::sleep(Duration::from_secs(2)); 
    
    Some(format!("{}:5555", ip_address))
}

pub fn connect_wireless(ip_address: &str) -> Result<(), String> {
    let connection_output = run_command(&["connect", ip_address])?;
    
    if !connection_output.contains("connected") { 
        return Err(connection_output); 
    }
    
    Ok(())
}

pub fn bootstrap_tcpip(serial_number: &str) -> Option<String> {
    let ip_address = serial_number.split(':').next()?;
    let _ = run_command(&["-s", serial_number, "tcpip", "5555"]);
    thread::sleep(Duration::from_secs(2));
    
    Some(format!("{}:5555", ip_address))
}

pub fn verify_connection(serial_number: &str) -> Result<(), String> {
    let device_state = run_command(&["-s", serial_number, "get-state"])
        .map_err(|_| "Device is not responding. (Is Wireless Debugging ON?)".to_string())?;

    if device_state.contains("device") {
        return Ok(());
    } 
    
    if device_state.contains("unauthorized") {
        return Err("Device is UNAUTHORIZED. Check phone screen.".to_string());
    } 
    
    if device_state.contains("offline") {
        return Err("Device is OFFLINE. Toggle Wireless Debugging OFF and ON again.".to_string());
    } 
    
    Err(format!("Device state unknown: {}", device_state))
}