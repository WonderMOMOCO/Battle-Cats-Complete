use std::path::{Path, PathBuf};
use regex::Regex;
use crate::features::stage::patterns;

pub struct StageMatcher {
    map_data: Regex,
    map_name: Regex,
    map_sn: Regex,
    map_global_name: Regex,
    stage_normal: Regex,
    stage_file: Regex,
    stage_name: Regex,
    legacy_stage_name: Regex,
    castle: Regex,
    bg_map: Regex,
    bg_battle: Regex,
    bg_data: Regex,
    bg_effect: Regex,
    limit_msg: Regex,
    ex_files: Regex,
    certification_preset: Regex,
    drop_item: Regex,
    charagroup: Regex,
    score_bonus: Regex,
}

impl StageMatcher {
    pub fn new() -> Self {
        Self {
            map_data: Regex::new(patterns::MAP_STAGE_DATA_PATTERN).unwrap(),
            map_name: Regex::new(patterns::MAP_NAME_PATTERN).unwrap(),
            map_sn: Regex::new(patterns::MAP_SN_PATTERN).unwrap(),
            map_global_name: Regex::new(patterns::MAP_GLOBAL_NAME_PATTERN).unwrap(),
            stage_normal: Regex::new(patterns::STAGE_NORMAL_PATTERN).unwrap(),
            stage_file: Regex::new(patterns::STAGE_FILE_PATTERN).unwrap(),
            stage_name: Regex::new(patterns::STAGE_NAME_PATTERN).unwrap(),
            legacy_stage_name: Regex::new(patterns::LEGACY_STAGE_NAME_PATTERN).unwrap(),
            castle: Regex::new(patterns::CASTLE_PATTERN).unwrap(),
            bg_map: Regex::new(patterns::BG_MAP_PATTERN).unwrap(),
            bg_battle: Regex::new(patterns::BG_BATTLE_PATTERN).unwrap(),
            bg_data: Regex::new(patterns::BG_DATA_PATTERN).unwrap(),
            bg_effect: Regex::new(patterns::BG_EFFECT_PATTERN).unwrap(),
            limit_msg: Regex::new(patterns::LIMIT_MSG_PATTERN).unwrap(),
            ex_files: Regex::new(patterns::EX_PATTERN).unwrap(),
            certification_preset: Regex::new(patterns::CERTIFICATION_PRESET_PATTERN).unwrap(),
            drop_item: Regex::new(patterns::DROP_ITEM_PATTERN).unwrap(),
            charagroup: Regex::new(patterns::CHARAGROUP_PATTERN).unwrap(),
            score_bonus: Regex::new(patterns::SCORE_BONUS_PATTERN).unwrap(),
        }
    }

    fn format_prefix(prefix: &str) -> String {
        let upper = prefix.to_uppercase();
        if upper == "SPACE" { return "Space".to_string(); }
        if upper.starts_with('R') && upper.len() > 1 { return upper[1..].to_string(); }
        upper
    }

