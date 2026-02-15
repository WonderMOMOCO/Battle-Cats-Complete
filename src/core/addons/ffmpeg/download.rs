use std::path::PathBuf;
use std::fs;
use std::sync::mpsc::Receiver;

use crate::core::addons::toolpaths::{get_tools_dir, AddonStatus, FFMPEG_BIN};
use crate::core::addons::manager::{self, DownloadConfig};

pub struct FfmpegManager {
    pub status: AddonStatus,
    rx: Option<Receiver<AddonStatus>>,
}

impl Default for FfmpegManager {
    fn default() -> Self {
        Self {
            status: if is_installed() { AddonStatus::Installed } else { AddonStatus::NotInstalled },
            rx: None,
        }
    }
}

impl FfmpegManager {
    pub fn update(&mut self) {
        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.status = msg;
            }
        }
        if let AddonStatus::Installed = self.status {
            self.rx = None;
        }
    }

    pub fn install(&mut self) {
        let asset_name = if cfg!(target_os = "windows") { "ffmpeg_win.zip" } 
                        else if cfg!(target_os = "macos") { "ffmpeg_mac.zip" } 
                        else { "ffmpeg_linux.zip" };

        let config = DownloadConfig {
            folder_name: "ffmpeg".to_string(),
            asset_name: asset_name.to_string(),
            binary_name: FFMPEG_BIN.to_string(),
        };

        self.rx = Some(manager::start_download(config));
        self.status = AddonStatus::Downloading(0.0, "Starting...".to_string());
    }

    pub fn uninstall(&mut self) {
        let dir = get_ffmpeg_dir();
        if dir.exists() { let _ = fs::remove_dir_all(dir); }
        self.status = AddonStatus::NotInstalled;
    }
}

pub fn get_ffmpeg_dir() -> PathBuf { get_tools_dir().join("ffmpeg") }
pub fn get_ffmpeg_path() -> Option<PathBuf> {
    let bin = get_ffmpeg_dir().join(FFMPEG_BIN);
    if bin.exists() { Some(bin) } else { None }
}
pub fn is_installed() -> bool { get_ffmpeg_path().is_some() }