use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

fn default_source() -> String {
    "Battle Cats Complete".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMetadata {
    #[serde(default)] pub title: String,
    #[serde(default)] pub author: String,
    #[serde(default)] pub version: String,
    #[serde(default)] pub description: String,
    #[serde(default = "default_source")] pub source: String, 
}

impl Default for ModMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            author: String::new(),
            version: String::new(),
            description: String::new(),
            source: default_source(),
        }
    }
}

impl ModMetadata {
    pub fn load<P: AsRef<Path>>(mod_folder_path: P) -> Self {
        let meta_path = mod_folder_path.as_ref().join("metadata.json");
        if let Ok(data) = fs::read_to_string(meta_path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save<P: AsRef<Path>>(&self, mod_folder_path: P) -> Result<(), std::io::Error> {
        let meta_path = mod_folder_path.as_ref().join("metadata.json");
        let data = serde_json::to_string_pretty(self)?;
        fs::write(meta_path, data)
    }
}