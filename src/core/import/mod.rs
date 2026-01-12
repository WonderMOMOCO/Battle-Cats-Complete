use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::env;
use eframe::egui;
use std::path::{Path, PathBuf};

pub mod game_data; 
pub mod sort;

use crate::core::settings::Settings;

#[cfg(feature = "dev")]
#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum GameRegion {
    Japan, Taiwan, Korean, Global, Mod,
}

#[cfg(feature = "dev")]
impl GameRegion {
    pub fn code(&self) -> &'static str {
        match self {
            GameRegion::Japan => "ja",
            GameRegion::Taiwan => "tw",
            GameRegion::Korean => "ko",
            GameRegion::Global => "en",
            GameRegion::Mod => "mod",
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DataTab {
    #[cfg(feature = "dev")] Decrypt, 
    Import, 
    Export 
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportMode { None, Folder, Zip }

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ImportState {
    pub import_path: String,
    #[serde(skip)] pub import_censored: String,
    
    pub decrypt_path: String,
    #[serde(skip)] pub decrypt_censored: String,
    
    pub active_tab: DataTab,
    pub import_mode: ImportMode,
    
    pub export_filename: String,
    
    #[cfg(feature = "dev")] pub selected_region: GameRegion,
    
    pub compression_level: i32,

    #[serde(skip)] pub status_message: String,
    #[serde(skip)] pub log_content: String,
    #[serde(skip)] pub rx: Option<Receiver<String>>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            import_path: String::new(),
            import_censored: String::new(),
            
            decrypt_path: String::new(),
            decrypt_censored: String::new(),
            
            #[cfg(feature = "dev")]
            active_tab: DataTab::Decrypt,
            #[cfg(not(feature = "dev"))]
            active_tab: DataTab::Import,

            import_mode: ImportMode::Zip,
            
            export_filename: String::new(),
            
            #[cfg(feature = "dev")] selected_region: GameRegion::Global,
            
            compression_level: 6,
            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
        }
    }
}

impl ImportState {
    pub fn set_import_path(&mut self, path: String) {
        self.import_path = path;
        self.import_censored = censor_path(&self.import_path);
    }
    
    #[cfg(feature = "dev")]
    pub fn set_decrypt_path(&mut self, path: String) {
        self.decrypt_path = path;
        self.decrypt_censored = censor_path(&self.decrypt_path);
    }

    pub fn update(&mut self, ctx: &egui::Context, settings: &mut Settings) -> bool {
        if self.import_censored.is_empty() && !self.import_path.is_empty() {
             self.import_censored = censor_path(&self.import_path);
        }
        if self.decrypt_censored.is_empty() && !self.decrypt_path.is_empty() {
             self.decrypt_censored = censor_path(&self.decrypt_path);
        }

        let mut finished_just_now = false;

        if let Some(rx) = self.rx.take() {
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
            } else {
                self.rx = Some(rx);
                ctx.request_repaint();
            }
        }

        if finished_just_now && self.status_message.contains("Success") {
            settings.rx_lang = Some(crate::core::settings::lang::start_scan());
        }

        finished_just_now
    }
}

fn censor_path(path: &str) -> String {
    if path.is_empty() || path == "No source selected" { return String::new(); }
    
    let mut clean = path.to_string();
    
    if let Ok(user) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !user.is_empty() { 
            clean = clean.replace(&user, "***"); 
        }
    }

    let path_obj = Path::new(&clean);
    let components: Vec<_> = path_obj.components().collect();
    
    if components.len() > 3 {
        let len = components.len();
        let last_parts: PathBuf = components[len-3..].iter().collect();
        return last_parts.to_string_lossy().to_string();
    }
    
    clean
}