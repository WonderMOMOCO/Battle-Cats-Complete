use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use regex::Regex;
use crate::patterns;

fn count_lines(path: &Path) -> usize {
    if let Ok(data) = fs::read(path) {
        data.iter().filter(|&&b| b == b'\n').count()
    } else {
        0
    }
}

fn move_if_bigger(src: &Path, dest: &Path) -> std::io::Result<bool> {
    if dest.exists() {
        let src_lines = count_lines(src);
        let dest_lines = count_lines(dest);

        if src_lines > dest_lines {
            let _ = fs::remove_file(dest);
            fs::rename(src, dest)?;
            Ok(true)
        } else {
            fs::remove_file(src)?;
            Ok(false)
        }
    } else {
        if let Some(parent) = dest.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        fs::rename(src, dest)?;
        Ok(true)
    }
}

pub fn sort_game_files(tx: Sender<String>) -> Result<(), String> {
    let raw_dir = Path::new("game/raw");
    let cats_dir = Path::new("game/cats");
    let assets_dir = Path::new("game/assets");

    if !raw_dir.exists() {
        return Err("Raw directory not found. Did extraction fail?".to_string());
    }

    let _ = tx.send("Sorting files...".to_string());

    for &sensitive_file in patterns::REGION_SENSITIVE_FILES {
        let target_path = raw_dir.join(sensitive_file);
        if target_path.exists() {
            let _ = fs::remove_file(target_path);
        }
    }

    let universal_pattern = Regex::new(patterns::CAT_UNIVERSAL_PATTERN).unwrap();
    let re_stats = Regex::new(patterns::CAT_STATS_PATTERN).unwrap();
    let re_icon = Regex::new(patterns::CAT_ICON_PATTERN).unwrap();
    let re_upgrade = Regex::new(patterns::CAT_UPGRADE_PATTERN).unwrap();
    let re_gacha = Regex::new(patterns::CAT_GACHA_PATTERN).unwrap();
    let re_anim = Regex::new(patterns::CAT_ANIM_PATTERN).unwrap();
    let re_maanim = Regex::new(patterns::CAT_MAANIM_PATTERN).unwrap();
    let re_explain = Regex::new(patterns::CAT_EXPLAIN_PATTERN).unwrap();

    let mut moved_count = 0;
    
    for entry in fs::read_dir(raw_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.is_dir() { continue; }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        if filename.starts_with("unitevolve_") && filename.ends_with(".csv") {
            let dest_folder = cats_dir.join("unitevolve");
            if !dest_folder.exists() { let _ = fs::create_dir_all(&dest_folder); }
            
            let dest_path = dest_folder.join(filename);
            if dest_path.exists() { let _ = fs::remove_file(&dest_path); }
            
            if fs::rename(&path, &dest_path).is_ok() {
                moved_count += 1;
            }
            continue;
        }

        if filename.starts_with("SkillDescriptions") && filename.ends_with(".csv") {
            let dest_folder = cats_dir.join("SkillDescriptions");
            if !dest_folder.exists() { let _ = fs::create_dir_all(&dest_folder); }
            
            let dest_path = dest_folder.join(filename);
            if dest_path.exists() { let _ = fs::remove_file(&dest_path); }

            if fs::rename(&path, &dest_path).is_ok() {
                moved_count += 1;
            }
            continue;
        }

        if patterns::CHECK_LINE_FILES.contains(&filename) {
            let dest_path = cats_dir.join(filename);
            if let Ok(was_moved) = move_if_bigger(&path, &dest_path) {
                if was_moved { moved_count += 1; }
            }
            continue;
        }

        let mut dest_folder = None;

        if patterns::CAT_UNIVERSAL_FILES.contains(&filename) || universal_pattern.is_match(filename) {
            dest_folder = Some(cats_dir.to_path_buf());
        }
        else if let Some(caps) = re_stats.captures(filename) {
            if let Ok(file_id) = caps[1].parse::<u32>() {
                if file_id > 0 {
                    let unit_id = file_id - 1;
                    let folder_id = format!("{:03}", unit_id);
                    dest_folder = Some(cats_dir.join(folder_id));
                }
            }
        }
        else if let Some(caps) = re_icon.captures(filename) {
            let (id, form) = (&caps[1], &caps[2]);
            dest_folder = Some(cats_dir.join(id).join(form));
        }
        else if let Some(caps) = re_upgrade.captures(filename) {
            let (id, form) = (&caps[1], &caps[2]);
            dest_folder = Some(cats_dir.join(id).join(form));
        }
        else if let Some(caps) = re_gacha.captures(filename) {
            let id = &caps[1];
            dest_folder = Some(cats_dir.join(id));
        }
        else if let Some(caps) = re_anim.captures(filename) {
            let (id, form) = (&caps[1], &caps[2]);
            dest_folder = Some(cats_dir.join(id).join(form).join("anim"));
        }
        else if let Some(caps) = re_maanim.captures(filename) {
            let (id, form) = (&caps[1], &caps[2]);
            dest_folder = Some(cats_dir.join(id).join(form).join("anim"));
        }
        else if let Some(caps) = re_explain.captures(filename) {
            let raw_id = &caps[1];
            if let Ok(file_id) = raw_id.parse::<u32>() {
                if file_id > 0 {
                    let unit_id = file_id - 1;
                    let folder_id = format!("{:03}", unit_id);
                    dest_folder = Some(cats_dir.join(folder_id).join("lang"));
                }
            }
        }
        else if filename.starts_with("img015") {
            dest_folder = Some(assets_dir.join("img015"));
        }

        if let Some(folder) = dest_folder {
            if !folder.exists() {
                fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            }
            let dest_path = folder.join(filename);
            
            if dest_path.exists() {
                let _ = fs::remove_file(&dest_path);
            }
            
            fs::rename(&path, &dest_path).map_err(|e| e.to_string())?;
            moved_count += 1;
            
            if moved_count % 500 == 0 {
                let _ = tx.send(format!("Sorted {} files...", moved_count));
            }
        }
    }

    let _ = tx.send(format!("Sorting complete! Moved {} files.", moved_count));
    Ok(())
}