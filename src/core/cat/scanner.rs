use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::{Arc, mpsc::{self, Receiver}};
use rayon::prelude::*;
use regex::Regex; 
use image::GenericImageView; 
use crate::core::patterns; 
use crate::data::cat::unitid::CatRaw;
use crate::data::cat::unitbuy::{self, UnitBuyRow};
use crate::data::cat::unitlevel::{self, CatLevelCurve};
use crate::data::cat::skillacquisition::{self, TalentRaw}; 
use crate::data::cat::unitevolve; 
use crate::core::utils; 
use crate::paths::cat::{self, AssetType, AnimType};
use crate::core::settings::handle::ScannerConfig;

#[derive(Clone, Debug)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: Option<PathBuf>, 
    pub names: Vec<String>,
    pub description: Vec<Vec<String>>,
    pub forms: [bool; 4],
    pub stats: Vec<Option<CatRaw>>,
    pub curve: Option<CatLevelCurve>,
    pub atk_anim_frames: [i32; 4], 
    pub egg_ids: (i32, i32),
    pub talent_data: Option<TalentRaw>,
    pub unit_buy: UnitBuyRow,
    pub evolve_text: [Vec<String>; 4], 
}

pub fn start_scan(config: ScannerConfig) -> Receiver<CatEntry> {
    let (cat_sender, cat_receiver) = mpsc::channel();

    thread::spawn(move || {
        let cats_directory = Path::new(cat::DIR_CATS);
        
        let level_curves_arc = Arc::new(unitlevel::load_level_curves(cats_directory));
        let unit_buy_map_arc = Arc::new(unitbuy::load_unitbuy(cats_directory));
        let talent_map_arc = Arc::new(skillacquisition::load(cats_directory));
        let evolve_text_map_arc = Arc::new(unitevolve::load(cats_directory, &config.language));
        
        let folder_entries: Vec<PathBuf> = match fs::read_dir(cats_directory) {
            Ok(read_dir_iter) => read_dir_iter
                .filter_map(|entry_result| entry_result.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect(),
            Err(_) => Vec::new(),
        };

        folder_entries.par_iter().for_each(|folder_path| {
            let sender_clone = cat_sender.clone();
            let curves_clone = Arc::clone(&level_curves_arc);
            let unit_buys_clone = Arc::clone(&unit_buy_map_arc);
            let talents_clone = Arc::clone(&talent_map_arc);
            let evolve_text_clone = Arc::clone(&evolve_text_map_arc);
            
            if let Some(cat_entry) = process_cat_entry(
                folder_path, 
                &curves_clone, 
                &unit_buys_clone, 
                &talents_clone, 
                &evolve_text_clone, 
                &config
            ) {
                let _ = sender_clone.send(cat_entry);
            }
        });
    });
    cat_receiver
}

pub fn process_cat_entry(
    original_folder_path: &Path, 
    level_curves: &Vec<CatLevelCurve>, 
    unit_buys: &std::collections::HashMap<u32, UnitBuyRow>,
    talents_map: &std::collections::HashMap<u16, TalentRaw>, 
    evolve_text_map: &std::collections::HashMap<u32, [Vec<String>; 4]>, 
    config: &ScannerConfig
) -> Option<CatEntry> {
    
    let folder_stem = original_folder_path.file_name()?.to_str()?;
    let cat_id = folder_stem.parse::<u32>().ok()?;

    let ub_row = unit_buys.get(&cat_id)?;

    let is_egg_unit = ub_row.egg_id_normal != -1;
    let is_hidden = ub_row.guide_order == -1 && cat_id != 673;

    if !config.show_invalid && !is_egg_unit && is_hidden {
        return None; 
    }

    let cats_root_dir = Path::new(cat::DIR_CATS);
    let egg_ids = (ub_row.egg_id_normal, ub_row.egg_id_evolved);

    let mut forms_existence = [false; 4];
    for i in 0..4 {
        let folder = cat::folder(cats_root_dir, cat_id, i, egg_ids);
        forms_existence[i] = folder.exists();
    }

    let mut final_image_path_opt = None;
    for form_idx in (0..=config.preferred_form).rev() {
        if let Some(path) = cat::image(cats_root_dir, AssetType::Banner, cat_id, form_idx, egg_ids) {
            final_image_path_opt = Some(path);
            break;
        }
    }

    if !config.show_invalid {
        if let Some(path) = &final_image_path_opt {
            match image::open(path) {
                Ok(img) => {
                    let (w, h) = img.dimensions();
                    if w < 50 || h < 30 { return None; }
                },
                Err(_) => { return None; }
            }
        } else {
            return None;
        }
    }

    let mut attack_anim_frames = [0; 4];
    for i in 0..4 {
        if forms_existence[i] {
            let anim_path = cat::anim(cats_root_dir, cat_id, i, egg_ids, AnimType::Maanim);
            if let Ok(file_content) = fs::read_to_string(&anim_path) {
                attack_anim_frames[i] = parse_anim_length(&file_content);
            }
        }
    }
    
    let mut cat_stats = vec![None; 4];
    let stats_file_path = cat::stats(cats_root_dir, cat_id);
    if let Ok(file_content) = fs::read_to_string(&stats_file_path) {
        let delimiter = utils::detect_csv_separator(&file_content);
        for (line_index, csv_line) in file_content.lines().enumerate().take(4) {
            cat_stats[line_index] = CatRaw::from_csv_line(csv_line, delimiter);
        }
    }

    let mut cat_names = vec![String::new(); 4];
    let mut cat_descriptions = vec![Vec::new(); 4];
    
    let target_file_id = cat_id + 1;
    let lang_directory = cat::lang(cats_root_dir, cat_id);

    let language_codes_to_check: Vec<&str> = if config.language.is_empty() {
        utils::LANGUAGE_PRIORITY.to_vec()
    } else {
        vec![&config.language]
    };

    for code in language_codes_to_check {
        let all_found = (0..4).all(|i| !forms_existence[i] || !cat_names[i].is_empty());
        if all_found { break; }

        if let Some(name_file_path) = find_name_file_for_code(&lang_directory, target_file_id, code) {
            if let Ok(file_bytes) = fs::read(&name_file_path) {
                let file_content = String::from_utf8_lossy(&file_bytes);
                let separator_char = utils::detect_csv_separator(&file_content);
                let mut current_lang_names = vec![String::new(); 4];
                let mut current_lang_descs = vec![Vec::new(); 4];

                for (line_index, file_line) in file_content.lines().enumerate().take(4) {
                    let parts: Vec<&str> = file_line.split(separator_char).collect();
                    if let Some(name_part) = parts.get(0) {
                        let trimmed_name = name_part.trim();
                        if !trimmed_name.is_empty() && !looks_like_garbage_id(trimmed_name) {
                            current_lang_names[line_index] = trimmed_name.to_string();
                        }
                    }
                    let desc_lines: Vec<String> = parts.iter().skip(1).take(3).map(|s| s.trim().to_string()).collect();
                    current_lang_descs[line_index] = desc_lines;
                }

                for i in 0..4 {
                    if !cat_names[i].is_empty() && forms_existence[i] { continue; }
                    if !forms_existence[i] { continue; }
                    let candidate = &current_lang_names[i];
                    if candidate.is_empty() { continue; }
                    if i > 0 {
                        let prev_name_source = &current_lang_names[i-1];
                        if candidate == prev_name_source { continue; }
                    }
                    cat_names[i] = candidate.clone();
                    cat_descriptions[i] = current_lang_descs[i].clone(); 
                }
            }
        }
    }
    
    if cat_id == 673 && cat_names[0].is_empty() {
        cat_names[0] = "Cheetah Cat".to_string();
    }

    let talent_data = talents_map.get(&(cat_id as u16)).cloned();
    let evolve_text = evolve_text_map.get(&cat_id).cloned().unwrap_or_default();

    Some(CatEntry { 
        id: cat_id, 
        image_path: final_image_path_opt, 
        names: cat_names,
        description: cat_descriptions,
        forms: forms_existence,
        stats: cat_stats, 
        curve: level_curves.get(cat_id as usize).cloned(),
        atk_anim_frames: attack_anim_frames,
        egg_ids,
        talent_data,
        unit_buy: ub_row.clone(),
        evolve_text,
    })
}

fn looks_like_garbage_id(text: &str) -> bool {
    text.chars().all(|char_check| char_check.is_ascii_digit() || char_check == '-' || char_check == '_')
}

fn find_name_file_for_code(lang_directory: &Path, target_id: u32, region_code: &str) -> Option<PathBuf> {
    if !lang_directory.exists() { return None; }
    
    if region_code.is_empty() {
        let default_path = lang_directory.join(format!("Unit_Explanation{}.csv", target_id));
        return if default_path.exists() { Some(default_path) } else { None };
    }

    let regex_pattern = Regex::new(patterns::CAT_EXPLAIN_PATTERN).ok()?;
    for entry_result in fs::read_dir(lang_directory).ok()? {
        let entry = entry_result.ok()?;
        let path = entry.path();
        let file_name = path.file_name()?.to_string_lossy();
        if let Some(captures) = regex_pattern.captures(&file_name) {
            let file_id_str = &captures[1];
            let file_code_str = &captures[2];
            if file_code_str == region_code {
                if let Ok(parsed_id) = file_id_str.parse::<u32>() {
                    if parsed_id == target_id { return Some(path); }
                }
            }
        }
    }
    None
}

fn parse_anim_length(file_content: &str) -> i32 {
    let mut max_frame_count = 0;
    let delimiter = utils::detect_csv_separator(file_content);
    let maanim_lines: Vec<Vec<i32>> = file_content.lines().map(|line| {
            line.split(delimiter).filter_map(|component| component.trim().parse::<i32>().ok()).collect()
        }).collect();
    for (line_index, line_values) in maanim_lines.iter().enumerate() {
        if line_values.len() < 5 { continue; }
        let following_lines_count = maanim_lines.get(line_index + 1).and_then(|l| l.get(0)).cloned().unwrap_or(0) as usize;
        if following_lines_count == 0 { continue; }
        let first_frame = maanim_lines.get(line_index + 2).and_then(|l| l.get(0)).cloned().unwrap_or(0);
        let last_frame = maanim_lines.get(line_index + following_lines_count + 1).and_then(|l| l.get(0)).cloned().unwrap_or(0);
        let animation_duration = last_frame - first_frame;
        let loop_repeats = std::cmp::max(line_values[2], 1); 
        let final_frame_used = (animation_duration * loop_repeats) + first_frame;
        max_frame_count = std::cmp::max(final_frame_used, max_frame_count);
    }
    max_frame_count + 1 
}