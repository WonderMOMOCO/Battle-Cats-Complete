use std::path::{Path, PathBuf};
use regex::Regex;
use crate::features::enemy::patterns;

pub struct EnemyMatcher {
    stats: Regex,
    icon: Regex,
    anim_base: Regex,
    maanim: Regex,
    name: Regex,
    pic_book: Regex,
    pic_book_2: Regex,
    pic_book_q: Regex,
    dict_list: Regex,
    autoset_exclude: Regex,
    zombie_effect: Regex,
}

impl EnemyMatcher {
    pub fn new() -> Self {
        Self {
            stats: Regex::new(patterns::ENEMY_STATS).unwrap(),
            icon: Regex::new(patterns::ENEMY_ICON).unwrap(),
            anim_base: Regex::new(patterns::ENEMY_ANIM_BASE).unwrap(),
            maanim: Regex::new(patterns::ENEMY_MAANIM).unwrap(),
            name: Regex::new(patterns::ENEMY_NAME).unwrap(),
            pic_book: Regex::new(patterns::ENEMY_PICTURE_BOOK).unwrap(),
            pic_book_2: Regex::new(patterns::ENEMY_PICTURE_BOOK_2).unwrap(),
            pic_book_q: Regex::new(patterns::ENEMY_PICTURE_BOOK_QUESTION).unwrap(),
            dict_list: Regex::new(patterns::ENEMY_DICT_LIST).unwrap(),
            autoset_exclude: Regex::new(patterns::AUTOSET_EXCLUDE).unwrap(),
            zombie_effect: Regex::new(patterns::ENEMY_ZOMBIE_EFFECT).unwrap(),
        }
    }

    pub fn get_dest(&self, name: &str, enemy_dir: &Path) -> Option<PathBuf> {
        // Root files
        if self.stats.is_match(name) || self.dict_list.is_match(name) || self.autoset_exclude.is_match(name) {
            return Some(enemy_dir.to_path_buf());
        }
        
        // Base Enemy Assets
        if let Some(caps) = self.icon.captures(name) {
            return Some(enemy_dir.join(&caps[1]));
        }
        if let Some(caps) = self.anim_base.captures(name) {
            return Some(enemy_dir.join(&caps[1]).join("anim"));
        }
        if let Some(caps) = self.maanim.captures(name) {
            return Some(enemy_dir.join(&caps[1]).join("anim"));
        }

        // Translation / Name Files
        if self.name.is_match(name) {
            return Some(enemy_dir.join("Enemyname"));
        }
        if self.pic_book.is_match(name) {
            return Some(enemy_dir.join("EnemyPictureBook"));
        }
        if self.pic_book_2.is_match(name) {
            return Some(enemy_dir.join("EnemyPictureBook2"));
        }
        if self.pic_book_q.is_match(name) {
            return Some(enemy_dir.join("EnemyPictureBookQuestion"));
        }

        // Shared Zombie Effects
        if self.zombie_effect.is_match(name) {
            return Some(enemy_dir.join("zombie"));
        }
        
        None
    }
}