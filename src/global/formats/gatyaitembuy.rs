#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Debug, Clone)]
pub struct GatyaItemBuy {
    pub rarity: i32,
    pub reflect_or_storage: i32,
    pub price: i32,
    pub stage_drop_item_id: u32,
    pub quantity: i32,
    pub server_id: i32,
    pub category: i32,
    pub index: i32,
    pub src_item_id: i32,
    pub main_menu_type: i32,
    pub gatya_ticket_id: i32,
    pub img_id: i32,
    pub comment: String,
    pub row_index: usize,
}

pub fn load(dir_path: &Path, filename: &str, lang_priority: &[String]) -> HashMap<u32, GatyaItemBuy> {
    let mut item_buy_map = HashMap::new();
    let file_paths = resolver::get(dir_path, &[filename], lang_priority);
    
    let Some(first_path) = file_paths.first() else { 
        return item_buy_map; 
    };
    
    let Ok(file_content) = fs::read_to_string(first_path) else { 
        return item_buy_map; 
    };

    let csv_separator = detect_csv_separator(&file_content);

    for (calculated_row_index, line_string) in file_content.lines().skip(1).enumerate() {
        let clean_line = line_string.split("//").next().unwrap_or("").trim();
        if clean_line.is_empty() { 
            continue; 
        }

        let line_parts: Vec<&str> = clean_line.split(csv_separator).collect();
        if line_parts.len() < 12 { 
            continue; 
        }

        let Ok(stage_drop_item_id) = line_parts[3].trim().parse::<u32>() else {
            continue;
        };

        let item_buy_data = GatyaItemBuy {
            rarity: line_parts[0].trim().parse().unwrap_or(0),
            reflect_or_storage: line_parts[1].trim().parse().unwrap_or(0),
            price: line_parts[2].trim().parse().unwrap_or(0),
            stage_drop_item_id,
            quantity: line_parts[4].trim().parse().unwrap_or(0),
            server_id: line_parts[5].trim().parse().unwrap_or(0),
            category: line_parts[6].trim().parse().unwrap_or(0),
            index: line_parts[7].trim().parse().unwrap_or(0),
            src_item_id: line_parts[8].trim().parse().unwrap_or(0),
            main_menu_type: line_parts[9].trim().parse().unwrap_or(0),
            gatya_ticket_id: line_parts[10].trim().parse().unwrap_or(0),
            img_id: line_parts[11].trim().parse().unwrap_or(-1),
            comment: line_parts.get(12).unwrap_or(&"").trim().to_string(),
            row_index: calculated_row_index,
        };

        item_buy_map.insert(stage_drop_item_id, item_buy_data);
    }

    item_buy_map
}