use std::sync::mpsc::Receiver;
use crate::features::import::logic::ImportSubTab;
use crate::global::ui::shared::DragGuard;
use crate::features::mods::logic::bridge::ModAdbEvent;
use crate::features::mods::logic::metadata::ModMetadata;

#[derive(PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum ModPackType {
    Apk,
    Zip,
    Folder,
    Pack,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct ModData {
    pub folder_name: String,
    pub enabled: bool,
    #[serde(skip)] pub metadata: ModMetadata,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ModImportState {
    pub is_open: bool,
    pub tab: ImportSubTab,
    pub package_suffix: String,
    pub pack_type: ModPackType,
    
    #[serde(skip)] pub is_busy: bool,
    #[serde(skip)] pub status_message: String,
    #[serde(skip)] pub log_content: String,
    
    #[serde(skip)] pub adb_rx: Option<Receiver<ModAdbEvent>>,
    #[serde(skip)] pub pack_rx: Option<Receiver<String>>,
}

impl Default for ModImportState {
    fn default() -> Self {
        Self {
            is_open: false,
            tab: ImportSubTab::Emulator,
            package_suffix: String::new(),
            pack_type: ModPackType::Apk,
            is_busy: false,
            status_message: String::new(),
            log_content: String::new(),
            adb_rx: None,
            pack_rx: None,
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ModState {
    pub search_query: String,
    pub selected_mod: Option<String>,
    pub loaded_mods: Vec<ModData>,
    
    #[serde(skip)] pub rename_buffer: String,
    pub import: ModImportState,
    #[serde(skip)] pub drag_guard: DragGuard,
    #[serde(skip)] pub needs_rescan: bool,
    
    #[serde(skip)] pub list: Option<crate::features::mods::ui::list::ModList>,
}

impl ModState {
    pub fn refresh_mods(&mut self) {
        let mods_dir = std::path::Path::new("mods");
        if !mods_dir.exists() { return; }

        let mut current_folders = std::collections::HashSet::new();
        
        if let Ok(entries) = std::fs::read_dir(mods_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() && entry.file_name() != "packages" {
                    let folder_name = entry.file_name().to_string_lossy().to_string();
                    current_folders.insert(folder_name.clone());
                    
                    if !self.loaded_mods.iter().any(|m| m.folder_name == folder_name) {
                        let metadata = ModMetadata::load(mods_dir.join(&folder_name));
                        
                        self.loaded_mods.push(ModData {
                            folder_name,
                            enabled: false,
                            metadata,
                        });
                    }
                }
            }
        }
        
        self.loaded_mods.retain(|m| current_folders.contains(&m.folder_name));
        
        let active = self.loaded_mods.iter().find(|m| m.enabled).map(|m| m.folder_name.clone());
        crate::global::resolver::set_active_mod(active);
    }
}