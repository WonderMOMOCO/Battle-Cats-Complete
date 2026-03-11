#![allow(dead_code)]
use std::path::{Path, PathBuf};

// Directory Constants
pub const DIR_ENEMIES: &str = "game/enemies";

#[derive(Copy, Clone, PartialEq)]
pub enum AnimType {
    Maanim,  // Animation Data
    Mamodel, // Model Data
    Imgcut,  // Sprite Cuts
    Png,     // Sprite Sheet
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

// Retrieves the path to the master t_unit.csv file
pub fn stats(root: &Path) -> PathBuf {
    root.join("t_unit.csv")
}

// Retrieves the path to the enemy's portrait icon
pub fn icon(root: &Path, id: u32) -> PathBuf {
    root.join(format!("{:03}", id))
        .join(format!("enemy_icon_{:03}.png", id))
}

// Helper to get the animation directory
pub fn anim_folder(root: &Path, id: u32) -> PathBuf {
    root.join(format!("{:03}", id)).join("anim")
}

// Helper to get the base prefix for animation files
pub fn anim_base_filename(id: u32) -> String {
    format!("{:03}_e", id)
}

// Retrieves paths for standard animation files (Png, Imgcut, Mamodel)
pub fn anim(root: &Path, id: u32, file_type: AnimType) -> PathBuf {
    let folder = anim_folder(root, id);
    let filename = anim_base_filename(id);
    let ext = file_type.ext();
    folder.join(format!("{}.{}", filename, ext))
}

// Retrieves paths specifically for standard Maanim files (00, 01, 02, 03)
pub fn maanim(root: &Path, id: u32, index: usize) -> PathBuf {
    let folder = anim_folder(root, id);
    let filename = anim_base_filename(id);
    folder.join(format!("{}{:02}.maanim", filename, index))
}

// Retrieves paths specifically for zombie Maanim files (00, 01, 02)
pub fn zombie_maanim(root: &Path, id: u32, index: usize) -> PathBuf {
    let folder = anim_folder(root, id);
    let filename = anim_base_filename(id);
    folder.join(format!("{}_zombie{:02}.maanim", filename, index))
}

// Retrieves enemy name path depending on language
pub fn enemy_name(root: &Path, lang: &str) -> PathBuf {
    if lang.is_empty() || lang == "--" { 
        root.join("Enemyname.tsv") 
    } else { 
        root.join(format!("Enemyname_{}.tsv", lang)) 
    }
}

// Retrieves enemy description path depending on language
pub fn enemy_picture_book(root: &Path, lang: &str) -> PathBuf {
    if lang.is_empty() || lang == "--" { 
        root.join("EnemyPictureBook.csv") 
    } else { 
        root.join(format!("EnemyPictureBook_{}.csv", lang)) 
    }
}