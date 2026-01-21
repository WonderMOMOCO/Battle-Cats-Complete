use eframe::egui;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use std::fs;

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
                        let valid_exts = ["csv", "png", "maanim", "imgcut", "mamodel", "txt"];
                        
                        if !valid_exts.contains(&ext) {
                             continue;
                        }

                        let sender_clone = sender.clone();
                        let ctx_clone = ctx.clone();
                        let path_clone = path.clone();

                        thread::spawn(move || {
                            wait_for_stability(&path_clone);
                            
                            if let Err(_e) = sender_clone.send(path_clone) {
                            }
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

fn wait_for_stability(path: &Path) {
    let mut last_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut stable_checks = 0;
    
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(50));
        let current_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        
        if current_size == last_size && current_size > 0 {
            stable_checks += 1;
            if stable_checks >= 2 { return; }
        } else {
            stable_checks = 0;
            last_size = current_size;
        }
    }
}