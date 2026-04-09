use std::collections::HashMap;
use crate::features::stage::data;

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Stage {
    pub id: String,
    pub name: String,
    pub category: String,
    pub category_name: String,
    pub map_id: u32,
    pub stage_id: u32,
    
    // Base layout
    pub base_id: i32,
    pub anim_base_id: u32,
    pub width: u32,
    pub base_hp: u32,
    pub background_id: u32,
    pub max_enemies: u32,
    pub is_no_continues: bool,
    pub is_base_indestructible: bool,
    pub enemies: Vec<data::stage::EnemyLine>,
    
    // Core Rewards & Media
    pub energy: u32,
    pub xp: u32,
    pub init_track: u32,
    pub bgm_change_percent: u32,
    pub boss_track: u32,
    pub rewards: data::mapstagedata::RewardStructure,

    // Stage Options & Restrictions
    pub target_crowns: i8, 
    pub rarity_mask: u8,
    pub deploy_limit: u32,
    pub allowed_rows: u8,
    pub min_cost: u32,
    pub max_cost: u32,
    pub charagroup: Option<data::charagroup::CharaGroup>,
}

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Map {
    pub id: String,
    pub name: String,
    pub category: String,
    pub category_name: String,
    pub map_id: u32,
    pub stages: Vec<String>,

    // Map Options
    pub max_crowns: u8,
    pub crown_2_mag: Option<u32>,
    pub crown_3_mag: Option<u32>,
    pub crown_4_mag: Option<u32>,
    pub reset_type: data::map_option::ResetType,
    pub max_clears: u32,
    pub cooldown_minutes: u32,
    pub hidden_upon_clear: bool,

    // Extraneous Map Configs
    pub ex_invasion: Option<u32>,
    pub score_bonuses: Option<data::scorebonusmap::ScoreBonus>,
    pub special_rules: Option<data::specialrulesmap::SpecialRule>,
    pub drop_items: Option<data::dropitem::DropItem>,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct StageRegistry {
    pub maps: HashMap<String, Map>,
    pub stages: HashMap<String, Stage>,
}

impl StageRegistry {
    pub fn clear_cache(&mut self) {
        self.maps.clear();
        self.stages.clear();
    }
}