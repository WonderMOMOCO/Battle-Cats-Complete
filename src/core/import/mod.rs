use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::path::{Path, PathBuf};
use std::env;
use eframe::egui;

pub mod import_data;
pub mod export_data;
pub mod sort_data;
pub mod log; 

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum DataTab {
    #[cfg(feature = "dev")]
    Extract, 
    Import,
    Export,
}

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ImportMode {
    None,
    Folder,
    Zip,
}

#[cfg(feature = "dev")]
#[derive(PartialEq, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum GameRegion {
    Global, Japan, Taiwan, Korean
}

#[cfg(feature = "dev")]
impl GameRegion {
    pub fn code(&self) -> &'static str {
        match self {
            GameRegion::Global => "en",
            GameRegion::Japan => "ja",
            GameRegion::Taiwan => "tw",
            GameRegion::Korean => "ko",
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ImportState {
    pub active_tab: DataTab,
    pub import_mode: ImportMode,
    
    // Extract Tab State
    pub selected_folder: String, 
    #[serde(skip)]
    pub censored_folder: String, 
    #[cfg(feature = "dev")]
    pub selected_region: GameRegion,

    // Import Tab State
    pub import_folder: String,
    pub import_zip: String,
    #[serde(skip)]
    pub censored_import_folder: String,
    #[serde(skip)]
    pub censored_import_zip: String,

    // Export Tab State
    pub export_name: String,
    pub compression_level: i32,

    // Shared State
    #[serde(skip)]
    pub status_message: String,
    #[serde(skip)]
    pub log_content: String,
    #[serde(skip)]
    pub rx: Option<Receiver<String>>,
    #[serde(skip)]
    pub reset_trigger: Option<f64>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            #[cfg(feature = "dev")]
            active_tab: DataTab::Extract,
            #[cfg(not(feature = "dev"))]
            active_tab: DataTab::Import,
            
            import_mode: ImportMode::Zip,
            
            selected_folder: "No folder selected".to_owned(),
            censored_folder: "No folder selected".to_owned(),
            #[cfg(feature = "dev")]
            selected_region: GameRegion::Global,

            import_folder: "No folder selected".to_owned(),
            import_zip: "No file selected".to_owned(),
            censored_import_folder: "No folder selected".to_owned(),
            censored_import_zip: "No file selected".to_owned(),

            export_name: String::new(), 
            compression_level: 6,

            status_message: "Ready".to_owned(),
            log_content: String::new(),
            rx: None,
            reset_trigger: None,
        }
    }
}

impl ImportState {
    pub fn update(&mut self, ctx: &egui::Context, settings: &mut crate::core::settings::Settings) -> bool {
        #[cfg(feature = "dev")]
        if self.censored_folder == "No folder selected" && self.selected_folder != "No folder selected" {
             self.censored_folder = censor_path(&self.selected_folder);
        }
        
        if self.censored_import_folder == "No folder selected" && self.import_folder != "No folder selected" {
             self.censored_import_folder = censor_path(&self.import_folder);
        }
        if self.censored_import_zip == "No file selected" && self.import_zip != "No file selected" {
             self.censored_import_zip = censor_path(&self.import_zip);
        }

        let mut finished_just_now = false;

        if let Some(rx) = &self.rx {
            // Poll for messages from the worker threads spawned in the Views
            while let Ok(msg) = rx.try_recv() {
                self.status_message = msg.clone();
                self.log_content.push_str(&format!("{}\n", msg));

                if self.status_message.contains("Success") 
                    || self.status_message.contains("Aborted") 
                    || self.status_message.contains("Error") 
                {
                    let current_time = ctx.input(|i| i.time);
                    self.reset_trigger = Some(current_time + 5.0);
                    finished_just_now = true;
                }
            }
            // Keep UI alive while working
            ctx.request_repaint();
        }

        // If an operation finished successfully, force a language Re-Scan
        if finished_just_now && self.status_message.contains("Success") {
            settings.trigger_language_scan();
        }

        if let Some(trigger_time) = self.reset_trigger {
            let current_time = ctx.input(|i| i.time);
            if current_time >= trigger_time {
                self.status_message = "Ready".to_string();
                self.rx = None;
                self.reset_trigger = None;
                
                // Reset inputs slightly for UX
                self.set_import_folder("No folder selected".to_string());
                self.set_import_zip("No file selected".to_string());
                #[cfg(feature = "dev")]
                self.set_extract_folder("No folder selected".to_string());
            } else {
                ctx.request_repaint();
            }
        }

        finished_just_now
    }

    #[cfg(feature = "dev")]
    pub fn set_extract_folder(&mut self, path: String) {
        self.selected_folder = path.clone();
        self.censored_folder = censor_path(&path);
    }

    pub fn set_import_folder(&mut self, path: String) {
        self.import_folder = path.clone();
        self.censored_import_folder = censor_path(&path);
    }

    pub fn set_import_zip(&mut self, path: String) {
        self.import_zip = path.clone();
        self.censored_import_zip = censor_path(&path);
    }
}

pub fn censor_path(path: &str) -> String {
    if path == "No folder selected" || path == "No source selected" || path == "No file selected" {
        return path.to_string();
    }

    let mut clean = path.to_string();

    if let Ok(user) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !user.is_empty() {
             clean = clean.replace(&user, "***");
        }
    }

    let path_obj = Path::new(&clean);
    let components: Vec<_> = path_obj.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    
    for i in 0..components.len() {
        if i + 1 < components.len() {
            let current = components[i].to_lowercase();
            if current == "users" || current == "home" {
                let target = &components[i+1];
                clean = clean.replace(target, "***");
            }
        }
    }

    let path_obj_clean = Path::new(&clean);
    let comps_clean: Vec<_> = path_obj_clean.components().collect();
    if comps_clean.len() > 3 {
        let count = comps_clean.len();
        let last_parts: PathBuf = comps_clean.iter().skip(count.saturating_sub(3)).collect();
        return format!("...{}{}", std::path::MAIN_SEPARATOR, last_parts.display());
    }

    clean
}