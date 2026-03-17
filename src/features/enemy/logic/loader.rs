use std::time::Instant;
use std::sync::mpsc::TryRecvError;
use super::state::EnemyListState;
use super::scanner;
use crate::features::settings::logic::state::ScannerConfig;

pub fn restart_scan(state: &mut EnemyListState, config: ScannerConfig) {
    state.is_cold_scan = true;
    state.last_update_time = None;
    state.incoming_enemies.clear();
    state.active_scan_ids.clear();
    state.enemy_list.clear_cache(); 
    state.detail_texture = None;
    state.detail_key.clear();

    state.scan_receiver = Some(scanner::start_scan(config));
}

pub fn resync_scan(state: &mut EnemyListState, config: ScannerConfig) {
    state.active_scan_ids.clear();
    state.scan_receiver = Some(scanner::start_scan(config));
}

pub fn refresh_enemy(state: &mut EnemyListState, id: u32, config: &ScannerConfig) {
    match scanner::scan_single(id, config) {
        Some(new_enemy) => {
            match state.enemies.binary_search_by_key(&new_enemy.id, |e| e.id) {
                Ok(pos) => state.enemies[pos] = new_enemy,
                Err(pos) => state.enemies.insert(pos, new_enemy),
            }
        }
        None => {
            if let Ok(pos) = state.enemies.binary_search_by_key(&id, |e| e.id) {
                state.enemies.remove(pos);
                if state.selected_enemy == Some(id) {
                    state.selected_enemy = None;
                }
            }
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
                let id = entry.id;
                
                state.active_scan_ids.insert(id);
                
                match state.enemies.binary_search_by_key(&id, |e| e.id) {
                    Ok(pos) => state.enemies[pos] = entry,       
                    Err(pos) => state.enemies.insert(pos, entry), 
                }
                
                state.enemy_list.flush_icon(id);
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
        state.last_update_time = Some(now);

        if state.selected_enemy.is_none() && !state.enemies.is_empty() {
            state.selected_enemy = Some(state.enemies[0].id);
        }
    }

    if is_done {
        state.enemies.retain(|e| state.active_scan_ids.contains(&e.id));
        
        if let Some(sel) = state.selected_enemy {
            if !state.active_scan_ids.contains(&sel) {
                state.selected_enemy = None;
            }
        }

        state.enemy_list.force_search_rebuild();
        state.scan_receiver = None;
    }
}