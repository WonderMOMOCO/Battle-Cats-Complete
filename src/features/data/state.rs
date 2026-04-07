use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::env;
use eframe::egui;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::time::Instant;

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
    
    #[serde(skip)] pub selected_job: Option<ImportSubTab>,
    pub import_path: String,
    #[serde(skip)] pub import_censored: String,
    pub import_mode: ImportMode,
    pub adb_import_type: AdbImportType,
    pub adb_region: AdbRegion,
    pub decrypt_path: String,
    #[serde(skip)] pub decrypt_censored: String,
    
    pub export_filename: String,
    pub compression_level: i32,
    pub include_raw: bool,
    
    #[serde(skip)] pub import_log_content: String,
    #[serde(skip)] pub import_rx: Option<Receiver<String>>,
    #[serde(skip)] pub import_job_status: Arc<AtomicU8>, 
    #[serde(skip)] pub import_abort_flag: Arc<AtomicBool>,
    #[serde(skip)] pub import_job_completed_time: Option<Instant>,
    #[serde(skip)] pub import_job_aborted_time: Option<Instant>,
    #[serde(skip)] pub import_progress_current: Arc<AtomicUsize>,
    #[serde(skip)] pub import_progress_maximum: Arc<AtomicUsize>,

    #[serde(skip)] pub export_log_content: String,
    #[serde(skip)] pub export_rx: Option<Receiver<String>>,
    #[serde(skip)] pub export_job_status: Arc<AtomicU8>, 
    #[serde(skip)] pub export_abort_flag: Arc<AtomicBool>,
    #[serde(skip)] pub export_job_completed_time: Option<Instant>,
    #[serde(skip)] pub export_job_aborted_time: Option<Instant>,
    #[serde(skip)] pub export_progress_current: Arc<AtomicUsize>,
    #[serde(skip)] pub export_progress_maximum: Arc<AtomicUsize>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            active_tab: DataTab::Import,
            selected_job: None,
            import_path: String::new(),
            import_censored: String::new(),
            import_mode: ImportMode::Zip,
            adb_import_type: AdbImportType::All,
            adb_region: AdbRegion::English,
            decrypt_path: String::new(),
            decrypt_censored: String::new(),
            export_filename: String::new(),
            compression_level: 9,
            include_raw: false,
            
            import_log_content: String::new(),
            import_rx: None,
            import_job_status: Arc::new(AtomicU8::new(0)),
            import_abort_flag: Arc::new(AtomicBool::new(false)),
            import_job_completed_time: None,
            import_job_aborted_time: None,
            import_progress_current: Arc::new(AtomicUsize::new(0)),
            import_progress_maximum: Arc::new(AtomicUsize::new(0)),

            export_log_content: String::new(),
            export_rx: None,
            export_job_status: Arc::new(AtomicU8::new(0)),
            export_abort_flag: Arc::new(AtomicBool::new(false)),
            export_job_completed_time: None,
            export_job_aborted_time: None,
            export_progress_current: Arc::new(AtomicUsize::new(0)),
            export_progress_maximum: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl ImportState {
    pub fn update(&mut self, egui_context: &egui::Context) -> bool {
        let mut finished_just_now = false;
        
        self.import_censored = censor_path(&self.import_path);
        self.decrypt_censored = censor_path(&self.decrypt_path);

        if let Some(receiver) = &self.import_rx {
            while let Ok(message) = receiver.try_recv() {
                self.import_log_content.push_str(&format!("{}\n", message));
            }
        }

        if let Some(receiver) = &self.export_rx {
            while let Ok(message) = receiver.try_recv() {
                self.export_log_content.push_str(&format!("{}\n", message));
            }
        }

        let import_status_value = self.import_job_status.load(Ordering::Relaxed);
        
        if import_status_value == 1 {
            egui_context.request_repaint();
        } else if import_status_value == 2 || import_status_value == 3 {
            if self.import_abort_flag.load(Ordering::Relaxed) {
                self.import_job_aborted_time = Some(Instant::now());
            } else if import_status_value == 2 {
                finished_just_now = true;
                self.import_job_completed_time = Some(Instant::now());
            }
            self.import_job_status.store(0, Ordering::Relaxed);
            self.import_abort_flag.store(false, Ordering::Relaxed);
            self.import_rx = None;
            egui_context.request_repaint(); 
        }

        if let Some(time) = self.import_job_completed_time {
            if time.elapsed().as_secs() < 2 { egui_context.request_repaint(); } 
            else { self.import_job_completed_time = None; }
        }
        
        if let Some(time) = self.import_job_aborted_time {
            if time.elapsed().as_secs() < 2 { egui_context.request_repaint(); } 
            else { self.import_job_aborted_time = None; }
        }

        let export_status_value = self.export_job_status.load(Ordering::Relaxed);
        
        if export_status_value == 1 {
            egui_context.request_repaint();
        } else if export_status_value == 2 || export_status_value == 3 {
            if self.export_abort_flag.load(Ordering::Relaxed) {
                self.export_job_aborted_time = Some(Instant::now());
            } else if export_status_value == 2 {
                self.export_job_completed_time = Some(Instant::now());
            }
            self.export_job_status.store(0, Ordering::Relaxed);
            self.export_abort_flag.store(false, Ordering::Relaxed);
            self.export_rx = None;
            egui_context.request_repaint(); 
        }

        if let Some(time) = self.export_job_completed_time {
            if time.elapsed().as_secs() < 2 { egui_context.request_repaint(); } 
            else { self.export_job_completed_time = None; }
        }
        
        if let Some(time) = self.export_job_aborted_time {
            if time.elapsed().as_secs() < 2 { egui_context.request_repaint(); } 
            else { self.export_job_aborted_time = None; }
        }

        finished_just_now
    }
}

pub fn censor_path(path_string: &str) -> String {
    if path_string.is_empty() || path_string == "No source selected" { return String::new(); }
    
    let mut clean_string = path_string.to_string();
    if let Ok(username) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !username.is_empty() { clean_string = clean_string.replace(&username, "***"); }
    }
    
    let path_object = Path::new(&clean_string);
    let path_components: Vec<_> = path_object.components().map(|component| component.as_os_str().to_string_lossy()).collect();
    
    if path_components.len() < 2 {
        if clean_string.chars().count() > 20 {
            return format!("...{}", clean_string.chars().skip(clean_string.chars().count() - 20).collect::<String>());
        }
        return clean_string;
    }

    let mut parent_folder = path_components[path_components.len()-2].to_string();
    let mut target_file = path_components[path_components.len()-1].to_string();
    
    let total_length = parent_folder.chars().count() + target_file.chars().count();
    
    if total_length > 20 {
        if target_file.chars().count() >= 20 {
            target_file = format!("{}...", target_file.chars().take(18).collect::<String>());
            parent_folder = String::new();
        } else {
            let allowed_parent_length = 20 - target_file.chars().count();
            if allowed_parent_length > 2 {
                parent_folder = format!("{}...", parent_folder.chars().take(allowed_parent_length - 2).collect::<String>());
            } else {
                parent_folder = String::new();
            }
        }
    }
    
    let ellipsis_prefix = if path_components.len() > 2 { "...\\" } else { "" };
    
    if parent_folder.is_empty() {
        format!("{}{}", ellipsis_prefix, target_file)
    } else {
        format!("{}{}\\{}", ellipsis_prefix, parent_folder, target_file)
    }
}