use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::fs;
use std::path::Path;
use self_update::cargo_crate_version;
use eframe::egui;
use std::process::Command;

use crate::core::settings::{Settings, upd::UpdateMode};
use crate::core::utils::DragGuard;

const REPO_OWNER: &str = "WonderMOMOCO"; 
const REPO_NAME: &str = "Battle-Cats-Complete";
const BIN_NAME: &str = "Battle Cats Complete"; 

const PROMPT_BUTTON_SIZE: [f32; 2] = [80.0, 40.0];  
const RESTART_BUTTON_SIZE: [f32; 2] = [80.0, 40.0]; 

pub fn cleanup_temp_files() {
    let temp_files = [
        "tmp_update.zip",
        "tmp_new_version.exe",
        "tmp_new_version",
    ];

    for file in temp_files {
        let path = Path::new(file);
        if path.exists() {
            let _ = fs::remove_file(path); 
        }
    }
}

fn restart_app() {
    let exe = std::env::current_exe().unwrap_or_default();

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let _ = Command::new(exe).exec(); 
        std::process::exit(1); 
    }

    #[cfg(not(unix))]
    {
        let _ = Command::new(exe).spawn();
        std::process::exit(0);
    }
}

#[derive(Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpdateFound(String, self_update::update::Release),
    Downloading(String),
    RestartPending(String),
    Error(String),
    UpToDate,
}

pub enum UpdaterMsg {
    UpdateFound(self_update::update::Release),
    UpToDate,
    Error(String),
    DownloadStarted(String),
    DownloadFinished(String),
}

pub struct Updater {
    rx: Receiver<UpdaterMsg>,
    tx: Sender<UpdaterMsg>,
    pub status: UpdateStatus,
}

impl Default for Updater {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            rx,
            tx,
            status: UpdateStatus::Idle,
        }
    }
}

impl Updater {
    pub fn check_for_updates(&mut self) {
        if !matches!(self.status, UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::Error(_)) {
            return;
        }
        
        let tx = self.tx.clone();
        self.status = UpdateStatus::Checking;
        
        thread::spawn(move || {
            let result = check_remote();
            match result {
                Ok(Some(release)) => { let _ = tx.send(UpdaterMsg::UpdateFound(release)); },
                Ok(None) => { let _ = tx.send(UpdaterMsg::UpToDate); },
                Err(e) => { let _ = tx.send(UpdaterMsg::Error(e.to_string())); }
            }
        });
    }

    pub fn download_and_install(&mut self, release: self_update::update::Release) {
        let tx = self.tx.clone();
        let version = release.version.clone();
        self.status = UpdateStatus::Downloading(version.clone());

        thread::spawn(move || {
            cleanup_temp_files();

            let _ = tx.send(UpdaterMsg::DownloadStarted(version.clone()));
            
            let target_tag = if version.starts_with('v') { version.clone() } else { format!("v{}", version) };
            
            let target_asset_name = if cfg!(target_os = "windows") { 
                "bcc_windows" 
            } else if cfg!(target_os = "macos") {
                "bcc_mac"
            } else { 
                "bcc_linux" 
            };
            
            let result = self_update::backends::github::Update::configure()
                .repo_owner(REPO_OWNER)
                .repo_name(REPO_NAME)
                .bin_name(BIN_NAME)
                .show_download_progress(false)
                .show_output(false)            
                .no_confirm(true)              
                .current_version(cargo_crate_version!())
                .target_version_tag(&target_tag)
                .target(target_asset_name)     
                .build();
                
            match result {
                Ok(update_box) => {
                     match update_box.update() {
                         Ok(_) => { 
                             cleanup_temp_files();
                             let _ = tx.send(UpdaterMsg::DownloadFinished(version)); 
                         },
                         Err(e) => { 
                             cleanup_temp_files();
                             let _ = tx.send(UpdaterMsg::Error(format!("Update failed: {}", e))); 
                         }
                     }
                }
                Err(e) => { 
                    cleanup_temp_files();
                    let _ = tx.send(UpdaterMsg::Error(format!("Config failed: {}", e))); 
                }
            }
        });
    }

