use crate::features::stage::logic::state::StageListState;
use crate::features::settings::logic::state::ScannerConfig;
use super::scanner;

pub fn restart_scan(state: &mut StageListState, config: ScannerConfig) {
    state.registry.clear_cache();
    
    state.scan_receiver = Some(scanner::start_scan(&config));
}

pub fn update_data(state: &mut StageListState) {
    let Some(rx) = &state.scan_receiver else { return };

    if let Ok(new_registry) = rx.try_recv() {
        state.registry = new_registry;
        state.scan_receiver = None;
    }
}