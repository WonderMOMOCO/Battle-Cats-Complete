use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::env;
use eframe::egui;
use std::path::Path;

pub mod game_data; 
pub mod sort;
pub mod keys;
pub mod decrypt;

use crate::core::addons::adb::bridge::AdbEvent; 
use crate::core::settings::Settings;

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum AdbImportType {
    All,
    Update,
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum AdbRegion {
    English,
    Japanese,
    Taiwan,
    Korean,
    All, 
}

impl AdbRegion {
    pub fn suffix(&self) -> &'static str {
        match self {
            AdbRegion::English => "en",
            AdbRegion::Japanese => "", 
            AdbRegion::Taiwan => "tw",
            AdbRegion::Korean => "kr",
            AdbRegion::All => "all", 
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DataTab {
    Import, 
    Export 
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportSubTab {
    Emulator, 
    Sort,
    Decrypt,
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportMode { None, Folder, Zip }

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ImportState {
    pub active_tab: DataTab,
    pub import_sub_tab: ImportSubTab,
    pub import_path: String,
    #[serde(skip)] pub import_censored: String,
    pub import_mode: ImportMode,
    #[serde(skip)] pub is_adb_busy: bool,
    #[serde(skip)] pub adb_rx: Option<Receiver<AdbEvent>>,
    #[serde(skip)] pub adb_status: String,
    pub adb_import_type: AdbImportType,
    pub adb_region: AdbRegion,
    pub decrypt_path: String,
    #[serde(skip)] pub decrypt_censored: String,
    pub export_filename: String,
    pub compression_level: i32,
    #[serde(skip)] pub status_message: String,
    #[serde(skip)] pub log_content: String,
    #[serde(skip)] pub rx: Option<Receiver<String>>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            active_tab: DataTab::Import,
            import_sub_tab: ImportSubTab::Emulator,
            import_path: String::new(),
            import_censored: String::new(),
            import_mode: ImportMode::Zip,
            is_adb_busy: false,
            adb_rx: None,
            adb_status: String::new(),           
            adb_import_type: AdbImportType::All,
            adb_region: AdbRegion::English,
            decrypt_path: String::new(),
            decrypt_censored: String::new(),
            export_filename: String::new(),
            compression_level: 9,
            status_message: String::new(),
            log_content: String::new(),
            rx: None,
        }
    }
}

impl ImportState {
    pub fn set_decrypt_path(&mut self, path: String) {
        self.decrypt_path = path;
    }

    pub fn update(&mut self, ctx: &egui::Context, settings: &mut Settings) -> bool {
        let mut finished_just_now = false;
        
        self.import_censored = censor_path(&self.import_path);
        self.decrypt_censored = censor_path(&self.decrypt_path);

        if let Some(rx) = &self.rx {
            let mut job_finished = false;
            while let Ok(msg) = rx.try_recv() {
                self.status_message = msg.clone();
                self.log_content.push_str(&format!("{}\n", msg));
                if self.status_message.contains("Success") || self.status_message.contains("Error") {
                    job_finished = true;
                }
            }
            if job_finished {
                finished_just_now = true; 
                self.rx = None; 
            } else {
                ctx.request_repaint();
            }
        }

        if self.is_adb_busy {
            let mut done = false;
            if let Some(rx) = self.adb_rx.as_ref() {
                while let Ok(event) = rx.try_recv() {
                    match event {
                        AdbEvent::Status(msg) => {
                            self.status_message = msg.clone();
                            self.log_content.push_str(&format!("{}\n", msg)); 
                        }
                        AdbEvent::Success(msg) => {
                            self.status_message = msg.clone();
                            self.log_content.push_str(&format!("{}\n", msg));
                            done = true;
                            finished_just_now = true;
                        }
                        AdbEvent::Error(err) => {
                            self.status_message = format!("Error: {}", err);
                            self.log_content.push_str(&format!("Error: {}\n", err));
                            done = true;
                        }
                    }
                }
            }
            
            if done {
                self.is_adb_busy = false;
                self.adb_rx = None; 
            } else {
                ctx.request_repaint();
            }
        }

        if finished_just_now && (self.status_message.contains("Success") || self.status_message.contains("Complete")) {
            settings.rx_lang = Some(crate::core::settings::lang::start_scan());
        }

        finished_just_now
    }
}

fn censor_path(path: &str) -> String {
    if path.is_empty() || path == "No source selected" { return String::new(); }
    let mut clean = path.to_string();
    if let Ok(user) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !user.is_empty() { clean = clean.replace(&user, "***"); }
    }
    let path_obj = Path::new(&clean);
    let components: Vec<_> = path_obj.components().map(|c| c.as_os_str().to_string_lossy()).collect();
    if components.len() > 3 {
        format!("...\\{}\\{}", components[components.len()-2], components[components.len()-1])
    } else {
        clean
    }
}