    pub fn update_state(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                UpdaterMsg::UpdateFound(release) => {
                    self.status = UpdateStatus::UpdateFound(release.version.clone(), release);
                },
                UpdaterMsg::UpToDate => {
                    self.status = UpdateStatus::UpToDate;
                },
                UpdaterMsg::Error(e) => {
                    self.status = UpdateStatus::Error(e);
                },
                UpdaterMsg::DownloadStarted(ver) => {
                    self.status = UpdateStatus::Downloading(ver);
                },
                UpdaterMsg::DownloadFinished(ver) => {
                    self.status = UpdateStatus::RestartPending(ver);
                },
            }
        }
    }

    pub fn show_ui(&mut self, ctx: &egui::Context, settings: &mut Settings, drag_guard: &mut DragGuard) {
        let is_idle = matches!(self.status, UpdateStatus::Idle);
        let allow_drag = if !is_idle {
            drag_guard.update(ctx)
        } else {
            false
        };

        let status = self.status.clone();

        match status {
            UpdateStatus::UpdateFound(tag, release) => {
                self.show_update_found_window(ctx, settings, allow_drag, tag, release);
            }
            UpdateStatus::Downloading(tag) => {
                self.show_downloading_window(ctx, allow_drag, tag);
            }
            UpdateStatus::RestartPending(tag) => {
                self.show_restart_pending_window(ctx, settings, allow_drag, tag);
            }
            UpdateStatus::Error(msg) => {
                self.show_error_window(ctx, allow_drag, msg);
            }
            _ => {}
        }
    }

    fn show_update_found_window(&mut self, ctx: &egui::Context, settings: &mut Settings, allow_drag: bool, tag: String, release: self_update::update::Release) {
        if matches!(settings.update_mode, UpdateMode::AutoReset | UpdateMode::AutoLoad) {
            self.download_and_install(release);
            return;
        }

        ctx.request_repaint();
        let mut start_download = false;
        let mut close_modal = false;
        let mut disable_future = false;
        let display_ver = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };
        let screen_rect = ctx.screen_rect();

        egui::Window::new("Update Available")
            .collapsible(false).resizable(false).order(egui::Order::Tooltip)
            .constrain(true).movable(allow_drag).pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen_rect.center()) 
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(format!("New Battle Cats Complete update found: {}", display_ver));
                    ui.add_space(10.0);
                    ui.label("Would you like to download the update now?");
                });
                ui.add_space(20.0);

                let mut style: egui::Style = (**ui.style()).clone();
                style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
                style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
                style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
                ui.set_style(style);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0; 

                    let btn_w = PROMPT_BUTTON_SIZE[0];
                    let count = 3.0; 
                    let spacing = 10.0;
                    let total_w = (btn_w * count) + (spacing * (count - 1.0)); 
                    
                    let available_w = ui.available_width();
                    let margin_left = (available_w - total_w) / 2.0;
                    if margin_left > 0.0 {
                        ui.add_space(margin_left);
                    }

                    if ui.add_sized(PROMPT_BUTTON_SIZE, egui::Button::new("Yes")).clicked() { start_download = true; }
                    if ui.add_sized(PROMPT_BUTTON_SIZE, egui::Button::new("No")).clicked() { close_modal = true; }
                    if ui.add_sized(PROMPT_BUTTON_SIZE, egui::Button::new("Never")).clicked() {
                        close_modal = true;
                        disable_future = true;
                    }
                });
            });

        if start_download {
            self.download_and_install(release);
        }
        if disable_future {
            settings.update_mode = UpdateMode::Ignore;
            close_modal = true;
        }
        if close_modal {
            self.status = UpdateStatus::Idle;
        }
    }

    fn show_downloading_window(&self, ctx: &egui::Context, allow_drag: bool, tag: String) {
        ctx.request_repaint();
        let screen_rect = ctx.screen_rect();
        
        egui::Window::new("Downloading Update")
            .collapsible(false).resizable(false).title_bar(false) 
            .order(egui::Order::Tooltip).constrain(true).movable(allow_drag)
            .pivot(egui::Align2::CENTER_CENTER).default_pos(screen_rect.center())
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    let display_tag = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };
                    ui.label(format!("Downloading {}...", display_tag));
                    ui.add_space(10.0);
                    let progress = (ctx.input(|i| i.time) % 1.0) as f32;
                    ui.add(egui::ProgressBar::new(progress).animate(false)); 
                });
            });
    }

    fn show_restart_pending_window(&mut self, ctx: &egui::Context, settings: &Settings, allow_drag: bool, tag: String) {
        if matches!(settings.update_mode, UpdateMode::AutoReset) {
            restart_app();
            return;
        }
        if matches!(settings.update_mode, UpdateMode::AutoLoad) {
            self.status = UpdateStatus::Idle; 
            return;
        }

        ctx.request_repaint();
        let mut should_restart = false;
        let mut close = false;
        let display_tag = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };
        let screen_rect = ctx.screen_rect();

        egui::Window::new("Update Complete")
            .collapsible(false).resizable(false).order(egui::Order::Tooltip)
            .constrain(true).movable(allow_drag).pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen_rect.center())
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(format!("{} update complete!", display_tag));
                    ui.add_space(5.0);
                    ui.label("Would you like to restart and apply the update now?");
                });
                ui.add_space(20.0);

                let mut style: egui::Style = (**ui.style()).clone();
                style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
                style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
                style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
                ui.set_style(style);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0; 

                    let btn_w = RESTART_BUTTON_SIZE[0];
                    let count = 2.0; 
                    let spacing = 10.0;
                    let total_w = (btn_w * count) + (spacing * (count - 1.0)); 
                    
                    let available_w = ui.available_width();
                    let margin_left = (available_w - total_w) / 2.0;
                    if margin_left > 0.0 {
                        ui.add_space(margin_left);
                    }

                    if ui.add_sized(RESTART_BUTTON_SIZE, egui::Button::new("Yes")).clicked() { should_restart = true; }
                    if ui.add_sized(RESTART_BUTTON_SIZE, egui::Button::new("No")).clicked() { close = true; }
                });
            });
        
        if should_restart {
            restart_app();
        }
        if close { self.status = UpdateStatus::Idle; }
    }

    fn show_error_window(&mut self, ctx: &egui::Context, allow_drag: bool, error_msg: String) {
        let mut close = false;
        let screen_rect = ctx.screen_rect();

        egui::Window::new("Update Failed")
            .collapsible(false).resizable(false).order(egui::Order::Tooltip)
            .constrain(true).movable(allow_drag).pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen_rect.center())
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label("An error occurred during update:");
                    ui.add_space(5.0);
                    ui.label(error_msg);
                });
                ui.add_space(20.0);
                if ui.button("Close").clicked() { close = true; }
            });
        
        if close { self.status = UpdateStatus::Idle; }
    }
}

fn check_remote() -> Result<Option<self_update::update::Release>, Box<dyn std::error::Error>> {
    let current = cargo_crate_version!();
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;
        
    if let Some(latest) = releases.first() {
        if self_update::version::bump_is_greater(current, &latest.version)? {
            return Ok(Some(latest.clone()));
        }
    }
    Ok(None)
}