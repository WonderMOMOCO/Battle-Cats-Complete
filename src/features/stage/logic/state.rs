use std::sync::mpsc::Receiver;
use std::collections::HashMap;
use eframe::egui;
use crate::features::settings::logic::state::ScannerConfig;
use crate::features::stage::registry::StageRegistry;
use crate::features::enemy::logic::scanner::EnemyEntry;
use super::loader;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct StageListState {
    #[serde(skip)] pub registry: StageRegistry,
    pub search_query: String,
    pub selected_category: Option<String>, 
    pub selected_map: Option<String>,      
    pub selected_stage: Option<String>,    
    pub is_list_open: bool,

    #[serde(skip)] pub scan_receiver: Option<Receiver<StageRegistry>>,
    
    #[serde(skip)] pub enemy_registry: HashMap<u32, EnemyEntry>,
    #[serde(skip)] pub enemy_texture_cache: HashMap<u32, egui::TextureHandle>,
}

impl Default for StageListState {
    fn default() -> Self {
        Self {
            registry: StageRegistry::default(),
            search_query: String::new(),
            selected_category: None,
            selected_map: None,
            selected_stage: None,
            scan_receiver: None,
            is_list_open: true,
            enemy_registry: HashMap::new(),
            enemy_texture_cache: HashMap::new(),
        }
    }
}

impl StageListState {
    pub fn restart_scan(&mut self, config: ScannerConfig) {
        loader::restart_scan(self, config);
    }

    pub fn update_data(&mut self) {
        loader::update_data(self);
    }

    pub fn sync_enemies(&mut self, enemies: &[EnemyEntry]) {
        self.enemy_registry = enemies.iter().map(|e| (e.id, e.clone())).collect();
    }
}