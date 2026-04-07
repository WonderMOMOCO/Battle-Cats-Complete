use std::path::{Path, PathBuf};
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use serde::{Serialize, Deserialize};
use std::sync::Mutex;

use crate::features::enemy::paths;
use crate::features::enemy::data::{t_unit::{self, EnemyRaw}, enemyname, enemypicturebook};
use crate::features::settings::logic::state::ScannerConfig;
use crate::global::formats::maanim::Animation;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemyEntry {
    pub id: u32,
    pub name: String,
    pub description: Vec<String>,
    pub stats: EnemyRaw,
    pub icon_path: Option<PathBuf>,
    pub atk_anim_frames: i32,
}

impl EnemyEntry {
    pub fn base_id_str(&self) -> String { format!("{:03}", self.id) }
    pub fn id_str(&self) -> String { format!("{}-E", self.base_id_str()) }
    pub fn display_name(&self) -> String {
        if self.name.is_empty() { self.id_str() } else { self.name.clone() }
    }
}

fn is_placeholder_png(path: &Path) -> bool {
    let mut file = match File::open(path) { Ok(f) => f, Err(_) => return true, };
    let mut buffer = [0u8; 25];
    if file.read_exact(&mut buffer).is_err() { return true; }
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if buffer[0..8] != PNG_SIG { return true; }
    buffer[24] < 4
}

pub fn start_scan(config: ScannerConfig) -> Receiver<EnemyEntry> {
    let (tx, rx) = mpsc::channel();
    
    std::thread::spawn(move || {
        let root = Path::new(paths::DIR_ENEMIES);
        let priority = &config.language_priority;

        let t_unit_p = paths::stats(root);
        
        let Some(t_unit_parent) = t_unit_p.parent() else { return; };
        let Some(t_unit_name) = t_unit_p.file_name().and_then(|n| n.to_str()) else { return; };
        
        let Some(raw_enemies) = t_unit::load_all(t_unit_parent, t_unit_name, priority) else { return; };

        let names = enemyname::load(root, priority);
        let descriptions = enemypicturebook::load(root, priority);

        let stream_sender = std::sync::Arc::new(Mutex::new(tx));

        let mut parsed_enemies: Vec<EnemyEntry> = raw_enemies.into_par_iter().enumerate().filter_map(|(id, stats)| {
            let id_u32 = id as u32;
            
            let icon_p = paths::icon(root, id_u32);
            let mut resolved_icon = None;
            if let (Some(parent), Some(name)) = (icon_p.parent(), icon_p.file_name().and_then(|n| n.to_str())) {
                resolved_icon = crate::global::resolver::get(parent, &[name], priority).into_iter().next();
            }

            if let Some(ref p) = resolved_icon {
                if is_placeholder_png(p) && !config.show_invalid_enemies {
                    resolved_icon = None;
                }
            }

            if resolved_icon.is_none() && !config.show_invalid_enemies {
                return None;
            }

            let mut atk_anim_frames = 0;
            let atk_p = paths::maanim(root, id_u32, 2);
            if let (Some(parent), Some(name)) = (atk_p.parent(), atk_p.file_name().and_then(|n| n.to_str())) {
                if let Some(resolved_atk) = crate::global::resolver::get(parent, &[name], priority).into_iter().next() {
                    if let Ok(bytes) = fs::read(&resolved_atk) {
                        let content = String::from_utf8_lossy(&bytes);
                        let duration = Animation::scan_duration(&content);
                        atk_anim_frames = if duration > 0 { duration + 1 } else { 0 };
                    }
                }
            }

            let enemy = EnemyEntry {
                id: id_u32, 
                name: names.get(id).cloned().unwrap_or_default(),
                description: descriptions.get(id).cloned().unwrap_or_default(),
                stats, 
                icon_path: resolved_icon, 
                atk_anim_frames,
            };

            if let Ok(sender) = stream_sender.lock() {
                let _ = sender.send(enemy.clone());
            }

            Some(enemy)
        }).collect();

        parsed_enemies.sort_by_key(|e| e.id);

        if !crate::global::resolver::is_mod_active() {
            let current_hash = crate::global::io::cache::get_game_hash(None);
            crate::global::io::cache::save("enemies_cache.bin", current_hash, &parsed_enemies);
        }
    });
    
    rx
}

pub fn scan_single(id: u32, config: &ScannerConfig) -> Option<EnemyEntry> {
    let root = Path::new(paths::DIR_ENEMIES);
    let priority = &config.language_priority;
    
    let t_unit_p = paths::stats(root);
    
    let Some(t_unit_parent) = t_unit_p.parent() else { return None; };
    let Some(t_unit_name) = t_unit_p.file_name().and_then(|n| n.to_str()) else { return None; };
    
    let raw_enemies = t_unit::load_all(t_unit_parent, t_unit_name, priority)?;
    let stats = raw_enemies.get(id as usize)?.clone();

    let name = enemyname::load(root, priority).get(id as usize).cloned().unwrap_or_default();
    let description = enemypicturebook::load(root, priority).get(id as usize).cloned().unwrap_or_default();
    
    let icon_p = paths::icon(root, id);
    let mut resolved_icon = None;
    if let (Some(parent), Some(name)) = (icon_p.parent(), icon_p.file_name().and_then(|n| n.to_str())) {
        resolved_icon = crate::global::resolver::get(parent, &[name], priority).into_iter().next();
    }

    if let Some(ref p) = resolved_icon {
        if is_placeholder_png(p) && !config.show_invalid_enemies {
            resolved_icon = None;
        }
    }

    if resolved_icon.is_none() && !config.show_invalid_enemies {
        return None;
    }

    let mut atk_anim_frames = 0;
    let atk_p = paths::maanim(root, id, 2);
    
    if let (Some(parent), Some(name)) = (atk_p.parent(), atk_p.file_name().and_then(|n| n.to_str())) {
        let resolved_atk = crate::global::resolver::get(parent, &[name], priority).into_iter().next();
        
        if let Some(p) = resolved_atk {
            if let Ok(bytes) = fs::read(&p) {
                let content = String::from_utf8_lossy(&bytes);
                let duration = Animation::scan_duration(&content);
                atk_anim_frames = if duration > 0 { duration + 1 } else { 0 };
            }
        }
    }

    Some(EnemyEntry { id, name, description, stats, icon_path: resolved_icon, atk_anim_frames })
}