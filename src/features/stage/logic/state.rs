use std::sync::mpsc::{channel, Receiver};
use std::thread;
use crate::features::settings::logic::ScannerConfig;
use crate::features::stage::registry::StageRegistry;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct StageListState {
    #[serde(skip)] pub registry: StageRegistry,
    pub search_query: String,
    pub selected_category: Option<String>, // Tier 1
    pub selected_map: Option<String>,      // Tier 2
    pub selected_stage: Option<String>,    // Tier 3
    pub is_list_open: bool,

    #[serde(skip)]
    pub scan_receiver: Option<Receiver<StageRegistry>>,
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
        }
    }
}

impl StageListState {
    pub fn restart_scan(&mut self, config: ScannerConfig) {
        let (tx, rx) = channel();
        self.scan_receiver = Some(rx);
        let priority = config.language_priority.clone();

        thread::spawn(move || {
            let mut new_registry = StageRegistry::default();
            new_registry.load_all(&priority);
            let _ = tx.send(new_registry);
        });
    }

    pub fn update_data(&mut self) {
        let Some(rx) = &self.scan_receiver else { return; };
        let Ok(new_registry) = rx.try_recv() else { return; };
        
        self.registry = new_registry;
        self.scan_receiver = None;
    }
}