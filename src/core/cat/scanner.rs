use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::{Arc, mpsc::{self, Receiver}};
use rayon::prelude::*;
use regex::Regex; 
use image::GenericImageView; 

use crate::core::patterns; 
use crate::core::files::unitid::CatRaw;
use crate::core::files::unitbuy::{self, UnitBuyRow};
use crate::core::files::unitlevel::{self, CatLevelCurve};
use crate::core::files::skillacquisition::{self, TalentRaw}; 
use crate::core::utils; 

#[derive(Clone, Debug)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: PathBuf,
    pub names: Vec<String>, 
    pub forms: [bool; 4],
    pub stats: Vec<Option<CatRaw>>,
    pub curve: Option<CatLevelCurve>,
    pub atk_anim_frames: [i32; 4], 
    pub egg_ids: (i32, i32),
    pub talent_data: Option<TalentRaw>, 
}

pub fn start_scan(language_code: String) -> Receiver<CatEntry> {
    let (cat_sender, cat_receiver) = mpsc::channel();

    thread::spawn(move || {
        let cats_directory = Path::new("game/cats");
        
        let level_curves_arc = Arc::new(unitlevel::load_level_curves(cats_directory));
        let unit_buy_map_arc = Arc::new(unitbuy::load_unitbuy(cats_directory));
        let talent_map_arc = Arc::new(skillacquisition::load(cats_directory));
        
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
            
            if let Some(cat_entry) = process_cat_entry(folder_path, &curves_clone, &unit_buys_clone, &talents_clone, &language_code) {
                let _ = sender_clone.send(cat_entry);
            }
        });
    });
    cat_receiver
}

