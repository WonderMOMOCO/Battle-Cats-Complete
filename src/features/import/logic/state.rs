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
    #[serde(skip)] pub import_progress_max: Arc<AtomicUsize>,

    #[serde(skip)] pub export_log_content: String,
    #[serde(skip)] pub export_rx: Option<Receiver<String>>,
    #[serde(skip)] pub export_job_status: Arc<AtomicU8>, 
    #[serde(skip)] pub export_abort_flag: Arc<AtomicBool>,
    #[serde(skip)] pub export_job_completed_time: Option<Instant>,
    #[serde(skip)] pub export_job_aborted_time: Option<Instant>,
    #[serde(skip)] pub export_progress_current: Arc<AtomicUsize>,
    #[serde(skip)] pub export_progress_max: Arc<AtomicUsize>,
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
            import_progress_max: Arc::new(AtomicUsize::new(0)),

            export_log_content: String::new(),
            export_rx: None,
            export_job_status: Arc::new(AtomicU8::new(0)),
            export_abort_flag: Arc::new(AtomicBool::new(false)),
            export_job_completed_time: None,
            export_job_aborted_time: None,
            export_progress_current: Arc::new(AtomicUsize::new(0)),
            export_progress_max: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl ImportState {
    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let mut finished_just_now = false;
        
        self.import_censored = censor_path(&self.import_path);
        self.decrypt_censored = censor_path(&self.decrypt_path);

        // Process Import Logs
        if let Some(rx) = &self.import_rx {
            while let Ok(msg) = rx.try_recv() {
                self.import_log_content.push_str(&format!("{}\n", msg));
            }
        }

        // Process Export Logs
        if let Some(rx) = &self.export_rx {
            while let Ok(msg) = rx.try_recv() {
                self.export_log_content.push_str(&format!("{}\n", msg));
            }
        }

        // Evaluate Import State
        let import_status = self.import_job_status.load(Ordering::Relaxed);
        if import_status == 1 {
            ctx.request_repaint();
        } else if import_status == 2 || import_status == 3 {
            if self.import_abort_flag.load(Ordering::Relaxed) {
                self.import_job_aborted_time = Some(Instant::now());
            } else if import_status == 2 {
                finished_just_now = true;
                self.import_job_completed_time = Some(Instant::now());
            }
            self.import_job_status.store(0, Ordering::Relaxed);
            self.import_abort_flag.store(false, Ordering::Relaxed);
            self.import_rx = None;
            ctx.request_repaint(); 
        }

        if let Some(time) = self.import_job_completed_time {
            if time.elapsed().as_secs() < 2 { ctx.request_repaint(); } 
            else { self.import_job_completed_time = None; }
        }
        if let Some(time) = self.import_job_aborted_time {
            if time.elapsed().as_secs() < 2 { ctx.request_repaint(); } 
            else { self.import_job_aborted_time = None; }
        }

        // Evaluate Export State
        let export_status = self.export_job_status.load(Ordering::Relaxed);
        if export_status == 1 {
            ctx.request_repaint();
        } else if export_status == 2 || export_status == 3 {
            if self.export_abort_flag.load(Ordering::Relaxed) {
                self.export_job_aborted_time = Some(Instant::now());
            } else if export_status == 2 {
                self.export_job_completed_time = Some(Instant::now());
            }
            self.export_job_status.store(0, Ordering::Relaxed);
            self.export_abort_flag.store(false, Ordering::Relaxed);
            self.export_rx = None;
            ctx.request_repaint(); 
        }

        if let Some(time) = self.export_job_completed_time {
            if time.elapsed().as_secs() < 2 { ctx.request_repaint(); } 
            else { self.export_job_completed_time = None; }
        }
        if let Some(time) = self.export_job_aborted_time {
            if time.elapsed().as_secs() < 2 { ctx.request_repaint(); } 
            else { self.export_job_aborted_time = None; }
        }

        finished_just_now
    }
}

pub fn censor_path(path: &str) -> String {
    if path.is_empty() || path == "No source selected" { return String::new(); }
    
    let mut clean = path.to_string();
    if let Ok(user) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !user.is_empty() { clean = clean.replace(&user, "***"); }
    }
    
    let path_obj = Path::new(&clean);
    let components: Vec<_> = path_obj.components().map(|c| c.as_os_str().to_string_lossy()).collect();
    
    if components.len() >= 2 {
        let mut parent = components[components.len()-2].to_string();
        let mut file = components[components.len()-1].to_string();
        
        let total_len = parent.chars().count() + file.chars().count();
        
        if total_len > 20 {
            if file.chars().count() >= 20 {
                // File name is too long on its own, truncate it and drop the parent
                file = format!("{}...", file.chars().take(18).collect::<String>());
                parent = String::new();
            } else {
                // File name fits, but combined with parent is too long. Truncate parent.
                let allowed_parent = 20 - file.chars().count();
                if allowed_parent > 2 {
                    parent = format!("{}...", parent.chars().take(allowed_parent - 2).collect::<String>());
                } else {
                    parent = String::new();
                }
            }
        }
        
        // Add the ellipsis prefix only if we chopped off earlier directories
        let prefix = if components.len() > 2 { "...\\" } else { "" };
        
        if parent.is_empty() {
            format!("{}{}", prefix, file)
        } else {
            format!("{}{}\\{}", prefix, parent, file)
        }
    } else {
        // Fallback for root files
        if clean.chars().count() > 20 {
            format!("...{}", clean.chars().skip(clean.chars().count() - 20).collect::<String>())
        } else {
            clean
        }
    }
}