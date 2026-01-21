#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::core::utils; // Import utils

#[derive(Debug, Clone, Default)]
pub struct UnitBuyRow {
    // Basic Info
    pub stage_unlock_requirement: i32,
    pub purchase_cost: i32,
    // Indices 2-11 stored in upgrade_costs
    pub currency_type: i32,
    pub rarity: i32,
    pub guide_order: i32,
    pub chapter_unlock_requirement: i32,
    pub sell_xp_yield: i32,
    pub unknown_17: i32,
    
    // Level Caps (Legacy)
    pub level_cap_ch2: i32,
    pub base_max_plus_level: i32,
    
    // Evolution
    pub evolve_level_xp: i32,
    pub unknown_21: i32,
    pub level_cap_ch1: i32,
    pub true_form_id: i32,
    pub ultra_form_id: i32,
    
    // Requirements
    pub true_form_unlock_level: i32,
    pub ultra_form_unlock_level: i32,
    pub true_form_xp_cost: i32,
    // Indices 28-37 stored in true_form_materials
    pub ultra_form_xp_cost: i32,
    // Indices 39-48 stored in ultra_form_materials
    
    // Limits & Meta
    pub unknown_49: i32,
    pub level_cap_standard: i32,
    pub level_cap_plus: i32,
    pub unknown_52: i32,
    pub unknown_53: i32,
    pub unknown_54: i32,
    pub unknown_55: i32,
    pub unknown_56: i32,
    pub version_added: i64,
    pub sell_np_yield: i32,
    pub unknown_59: i32,
    pub unknown_60: i32,
    
    // Eggs
    pub egg_id_normal: i32,
    pub egg_id_evolved: i32,

    // Update Fallback
    pub rest: Vec<i32>,

    // Compressed Vectors
    pub upgrade_costs: Vec<i32>,                // Indices 2-11
    pub true_form_materials: Vec<(i32, i32)>,   // Indices 28-37
    pub ultra_form_materials: Vec<(i32, i32)>,  // Indices 39-48
}

impl UnitBuyRow {
    pub fn from_csv_line(csv_line: &str, delimiter: char) -> Option<Self> {
        let parts: Vec<&str> = csv_line.split(delimiter).map(|s| s.trim()).collect();
        
        let get = |idx: usize| -> i32 {
            parts.get(idx).and_then(|s| s.parse::<i32>().ok()).unwrap_or(-1)
        };
        
        let get_i64 = |idx: usize| -> i64 {
            parts.get(idx).and_then(|s| s.parse::<i64>().ok()).unwrap_or(-1)
        };

        // Parse pair helper
        let parse_materials = |start_index: usize| -> Vec<(i32, i32)> {
            let mut mats = Vec::new();
            for i in 0..5 {
                let base = start_index + (i * 2);
                let item_id = get(base);
                let cost = get(base + 1);
                if item_id != -1 && cost > 0 {
                    mats.push((item_id, cost));
                }
            }
            mats
        };

        let parse_upgrades = |start_index: usize| -> Vec<i32> {
            (0..10).map(|i| get(start_index + i)).collect()
        };

        let mut rest_vec = Vec::new();
        if parts.len() > 63 {
            for i in 63..parts.len() {
                if let Ok(val) = parts[i].parse::<i32>() {
                    rest_vec.push(val);
                }
            }
        }

        Some(Self {
            stage_unlock_requirement: get(0),
            purchase_cost: get(1),
            upgrade_costs: parse_upgrades(2), 
            
            currency_type: get(12),
            rarity: get(13),
            guide_order: get(14),
            chapter_unlock_requirement: get(15),
            sell_xp_yield: get(16),
            unknown_17: get(17),
            
            level_cap_ch2: get(18),
            base_max_plus_level: get(19),
            
            evolve_level_xp: get(20),
            unknown_21: get(21),
            level_cap_ch1: get(22),
            true_form_id: get(23),
            ultra_form_id: get(24),
            
            true_form_unlock_level: get(25),
            ultra_form_unlock_level: get(26),
            true_form_xp_cost: get(27),
            true_form_materials: parse_materials(28),
            
            ultra_form_xp_cost: get(38),
            ultra_form_materials: parse_materials(39),
            
            unknown_49: get(49),
            level_cap_standard: get(50),
            level_cap_plus: get(51),
            unknown_52: get(52),
            unknown_53: get(53),
            unknown_54: get(54),
            unknown_55: get(55),
            unknown_56: get(56),
            version_added: get_i64(57),
            sell_np_yield: get(58),
            unknown_59: get(59),
            unknown_60: get(60),
            
            egg_id_normal: get(61),
            egg_id_evolved: get(62),
            
            rest: rest_vec,
        })
    }
}

pub fn load_unitbuy(cats_directory: &Path) -> HashMap<u32, UnitBuyRow> {
    let mut unit_buy_map = HashMap::new();
    let file_path = cats_directory.join("unitbuy.csv");
    
    if let Ok(file_content) = fs::read_to_string(&file_path) {
        let delimiter = utils::detect_csv_separator(&file_content);

        for (line_index, csv_line) in file_content.lines().enumerate() {
            if csv_line.trim().is_empty() { continue; }
            
            if let Some(row_data) = UnitBuyRow::from_csv_line(csv_line, delimiter) {
                unit_buy_map.insert(line_index as u32, row_data);
            }
        }
    } 
    unit_buy_map
}