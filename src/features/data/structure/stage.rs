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
    difficulty_level: Regex,
    drop_chara: Regex,
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
            difficulty_level: Regex::new(patterns::DIFFICULTY_LEVEL_PATTERN).unwrap(),
            drop_chara: Regex::new(patterns::DROP_CHARA_PATTERN).unwrap(),
        }
    }

    fn format_prefix(prefix: &str) -> String {
        let upper = prefix.to_uppercase();
        if upper == "SPACE" { return "Space".to_string(); }
        if upper.starts_with('R') && upper.len() > 1 { return upper[1..].to_string(); }
        upper
    }

    pub fn get_dest(&self, target_file_name: &str, base_stages_dir: &Path) -> Option<PathBuf> {
        if self.map_global_name.is_match(target_file_name) { 
            return Some(base_stages_dir.join("Map_Name")); 
        }

        match target_file_name {
            "Map_option.csv" | "MapConditions.json" | "Stage_option.csv" | 
            "DropItem.csv" | "Charagroup.csv" | "drop_chara.csv" => return Some(base_stages_dir.to_path_buf()),
            "ScoreBonusMap.json" => return Some(base_stages_dir.join("R")),
            "SpecialRulesMap.json" | "SpecialRulesMapOption.json" => return Some(base_stages_dir.join("SR")),
            "bg.csv" => return Some(base_stages_dir.join("backgrounds").join("battle")),
            "fixed_formation.csv" => return Some(base_stages_dir.join("fixedlineup")), 
            "stage.csv" => return Some(base_stages_dir.join("EC").join("000")),
            "tower_layout.csv" => return Some(base_stages_dir.join("V")), 
            "stage_conditions.csv" => return Some(base_stages_dir.join("L")),
            "stage_hint_popup.csv" => return Some(base_stages_dir.join("G")),
            _ => {} 
        }

        // Regex matches
        if self.drop_item.is_match(target_file_name) { return Some(base_stages_dir.to_path_buf()); }
        if self.charagroup.is_match(target_file_name) { return Some(base_stages_dir.to_path_buf()); }
        if self.difficulty_level.is_match(target_file_name) { return Some(base_stages_dir.to_path_buf()); }
        if self.drop_chara.is_match(target_file_name) { return Some(base_stages_dir.to_path_buf()); }
        
        if self.score_bonus.is_match(target_file_name) { return Some(base_stages_dir.join("R")); }
        if self.certification_preset.is_match(target_file_name) { return Some(base_stages_dir.join("fixedlineup")); }
        if self.ex_files.is_match(target_file_name) { return Some(base_stages_dir.join("EX")); }
        if self.limit_msg.is_match(target_file_name) { return Some(base_stages_dir.join("MapStageLimitMessage")); }
        
        if let Some(regex_captures) = self.stage_name.captures(target_file_name) {
            return Some(base_stages_dir.join(Self::format_prefix(&regex_captures[1])));
        }

        // Legacy Images (Forced to Category/000/StageID)
        if let Some(regex_captures) = self.legacy_stage_name.captures(target_file_name) {
            let raw_prefix_string = regex_captures[1].to_lowercase();
            let mut mapped_prefix_string = match raw_prefix_string.as_str() {
                "wc" => "W", "sc" => "Space", _ => "EC"
            };

            let Ok(parsed_stage_identifier) = regex_captures[2].parse::<u32>() else { return None; };
            let mut target_folder_identifier = parsed_stage_identifier;

            if raw_prefix_string == "ec" {
                if parsed_stage_identifier == 48 { mapped_prefix_string = "M"; }
                if parsed_stage_identifier >= 49 && parsed_stage_identifier <= 50 { 
                    mapped_prefix_string = "PT"; 
                    target_folder_identifier = parsed_stage_identifier + 2; 
                }
            }

            return Some(base_stages_dir.join(mapped_prefix_string).join("000").join(format!("{:02}", target_folder_identifier)));
        }

        // Stage Normal (EoC, ItF, CotC, and Zombies)
        if let Some(regex_captures) = self.stage_normal.captures(target_file_name) {
            let chapter_string = &regex_captures[1];
            let sub_chapter_string = regex_captures.get(2).map(|m| m.as_str()).unwrap_or("0");
            let is_zombie_stage = target_file_name.ends_with("_Z.csv");

            let category_string = if is_zombie_stage {
                "Z".to_string()
            } else {
                match chapter_string {
                    "0" => "EC".to_string(),
                    "1" => "W".to_string(),
                    "2" => "Space".to_string(),
                    _ => format!("Normal_{}", chapter_string),
                }
            };

            let map_identifier_string = match (chapter_string, sub_chapter_string) {
                ("0", _) => "000",   
                ("1", "0") => "004", 
                ("1", "1") => "005", 
                ("1", "2") => "006", 
                ("2", "0") => "007", 
                ("2", "1") => "008", 
                ("2", "2") => "009", 
                _ => "000",          
            };

            return Some(base_stages_dir.join(category_string).join(map_identifier_string));
        }

        // Stage Files (Unified to Category/Map/Stage)
        if let Some(regex_captures) = self.stage_file.captures(target_file_name) {
            let captured_prefix = regex_captures.get(1).map(|m| m.as_str());
            
            let Ok(parsed_map_identifier) = regex_captures[2].parse::<u32>() else { return None; };

            if let Some(valid_prefix) = captured_prefix {
                let mut constructed_path = base_stages_dir.join(Self::format_prefix(valid_prefix)).join(format!("{:03}", parsed_map_identifier));
                
                if let Some(stage_capture) = regex_captures.get(3) {
                    if let Ok(parsed_stage_identifier) = stage_capture.as_str().parse::<u32>() {
                        constructed_path = constructed_path.join(format!("{:02}", parsed_stage_identifier));
                    }
                }
                return Some(constructed_path);
            } else {
                let mut fallback_prefix_string = "EC";
                let target_folder_identifier = parsed_map_identifier;
                
                if parsed_map_identifier == 48 { fallback_prefix_string = "M"; } 
                if parsed_map_identifier >= 51 && parsed_map_identifier <= 52 { fallback_prefix_string = "PT"; }
                
                return Some(base_stages_dir.join(fallback_prefix_string).join("000").join(format!("{:02}", target_folder_identifier)));
            }
        }

        // Map & Stage Dynamic Content
        if let Some(regex_captures) = self.map_data.captures(target_file_name) {
            let Ok(parsed_map_identifier) = regex_captures[2].parse::<u32>() else { return None; };
            return Some(base_stages_dir.join(Self::format_prefix(&regex_captures[1])).join(format!("{:03}", parsed_map_identifier)));
        }
        
        if let Some(regex_captures) = self.map_name.captures(target_file_name) {
            let Ok(parsed_map_identifier) = regex_captures[1].parse::<u32>() else { return None; };
            return Some(base_stages_dir.join(Self::format_prefix(&regex_captures[2])).join(format!("{:03}", parsed_map_identifier)));
        }
        
        if let Some(regex_captures) = self.map_sn.captures(target_file_name) {
            let Ok(parsed_map_identifier) = regex_captures[1].parse::<u32>() else { return None; };
            let Ok(parsed_stage_identifier) = regex_captures[2].parse::<u32>() else { return None; };
            return Some(base_stages_dir.join(Self::format_prefix(&regex_captures[3])).join(format!("{:03}", parsed_map_identifier)).join(format!("{:02}", parsed_stage_identifier)));
        }

        // Assorted Assets
        if let Some(regex_captures) = self.castle.captures(target_file_name) {
            if target_file_name.starts_with("fc000") { return None; } 
            return Some(base_stages_dir.join("castles").join(&regex_captures[1]));
        }
        if let Some(regex_captures) = self.bg_map.captures(target_file_name) {
            if let Ok(parsed_id) = regex_captures[1].parse::<u32>() { 
                return Some(base_stages_dir.join("backgrounds").join("maps").join(format!("{:03}", parsed_id))); 
            }
        }
        if let Some(regex_captures) = self.bg_battle.captures(target_file_name) {
            if let Ok(parsed_id) = regex_captures[1].parse::<u32>() { 
                return Some(base_stages_dir.join("backgrounds").join("battle").join(format!("{:03}", parsed_id))); 
            }
        }
        if let Some(regex_captures) = self.bg_effect.captures(target_file_name) {
            if let Ok(parsed_id) = regex_captures[1].parse::<u32>() { 
                return Some(base_stages_dir.join("backgrounds").join("effects").join(format!("{:03}", parsed_id))); 
            }
        }
        if self.bg_data.is_match(target_file_name) { 
            return Some(base_stages_dir.join("backgrounds").join("effects").join("data")); 
        }

        None
    }
}