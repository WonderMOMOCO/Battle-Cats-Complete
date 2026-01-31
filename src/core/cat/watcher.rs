use eframe::egui;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;
use std::fs;
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
        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    for path in event.paths {
                        let path_str = path.to_string_lossy().to_lowercase();

                        if path_str.contains("raw") {
                            continue;
                        }

                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        let is_in_cats_dir = path_str.contains("cats");
                        let valid_exts = ["csv", "png", "maanim", "imgcut", "mamodel", "txt"];
                        
                        let is_valid_file = valid_exts.contains(&ext);
                        let is_likely_folder = is_in_cats_dir && ext.is_empty();

                        if !is_valid_file && !is_likely_folder {
                             continue;
                        }

                        let sender_clone = sender.clone();
                        let ctx_clone = ctx.clone();
                        let path_clone = path.clone();

                        thread::spawn(move || {
                            wait_for_stability(&path_clone);
                            if let Err(_e) = sender_clone.send(path_clone) { }
                            ctx_clone.request_repaint();
                        });
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

fn wait_for_stability(path: &Path) {
    let mut last_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut stable_checks = 0;
    
    for _ in 0..50 {
        thread::sleep(Duration::from_millis(50));
        
        match fs::metadata(path) {
            Ok(meta) => {
                let current_size = meta.len();
                if current_size == last_size {
                    stable_checks += 1;
                    if stable_checks >= 5 { return; }
                } else {
                    stable_checks = 0; 
                    last_size = current_size;
                }
            },
            Err(_) => {
                return;
            }
        }
    }
}