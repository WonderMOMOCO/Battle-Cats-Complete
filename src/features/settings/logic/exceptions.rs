use serde::{Deserialize, Serialize};
use indexmap::IndexMap; 
use std::path::Path;
use std::fs;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum RuleHandling {
    Include,
    Only,
    Ignore,
}

impl RuleHandling {
    pub fn all() -> [Self; 3] {
        [Self::Include, Self::Only, Self::Ignore]
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Include => "Include".to_string(),
            Self::Only => "Only".to_string(),
            Self::Ignore => "Ignore".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionRule {
    pub pattern: String,
    pub extension: String,
    pub handling: RuleHandling,
    pub languages: IndexMap<String, bool>,
    pub locked: bool,
}

impl Default for ExceptionRule {
    fn default() -> Self {
        let mut languages = IndexMap::new();
        for lang in ["en", "ja", "tw", "ko", "es", "de", "fr", "it", "th"] {
            languages.insert(lang.to_string(), false);
        }
        Self {
            pattern: String::new(),
            extension: String::new(),
            handling: RuleHandling::Include,
            languages,
            locked: false,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionList {
    pub rules: Vec<ExceptionRule>,
}

impl Default for ExceptionList {
    fn default() -> Self {
        let default_json = include_str!("exceptions.json");
        serde_json::from_str(default_json).unwrap_or_else(|_| ExceptionList { 
            rules: vec![ExceptionRule::default()] 
        })
    }
}

impl ExceptionList {
    pub fn save(&self) {
        crate::global::io::json::save("exceptions.json", self);
    }

    pub fn load_or_default() -> Self {
        crate::global::io::json::load("exceptions.json").unwrap_or_default()
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }
}