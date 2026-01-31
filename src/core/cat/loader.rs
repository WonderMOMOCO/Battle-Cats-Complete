use std::path::Path;
use std::time::{Instant, Duration};
use std::sync::mpsc::TryRecvError;

use super::CatListState;
use super::scanner;
use crate::data::global::imgcut::SpriteSheet; 
use crate::data::cat::unitlevel;
use crate::data::cat::unitbuy;
use crate::data::cat::skillacquisition;
use crate::data::cat::unitevolve;
use crate::paths::cat;
use crate::core::settings::handle::ScannerConfig;

pub fn ensure_global_data_loaded(state: &mut CatListState, language_code: &str) {
    let cats_dir = Path::new(cat::DIR_CATS);

    if state.cached_level_curves.is_none() {
        state.cached_level_curves = Some(unitlevel::load_level_curves(cats_dir));
    }
    if state.cached_unit_buy.is_none() {
        state.cached_unit_buy = Some(unitbuy::load_unitbuy(cats_dir));
    }
    if state.cached_talents.is_none() {
        state.cached_talents = Some(skillacquisition::load(cats_dir));
    }
    if state.cached_evolve_text.is_none() {
        state.cached_evolve_text = Some(unitevolve::load(cats_dir, language_code));
    }
}

pub fn refresh_cat(state: &mut CatListState, id: u32, config: ScannerConfig) {
    ensure_global_data_loaded(state, &config.language);

    let cats_dir = Path::new(cat::DIR_CATS);
    let unit_folder = cats_dir.join(format!("{:03}", id));

    let new_entry = scanner::process_cat_entry(
        &unit_folder,
        state.cached_level_curves.as_ref().unwrap(),
        state.cached_unit_buy.as_ref().unwrap(),
        state.cached_talents.as_ref().unwrap(),
        state.cached_evolve_text.as_ref().unwrap(),
        &config 
    );

    match new_entry {
        Some(entry) => {
            if let Some(index) = state.cats.iter().position(|c| c.id == id) {
                state.cats[index] = entry;
            } else {
                state.cats.push(entry);
                state.cats.sort_unstable_by_key(|c| c.id);
            }
        },
        None => {
            if let Some(index) = state.cats.iter().position(|c| c.id == id) {
                state.cats.remove(index);
                if state.selected_cat == Some(id) {
                    state.selected_cat = None;
                }
            }
        }
    }
}

pub fn reload_selected_cat_data(state: &mut CatListState, config: ScannerConfig) {
    if let Some(id) = state.selected_cat {
        refresh_cat(state, id, config);
    }
}

pub fn update_data(state: &mut CatListState) {
    let Some(receiver_handle) = &state.scan_receiver else { return };

    let mut received_batch = Vec::new();
    let mut is_scan_complete = false;

    loop {
        match receiver_handle.try_recv() {
            Ok(cat_entry) => received_batch.push(cat_entry),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                is_scan_complete = true;
                break;
            }
        }
    }

    if !received_batch.is_empty() {
        if state.is_cold_scan {
            state.cats.extend(received_batch);
            
            let now = Instant::now();
            let should_sort = match state.last_update_time {
                Some(last) => now.duration_since(last) > Duration::from_millis(100),
                None => true,
            };

            if should_sort {
                state.cats.sort_unstable_by_key(|cat| cat.id); 
                state.last_update_time = Some(now);

                if state.selected_cat.is_none() {
                    if let Some(first_cat) = state.cats.first() {
                        state.selected_cat = Some(first_cat.id);
                    }
                }
            }
        } else {
            state.incoming_cats.extend(received_batch);
        }
    }

    if is_scan_complete {
        if state.is_cold_scan {
            state.cats.sort_unstable_by_key(|cat| cat.id);
        } else if !state.incoming_cats.is_empty() {
            state.incoming_cats.sort_unstable_by_key(|cat| cat.id);
            state.cats = std::mem::take(&mut state.incoming_cats);
            
            state.cat_list.clear_cache();

            if let Some(target_id) = state.selected_cat {
                if !state.cats.iter().any(|cat| cat.id == target_id) {
                    if let Some(first_cat) = state.cats.first() {
                        state.selected_cat = Some(first_cat.id);
                    } else {
                        state.selected_cat = None;
                    }
                }
            } else {
                if let Some(first_cat) = state.cats.first() {
                    state.selected_cat = Some(first_cat.id);
                }
            }
        }
        state.scan_receiver = None;
        state.last_update_time = None; 
    }
}

pub fn restart_scan(state: &mut CatListState, config: ScannerConfig) {
    state.skill_descriptions = None; 
    
    let current_selection_id = state.selected_cat;
    let current_form = state.selected_form;
    let current_tab = state.selected_detail_tab;

    state.is_cold_scan = state.cats.is_empty();
    state.last_update_time = None;

    state.incoming_cats.clear();
    
    state.cached_level_curves = None;
    state.cached_unit_buy = None;
    state.cached_talents = None;
    state.cached_evolve_text = None;

    state.cat_list.clear_cache(); 
    state.detail_texture = None;
    state.detail_key.clear();
    state.talent_name_textures.clear();
    state.gatya_item_textures.clear(); 
    state.texture_cache_version += 1;

    state.sprite_sheet = SpriteSheet::default();
    state.multihit_texture = None;
    state.kamikaze_texture = None;
    state.boss_wave_immune_texture = None;

    state.selected_cat = current_selection_id;
    state.selected_form = current_form;
    state.selected_detail_tab = current_tab;

    state.scan_receiver = Some(scanner::start_scan(config));
}