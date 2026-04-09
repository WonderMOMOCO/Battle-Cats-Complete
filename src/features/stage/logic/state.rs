use std::sync::mpsc::Receiver;
use std::collections::HashMap;
use std::path::Path;
use eframe::egui;
use crate::features::settings::logic::state::ScannerConfig;
use crate::features::stage::registry::StageRegistry;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::stage::data::drop_chara;
use crate::features::cat::data::unitbuy::{self, UnitBuyRow};
use crate::global::formats::gatyaitembuy::{self, GatyaItemBuy};
use crate::global::formats::gatyaitemname::{self, GatyaItemName};
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

    #[serde(skip)] pub item_buy_registry: HashMap<u32, GatyaItemBuy>,
    #[serde(skip)] pub item_name_registry: HashMap<usize, GatyaItemName>,
    #[serde(skip)] pub drop_chara_registry: HashMap<u32, u32>,
    #[serde(skip)] pub unit_buy_registry: HashMap<u32, UnitBuyRow>,
    #[serde(skip)] pub item_texture_cache: HashMap<u32, egui::TextureHandle>,
    
    #[serde(skip)] pub active_language_priority: Vec<String>,
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
            item_buy_registry: HashMap::new(),
            item_name_registry: HashMap::new(),
            drop_chara_registry: HashMap::new(),
            unit_buy_registry: HashMap::new(),
            item_texture_cache: HashMap::new(),
            active_language_priority: Vec::new(),
        }
    }
}

impl StageListState {
    pub fn restart_scan(&mut self, scanner_configuration: ScannerConfig) {
        self.active_language_priority = scanner_configuration.language_priority.clone();

        let tables_directory_path = Path::new("game/tables");
        self.item_buy_registry = gatyaitembuy::load(
            tables_directory_path, 
            "Gatyaitembuy.csv", 
            &scanner_configuration.language_priority
        );
        
        let names_directory_path = tables_directory_path.join("GatyaitemName");
        self.item_name_registry = gatyaitemname::load(
            &names_directory_path, 
            "GatyaitemName.csv", 
            &scanner_configuration.language_priority
        );

        let stages_directory_path = Path::new("game/stages");
        self.drop_chara_registry = drop_chara::load(
            stages_directory_path,
            "drop_chara.csv",
            &scanner_configuration.language_priority
        );

        let cats_directory_path = Path::new("game/cats");
        self.unit_buy_registry = unitbuy::load_unitbuy(
            cats_directory_path, 
            &scanner_configuration.language_priority
        );

        loader::restart_scan(self, scanner_configuration);
    }

    pub fn update_data(&mut self) {
        loader::update_data(self);
    }

    pub fn sync_enemies(&mut self, extracted_enemies_array: &[EnemyEntry]) {
        self.enemy_registry = extracted_enemies_array.iter().map(|enemy_entry| (enemy_entry.id, enemy_entry.clone())).collect();
    }
}