fn process_cat_entry(
    original_folder_path: &Path, 
    level_curves: &Vec<CatLevelCurve>, 
    unit_buys: &std::collections::HashMap<u32, UnitBuyRow>,
    talents_map: &std::collections::HashMap<u16, TalentRaw>, 
    language_code: &str
) -> Option<CatEntry> {
    
    let folder_stem = original_folder_path.file_name()?.to_str()?;
    let cat_id = folder_stem.parse::<u32>().ok()?;

    let unit_buy_data = unit_buys.get(&cat_id);
    if let Some(row_data) = unit_buy_data {
        let is_egg_unit = row_data.egg_id_normal != -1;
        if !is_egg_unit && row_data.guide_order == -1 && cat_id != 673 {
            return None; 
        }
    } else {
        return None;
    }
    let ub_row = unit_buy_data.unwrap(); 

    let cats_root_dir = Path::new("game/cats");
    
    let get_form_path = |form_index: usize, form_char: char| -> PathBuf {
        let is_egg_normal_form = form_index == 0 && ub_row.egg_id_normal != -1;
        let is_egg_evolved_form = form_index == 1 && ub_row.egg_id_evolved != -1;

        if is_egg_normal_form {
            cats_root_dir.join(format!("egg_{:03}", ub_row.egg_id_normal)).join(form_char.to_string())
        } else if is_egg_evolved_form {
            cats_root_dir.join(format!("egg_{:03}", ub_row.egg_id_evolved)).join(form_char.to_string())
        } else {
            original_folder_path.join(form_char.to_string())
        }
    };

    let get_anim_file_path = |form_index: usize, form_char: char| -> PathBuf {
        let is_egg_normal_form = form_index == 0 && ub_row.egg_id_normal != -1;
        let is_egg_evolved_form = form_index == 1 && ub_row.egg_id_evolved != -1;

        if is_egg_normal_form {
            cats_root_dir.join(format!("egg_{:03}", ub_row.egg_id_normal))
                     .join("anim")
                     .join(format!("{:03}_m02.maanim", ub_row.egg_id_normal))
        } else if is_egg_evolved_form {
            cats_root_dir.join(format!("egg_{:03}", ub_row.egg_id_evolved))
                     .join("anim")
                     .join(format!("{:03}_m02.maanim", ub_row.egg_id_evolved))
        } else {
            original_folder_path.join(form_char.to_string())
                         .join("anim")
                         .join(format!("{:03}_{}02.maanim", cat_id, form_char))
        }
    };

    let form_suffixes = ['f', 'c', 's', 'u'];
    let form_folder_paths = [
        get_form_path(0, 'f'),
        get_form_path(1, 'c'),
        get_form_path(2, 's'),
        get_form_path(3, 'u'),
    ];

    let forms_existence = [
        form_folder_paths[0].exists(),
        form_folder_paths[1].exists(),
        form_folder_paths[2].exists(),
        form_folder_paths[3].exists(),
    ];

    let mut attack_anim_frames = [0; 4];
    for i in 0..4 {
        if forms_existence[i] {
            let anim_file_path = get_anim_file_path(i, form_suffixes[i]);
            if let Ok(file_content) = fs::read_to_string(&anim_file_path) {
                attack_anim_frames[i] = parse_anim_length(&file_content);
            }
        }
    }

    let find_image_path = |search_dir: &Path, file_stem: &str| -> Option<PathBuf> {
        let png_path = search_dir.join(format!("{}.png", file_stem));
        if png_path.exists() { return Some(png_path); }
        let uppercase_png_path = search_dir.join(format!("{}.PNG", file_stem));
        if uppercase_png_path.exists() { return Some(uppercase_png_path); }
        None
    };

    let final_image_path_opt = if ub_row.egg_id_normal != -1 {
        find_image_path(&form_folder_paths[0], &format!("udi{:03}_m00", ub_row.egg_id_normal))
            .or_else(|| find_image_path(&form_folder_paths[0], &format!("uni{:03}_m00", ub_row.egg_id_normal)))
    } else {
        find_image_path(&form_folder_paths[0], &format!("udi{:03}_f", cat_id))
            .or_else(|| find_image_path(&form_folder_paths[0], &format!("uni{:03}_f00", cat_id)))
    };

    let valid_image_path = match final_image_path_opt {
        Some(p) => p,
        None => return None, 
    };

    match image::open(&valid_image_path) {
        Ok(img) => {
            let (w, h) = img.dimensions();
            if w < 50 || h < 30 { return None; }
        },
        Err(_) => { return None; }
    }
    
    let mut cat_names = vec![String::new(); 4];
    
    let target_file_id = cat_id + 1;
    let lang_directory = original_folder_path.join("lang"); 

    let language_codes_to_check: Vec<&str> = if language_code.is_empty() {
        utils::LANGUAGE_PRIORITY.to_vec()
    } else {
        vec![language_code]
    };

    for code in language_codes_to_check {
        let all_found = (0..4).all(|i| !forms_existence[i] || !cat_names[i].is_empty());
        if all_found { break; }

        if let Some(name_file_path) = find_name_file_for_code(&lang_directory, target_file_id, code) {
            if let Ok(file_bytes) = fs::read(&name_file_path) {
                let file_content = String::from_utf8_lossy(&file_bytes);
                let separator_char = if code == "ja" { ',' } else { '|' };

                let mut current_lang_names = vec![String::new(); 4];
                for (line_index, file_line) in file_content.lines().enumerate().take(4) {
                    if let Some(name_part) = file_line.split(separator_char).next() {
                        let trimmed_name = name_part.trim();
                        if !trimmed_name.is_empty() && !looks_like_garbage_id(trimmed_name) {
                            current_lang_names[line_index] = trimmed_name.to_string();
                        }
                    }
                }

                for i in 0..4 {
                    if !cat_names[i].is_empty() { continue; }
                    
                    if !forms_existence[i] { continue; }

                    let candidate = &current_lang_names[i];
                    
                    if candidate.is_empty() { continue; }

                    if i > 0 {
                        let prev_name_source = &current_lang_names[i-1];
                        if candidate == prev_name_source {
                             continue;
                        }
                    }

                    cat_names[i] = candidate.clone();
                }
            }
        }
    }
    
    if cat_id == 673 && cat_names[0].is_empty() {
        cat_names[0] = "Cheetah Cat".to_string();
    }
    
    let mut cat_stats = vec![None; 4];
    let stats_file_path = original_folder_path.join(format!("unit{:03}.csv", target_file_id));
    if let Ok(file_content) = fs::read_to_string(&stats_file_path) {
        for (line_index, csv_line) in file_content.lines().enumerate().take(4) {
            cat_stats[line_index] = CatRaw::from_csv_line(csv_line);
        }
    }

    let talent_data = talents_map.get(&(cat_id as u16)).cloned();

    Some(CatEntry { 
        id: cat_id, 
        image_path: valid_image_path,
        names: cat_names,
        forms: forms_existence,
        stats: cat_stats, 
        curve: level_curves.get(cat_id as usize).cloned(),
        atk_anim_frames: attack_anim_frames,
        egg_ids: (ub_row.egg_id_normal, ub_row.egg_id_evolved),
        talent_data, 
    })
}

fn looks_like_garbage_id(text: &str) -> bool {
    text.chars().all(|char_check| char_check.is_ascii_digit() || char_check == '-' || char_check == '_')
}

fn find_name_file_for_code(lang_directory: &Path, target_id: u32, region_code: &str) -> Option<PathBuf> {
    if !lang_directory.exists() { return None; }
    
    if region_code.is_empty() {
        let expected_filename = format!("Unit_Explanation{}.csv", target_id);
        let default_path = lang_directory.join(&expected_filename);
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
                    if parsed_id == target_id {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

fn parse_anim_length(file_content: &str) -> i32 {
    let mut max_frame_count = 0;
    let maanim_lines: Vec<Vec<i32>> = file_content
        .lines()
        .map(|line| {
            line.split(',')
                .filter_map(|component| component.trim().parse::<i32>().ok())
                .collect()
        })
        .collect();

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