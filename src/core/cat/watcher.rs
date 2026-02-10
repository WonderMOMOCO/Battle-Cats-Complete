use eframe::egui;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use super::CatListState;
use super::loader;
use crate::core::patterns;
use crate::data::cat::unitbuy;
use crate::data::cat::unitevolve;
use crate::data::global::imgcut::SpriteSheet;
use crate::paths::cat;
use crate::core::settings::handle::ScannerConfig;

pub struct CatWatchers {
    _watcher: RecommendedWatcher,
}

impl CatWatchers {
    pub fn new(sender: Sender<PathBuf>, ctx: egui::Context) -> Option<Self> {
        // Internal channel for raw events
        let (internal_tx, internal_rx) = channel();

        // Spawn Debounce Thread
        thread::spawn(move || {
            debounce_loop(internal_rx, sender, ctx);
        });

        // Create Watcher
        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    for path in event.paths {
                        // Filter basic invalid paths immediately
                        let path_str = path.to_string_lossy().to_lowercase();
                        if path_str.contains("raw") { continue; }
                        
                        let _ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        let is_in_cats_dir = path_str.contains("cats");
                        
                        // We track everything relevant here, strict filtering happens in debounce or handle_event
                        if is_in_cats_dir || path_str.contains("unitbuy") || path_str.contains("unitevolve") {
                             let _ = internal_tx.send(path);
                        }
                    }
                }
            }
        }).ok()?;

        Some(Self { _watcher: watcher })
    }

    pub fn watch_all(&mut self, path: &Path) {
        let _ = self._watcher.watch(path, RecursiveMode::Recursive);
    }
}

// Dedicated Debounce Logic
fn debounce_loop(rx: Receiver<PathBuf>, final_sender: Sender<PathBuf>, ctx: egui::Context) {
    let mut pending_paths: HashSet<PathBuf> = HashSet::new();
    let mut deadline: Option<Instant> = None;
    // Wait this long after the last file change to trigger update
    let buffer_duration = Duration::from_millis(200); 

    loop {
        // Determine how long to wait for next message
        let timeout = if let Some(d) = deadline {
            let now = Instant::now();
            if now >= d {
                // Flush pending paths
                if !pending_paths.is_empty() {
                    // Send unique paths
                    for path in pending_paths.drain() {
                        let _ = final_sender.send(path);
                    }
                    ctx.request_repaint();
                }
                deadline = None;
                // Go back to infinite sleep
                Duration::from_millis(u64::MAX)
            } else {
                d - now
            }
        } else {
            Duration::from_millis(u64::MAX)
        };

        // Wait for event or timeout
        match rx.recv_timeout(timeout) {
            Ok(path) => {
                // New event received
                pending_paths.insert(path);
                // Reset/Extend the deadline
                deadline = Some(Instant::now() + buffer_duration);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Timeout handled by top of loop
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break; // Channel closed, exit thread
            }
        }
    }
}

pub fn init(state: &mut CatListState, ctx: &egui::Context) {
    if state.watchers.is_none() {
        let (tx, rx) = channel();
        if let Some(mut w) = CatWatchers::new(tx, ctx.clone()) {
            let path = Path::new("game");
            if path.exists() {
                w.watch_all(path);
                state.watchers = Some(w);
                state.watch_receiver = Some(rx);
            }
        }
    }
}

pub fn handle_event(state: &mut CatListState, ctx: &egui::Context, path: &PathBuf, config: ScannerConfig) {
    let path_str = path.to_string_lossy().to_lowercase();
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    
    let cats_dir = Path::new(cat::DIR_CATS);

    if patterns::CAT_UNIVERSAL_FILES.contains(&file_name) || patterns::CHECK_LINE_FILES.contains(&file_name) {
        loader::restart_scan(state, config);
        ctx.request_repaint();
        return;
    }

    if file_name == cat::UNIT_BUY {
        state.cached_unit_buy = Some(unitbuy::load_unitbuy(cats_dir));
        if let Some(ref map) = state.cached_unit_buy {
            for cat in &mut state.cats {
                if let Some(row) = map.get(&cat.id) {
                    cat.unit_buy = row.clone();
                }
            }
        }
        ctx.request_repaint();
        return;
    }

    if path_str.contains(cat::DIR_UNIT_EVOLVE) || path_str.contains("unitevolve") {
        state.cached_evolve_text = Some(unitevolve::load(cats_dir, &config.language));
            if let Some(ref map) = state.cached_evolve_text {
            for cat in &mut state.cats {
                if let Some(text_arr) = map.get(&cat.id) {
                    cat.evolve_text = text_arr.clone();
                }
            }
        }
        if state.selected_cat.is_some() {
            loader::reload_selected_cat_data(state, config);
        }
        ctx.request_repaint();
        return;
    }

    if path_str.contains("assets") || path_str.contains("gatyaitem") {
        state.gatya_item_textures.clear();
        state.sprite_sheet = SpriteSheet::default(); 
        state.texture_cache_version += 1; 
        ctx.request_repaint();
        return;
    }
    
    if path_str.contains("cats") {
        let components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy()).collect();
        if let Some(cats_idx) = components.iter().position(|c| c == "cats") {
            if let Some(id_str) = components.get(cats_idx + 1) {
                if let Ok(id) = id_str.parse::<u32>() {
                    
                    // Check if modification is inside 'anim'
                    if let Some(anim_seg) = components.get(cats_idx + 3) {
                        if anim_seg == "anim" {
                            if state.selected_cat == Some(id) {
                                // Extract form char (f, c, s, u)
                                let form_char_modified = components.get(cats_idx + 2)
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| "f".to_string());
                                
                                // Only reload if the form being viewed matches the file modified
                                let current_loaded_id = &state.anim_viewer.loaded_id;
                                let form_marker = format!("_{}_", form_char_modified);
                                
                                // Partial match check handles the ID format "{id}_{char}_{ver}"
                                if current_loaded_id.is_empty() || current_loaded_id.contains(&form_marker) {
                                    state.anim_viewer.loaded_id.clear();
                                    state.anim_viewer.texture_version += 1; 
                                    ctx.request_repaint();
                                }
                                return;
                            }
                        }
                    }

                    state.cat_list.flush_icon(id);
                    if state.selected_cat == Some(id) {
                        state.detail_texture = None; 
                        state.detail_key.clear();
                        state.texture_cache_version += 1; 
                    }
                    loader::refresh_cat(state, id, config);
                    ctx.request_repaint();
                }
            }
        }
    }
}