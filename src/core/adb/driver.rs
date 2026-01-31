use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
const ADB_EXE_WIN: &[u8] = include_bytes!("../../assets/adb/win/adb.exe");
#[cfg(target_os = "windows")]
const ADB_API_WIN: &[u8] = include_bytes!("../../assets/adb/win/AdbWinApi.dll");
#[cfg(target_os = "windows")]
const ADB_USB_WIN: &[u8] = include_bytes!("../../assets/adb/win/AdbWinUsbApi.dll");

#[cfg(target_os = "linux")]
const ADB_BIN_LINUX: &[u8] = include_bytes!("../../assets/adb/linux/adb");

pub fn get_adb_command() -> Result<PathBuf, String> {
    let temp_dir = env::temp_dir().join("battle_cats_manager_adb");
    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        let adb_path = temp_dir.join("adb.exe");
        let dll_api = temp_dir.join("AdbWinApi.dll");
        let dll_usb = temp_dir.join("AdbWinUsbApi.dll");

        if !adb_path.exists() { fs::write(&adb_path, ADB_EXE_WIN).map_err(|e| e.to_string())?; }
        if !dll_api.exists() { fs::write(&dll_api, ADB_API_WIN).map_err(|e| e.to_string())?; }
        if !dll_usb.exists() { fs::write(&dll_usb, ADB_USB_WIN).map_err(|e| e.to_string())?; }

        Ok(adb_path)
    }

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        let adb_path = temp_dir.join("adb");

        if !adb_path.exists() {
            fs::write(&adb_path, ADB_BIN_LINUX).map_err(|e| e.to_string())?;
            let mut perms = fs::metadata(&adb_path).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&adb_path, perms).map_err(|e| e.to_string())?;
        }
        Ok(adb_path)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    Err("Unsupported OS for bundled ADB.".to_string())
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
    let ports = [7555, 5555, 62001, 21503, 16384]; // Common emulator ports
    
    let devices = run_command(&["devices"])?;
    if devices.lines().count() > 2 {
        return Ok("Device already connected".to_string());
    }

    for port in ports {
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(out) = run_command(&["connect", &addr]) {
            if out.contains("connected") {
                return Ok(format!("Connected to {}", addr));
            }
        }
    }
    Err("No emulators found.".to_string())
}