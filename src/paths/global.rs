#![allow(dead_code)]
use std::path::{Path, PathBuf};

// Directory Constants
pub const DIR_ASSETS: &str = "game/assets";
pub const DIR_IMG015: &str = "img015";
pub const DIR_GATYA_ITEM: &str = "gatyaitemD";

// im015 Path
pub fn img015_folder(root: &Path) -> PathBuf {
    root.join(DIR_ASSETS).join(DIR_IMG015)
}

// Gatya Items
pub fn gatya_item_icon(root: &Path, id: i32) -> Option<PathBuf> {
    let base = root.join(DIR_ASSETS).join(DIR_GATYA_ITEM);
    
    // Check 3 digits
    let p3 = base.join(format!("gatyaitemD_{:03}_f.png", id));
    if p3.exists() { return Some(p3); }

    // Check 2 digits
    let p2 = base.join(format!("gatyaitemD_{:02}_f.png", id));
    if p2.exists() { return Some(p2); }

    // Check 1 digit
    let p1 = base.join(format!("gatyaitemD_{}_f.png", id));
    if p1.exists() { return Some(p1); }

    None
}