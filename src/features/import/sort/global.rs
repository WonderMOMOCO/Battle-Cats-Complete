use std::path::{Path, PathBuf};
use regex::Regex;
use crate::global::io::patterns;

pub struct GlobalMatcher {
    skill_name: Regex,
    gatya_item_d: Regex,
    gatya_item_buy: Regex,
    gatya_item_name: Regex,
    img015: Regex,
    img015_cut: Regex,
    img022: Regex,
    img022_cut: Regex,
}

impl GlobalMatcher {
    pub fn new() -> Self {
        Self {
            skill_name: Regex::new(patterns::SKILL_NAME_PATTERN).unwrap(),
            gatya_item_d: Regex::new(patterns::GATYA_ITEM_D_PATTERN).unwrap(),
            gatya_item_buy: Regex::new(patterns::GATYA_ITEM_BUY_PATTERN).unwrap(),
            gatya_item_name: Regex::new(patterns::GATYA_ITEM_NAME_PATTERN).unwrap(),
            img015: Regex::new(patterns::ASSET_IMG015_PATTERN).unwrap(),
            img015_cut: Regex::new(patterns::ASSET_015CUT_PATTERN).unwrap(),
            img022: Regex::new(patterns::ASSET_IMG022_PATTERN).unwrap(),
            img022_cut: Regex::new(patterns::ASSET_022CUT_PATTERN).unwrap(),
        }
    }

    pub fn get_dest(&self, name: &str, assets_dir: &Path) -> Option<PathBuf> {
        if self.skill_name.is_match(name) {
            return Some(assets_dir.join("Skill_name"));
        }
        if self.gatya_item_d.is_match(name) || self.gatya_item_buy.is_match(name) {
            return Some(assets_dir.join("gatyaitemD"));
        }
        if self.gatya_item_name.is_match(name) {
            return Some(assets_dir.join("gatyaitemD").join("GatyaitemName"));
        }
        if self.img015.is_match(name) || self.img015_cut.is_match(name) {
            return Some(assets_dir.join("img015"));
        }
        if self.img022.is_match(name) || self.img022_cut.is_match(name) {
            return Some(assets_dir.join("img022"));
        }
        
        None
    }
}