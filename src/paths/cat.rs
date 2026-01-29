#![allow(dead_code)]
use std::path::{Path, PathBuf};

// Global File Constants
pub const UNIT_BUY: &str = "unitbuy.csv";
pub const UNIT_LEVEL: &str = "unitLevel.csv";
pub const SKILL_ACQUISITION: &str = "SkillAcquisition.csv";
pub const SKILL_LEVEL: &str = "SkillLevel.csv"; 

// Directory Constants
pub const DIR_CATS: &str = "game/cats";
pub const DIR_ANIM: &str = "anim";
pub const DIR_LANG: &str = "lang";
pub const DIR_UNIT_EVOLVE: &str = "unitevolve";
pub const DIR_SKILL_DESCRIPTIONS: &str = "SkillDescriptions";
pub const DIR_SKILL_NAME: &str = "game/assets/Skill_name"; 

// Asset Constants
pub const FALLBACK_ICON: &str = "game/cats/uni.png"; 

#[derive(Copy, Clone, PartialEq)]
pub enum AssetType {
    Icon,   // uni
    Banner, // udi
}

#[derive(Copy, Clone, PartialEq)]
pub enum AnimType {
    Maanim,
    Mamodel,
    Imgcut,
    Png,
}

impl AnimType {
    pub fn ext(&self) -> &str {
        match self {
            AnimType::Maanim => "maanim",
            AnimType::Mamodel => "mamodel",
            AnimType::Imgcut => "imgcut",
            AnimType::Png => "png",
        }
    }
}

// Unit Specific Paths

pub fn folder(root: &Path, id: u32, form: usize, egg_ids: (i32, i32)) -> PathBuf {
    let (egg_norm, egg_evol) = egg_ids;
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };
    
    if form == 0 && egg_norm != -1 {
        return root.join(format!("egg_{:03}", egg_norm)).join(form_char);
    }
    if form == 1 && egg_evol != -1 {
        return root.join(format!("egg_{:03}", egg_evol)).join(form_char);
    }
    root.join(format!("{:03}", id)).join(form_char)
}

pub fn image_stem(asset_type: AssetType, id: u32, form: usize, egg_ids: (i32, i32)) -> String {
    let (egg_norm, egg_evol) = egg_ids;
    let prefix = match asset_type { AssetType::Icon => "uni", AssetType::Banner => "udi" };
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };

    if form == 0 && egg_norm != -1 {
        return format!("{}{:03}_m00", prefix, egg_norm);
    }
    if form == 1 && egg_evol != -1 {
        return format!("{}{:03}_m01", prefix, egg_evol);
    }
    match asset_type {
        AssetType::Icon => format!("{}{:03}_{}00", prefix, id, form_char),
        AssetType::Banner => format!("{}{:03}_{}", prefix, id, form_char),
    }
}

pub fn image(root: &Path, asset_type: AssetType, id: u32, form: usize, egg_ids: (i32, i32)) -> Option<PathBuf> {
    let folder_path = folder(root, id, form, egg_ids);
    let stem = image_stem(asset_type, id, form, egg_ids);
    
    let png = folder_path.join(format!("{}.png", stem));
    if png.exists() { return Some(png); }

    if form == 1 && egg_ids.1 != -1 {
         let prefix = match asset_type { AssetType::Icon => "uni", AssetType::Banner => "udi" };
         let fallback_stem = format!("{}{:03}_m00", prefix, egg_ids.1);
         let fallback_png = folder_path.join(format!("{}.png", fallback_stem));
         if fallback_png.exists() { return Some(fallback_png); }
    }
    None
}

pub fn anim(root: &Path, id: u32, form: usize, egg_ids: (i32, i32), file_type: AnimType) -> PathBuf {
    let (egg_norm, egg_evol) = egg_ids;
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };
    let ext = file_type.ext();

    if form == 0 && egg_norm != -1 {
        return root.join(format!("egg_{:03}", egg_norm))
                   .join(DIR_ANIM)
                   .join(format!("{:03}_m02.{}", egg_norm, ext));
    }
    if form == 1 && egg_evol != -1 {
        return root.join(format!("egg_{:03}", egg_evol))
                   .join(DIR_ANIM)
                   .join(format!("{:03}_m02.{}", egg_evol, ext));
    }
    root.join(format!("{:03}", id))
        .join(form_char)
        .join(DIR_ANIM)
        .join(format!("{:03}_{}02.{}", id, form_char, ext))
}

pub fn stats(root: &Path, id: u32) -> PathBuf {
    let file_id = id + 1;
    root.join(format!("{:03}", id))
        .join(format!("unit{:03}.csv", file_id))
}

pub fn lang(root: &Path, id: u32) -> PathBuf {
    root.join(format!("{:03}", id)).join(DIR_LANG)
}

pub fn explanation(root: &Path, id: u32) -> PathBuf {
    let file_id = id + 1;
    lang(root, id).join(format!("Unit_Explanation{}.csv", file_id))
}

pub fn skill_icon(root: &Path, id: i16, lang: &str) -> PathBuf {
    let filename = if lang.is_empty() {
        format!("Skill_name_{:03}.png", id)
    } else {
        format!("Skill_name_{:03}_{}.png", id, lang)
    };
    root.join(DIR_SKILL_NAME).join(filename)
}