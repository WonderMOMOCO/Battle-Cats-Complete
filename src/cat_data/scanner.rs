use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::mpsc::{self, Receiver};
use image::GenericImageView; 
use super::stats::{CatRaw, CatLevelCurve}; 

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

pub fn start_scan() -> Receiver<CatEntry> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let cats_dir = Path::new("game/cats");
        let mut level_curves: Vec<CatLevelCurve> = Vec::new();
        let level_file = cats_dir.join("unitlevel.csv");
        
        if level_file.exists() {
            if let Ok(content) = fs::read_to_string(&level_file) {
                for line in content.lines() {
                    level_curves.push(CatLevelCurve::from_csv_line(line));
                }
            }
        }
        
        if let Ok(entries) = fs::read_dir(cats_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(stem) = path.file_name().and_then(|s| s.to_str()) {
                        if let Ok(id) = stem.parse::<u32>() {
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
                                    let form_char = forms_chars[i];
                                    let anim_path = path.join(format!("{}", form_char))
                                        .join("anim")
                                        .join(format!("{:03}_{}02.maanim", id, form_char));
                                    if anim_path.exists() {
                                        if let Ok(content) = fs::read_to_string(&anim_path) {
                                            atk_anim_frames[i] = parse_max_frame(&content);
                                        }
                                    }
                                }
                            }

                            let filename = format!("udi{:03}_f.png", id);
                            let img_path = path.join("f").join(&filename);

                            if img_path.exists() {
                                if let Ok(img) = image::open(&img_path) {
                                    let (w, h) = img.dimensions();
                                    if w <= 14 || h <= 2 { continue; }
                                    if id > 25 {
                                        let p = img.get_pixel(14, 2);
                                        if p[3] == 0 { continue; }
                                    }
                                    
                                    if img.pixels().any(|(_, _, pixel)| pixel[3] > 0) {
                                        let mut names = vec![String::new(); 4];
                                        let target_file_id = id + 1; 
                                        
                                        let mut name_file_path = None;
                                        
                                        let lang_dir = path.join("lang");
                                        if lang_dir.exists() {
                                            if let Ok(cat_files) = fs::read_dir(&lang_dir) {
                                                for cf in cat_files.flatten() {
                                                    let name = cf.file_name().to_string_lossy().to_string();
                                                    
                                                    if name.starts_with("Unit_Explanation") && name.ends_with("_en.csv") {
                                                        let num_part = name
                                                            .trim_start_matches("Unit_Explanation")
                                                            .trim_end_matches("_en.csv");
                                                        
                                                        if let Ok(num) = num_part.parse::<u32>() {
                                                            if num == target_file_id {
                                                                name_file_path = Some(cf.path());
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if let Some(p) = name_file_path {
                                            if let Ok(bytes) = fs::read(&p) {
                                                let content = String::from_utf8_lossy(&bytes);
                                                for (i, line) in content.lines().enumerate().take(4) {
                                                    if let Some(name_part) = line.split('|').next() {
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

                                        let entry = CatEntry { 
                                            id, 
                                            image_path: img_path, 
                                            names,
                                            forms,
                                            stats, 
                                            curve: level_curves.get(id as usize).cloned(),
                                            atk_anim_frames,
                                        };
                                        let _ = tx.send(entry);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    rx
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