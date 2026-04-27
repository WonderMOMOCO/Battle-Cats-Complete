#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::utils; 
use crate::features::cat::paths;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UnitBuyRow {
    pub stage_unlock_requirement: i32,
    pub purchase_cost: i32,
    pub currency_type: i32,
    pub rarity: i32,
    pub guide_order: i32,
    pub chapter_unlock_requirement: i32,
    pub sell_xp_yield: i32,
    pub unknown_17: i32,
    pub level_cap_ch2: i32,
    pub base_max_plus_level: i32,
    pub evolve_level_xp: i32,
    pub unknown_21: i32,
    pub level_cap_ch1: i32,
    pub true_form_id: i32,
    pub ultra_form_id: i32,
    pub true_form_unlock_level: i32,
    pub ultra_form_unlock_level: i32,
    pub true_form_xp_cost: i32,
    pub ultra_form_xp_cost: i32,
    pub level_cap_standard: i32,
    pub level_cap_catseye: i32,
    pub level_cap_plus: i32,
    pub normal_evolution_y_offset: i32,
    pub evolved_evolution_y_offset: i32,
    pub true_evolution_y_offset: i32,
    pub ultra_evolution_y_offset: i32,
    pub unknown_56: i32,
    pub version_added: i64,
    pub sell_np_yield: i32,
    pub unknown_59: i32,
    pub unknown_60: i32,
    pub egg_id_normal: i32,
    pub egg_id_evolved: i32,
    pub rest: Vec<i32>,
    pub upgrade_costs: Vec<i32>,
    pub true_form_materials: Vec<(i32, i32)>, 
    pub ultra_form_materials: Vec<(i32, i32)>, 
}

impl UnitBuyRow {
    pub fn from_csv_line(csv_line: &str, delimiter: char) -> Option<Self> {
        let parts: Vec<&str> = csv_line.split(delimiter).map(|string_part| string_part.trim()).collect();
        
        let get_integer = |column_index: usize| -> i32 { 
            parts.get(column_index).and_then(|string_part| string_part.parse::<i32>().ok()).unwrap_or(-1) 
        };
        
        let get_long = |column_index: usize| -> i64 { 
            parts.get(column_index).and_then(|string_part| string_part.parse::<i64>().ok()).unwrap_or(-1) 
        };
        
        let parse_materials = |start_index: usize| -> Vec<(i32, i32)> {
            let mut material_list = Vec::new();
            for index in 0..5 {
                let base_index = start_index + (index * 2);
                let item_id = get_integer(base_index);
                let item_cost = get_integer(base_index + 1);
                if item_id != -1 && item_cost > 0 { 
                    material_list.push((item_id, item_cost)); 
                }
            }
            material_list
        };
        
        let parse_upgrades = |start_index: usize| -> Vec<i32> { 
            (0..10).map(|index| get_integer(start_index + index)).collect() 
        };
        
        let mut rest_vector = Vec::new();
        if parts.len() > 63 {
            for index in 63..parts.len() { 
                let Ok(parsed_value) = parts[index].parse::<i32>() else { continue; };
                rest_vector.push(parsed_value); 
            }
        }
        
        Some(Self {
            stage_unlock_requirement: get_integer(0),
            purchase_cost: get_integer(1),
            upgrade_costs: parse_upgrades(2), 
            currency_type: get_integer(12),
            rarity: get_integer(13),
            guide_order: get_integer(14),
            chapter_unlock_requirement: get_integer(15),
            sell_xp_yield: get_integer(16),
            unknown_17: get_integer(17),
            level_cap_ch2: get_integer(18),
            base_max_plus_level: get_integer(19),
            evolve_level_xp: get_integer(20),
            unknown_21: get_integer(21),
            level_cap_ch1: get_integer(22),
            true_form_id: get_integer(23),
            ultra_form_id: get_integer(24),
            true_form_unlock_level: get_integer(25),
            ultra_form_unlock_level: get_integer(26),
            true_form_xp_cost: get_integer(27),
            true_form_materials: parse_materials(28),
            ultra_form_xp_cost: get_integer(38),
            ultra_form_materials: parse_materials(39),
            level_cap_standard: get_integer(49),
            level_cap_catseye: get_integer(50),
            level_cap_plus: get_integer(51),
            normal_evolution_y_offset: get_integer(52),
            evolved_evolution_y_offset: get_integer(53),
            true_evolution_y_offset: get_integer(54),
            ultra_evolution_y_offset: get_integer(55),
            unknown_56: get_integer(56),
            version_added: get_long(57),
            sell_np_yield: get_integer(58),
            unknown_59: get_integer(59),
            unknown_60: get_integer(60),
            egg_id_normal: get_integer(61),
            egg_id_evolved: get_integer(62),
            rest: rest_vector,
        })
    }
}

pub fn load_unitbuy(cats_directory: &Path, priority: &[String]) -> HashMap<u32, UnitBuyRow> {
    let mut unit_buy_map = HashMap::new();
    let Some(file_path) = crate::global::resolver::get(cats_directory, &[paths::UNIT_BUY], priority).into_iter().next() else {
        return unit_buy_map;
    };
    
    let Ok(file_content) = fs::read_to_string(&file_path) else {
        return unit_buy_map;
    };
    
    let delimiter = utils::detect_csv_separator(&file_content);
    for (line_index, csv_line) in file_content.lines().enumerate() {
        if csv_line.trim().is_empty() { continue; }
        
        let Some(row_data) = UnitBuyRow::from_csv_line(csv_line, delimiter) else { continue; };
        unit_buy_map.insert(line_index as u32, row_data);
    }
    
    unit_buy_map
}