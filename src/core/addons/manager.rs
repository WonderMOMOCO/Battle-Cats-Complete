use std::fs;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use std::io::Cursor;
use zip::ZipArchive;

use crate::core::addons::toolpaths::{get_tools_dir, AddonStatus};

const RELEASE_TAG: &str = "tools"; 
const REPO_OWNER: &str = "WonderMOMOCO";
const REPO_NAME: &str = "Battle-Cats-Complete";

#[allow(dead_code)]
pub struct DownloadConfig {
    pub folder_name: String,
    pub asset_name: String,
    pub binary_name: String,
}

pub fn start_download(config: DownloadConfig) -> Receiver<AddonStatus> {
    let (tx, rx) = mpsc::channel();
    
    thread::spawn(move || {
        if let Err(e) = download_thread(tx.clone(), config) {
            let _ = tx.send(AddonStatus::Error(e));
        }
    });

    rx
}

fn download_thread(tx: Sender<AddonStatus>, config: DownloadConfig) -> Result<(), String> {
    let url = format!(
        "https://github.com/{}/{}/releases/download/{}/{}", 
        REPO_OWNER, REPO_NAME, RELEASE_TAG, config.asset_name
    );
    
    let _ = tx.send(AddonStatus::Downloading(0.1, "Connecting...".to_string()));
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("BattleCatsComplete/0.8.0")
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let response = client.get(&url)
        .send()
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: Status {}", response.status()));
    }

    let _ = tx.send(AddonStatus::Downloading(0.3, "Downloading...".to_string()));
    let bytes = response.bytes().map_err(|e| format!("Read error: {}", e))?;
    
    let _ = tx.send(AddonStatus::Downloading(0.7, "Extracting...".to_string()));
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Zip error: {}", e))?;
    
    let dest_dir = get_tools_dir().join(config.folder_name);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        if let Some(name) = file.enclosed_name() {
            let out_path = dest_dir.join(name);
            
            if file.is_dir() {
                fs::create_dir_all(&out_path).unwrap_or(());
            } else {
                if let Some(p) = out_path.parent() { fs::create_dir_all(p).unwrap_or(()); }
                let mut outfile = fs::File::create(&out_path).map_err(|e| format!("File creation error: {}", e))?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Write error: {}", e))?;
            }
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(fname) = out_path.file_name() {
                    if fname == config.binary_name.as_str() {
                        let _ = fs::set_permissions(&out_path, fs::Permissions::from_mode(0o755));
                    }
                }
            }
        }
    }

    let _ = tx.send(AddonStatus::Installed);
    Ok(())
}