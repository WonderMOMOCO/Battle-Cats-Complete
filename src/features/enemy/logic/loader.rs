use std::time::{Instant, Duration};
use std::sync::mpsc::TryRecvError;
use super::state::EnemyListState;
use super::scanner;
use crate::features::settings::logic::state::ScannerConfig;

pub fn restart_scan(state: &mut EnemyListState, config: ScannerConfig) {
    state.enemies.clear();
    state.is_cold_scan = true;
    state.last_update_time = None;
    state.incoming_enemies.clear();
    
    state.enemy_list.clear_cache(); 
    state.detail_texture = None;
    state.detail_key.clear();

    state.scan_receiver = Some(scanner::start_scan(config));
}

pub fn refresh_enemy(state: &mut EnemyListState, id: u32, config: &ScannerConfig) {
    if let Some(new_enemy) = scanner::scan_single(id, config) {
        if let Some(pos) = state.enemies.iter().position(|e| e.id == id) { 
            state.enemies[pos] = new_enemy; 
        } else {
            state.enemies.push(new_enemy); 
            state.enemies.sort_unstable_by_key(|e| e.id); 
        }
    }
}

pub fn update_data(state: &mut EnemyListState) {
    let Some(rx) = &state.scan_receiver else { return };

    let mut received_any = false;
    let mut is_done = false;

    loop {
        match rx.try_recv() {
            Ok(entry) => {
                state.enemies.push(entry);
                received_any = true;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                is_done = true;
                break;
            }
        }
    }

    if received_any {
        let now = Instant::now();
        let should_sort = state.last_update_time
            .map(|last| now.duration_since(last) > Duration::from_millis(150))
            .unwrap_or(true);

        if should_sort || is_done {
            state.enemies.sort_unstable_by_key(|e| e.id);
            state.last_update_time = Some(now);

            if state.selected_enemy.is_none() {
                state.selected_enemy = state.enemies.first().map(|e| e.id);
            }
        }
    }

    if is_done {
        state.scan_receiver = None;
    }
}