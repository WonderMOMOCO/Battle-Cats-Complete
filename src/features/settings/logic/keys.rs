use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct RegionKey {
    pub key: String,
    pub iv: String,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct UserKeys {
    pub jp: RegionKey,
    pub en: RegionKey,
    pub tw: RegionKey,
    pub kr: RegionKey,
}

impl UserKeys {
    pub fn load() -> Self {
        // Now automatically uses the User/AppData folder via our global helper
        crate::global::io::json::load("keys.json").unwrap_or_default()
    }

    pub fn save(&self) {
        crate::global::io::json::save("keys.json", self);
    }

    pub fn is_empty(&self) -> bool {
        self.jp.key.is_empty() && self.en.key.is_empty() && 
        self.tw.key.is_empty() && self.kr.key.is_empty()
    }

    pub fn as_tuples(&self) -> Vec<(String, String, String)> {
        let mut vec = Vec::new();
        if !self.jp.key.is_empty() && !self.jp.iv.is_empty() { vec.push((self.jp.key.clone(), self.jp.iv.clone(), "JP".to_string())); }
        if !self.en.key.is_empty() && !self.en.iv.is_empty() { vec.push((self.en.key.clone(), self.en.iv.clone(), "EN".to_string())); }
        if !self.tw.key.is_empty() && !self.tw.iv.is_empty() { vec.push((self.tw.key.clone(), self.tw.iv.clone(), "TW".to_string())); }
        if !self.kr.key.is_empty() && !self.kr.iv.is_empty() { vec.push((self.kr.key.clone(), self.kr.iv.clone(), "KR".to_string())); }
        vec
    }
}

/// Moves keys.json from the app root to the secure AppData folder
pub fn migrate_keys_to_appdata() {
    let root_path = Path::new("keys.json");
    if root_path.exists() {
        if let Ok(data) = fs::read_to_string(root_path) {
            if let Ok(keys) = serde_json::from_str::<UserKeys>(&data) {
                keys.save(); // Saves to new AppData location
                let _ = fs::remove_file(root_path); // Deletes from root
            }
        }
    }
}