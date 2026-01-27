use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use self_update::cargo_crate_version;

const REPO_OWNER: &str = "WonderMOMOCO"; 
const REPO_NAME: &str = "Battle-Cats-Complete";
const BIN_NAME: &str = "Battle Cats Complete"; 

#[derive(Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpdateFound(String, self_update::update::Release),
    Downloading(String),
    RestartPending(String),
    #[allow(dead_code)]
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
            let _ = tx.send(UpdaterMsg::DownloadStarted(version.clone()));
            
            let result = self_update::backends::github::Update::configure()
                .repo_owner(REPO_OWNER)
                .repo_name(REPO_NAME)
                .bin_name(BIN_NAME)
                .show_download_progress(true)
                .current_version(cargo_crate_version!())
                .target_version_tag(&version) 
                .build();
                
            match result {
                Ok(update_box) => {
                     match update_box.update() {
                         Ok(_) => { let _ = tx.send(UpdaterMsg::DownloadFinished(version)); },
                         Err(e) => { let _ = tx.send(UpdaterMsg::Error(format!("Update failed: {}", e))); }
                     }
                }
                Err(e) => { let _ = tx.send(UpdaterMsg::Error(format!("Config failed: {}", e))); }
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
                    println!("Updater Error: {}", e);
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