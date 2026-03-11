use std::path::{Path, PathBuf};
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::Read;
use std::sync::mpsc::{self, Receiver};

use crate::features::enemy::paths;
use crate::features::enemy::data::{t_unit::{self, EnemyRaw}, enemyname, enemypicturebook};
use crate::features::settings::logic::state::ScannerConfig;
use crate::global::formats::maanim::Animation;

#[derive(Clone, Debug)]
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
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return true,
    };
    let mut buffer = [0u8; 25];
    if file.read_exact(&mut buffer).is_err() { return true; }
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if buffer[0..8] != PNG_SIG { return true; }
    buffer[24] < 8
}

pub fn start_scan(config: ScannerConfig) -> Receiver<EnemyEntry> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let root = Path::new(paths::DIR_ENEMIES);
        let t_unit_path = paths::stats(root);
        let raw_enemies = match t_unit::load_all(&t_unit_path) {
            Some(e) => e,
            None => return,
        };

        let lang_code = &config.language;
        let names = enemyname::load(root, lang_code);
        let descriptions = enemypicturebook::load(root, lang_code);

        raw_enemies.into_par_iter().enumerate().for_each(|(id, stats)| {
            let id_u32 = id as u32;
            let icon_p = paths::icon(root, id_u32);
            
            if !icon_p.exists() || is_placeholder_png(&icon_p) { return; }

            let atk_maanim_path = paths::maanim(root, id_u32, 2);
            let mut atk_anim_frames = 0;
            if let Ok(file_content) = fs::read_to_string(&atk_maanim_path) {
                let duration = Animation::scan_duration(&file_content);
                atk_anim_frames = if duration > 0 { duration + 1 } else { 0 };
            }

            let name = names.get(id).cloned().unwrap_or_default();
            let description = descriptions.get(id).cloned().unwrap_or_default();

            let _ = tx.send(EnemyEntry {
                id: id_u32, name, description, stats,
                icon_path: Some(icon_p), atk_anim_frames,
            });
        });
    });
    rx
}

pub fn scan_single(id: u32, config: &ScannerConfig) -> Option<EnemyEntry> {
    let root = Path::new(paths::DIR_ENEMIES);
    let t_unit_path = paths::stats(root);
    let raw_enemies = t_unit::load_all(&t_unit_path)?;
    let stats = raw_enemies.get(id as usize)?.clone();

    let lang_code = &config.language;
    let name = enemyname::load(root, lang_code).get(id as usize).cloned().unwrap_or_default();
    let description = enemypicturebook::load(root, lang_code).get(id as usize).cloned().unwrap_or_default();
    let icon_p = paths::icon(root, id);

    let atk_maanim_path = paths::maanim(root, id, 2);
    let mut atk_anim_frames = 0;
    if let Ok(file_content) = fs::read_to_string(&atk_maanim_path) {
        let duration = Animation::scan_duration(&file_content);
        atk_anim_frames = if duration > 0 { duration + 1 } else { 0 };
    }

    Some(EnemyEntry { id, name, description, stats, icon_path: Some(icon_p), atk_anim_frames })
}