use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::mpsc::Sender;
use regex::Regex;
use crate::core::patterns; 

fn count_lines(path: &Path) -> usize {
    if let Some(ext) = path.extension() {
        let s = ext.to_string_lossy();
        if s == "png" || s == "imgcut" || s == "mamodel" { return 0; }
    }

    if let Ok(f) = fs::File::open(path) {
        let reader = io::BufReader::new(f);
        reader.lines().count()
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
            if !parent.exists() { let _ = fs::create_dir_all(parent); }
        }
        fs::rename(src, dest)?;
        Ok(true)
    }
}

fn move_fast(src: &Path, dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        if !parent.exists() { let _ = fs::create_dir_all(parent); }
    }
    if dest.exists() {
        let _ = fs::remove_file(dest);
    }
    fs::rename(src, dest)?;
    Ok(())
}

fn map_egg(code: &str) -> &str {
    match code { "00" => "f", _ => "c" }
}

pub fn sort_game_files(tx: Sender<String>) -> Result<(), String> {
    let raw_dir = Path::new("game/raw");
    let cats_dir = Path::new("game/cats");
    let assets_dir = Path::new("game/assets");

    if !raw_dir.exists() {
        return Err("Raw directory not found.".to_string());
    }

    let _ = tx.send("Sorting files...".to_string());

    let universal = Regex::new(patterns::CAT_UNIVERSAL_PATTERN).unwrap();
    let skill_desc = Regex::new(patterns::SKILL_DESC_PATTERN).unwrap();
    let stats = Regex::new(patterns::CAT_STATS_PATTERN).unwrap();
    let icon = Regex::new(patterns::CAT_ICON_PATTERN).unwrap();
    let upgrade = Regex::new(patterns::CAT_UPGRADE_PATTERN).unwrap();
    let gacha = Regex::new(patterns::CAT_GACHA_PATTERN).unwrap();
    let anim = Regex::new(patterns::CAT_ANIM_PATTERN).unwrap();
    let maanim = Regex::new(patterns::CAT_MAANIM_PATTERN).unwrap();
    let explain = Regex::new(patterns::CAT_EXPLAIN_PATTERN).unwrap();
    let skill_name = Regex::new(patterns::SKILL_NAME_PATTERN).unwrap();
    
    let egg_icon = Regex::new(patterns::EGG_ICON_PATTERN).unwrap();
    let egg_upgrade = Regex::new(patterns::EGG_UPGRADE_PATTERN).unwrap();
    let egg_gacha = Regex::new(patterns::EGG_GACHA_PATTERN).unwrap();
    let egg_anim = Regex::new(patterns::EGG_ANIM_PATTERN).unwrap();
    let egg_maanim = Regex::new(patterns::EGG_MAANIM_PATTERN).unwrap();

    let gatya_item_d = Regex::new(patterns::GATYA_ITEM_D_PATTERN).unwrap();
    let gatya_item_buy = Regex::new(patterns::GATYA_ITEM_BUY_PATTERN).unwrap();
    let gatya_item_name = Regex::new(patterns::GATYA_ITEM_NAME_PATTERN).unwrap();
    
    let img015 = Regex::new(patterns::ASSET_IMG015_PATTERN).unwrap();
    let img015_cut = Regex::new(patterns::ASSET_015CUT_PATTERN).unwrap();

    let mut count = 0;
    
    for entry in fs::read_dir(raw_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() { continue; }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        let mut dest_folder = None;

        if skill_desc.is_match(name) {
            dest_folder = Some(cats_dir.join("SkillDescriptions"));
        }
        else if universal.is_match(name) {
            dest_folder = Some(cats_dir.join("unitevolve"));
        }
        else if patterns::CAT_UNIVERSAL_FILES.contains(&name) {
            let dest = cats_dir.join(name);
            
            if patterns::CHECK_LINE_FILES.contains(&name) {
                if let Ok(moved) = move_if_bigger(&path, &dest) {
                    if moved { count += 1; }
                }
            } else {
                if move_fast(&path, &dest).is_ok() { count += 1; }
            }
            continue; 
        }
        else if let Some(caps) = stats.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() {
                if id > 0 { dest_folder = Some(cats_dir.join(format!("{:03}", id - 1))); }
            }
        }
        else if let Some(caps) = icon.captures(name) { dest_folder = Some(cats_dir.join(&caps[1]).join(&caps[2])); }
        else if let Some(caps) = upgrade.captures(name) { dest_folder = Some(cats_dir.join(&caps[1]).join(&caps[2])); }
        else if let Some(caps) = gacha.captures(name) { dest_folder = Some(cats_dir.join(&caps[1])); }
        else if let Some(caps) = anim.captures(name) { dest_folder = Some(cats_dir.join(&caps[1]).join(&caps[2]).join("anim")); }
        else if let Some(caps) = maanim.captures(name) { dest_folder = Some(cats_dir.join(&caps[1]).join(&caps[2]).join("anim")); }
        else if let Some(caps) = explain.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() {
                if id > 0 { dest_folder = Some(cats_dir.join(format!("{:03}", id - 1)).join("lang")); }
            }
        }
        else if let Some(_caps) = skill_name.captures(name) {
            dest_folder = Some(assets_dir.join("Skill_name"));
        }
        else if gatya_item_d.is_match(name) || gatya_item_buy.is_match(name) {
            dest_folder = Some(assets_dir.join("gatyaitemD"));
        }
        else if gatya_item_name.is_match(name) {
            dest_folder = Some(assets_dir.join("gatyaitemD").join("GatyaitemName"));
        }
        else if let Some(caps) = egg_icon.captures(name) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])).join(map_egg(&caps[2])));
        }
        else if let Some(caps) = egg_upgrade.captures(name) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])).join(map_egg(&caps[2])));
        }
        else if let Some(caps) = egg_gacha.captures(name) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])));
        }
        else if let Some(caps) = egg_anim.captures(name) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])).join("anim"));
        }
        else if let Some(caps) = egg_maanim.captures(name) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])).join("anim"));
        }
        else if img015.is_match(name) || img015_cut.is_match(name) {
            dest_folder = Some(assets_dir.join("img015"));
        }

        if let Some(folder) = dest_folder {
            if !folder.exists() { 
                let _ = fs::create_dir_all(&folder); 
            }
            let dest = folder.join(name);

            if patterns::CHECK_LINE_FILES.contains(&name) {
                if let Ok(moved) = move_if_bigger(&path, &dest) {
                    if moved { count += 1; }
                }
            } else {
                if move_fast(&path, &dest).is_ok() { count += 1; }
            }
            
            if count % 500 == 0 {
                let _ = tx.send(format!("Sorted {} files | Current: {}", count, name));
            }
        }
    }

    let _ = tx.send("Success! Files sorted.".to_string());
    Ok(())
}