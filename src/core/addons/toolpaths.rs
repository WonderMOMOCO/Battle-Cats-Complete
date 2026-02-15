use std::path::PathBuf;
use std::fs;

#[derive(Clone, PartialEq, Debug)]
pub enum AddonStatus {
    NotInstalled,
    Installed,
    Downloading(f32, String),
    Error(String),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Presence {
    Installed,
    Missing,
}

// BINARY NAMES
#[cfg(target_os = "windows")]
pub const ADB_BIN: &str = "adb.exe";
#[cfg(not(target_os = "windows"))]
pub const ADB_BIN: &str = "adb";

#[cfg(target_os = "windows")]
pub const AVIF_BIN: &str = "avifenc.exe";
#[cfg(not(target_os = "windows"))]
pub const AVIF_BIN: &str = "avifenc";

#[cfg(target_os = "windows")]
pub const FFMPEG_BIN: &str = "ffmpeg.exe";
#[cfg(not(target_os = "windows"))]
pub const FFMPEG_BIN: &str = "ffmpeg";

pub fn get_tools_dir() -> PathBuf {
    let base_dir = if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "Battle_Cats_Complete") {
        proj_dirs.data_dir().join("tools")
    } else {
        PathBuf::from("tools")
    };

    if !base_dir.exists() {
        let _ = fs::create_dir_all(&base_dir);
    }

    base_dir
}

// STATUS TRACKING

pub fn adb_status() -> Presence {
    if get_tools_dir().join("adb").join(ADB_BIN).exists() { Presence::Installed } else { Presence::Missing }
}

pub fn avifenc_status() -> Presence {
    if get_tools_dir().join("avifenc").join(AVIF_BIN).exists() { Presence::Installed } else { Presence::Missing }
}

pub fn ffmpeg_status() -> Presence {
    if get_tools_dir().join("ffmpeg").join(FFMPEG_BIN).exists() { Presence::Installed } else { Presence::Missing }
}