use std::path::Path;
use std::time::{Instant, Duration};
use std::sync::mpsc::TryRecvError;

use super::CatListState;
use super::scanner;
use crate::global::formats::imgcut::SpriteSheet; 
use crate::features::cat::data::unitlevel;
use crate::features::cat::data::unitbuy;
use crate::features::cat::data::skillacquisition;
use crate::features::cat::data::unitevolve;
use crate::features::cat::paths;
use crate::features::settings::logic::state::ScannerConfig;

pub fn ensure_global_data_loaded(state: &mut CatListState, language_code: &str) {
    let cats_dir = Path::new(paths::DIR_CATS);

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

    let cats_dir = Path::new(paths::DIR_CATS);
    let unit_folder = cats_dir.join(format!("{:03}", id));

    let curves = match &state.cached_level_curves { Some(c) => c, None => return };
    let buy = match &state.cached_unit_buy { Some(b) => b, None => return };
    let talents = match &state.cached_talents { Some(t) => t, None => return };
    let evolve = match &state.cached_evolve_text { Some(e) => e, None => return };

    let new_entry = scanner::process_cat_entry(
        &unit_folder,
        curves, 
        buy,
        talents,
        evolve,
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
    let Some(rx) = &state.scan_receiver else { return };

    let mut received_any = false;
    let mut is_done = false;

    loop {
        match rx.try_recv() {
            Ok(cat_entry) => {
                state.cats.push(cat_entry);
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
        let should_sort = state.last_update_time
            .map(|last| now.duration_since(last) > Duration::from_millis(150))
            .unwrap_or(true);

        if should_sort || is_done {
            state.cats.sort_unstable_by_key(|cat| cat.id); 
            state.last_update_time = Some(now);

            if state.selected_cat.is_none() {
                state.selected_cat = state.cats.first().map(|c| c.id);
            }
        }
    }

    if is_done {
        state.scan_receiver = None;
    }
}

pub fn restart_scan(state: &mut CatListState, config: ScannerConfig) {
    state.cats.clear();
    state.skill_descriptions = None; 
    
    let current_selection_id = state.selected_cat;
    let current_form = state.selected_form;
    let current_tab = state.selected_detail_tab;

    state.is_cold_scan = true;
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
    state.custom_assets = None;

    state.selected_cat = current_selection_id;
    state.selected_form = current_form;
    state.selected_detail_tab = current_tab;

    state.scan_receiver = Some(scanner::start_scan(config));
}