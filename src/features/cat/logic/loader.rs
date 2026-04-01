use std::time::Instant;
use std::sync::mpsc::TryRecvError;

use super::CatListState;
use super::scanner;
use crate::global::formats::imgcut::SpriteSheet; 
use crate::features::settings::logic::state::ScannerConfig;

pub fn refresh_cat(state: &mut CatListState, id: u32, config: ScannerConfig) {
    match scanner::scan_single(id, &config) {
        Some(entry) => {
            match state.cats.binary_search_by_key(&id, |c| c.id) {
                Ok(pos) => state.cats[pos] = entry,
                Err(pos) => state.cats.insert(pos, entry),
            }
        },
        None => {
            if let Ok(pos) = state.cats.binary_search_by_key(&id, |c| c.id) {
                state.cats.remove(pos);
                if state.selected_cat == Some(id) {
                    state.selected_cat = None;
                }
            }
        }
    }
}

pub fn update_data(state: &mut CatListState) {
    let Some(rx) = &state.scan_receiver else { return };

    let mut received_any = false;
    let mut is_done = false;

    loop {
        match rx.try_recv() {
            Ok(cat_entry) => {
                let id = cat_entry.id;
                
                state.active_scan_ids.insert(id);
                
                match state.cats.binary_search_by_key(&id, |c| c.id) {
                    Ok(pos) => state.cats[pos] = cat_entry,      
                    Err(pos) => state.cats.insert(pos, cat_entry), 
                }
                
                state.cat_list.flush_icon(id);
                received_any = true;
            },
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

        if state.selected_cat.is_none() && !state.cats.is_empty() {
            state.selected_cat = Some(state.cats[0].id);
        }
    }

    if is_done {
        state.cats.retain(|c| state.active_scan_ids.contains(&c.id));
        
        if let Some(sel) = state.selected_cat {
            if !state.active_scan_ids.contains(&sel) {
                state.selected_cat = None;
            }
        }

        state.cat_list.force_search_rebuild();
        state.scan_receiver = None;
    }
}

pub fn resync_scan(state: &mut CatListState, config: ScannerConfig) {
    state.active_scan_ids.clear();
    state.scan_receiver = Some(scanner::start_scan(config));
}

pub fn restart_scan(state: &mut CatListState, config: ScannerConfig) {
    let current_selection_id = state.selected_cat;
    let current_form = state.selected_form;
    let current_tab = state.selected_detail_tab;

    state.is_cold_scan = true;
    state.last_update_time = None;
    state.incoming_cats.clear();
    state.active_scan_ids.clear();

    state.cat_list.clear_cache(); 
    state.detail_texture = None;
    state.detail_key.clear();
    state.talent_name_textures.clear();
    state.gatya_item_textures.clear(); 
    state.texture_cache_version += 1;
    state.sprite_sheet = SpriteSheet::default();
    state.custom_assets = None;

    state.selected_cat = current_selection_id;
    state.selected_form = current_form;
    state.selected_detail_tab = current_tab;

    state.scan_receiver = Some(scanner::start_scan(config));
}