use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use regex::Regex;
use crate::core::patterns; 
use crate::core::import::log::Logger;

fn count_file_lines(path: &Path) -> usize {
    if let Ok(file_data) = fs::read(path) {
        file_data.iter().filter(|&&byte| byte == b'\n').count()
    } else {
        0
    }
}

fn move_file_if_larger(source_path: &Path, destination_path: &Path) -> std::io::Result<bool> {
    if destination_path.exists() {
        let source_line_count = count_file_lines(source_path);
        let destination_line_count = count_file_lines(destination_path);

        if source_line_count > destination_line_count {
            let _ = fs::remove_file(destination_path);
            fs::rename(source_path, destination_path)?;
            Ok(true)
        } else {
            fs::remove_file(source_path)?;
            Ok(false)
        }
    } else {
        if let Some(parent_directory) = destination_path.parent() {
            if !parent_directory.exists() { let _ = fs::create_dir_all(parent_directory); }
        }
        fs::rename(source_path, destination_path)?;
        Ok(true)
    }
}

fn map_egg_form_code(form_code: &str) -> &str {
    match form_code {
        "00" => "f", 
        _ => "c",    
    }
}

pub fn sort_files(status_sender: Sender<String>) -> Result<(), String> {
    let logger = Logger::new(status_sender);
    let raw_files_directory = Path::new("game/raw");
    let sorted_cats_directory = Path::new("game/cats");
    let sorted_assets_directory = Path::new("game/assets");

    if !raw_files_directory.exists() {
        logger.success("No new raw files to sort. Process complete.");
        return Ok(());
    }

    logger.info("Sorting files...");

    let universal_file_pattern = Regex::new(patterns::CAT_UNIVERSAL_PATTERN).unwrap();
    let skill_desc_pattern = Regex::new(patterns::SKILL_DESC_PATTERN).unwrap();

    let stats_regex = Regex::new(patterns::CAT_STATS_PATTERN).unwrap();
    let icon_regex = Regex::new(patterns::CAT_ICON_PATTERN).unwrap();
    let upgrade_regex = Regex::new(patterns::CAT_UPGRADE_PATTERN).unwrap();
    let gacha_regex = Regex::new(patterns::CAT_GACHA_PATTERN).unwrap();
    let anim_regex = Regex::new(patterns::CAT_ANIM_PATTERN).unwrap();
    let maanim_regex = Regex::new(patterns::CAT_MAANIM_PATTERN).unwrap();
    let explain_regex = Regex::new(patterns::CAT_EXPLAIN_PATTERN).unwrap();

    let egg_icon_regex = Regex::new(patterns::EGG_ICON_PATTERN).unwrap();
    let egg_upgrade_regex = Regex::new(patterns::EGG_UPGRADE_PATTERN).unwrap();
    let egg_gacha_regex = Regex::new(patterns::EGG_GACHA_PATTERN).unwrap();
    let egg_anim_regex = Regex::new(patterns::EGG_ANIM_PATTERN).unwrap();
    let egg_maanim_regex = Regex::new(patterns::EGG_MAANIM_PATTERN).unwrap();

    let mut moved_files_count = 0;
    
    let directory_entries = match fs::read_dir(raw_files_directory) {
        Ok(entries) => entries,
        Err(e) => return Err(format!("Failed to read raw dir: {}", e)),
    };

    for entry_result in directory_entries {
        let entry = entry_result.map_err(|e| e.to_string())?;
        let entry_path = entry.path();
        
        if entry_path.is_dir() { continue; }

        let file_name = match entry_path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        let mut destination_folder = None;

        if skill_desc_pattern.is_match(file_name) {
            destination_folder = Some(sorted_cats_directory.join("SkillDescriptions"));
        }
        else if universal_file_pattern.is_match(file_name) {
            destination_folder = Some(sorted_cats_directory.join("unitevolve"));
        }
        else if patterns::CAT_UNIVERSAL_FILES.contains(&file_name) {
            let destination_file_path = sorted_cats_directory.join(file_name);
            
            if patterns::CHECK_LINE_FILES.contains(&file_name) {
                if let Ok(was_moved) = move_file_if_larger(&entry_path, &destination_file_path) {
                    if was_moved { moved_files_count += 1; }
                }
            } else {
                if destination_file_path.exists() { let _ = fs::remove_file(&destination_file_path); }
                if let Some(parent_dir) = destination_file_path.parent() {
                    if !parent_dir.exists() { let _ = fs::create_dir_all(parent_dir); }
                }
                if fs::rename(&entry_path, &destination_file_path).is_ok() { moved_files_count += 1; }
            }
            continue; 
        }
        else if let Some(caps) = stats_regex.captures(file_name) {
            if let Ok(unit_id) = caps[1].parse::<u32>() {
                if unit_id > 0 {
                    let folder_id = format!("{:03}", unit_id - 1);
                    destination_folder = Some(sorted_cats_directory.join(folder_id));
                }
            }
        }
        else if let Some(caps) = icon_regex.captures(file_name) {
            destination_folder = Some(sorted_cats_directory.join(&caps[1]).join(&caps[2]));
        }
        else if let Some(caps) = upgrade_regex.captures(file_name) {
            destination_folder = Some(sorted_cats_directory.join(&caps[1]).join(&caps[2]));
        }
        else if let Some(caps) = gacha_regex.captures(file_name) {
            destination_folder = Some(sorted_cats_directory.join(&caps[1]));
        }
        else if let Some(caps) = anim_regex.captures(file_name) {
            destination_folder = Some(sorted_cats_directory.join(&caps[1]).join(&caps[2]).join("anim"));
        }
        else if let Some(caps) = maanim_regex.captures(file_name) {
            destination_folder = Some(sorted_cats_directory.join(&caps[1]).join(&caps[2]).join("anim"));
        }
        else if let Some(caps) = explain_regex.captures(file_name) {
            if let Ok(unit_id) = caps[1].parse::<u32>() {
                if unit_id > 0 {
                    let folder_id = format!("{:03}", unit_id - 1);
                    destination_folder = Some(sorted_cats_directory.join(folder_id).join("lang"));
                }
            }
        }
        else if let Some(caps) = egg_icon_regex.captures(file_name) {
            let form_directory = map_egg_form_code(&caps[2]);
            destination_folder = Some(sorted_cats_directory.join(format!("egg_{}", &caps[1])).join(form_directory));
        }
        else if let Some(caps) = egg_upgrade_regex.captures(file_name) {
            let form_directory = map_egg_form_code(&caps[2]);
            destination_folder = Some(sorted_cats_directory.join(format!("egg_{}", &caps[1])).join(form_directory));
        }
        else if let Some(caps) = egg_gacha_regex.captures(file_name) {
            destination_folder = Some(sorted_cats_directory.join(format!("egg_{}", &caps[1])));
        }
        else if let Some(caps) = egg_anim_regex.captures(file_name) {
            destination_folder = Some(sorted_cats_directory.join(format!("egg_{}", &caps[1])).join("anim"));
        }
        else if let Some(caps) = egg_maanim_regex.captures(file_name) {
            destination_folder = Some(sorted_cats_directory.join(format!("egg_{}", &caps[1])).join("anim"));
        }
        else if file_name.starts_with("img015") {
            destination_folder = Some(sorted_assets_directory.join("img015"));
        }

        if let Some(target_folder) = destination_folder {
            if !target_folder.exists() {
                fs::create_dir_all(&target_folder).map_err(|e| e.to_string())?;
            }
            let destination_file_path = target_folder.join(file_name);
            
            if patterns::CHECK_LINE_FILES.contains(&file_name) {
                if let Ok(was_moved) = move_file_if_larger(&entry_path, &destination_file_path) {
                    if was_moved { moved_files_count += 1; }
                }
            } else {
                if destination_file_path.exists() { let _ = fs::remove_file(&destination_file_path); }
                fs::rename(&entry_path, &destination_file_path).map_err(|e| e.to_string())?;
                moved_files_count += 1;
            }
            
            if moved_files_count % 500 == 0 {
                logger.info(format!("Sorted {} files...", moved_files_count));
            }
        }
    }

    logger.success(format!("Sorting complete. Organized {} files.", moved_files_count));
    Ok(())
}