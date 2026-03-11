use std::path::{Path, PathBuf};
use regex::Regex;
use crate::features::cat::patterns;

pub struct CatMatcher {
    universal: Regex,
    skill_desc: Regex,
    stats: Regex,
    icon: Regex,
    upgrade: Regex,
    gacha: Regex,
    anim: Regex,
    maanim: Regex,
    explain: Regex,
    egg_icon: Regex,
    egg_upgrade: Regex,
    egg_gacha: Regex,
    egg_anim: Regex,
    egg_maanim: Regex,
}

impl CatMatcher {
    pub fn new() -> Self {
        Self {
            universal: Regex::new(patterns::CAT_UNIVERSAL_PATTERN).unwrap(),
            skill_desc: Regex::new(patterns::SKILL_DESC_PATTERN).unwrap(),
            stats: Regex::new(patterns::CAT_STATS_PATTERN).unwrap(),
            icon: Regex::new(patterns::CAT_ICON_PATTERN).unwrap(),
            upgrade: Regex::new(patterns::CAT_UPGRADE_PATTERN).unwrap(),
            gacha: Regex::new(patterns::CAT_GACHA_PATTERN).unwrap(),
            anim: Regex::new(patterns::CAT_ANIM_PATTERN).unwrap(),
            maanim: Regex::new(patterns::CAT_MAANIM_PATTERN).unwrap(),
            explain: Regex::new(patterns::CAT_EXPLAIN_PATTERN).unwrap(),
            egg_icon: Regex::new(patterns::EGG_ICON_PATTERN).unwrap(),
            egg_upgrade: Regex::new(patterns::EGG_UPGRADE_PATTERN).unwrap(),
            egg_gacha: Regex::new(patterns::EGG_GACHA_PATTERN).unwrap(),
            egg_anim: Regex::new(patterns::EGG_ANIM_PATTERN).unwrap(),
            egg_maanim: Regex::new(patterns::EGG_MAANIM_PATTERN).unwrap(),
        }
    }

    fn map_egg(code: &str) -> &str {
        match code { "00" => "f", _ => "c" }
    }

    pub fn get_dest(&self, name: &str, cats_dir: &Path) -> Option<PathBuf> {
        if self.skill_desc.is_match(name) {
            return Some(cats_dir.join("SkillDescriptions"));
        }
        if self.universal.is_match(name) {
            return Some(cats_dir.join("unitevolve"));
        }
        
        if let Some(caps) = self.stats.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() {
                if id > 0 { return Some(cats_dir.join(format!("{:03}", id - 1))); }
            }
        }
        if let Some(caps) = self.icon.captures(name) { return Some(cats_dir.join(&caps[1]).join(&caps[2])); }
        if let Some(caps) = self.upgrade.captures(name) { return Some(cats_dir.join(&caps[1]).join(&caps[2])); }
        if let Some(caps) = self.gacha.captures(name) { return Some(cats_dir.join(&caps[1])); }
        if let Some(caps) = self.anim.captures(name) { return Some(cats_dir.join(&caps[1]).join(&caps[2]).join("anim")); }
        if let Some(caps) = self.maanim.captures(name) { return Some(cats_dir.join(&caps[1]).join(&caps[2]).join("anim")); }
        if let Some(caps) = self.explain.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() {
                if id > 0 { return Some(cats_dir.join(format!("{:03}", id - 1)).join("lang")); }
            }
        }
        if let Some(caps) = self.egg_icon.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])).join(Self::map_egg(&caps[2])));
        }
        if let Some(caps) = self.egg_upgrade.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])).join(Self::map_egg(&caps[2])));
        }
        if let Some(caps) = self.egg_gacha.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])));
        }
        if let Some(caps) = self.egg_anim.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])).join("anim"));
        }
        if let Some(caps) = self.egg_maanim.captures(name) {
            return Some(cats_dir.join(format!("egg_{}", &caps[1])).join("anim"));
        }

        None
    }
}