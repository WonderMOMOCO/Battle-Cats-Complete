use std::collections::HashMap;
use std::path::{Path, PathBuf};
use eframe::egui;
use crate::global::formats::gatyaitembuy::GatyaItemBuy;
use crate::global::formats::gatyaitemname::GatyaItemName;
use crate::global::utils::autocrop;
use crate::features::cat::data::unitbuy::UnitBuyRow;
use crate::features::cat::data::unitexplanation::UnitExplanation;

pub fn resolve_item_name(
    target_item_id: u32, 
    item_buy_registry: &HashMap<u32, GatyaItemBuy>, 
    item_name_registry: &HashMap<usize, GatyaItemName>,
    drop_chara_registry: &HashMap<u32, u32>,
    unit_buy_registry: &HashMap<u32, UnitBuyRow>,
    active_language_priority_array: &[String]
) -> String {
    if let Some(located_item_buy_data) = item_buy_registry.get(&target_item_id) {
        let target_name_row_index = located_item_buy_data.row_index;
        if let Some(located_item_name_data) = item_name_registry.get(&target_name_row_index) {
            return located_item_name_data.name.clone();
        }
    }

    if let Some(located_chara_id) = drop_chara_registry.get(&target_item_id) {
        let chara_directory_path = PathBuf::from(format!("game/cats/{:03}/lang", located_chara_id));
        let explanation_file_name = format!("Unit_Explanation{}.csv", located_chara_id + 1);
        
        let resolved_explanation_paths = crate::global::resolver::get(
            &chara_directory_path, 
            &[&explanation_file_name], 
            active_language_priority_array
        );
        
        if let Some(first_explanation_path) = resolved_explanation_paths.first() {
            if let Some(parsed_explanation_data) = UnitExplanation::load(first_explanation_path) {
                let first_form_name_string = &parsed_explanation_data.names[0];
                if !first_form_name_string.is_empty() {
                    return first_form_name_string.clone();
                }
            }
        }
        return format!("Cat {}-1", located_chara_id);
    }

    if let Some((&unit_id, _)) = unit_buy_registry.iter().find(|(_, row_data)| row_data.true_form_id == target_item_id as i32) {
        let chara_directory_path = PathBuf::from(format!("game/cats/{:03}/lang", unit_id));
        let explanation_file_name = format!("Unit_Explanation{}.csv", unit_id + 1);
        
        let resolved_explanation_paths = crate::global::resolver::get(
            &chara_directory_path, 
            &[&explanation_file_name], 
            active_language_priority_array
        );
        
        if let Some(first_explanation_path) = resolved_explanation_paths.first() {
            if let Some(parsed_explanation_data) = UnitExplanation::load(first_explanation_path) {
                let true_form_name_string = &parsed_explanation_data.names[2];
                if !true_form_name_string.is_empty() {
                    return true_form_name_string.clone();
                }
            }
        }
        return format!("Cat {}-3", unit_id);
    }

    target_item_id.to_string()
}

pub fn format_drop_chance(raw_chance: u32, drop_rule: i32) -> String {
    if drop_rule == -3 || drop_rule == -4 {
        return "100%".to_string();
    }
    format!("{}%", raw_chance)
}

pub fn resolve_item_image_path(
    target_item_id: u32,
    item_buy_registry: &HashMap<u32, GatyaItemBuy>,
    drop_chara_registry: &HashMap<u32, u32>,
    unit_buy_registry: &HashMap<u32, UnitBuyRow>,
    active_language_priority_array: &[String]
) -> Option<PathBuf> {
    if let Some(located_item_buy_data) = item_buy_registry.get(&target_item_id) {
        let resolved_image_identifier = if located_item_buy_data.img_id != -1 {
            located_item_buy_data.img_id as u32
        } else {
            located_item_buy_data.row_index as u32
        };

        let gatya_directory_path = PathBuf::from("game/ui/gatyaitemD");
        let gatya_file_name = format!("gatyaitemD_{:02}_f.png", resolved_image_identifier);
        
        let resolved_gatya_paths = crate::global::resolver::get(&gatya_directory_path, &[&gatya_file_name], active_language_priority_array);
        
        if let Some(first_gatya_path) = resolved_gatya_paths.first() {
            return Some(first_gatya_path.clone());
        }
        return None;
    }

    if let Some(located_chara_id) = drop_chara_registry.get(&target_item_id) {
        let chara_directory_path = PathBuf::from(format!("game/cats/{:03}/f", located_chara_id));
        let chara_file_name = format!("uni{:03}_f00.png", located_chara_id);
        
        let resolved_chara_paths = crate::global::resolver::get(&chara_directory_path, &[&chara_file_name], active_language_priority_array);
        
        if let Some(first_chara_path) = resolved_chara_paths.first() {
            return Some(first_chara_path.clone());
        }
    }

    if let Some((&unit_id, _)) = unit_buy_registry.iter().find(|(_, row_data)| row_data.true_form_id == target_item_id as i32) {
        let chara_directory_path = PathBuf::from(format!("game/cats/{:03}/s", unit_id));
        let chara_file_name = format!("uni{:03}_s00.png", unit_id);
        
        let resolved_chara_paths = crate::global::resolver::get(&chara_directory_path, &[&chara_file_name], active_language_priority_array);
        
        if let Some(first_chara_path) = resolved_chara_paths.first() {
            return Some(first_chara_path.clone());
        }
    }

    None
}

pub fn process_item_icon_texture(icon_file_path: &Path) -> Option<egui::ColorImage> {
    let Ok(loaded_raw_image_data) = image::open(icon_file_path) else {
        return None;
    };
    
    let autocropped_rgba_image = autocrop(loaded_raw_image_data.to_rgba8());
    let (crop_width, crop_height) = autocropped_rgba_image.dimensions();
    let max_dimension = crop_width.max(crop_height) as f32;
    let scale_factor = 32.0 / max_dimension;
    
    let target_width = (crop_width as f32 * scale_factor).round() as u32;
    let target_height = (crop_height as f32 * scale_factor).round() as u32;
    
    let resized_rgba_image = image::imageops::resize(
        &autocropped_rgba_image, 
        target_width.max(1), 
        target_height.max(1), 
        image::imageops::FilterType::Triangle
    );
    
    let image_dimensions = [resized_rgba_image.width() as usize, resized_rgba_image.height() as usize];
    
    Some(egui::ColorImage::from_rgba_unmultiplied(image_dimensions, resized_rgba_image.as_flat_samples().as_slice()))
}

pub fn format_treasure_rule(drop_rule: i32) -> &'static str {
    match drop_rule {
        1 => "Once, Then Unlimited",
        0 => "Unlimited",
        -1 => "Raw Percentages (Unlimited)",
        -3 => "Guaranteed (Once)",
        -4 => "Guaranteed (Unlimited)",
        _ => "Unknown Rule",
    }
}