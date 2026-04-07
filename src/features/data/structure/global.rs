use std::path::{Path, PathBuf};
use regex::Regex;
use crate::global::io::patterns;

pub struct GlobalMatcher {
    gatya_item_d: Regex,
    gatya_item_buy: Regex,
    gatya_item_name: Regex,
    img015: Regex,
    img015_cut: Regex,
    img022: Regex,
    img022_cut: Regex,
    localizable: Regex,
    param: Regex,
    audio_ogg: Regex,
    audio_caf: Regex,
}

impl GlobalMatcher {
    pub fn new() -> Self {
        Self {
            gatya_item_d: Regex::new(patterns::GATYA_ITEM_D_PATTERN).unwrap(),
            gatya_item_buy: Regex::new(patterns::GATYA_ITEM_BUY_PATTERN).unwrap(),
            gatya_item_name: Regex::new(patterns::GATYA_ITEM_NAME_PATTERN).unwrap(),
            img015: Regex::new(patterns::ASSET_IMG015_PATTERN).unwrap(),
            img015_cut: Regex::new(patterns::ASSET_015CUT_PATTERN).unwrap(),
            img022: Regex::new(patterns::ASSET_IMG022_PATTERN).unwrap(),
            img022_cut: Regex::new(patterns::ASSET_022CUT_PATTERN).unwrap(),
            localizable: Regex::new(patterns::LOCALIZEABLE_PATTERN).unwrap(),
            param: Regex::new(patterns::PARAM_PATTERN).unwrap(),
            audio_ogg: Regex::new(patterns::AUDIO_OGG_PATTERN).unwrap(),
            audio_caf: Regex::new(patterns::AUDIO_CAF_PATTERN).unwrap(),
        }
    }

    pub fn get_dest(&self, name: &str, sheets_dir: &Path, ui_dir: &Path, tables_dir: &Path, audio_dir: &Path) -> Option<PathBuf> {
        if self.param.is_match(name) {
            return Some(tables_dir.to_path_buf());
        }
        if self.localizable.is_match(name) {
            return Some(tables_dir.join("localizable"));
        }
        if self.gatya_item_d.is_match(name) || self.gatya_item_buy.is_match(name) {
            return Some(ui_dir.join("gatyaitemD"));
        }
        if self.gatya_item_name.is_match(name) {
            return Some(ui_dir.join("gatyaitemD").join("GatyaitemName"));
        }
        if self.img015.is_match(name) || self.img015_cut.is_match(name) {
            return Some(sheets_dir.join("img015"));
        }
        if self.img022.is_match(name) || self.img022_cut.is_match(name) {
            return Some(sheets_dir.join("img022"));
        }
        if self.audio_ogg.is_match(name) {
            return Some(audio_dir.join("ogg"));
        }
        if self.audio_caf.is_match(name) {
            return Some(audio_dir.join("caf"));
        }
        
        None
    }
}