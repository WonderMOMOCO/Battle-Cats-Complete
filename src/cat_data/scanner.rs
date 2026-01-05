use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::{Arc, mpsc::{self, Receiver}};
use rayon::prelude::*;
use super::stats::{CatRaw, CatLevelCurve}; 
use image::GenericImageView; 

const SCAN_PRIORITY: &[&str] = &["au", "en", "ja", "tw", "ko", "es", "de", "fr", "it", "th"];

#[derive(Clone, Debug)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: PathBuf,
    pub names: Vec<String>, 
    pub forms: [bool; 4],
    pub stats: Vec<Option<CatRaw>>,
    pub curve: Option<CatLevelCurve>,
    pub atk_anim_frames: [i32; 4], 
}

pub fn start_scan(lang: String) -> Receiver<CatEntry> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let cats_dir = Path::new("game/cats");
        
        let level_curves = Arc::new(load_level_curves(cats_dir));
        
        let entries: Vec<PathBuf> = match fs::read_dir(cats_dir) {
            Ok(read_dir) => read_dir
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect(),
            Err(_) => Vec::new(),
        };

        entries.par_iter().for_each(|path| {
            let tx = tx.clone();
            let curves = Arc::clone(&level_curves);
            
            if let Some(entry) = process_cat_entry(path, &curves, &lang) {
                let _ = tx.send(entry);
            }
        });
    });
    rx
}

fn load_level_curves(cats_dir: &Path) -> Vec<CatLevelCurve> {
    let mut curves = Vec::new();
    let level_file = cats_dir.join("unitlevel.csv");
    if let Ok(content) = fs::read_to_string(&level_file) {
        for line in content.lines() {
            curves.push(CatLevelCurve::from_csv_line(line));
        }
    }
    curves
}

fn process_cat_entry(path: &Path, level_curves: &Vec<CatLevelCurve>, lang: &str) -> Option<CatEntry> {
    let stem = path.file_name()?.to_str()?;
    let id = stem.parse::<u32>().ok()?;

    let forms_chars = ['f', 'c', 's', 'u'];
    let forms = [
        path.join("f").exists(),
        path.join("c").exists(),
        path.join("s").exists(),
        path.join("u").exists(),
    ];

    let mut atk_anim_frames = [0; 4];
    for i in 0..4 {
        if forms[i] {
            let anim_path = path.join(format!("{}", forms_chars[i]))
                .join("anim")
                .join(format!("{:03}_{}02.maanim", id, forms_chars[i]));
            
            if let Ok(content) = fs::read_to_string(&anim_path) {
                atk_anim_frames[i] = parse_max_frame(&content);
            }
        }
    }

    let filename = format!("udi{:03}_f.png", id);
    let img_path = path.join("f").join(&filename);
    
    if !img_path.exists() { return None; }
    
    let img = image::open(&img_path).ok()?;
    let (w, h) = img.dimensions();

    if w <= 14 || h <= 2 { return None; }
    if !img.pixels().any(|(_, _, pixel)| pixel[3] > 0) { return None; }

    let mut names = vec![String::new(); 4];
    let target_file_id = id + 1;
    let lang_dir = path.join("lang");
    
    if let Some(name_file_path) = find_name_file(&lang_dir, target_file_id, lang) {
        if let Ok(bytes) = fs::read(&name_file_path) {
            let content = String::from_utf8_lossy(&bytes);
            let separator = if lang == "ja" { ',' } else { '|' };

            for (i, line) in content.lines().enumerate().take(4) {
                if let Some(name_part) = line.split(separator).next() {
                    names[i] = name_part.trim().to_string();
                }
            }
        }
    }

    let mut stats = vec![None; 4];
    let stats_path = path.join(format!("unit{:03}.csv", target_file_id));
    if let Ok(content) = fs::read_to_string(&stats_path) {
        for (i, line) in content.lines().enumerate().take(4) {
            stats[i] = CatRaw::from_csv_line(line);
        }
    }

    Some(CatEntry { 
        id, 
        image_path: img_path, 
        names,
        forms,
        stats, 
        curve: level_curves.get(id as usize).cloned(),
        atk_anim_frames,
    })
}

fn find_name_file(lang_dir: &Path, target_id: u32, lang: &str) -> Option<PathBuf> {
    if !lang_dir.exists() || lang.is_empty() { return None; }
    
    let codes_to_try: Vec<&str> = if lang == "au" {
        SCAN_PRIORITY.to_vec()
    } else {
        vec![lang]
    };

    for code in codes_to_try {
        let suffix = format!("_{}.csv", code);

        let found = fs::read_dir(lang_dir).ok()?
            .flatten()
            .find_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                
                if !name.starts_with("Unit_Explanation") || !name.ends_with(&suffix) {
                    return None;
                }

                let num_part = name
                    .trim_start_matches("Unit_Explanation")
                    .trim_end_matches(&suffix);
                
                if let Ok(num) = num_part.parse::<u32>() {
                    if num == target_id {
                        return Some(entry.path());
                    }
                }
                None
            });

        if found.is_some() {
            return found;
        }
    }
    
    None
}

fn parse_max_frame(content: &str) -> i32 {
    let mut max_frame = 0;
    for line in content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 2 {
            if let Ok(frame) = parts[0].trim().parse::<i32>() {
                if frame > max_frame { max_frame = frame; }
            }
        }
    }
    max_frame + 1
}