    pub fn get_dest(&self, name: &str, stages_dir: &Path) -> Option<PathBuf> {
        if self.map_global_name.is_match(name) { return Some(stages_dir.join("Map_Name")); }

        match name {
            "Map_option.csv" | "MapConditions.json" | "Stage_option.csv" | 
            "DropItem.csv" | "Charagroup.csv" => return Some(stages_dir.to_path_buf()),
            "ScoreBonusMap.json" => return Some(stages_dir.join("R")),
            "SpecialRulesMap.json" | "SpecialRulesMapOption.json" => return Some(stages_dir.join("SR")),
            "bg.csv" => return Some(stages_dir.join("backgrounds").join("battle")),
            "fixed_formation.csv" => return Some(stages_dir.join("fixedlineup")), 
            "stage.csv" => return Some(stages_dir.join("EC").join("000")),
            "tower_layout.csv" => return Some(stages_dir.join("V")), 
            "stage_conditions.csv" => return Some(stages_dir.join("L")),
            "stage_hint_popup.csv" => return Some(stages_dir.join("G")),
            _ => {} 
        }

        // Regex matches for catching localized variants (e.g. DropItem_en.csv)
        if self.drop_item.is_match(name) { return Some(stages_dir.to_path_buf()); }
        if self.charagroup.is_match(name) { return Some(stages_dir.to_path_buf()); }
        if self.score_bonus.is_match(name) { return Some(stages_dir.join("R")); }
        if self.certification_preset.is_match(name) { return Some(stages_dir.join("fixedlineup")); }
        if self.ex_files.is_match(name) { return Some(stages_dir.join("EX")); }
        if self.limit_msg.is_match(name) { return Some(stages_dir.join("MapStageLimitMessage")); }
        
        if let Some(caps) = self.stage_name.captures(name) {
            return Some(stages_dir.join(Self::format_prefix(&caps[1])));
        }

        // --- Legacy Images (Forced to Category/000/StageID) ---
        if let Some(caps) = self.legacy_stage_name.captures(name) {
            let raw_prefix = caps[1].to_lowercase();
            let mut mapped_prefix = match raw_prefix.as_str() {
                "wc" => "W", "sc" => "Space", _ => "EC"
            };

            let Ok(id) = caps[2].parse::<u32>() else { return None; };
            let mut folder_id = id;

            if raw_prefix == "ec" {
                if id == 48 { mapped_prefix = "M"; } // Reroute 48 to Challenge Battle (M) but keep ID 48
                if id >= 49 && id <= 50 { mapped_prefix = "PT"; folder_id = id + 2; }
            }

            return Some(stages_dir.join(mapped_prefix).join("000").join(format!("{:02}", folder_id)));
        }

        // --- Stage Normal (EoC, ItF, CotC, and Zombies) ---
        if let Some(caps) = self.stage_normal.captures(name) {
            let chapter = &caps[1];
            // If sub_chapter doesn't exist (like in EoC), default to "0"
            let sub_chapter = caps.get(2).map(|m| m.as_str()).unwrap_or("0");
            let is_zombie = name.ends_with("_Z.csv");

            // Determine Category
            let category = if is_zombie {
                "Z".to_string()
            } else {
                match chapter {
                    "0" => "EC".to_string(),
                    "1" => "W".to_string(),
                    "2" => "Space".to_string(),
                    _ => format!("Normal_{}", chapter),
                }
            };

            // Map to the internal Map ID
            let map_id = match (chapter, sub_chapter) {
                ("0", _) => "000",   // EoC is Map 0
                ("1", "0") => "004", // ItF Ch 1
                ("1", "1") => "005", // ItF Ch 2
                ("1", "2") => "006", // ItF Ch 3
                ("2", "0") => "007", // CotC Ch 1
                ("2", "1") => "008", // CotC Ch 2
                ("2", "2") => "009", // CotC Ch 3
                _ => "000",          // Fallback
            };

            return Some(stages_dir.join(category).join(map_id));
        }

        // --- Stage Files (Unified to Category/Map/Stage) ---
        if let Some(caps) = self.stage_file.captures(name) {
            let prefix = caps.get(1).map(|m| m.as_str());
            
            let Ok(map_id) = caps[2].parse::<u32>() else { return None; };

            if let Some(p) = prefix {
                let mut path = stages_dir.join(Self::format_prefix(p)).join(format!("{:03}", map_id));
                
                if let Some(s) = caps.get(3) {
                    if let Ok(stage_id) = s.as_str().parse::<u32>() {
                        path = path.join(format!("{:02}", stage_id));
                    }
                }
                return Some(path);
            } else {
                // Legacy Fallback: map_id capture is actually the stage ID here
                let mut p = "EC";
                let folder_id = map_id;
                
                if map_id == 48 { p = "M"; } // Reroute 48 to Challenge Battle (M) but keep ID 48
                if map_id >= 51 && map_id <= 52 { p = "PT"; }
                
                return Some(stages_dir.join(p).join("000").join(format!("{:02}", folder_id)));
            }
        }

        // --- Map & Stage Dynamic Content ---
        if let Some(caps) = self.map_data.captures(name) {
            let Ok(map_id) = caps[2].parse::<u32>() else { return None; };
            return Some(stages_dir.join(Self::format_prefix(&caps[1])).join(format!("{:03}", map_id)));
        }
        
        if let Some(caps) = self.map_name.captures(name) {
            let Ok(map_id) = caps[1].parse::<u32>() else { return None; };
            return Some(stages_dir.join(Self::format_prefix(&caps[2])).join(format!("{:03}", map_id)));
        }
        
        if let Some(caps) = self.map_sn.captures(name) {
            let Ok(map_id) = caps[1].parse::<u32>() else { return None; };
            let Ok(stage_id) = caps[2].parse::<u32>() else { return None; };
            return Some(stages_dir.join(Self::format_prefix(&caps[3])).join(format!("{:03}", map_id)).join(format!("{:02}", stage_id)));
        }

        // --- Assorted Assets ---
        if let Some(caps) = self.castle.captures(name) {
            if name.starts_with("fc000") { return None; } 
            return Some(stages_dir.join("castles").join(&caps[1]));
        }
        if let Some(caps) = self.bg_map.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() { return Some(stages_dir.join("backgrounds").join("maps").join(format!("{:03}", id))); }
        }
        if let Some(caps) = self.bg_battle.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() { return Some(stages_dir.join("backgrounds").join("battle").join(format!("{:03}", id))); }
        }
        if let Some(caps) = self.bg_effect.captures(name) {
            if let Ok(id) = caps[1].parse::<u32>() { return Some(stages_dir.join("backgrounds").join("effects").join(format!("{:03}", id))); }
        }
        if self.bg_data.is_match(name) { return Some(stages_dir.join("backgrounds").join("effects").join("data")); }

        None
    }
}