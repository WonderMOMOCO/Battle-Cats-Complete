use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use image::{RgbaImage, Rgba};
use ab_glyph::{Font, FontRef, PxScale}; 
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use arboard::{Clipboard, ImageData};
use eframe::egui;

use crate::global::utils::autocrop;
use crate::global::formats::imgcut::SpriteCut;
use crate::global::game::abilities::{CustomIcon, AbilityItem, ABILITY_X, ABILITY_Y, TRAIT_Y};
use crate::global::assets;
use super::draw::*;

#[derive(Clone)]
pub struct SpiritData {
    pub dmg_text: String,
    pub traits: Vec<AbilityItem>,
    pub h1: Vec<AbilityItem>,
    pub h2: Vec<AbilityItem>,
    pub b1: Vec<AbilityItem>,
    pub b2: Vec<AbilityItem>,
    pub footer: Vec<AbilityItem>,
}

#[derive(Clone)]
pub struct StatblockData {
    pub is_cat: bool,
    pub id_str: String,
    pub name: String,
    pub icon_path: Option<PathBuf>,
    pub top_label: String,
    pub top_value: String,
    pub hp: String,
    pub kb: String,
    pub speed: String,
    pub cd_label: String,
    pub cd_value: String,
    pub is_cd_time: bool,
    pub cd_frames: i32,
    pub cost_label: String,
    pub cost_value: String,
    pub atk: String,
    pub dps: String,
    pub range: String,
    pub atk_cycle: i32,
    pub atk_type: String,
    pub traits: Vec<AbilityItem>,
    pub h1: Vec<AbilityItem>,
    pub h2: Vec<AbilityItem>,
    pub b1: Vec<AbilityItem>,
    pub b2: Vec<AbilityItem>,
    pub footer: Vec<AbilityItem>,
    pub spirit_data: Option<SpiritData>,
}

const NAME_BOX_WIDTH: f32 = 126.0;
const NAME_BOX_HEIGHT: f32 = 50.0;
const HEADER_PADDING_Y: i32 = 10;
const STAT_GRID_PADDING_Y: i32 = 14;

const HEADER_CONTENT_SCALE: f32 = 1.10; 
const HEADER_TEXT_Y_SHIFT: i32 = -10;

const NAME_BASE_FONT_SIZE: f32 = 26.0; 
const NAME_Y_OFFSET: i32 = -8; 
const NAME_LINE_SPACING: i32 = -5;

const STAT_GRID_TEXT_SCALE: f32 = 1.1; 

const ABILITY_FONT_SIZE: f32 = 18.0;
const ABILITY_LINE_SPACING: i32 = -2; 
const ABILITY_TEXT_Y_OFFSET: i32 = -1; 

const CANVAS_BORDER_THICKNESS: i32 = 5; 
const CANVAS_BORDER_RADIUS: i32 = 8; 
const CANVAS_BORDER_INNER_RADIUS: i32 = 8; 
const CANVAS_BORDER_PADDING: i32 = 4; 
const CANVAS_BORDER_COLOR: Rgba<u8> = Rgba([31, 106, 165, 255]); 

const SPIRIT_PADDING_X: f32 = 8.0;

fn build_statblock_image(
    priority: &[String],
    data: StatblockData,
    cuts_map: HashMap<usize, SpriteCut>,
) -> RgbaImage {
    let scale: i32 = 2;
    let scale_f: f32 = 2.0;
    let padding = 8 * scale;
    let col_w = 66 * scale; 
    let gap = 4 * scale;
    let export_icon_size = 40 * scale;
    let icon_gap_x = (ABILITY_X * scale_f).round() as i32;
    let icon_gap_y = (ABILITY_Y * scale_f).round() as i32;
    let trait_gap_y = (TRAIT_Y * scale_f).round() as i32;
    let list_text_y_offset = ABILITY_TEXT_Y_OFFSET * scale;
    let list_text_gap_x = 8 * scale;
    
    let base_grid_width: f32 = (8.0 * 2.0) + (66.0 * 5.0) + (4.0 * 4.0); 

    let jp_font = FontRef::try_from_slice(include_bytes!("../../../assets/NotoSansJP-Regular.ttf")).expect("Failed to load JP font");
    let kr_font = FontRef::try_from_slice(include_bytes!("../../../assets/NotoSansKR-Regular.ttf")).expect("Failed to load KR font");
    let tc_font = FontRef::try_from_slice(include_bytes!("../../../assets/NotoSansTC-Regular.ttf")).expect("Failed to load TC font");
    let th_font = FontRef::try_from_slice(include_bytes!("../../../assets/NotoSansThai-Regular.ttf")).expect("Failed to load TH font");

    let get_font = |lang: &str| -> &FontRef {
        match lang {
            "ko" | "kr" => &kr_font,
            "tw" | "zh" | "zh-tw" => &tc_font,
            "th" => &th_font,
            _ => &jp_font, 
        }
    };

    let font_supports_string = |candidate_font: &FontRef, text: &str| -> bool {
        for character in text.chars() {
            if character.is_ascii() || character.is_whitespace() { continue; }
            if candidate_font.glyph_id(character).0 == 0 {
                return false; 
            }
        }
        true
    };

    let mut selected_font = &jp_font;
    let mut found_match = false;

    for lang in priority {
        let candidate_font = get_font(lang);
        if font_supports_string(candidate_font, &data.name) {
            selected_font = candidate_font;
            found_match = true;
            break;
        }
    }

    if !found_match {
        selected_font = get_font(priority.first().map(|s| s.as_str()).unwrap_or("en"));
    }

    let font = selected_font;

    let check_icon_row_width = |items: &Vec<AbilityItem>| -> f32 {
        if items.is_empty() { return 0.0; }
        8.0 + (items.len() as f32 * (40.0 + ABILITY_X)) - ABILITY_X + 8.0 
    };

    let calc_spirit_width = |spirit: &SpiritData| -> f32 {
        let mut spirit_max = 0.0_f32;
        let start_x = 8.0;
        
        for line in spirit.dmg_text.split('\n') {
            let text_width = measure_text_with_superscript(PxScale::from(ABILITY_FONT_SIZE), font, line);
            spirit_max = spirit_max.max(start_x + 40.0 + 8.0 + text_width as f32);
        }
        
        for spirit_item in spirit.b1.iter().chain(spirit.b2.iter()) {
            for line in spirit_item.text.split('\n') {
                let text_width = measure_text_with_superscript(PxScale::from(ABILITY_FONT_SIZE), font, line);
                spirit_max = spirit_max.max(start_x + 40.0 + 8.0 + text_width as f32);
            }
        }
        
        for spirit_items in [&spirit.traits, &spirit.h1, &spirit.h2, &spirit.footer] {
            if spirit_items.is_empty() { continue; }
            let icon_width = start_x + (spirit_items.len() as f32 * (40.0 + ABILITY_X)) - ABILITY_X;
            spirit_max = spirit_max.max(icon_width);
        }
        spirit_max
    };

    let mut list_max_width = 0.0_f32;
    for item in data.b1.iter().chain(data.b2.iter()) {
        let mut max_line_width = 0.0_f32;
        for line in item.text.split('\n') {
            let text_width = measure_text_with_superscript(PxScale::from(ABILITY_FONT_SIZE), font, line); 
            max_line_width = max_line_width.max(text_width as f32);
        }
        
        let mut container_width = 8.0 + 40.0 + 8.0 + max_line_width + 8.0; 
        
        if item.icon_id == Some(crate::global::game::img015::ICON_CONJURE) {
            if let Some(spirit) = &data.spirit_data {
                container_width = container_width.max(8.0 + calc_spirit_width(spirit) + SPIRIT_PADDING_X); 
            }
        }
        list_max_width = list_max_width.max(container_width);
    }

    let mut max_needed_width = base_grid_width;
    max_needed_width = max_needed_width.max(list_max_width);
    max_needed_width = max_needed_width.max(check_icon_row_width(&data.traits));
    max_needed_width = max_needed_width.max(check_icon_row_width(&data.h1));
    max_needed_width = max_needed_width.max(check_icon_row_width(&data.h2));
    max_needed_width = max_needed_width.max(check_icon_row_width(&data.footer));

    let canvas_width = (max_needed_width.ceil() as i32) * scale;
    let mut target_image = RgbaImage::new(canvas_width as u32, 4000 * scale as u32); 
    
    let bg_color = Rgba([33, 33, 33, 255]);
    let separator_color = Rgba([60, 60, 60, 255]);
    let text_white = Rgba([255, 255, 255, 255]);
    let text_weak = Rgba([150, 150, 150, 255]);
    let header_bg = Rgba([20, 20, 20, 255]);
    let data_bg = Rgba([60, 60, 60, 255]);

    let img015_folder = crate::global::io::paths::img015_folder(Path::new(""));
    let mut img015_base = RgbaImage::new(1024, 1024);
    if let Some(resolved_path) = crate::global::resolver::get(&img015_folder, &["img015.png"], priority).into_iter().next() {
        if let Ok(loaded) = image::open(&resolved_path) { img015_base = loaded.to_rgba8(); }
    }    

    let mut custom_assets = HashMap::new();
    for (variant, bytes) in assets::CUSTOM_ICON_DATA {
        if let Ok(loaded_img) = image::load_from_memory(bytes) {
            custom_assets.insert(variant.clone(), loaded_img.to_rgba8());
        }
    }
    
    // === HEADER ===
    if let Some(path) = &data.icon_path {
        if let Ok(icon_img) = image::open(path) {
            let mut rgba = autocrop(icon_img.to_rgba8());
            let max_w = 110 * scale as u32;
            let max_h = 85 * scale as u32;
            let aspect = rgba.width() as f32 / rgba.height() as f32;
            let target_aspect = max_w as f32 / max_h as f32;
            
            let (target_w, target_h) = if aspect > target_aspect {
                (max_w, (max_w as f32 / aspect).round() as u32)
            } else {
                ((max_h as f32 * aspect).round() as u32, max_h)
            };
            
            if rgba.width() != target_w || rgba.height() != target_h {
                rgba = image::imageops::resize(&rgba, target_w, target_h, image::imageops::FilterType::Lanczos3);
            }
            
            let x_offset = padding as i64 + ((max_w - target_w) / 2) as i64;
            let y_offset = padding as i64 + (max_h - target_h) as i64;            
            image::imageops::overlay(&mut target_image, &rgba, x_offset, y_offset);
        }
    }

    let text_start_x = padding + 110 * scale + 12 * scale;
    let shift_y = HEADER_TEXT_Y_SHIFT * scale;
    let max_name_width = NAME_BOX_WIDTH * HEADER_CONTENT_SCALE * scale_f;
    let name_box_height = NAME_BOX_HEIGHT * HEADER_CONTENT_SCALE * scale_f; 
    let mut name_scale = NAME_BASE_FONT_SIZE * HEADER_CONTENT_SCALE;
    let mut name_lines = wrap_text(&data.name, font, PxScale::from(name_scale * scale_f), max_name_width);
    let scaled_line_spacing = (NAME_LINE_SPACING as f32 * HEADER_CONTENT_SCALE).round() as i32;

    while name_lines.len() > 2 && name_scale > 8.0 {
        name_scale -= 1.0;
        name_lines = wrap_text(&data.name, font, PxScale::from(name_scale * scale_f), max_name_width);
    }

    let line_height = (name_scale * scale_f) as i32 + (scaled_line_spacing * scale);
    let total_text_height = name_lines.len() as i32 * line_height;

    let base_box_y = padding + 8 * scale + shift_y;
    let scaled_y_offset = (NAME_Y_OFFSET as f32 * HEADER_CONTENT_SCALE).round() as i32;
    let mut current_name_y = base_box_y + ((name_box_height as i32 - total_text_height) / 2).max(0) + (scaled_y_offset * scale);

    for line in &name_lines {
        draw_text_mut(&mut target_image, text_white, text_start_x, current_name_y, PxScale::from(name_scale * scale_f), font, line);
        current_name_y += line_height;
    }

    let final_id_y = padding + (52.0 * HEADER_CONTENT_SCALE).round() as i32 * scale + shift_y;
    let final_level_y = padding + (70.0 * HEADER_CONTENT_SCALE).round() as i32 * scale + shift_y;

    draw_text_mut(&mut target_image, text_weak, text_start_x, final_id_y, PxScale::from(14.0 * HEADER_CONTENT_SCALE * scale_f), font, &format!("ID: {}", data.id_str));
    
    // --- STYLIZED LEVEL FIELD ---
    let lvl_prefix_scale = PxScale::from(16.0 * HEADER_CONTENT_SCALE * scale_f);
    let (prefix_width, _) = text_size(lvl_prefix_scale, font, &data.top_label);

    let lvl_val_scale = PxScale::from(15.0 * HEADER_CONTENT_SCALE * scale_f); 
    let (val_width, _) = text_size(lvl_val_scale, font, &data.top_value);

    let box_pad_x = (8.0 * HEADER_CONTENT_SCALE).round() as i32 * scale; 
    let box_pad_y = (2.0 * HEADER_CONTENT_SCALE).round() as i32 * scale;
    let box_height = lvl_val_scale.y as i32 + box_pad_y * 2;
    let box_width = val_width as i32 + box_pad_x * 2;
    let spacing = (4.0 * HEADER_CONTENT_SCALE).round() as i32 * scale; 
    let box_x = text_start_x + prefix_width as i32 + spacing;
    let box_y = final_level_y + (lvl_prefix_scale.y as i32 - box_height) / 2;

    draw_text_mut(&mut target_image, text_white, text_start_x, final_level_y, lvl_prefix_scale, font, &data.top_label);

    let input_bg = Rgba([10, 10, 10, 255]); 
    draw_rounded_rect_mut(&mut target_image, Rect::at(box_x, box_y).of_size(box_width as u32, box_height as u32), box_height / 2, input_bg);
    draw_text_mut(&mut target_image, text_white, box_x + box_pad_x, box_y + box_pad_y, lvl_val_scale, font, &data.top_value);

    let lowest_element_y = std::cmp::max(padding + 85 * scale, box_y + box_height);
    let mut current_y_global = lowest_element_y + HEADER_PADDING_Y * scale; 
    draw_filled_rect_mut(&mut target_image, Rect::at(padding, current_y_global).of_size(canvas_width as u32 - (padding * 2) as u32, 1 * scale as u32), separator_color);
    current_y_global += STAT_GRID_PADDING_Y * scale;

    // === STAT GRID ===
    let get_label = |key: &str| -> &'static str {
        if data.is_cat { crate::features::cat::registry::get_cat_stat(key).display_name } 
        else { crate::features::enemy::registry::get_enemy_stat(key).display_name }
    };

    let stat_headers_1 = [get_label("Attack"), get_label("Dps"), get_label("Range"), get_label("Atk Cycle"), get_label("Atk Type")];
    let stat_headers_2 = [get_label("Hitpoints"), get_label("Knockbacks"), get_label("Speed"), data.cd_label.as_str(), data.cost_label.as_str()];
    let stat_data_1 = [&data.atk, &data.dps, &data.range, &String::new(), &data.atk_type];
    let stat_data_2 = [&data.hp, &data.kb, &data.speed, &data.cd_value, &data.cost_value];

    let row_height = 24 * scale;
    let cell_radius = 4 * scale;
    
    let r1_hy = current_y_global;
    let r1_dy = current_y_global + row_height + gap;
    let r2_hy = current_y_global + (row_height * 2) + (gap * 2);
    let r2_dy = current_y_global + (row_height * 3) + (gap * 3);

    for col in 0..5 {
        let current_x = padding + (col * (col_w + gap));
        
        let h1_rect = Rect::at(current_x, r1_hy).of_size(col_w as u32, row_height as u32);
        draw_rounded_rect_mut(&mut target_image, h1_rect, cell_radius, header_bg);
        draw_centered_text(&mut target_image, text_white, h1_rect, PxScale::from(14.0 * STAT_GRID_TEXT_SCALE * scale_f), font, stat_headers_1[col as usize]);
        
        let d1_rect = Rect::at(current_x, r1_dy).of_size(col_w as u32, row_height as u32);
        if col == 3 {
            draw_time_cell(&mut target_image, data_bg, d1_rect, data.atk_cycle, font, scale_f, scale, cell_radius, STAT_GRID_TEXT_SCALE);
        } else {
            draw_rounded_rect_mut(&mut target_image, d1_rect, cell_radius, data_bg);
            draw_centered_text(&mut target_image, text_white, d1_rect, PxScale::from(15.0 * STAT_GRID_TEXT_SCALE * scale_f), font, stat_data_1[col as usize]);
        }
        
        let h2_rect = Rect::at(current_x, r2_hy).of_size(col_w as u32, row_height as u32);
        draw_rounded_rect_mut(&mut target_image, h2_rect, cell_radius, header_bg);
        draw_centered_text(&mut target_image, text_white, h2_rect, PxScale::from(14.0 * STAT_GRID_TEXT_SCALE * scale_f), font, stat_headers_2[col as usize]);
        
        let d2_rect = Rect::at(current_x, r2_dy).of_size(col_w as u32, row_height as u32);
        if col == 3 && data.is_cd_time {
            draw_time_cell(&mut target_image, data_bg, d2_rect, data.cd_frames, font, scale_f, scale, cell_radius, STAT_GRID_TEXT_SCALE);
        } else {
            draw_rounded_rect_mut(&mut target_image, d2_rect, cell_radius, data_bg);
            draw_centered_text(&mut target_image, text_white, d2_rect, PxScale::from(15.0 * STAT_GRID_TEXT_SCALE * scale_f), font, stat_data_2[col as usize]);
        }
    }

    current_y_global += (row_height * 4) + (gap * 3) + STAT_GRID_PADDING_Y * scale;
    draw_filled_rect_mut(&mut target_image, Rect::at(padding, current_y_global).of_size(canvas_width as u32 - (padding * 2) as u32, 1 * scale as u32), separator_color);
    current_y_global += 10 * scale;

    // === ABILITIES ===
    let ability_line_height = (ABILITY_FONT_SIZE * scale_f).round() as i32 + (ABILITY_LINE_SPACING * scale);

    let draw_icon_row = |canvas_image: &mut RgbaImage, items: &Vec<AbilityItem>, start_y: i32, start_x: i32| -> i32 {
        if items.is_empty() { return start_y; }
        let mut current_x = start_x;
        let mut current_y = start_y;
        for ability_item in items {
            if current_x + export_icon_size > canvas_width - padding {
                current_x = start_x;
                current_y += export_icon_size + icon_gap_y;
            }
            let icon_surface = get_icon_image(ability_item, &cuts_map, &img015_base, &custom_assets, export_icon_size as u32);
            image::imageops::overlay(canvas_image, &icon_surface, current_x as i64, current_y as i64);
            current_x += export_icon_size + icon_gap_x; 
        }
        current_y + export_icon_size 
    };

    let draw_spirit_icons = |spirit_image: &mut RgbaImage, spirit_items: &[AbilityItem], start_y: i32, start_x_absolute: i32| -> i32 {
        if spirit_items.is_empty() { return start_y; }
        let mut current_x = start_x_absolute;
        for spirit_item in spirit_items {
            let icon_surface = get_icon_image(spirit_item, &cuts_map, &img015_base, &custom_assets, export_icon_size as u32);
            image::imageops::overlay(spirit_image, &icon_surface, current_x as i64, start_y as i64);
            current_x += export_icon_size as i32 + icon_gap_x;
        }
        start_y + export_icon_size as i32
    };

    let draw_spirit_list = |spirit_image: &mut RgbaImage, spirit_items: &[AbilityItem], start_y: i32, start_x_absolute: i32| -> i32 {
        if spirit_items.is_empty() { return start_y; }
        let mut current_y = start_y;
        for (index, spirit_item) in spirit_items.iter().enumerate() {
            let icon_surface = get_icon_image(spirit_item, &cuts_map, &img015_base, &custom_assets, export_icon_size as u32);
            image::imageops::overlay(spirit_image, &icon_surface, start_x_absolute as i64, current_y as i64);
            
            let text_lines: Vec<&str> = spirit_item.text.split('\n').collect();
            let mut current_text_y = current_y;
            let total_text_height = text_lines.len() as i32 * ability_line_height;
            if total_text_height < export_icon_size as i32 { current_text_y += (export_icon_size as i32 - total_text_height) / 2; }
            current_text_y += list_text_y_offset;
            
            for line in text_lines {
                draw_text_with_superscript(spirit_image, text_white, start_x_absolute + export_icon_size as i32 + list_text_gap_x, current_text_y, PxScale::from(ABILITY_FONT_SIZE * scale_f), font, line);
                current_text_y += ability_line_height;
            }
            current_y += (export_icon_size as i32).max(total_text_height);
            if index < spirit_items.len() - 1 { current_y += icon_gap_y; }
        }
        current_y
    };

    let draw_spirit_card = |canvas_image: &mut RgbaImage, spirit: &SpiritData, card_start_y: i32| -> i32 {
        let card_inner_y = card_start_y + icon_gap_y; 
        let start_x_absolute = padding as i32 + 8 * scale;
        let spirit_panel_width = (calc_spirit_width(spirit) * scale_f) as i32 + (SPIRIT_PADDING_X * scale_f) as i32;

        let damage_lines: Vec<&str> = spirit.dmg_text.split('\n').collect();
        let damage_total_height = damage_lines.len() as i32 * ability_line_height;
        
        let mut final_panel_height = 8 * scale;
        final_panel_height += (export_icon_size as i32).max(damage_total_height) + icon_gap_y;
        
        let mut has_previous_section = false;
        let mut last_section_was_trait = false;
        
        let add_gap = |total_height: &mut i32, has_previous_element: &mut bool, current_is_trait: bool, was_last_element_trait: &mut bool| {
            if *has_previous_element { *total_height += if *was_last_element_trait { trait_gap_y } else { icon_gap_y }; }
            *has_previous_element = true;
            *was_last_element_trait = current_is_trait;
        };

        if !spirit.traits.is_empty() { final_panel_height += export_icon_size as i32; has_previous_section = true; last_section_was_trait = true; }
        if !spirit.h1.is_empty() { add_gap(&mut final_panel_height, &mut has_previous_section, false, &mut last_section_was_trait); final_panel_height += export_icon_size as i32; }
        if !spirit.h2.is_empty() { add_gap(&mut final_panel_height, &mut has_previous_section, false, &mut last_section_was_trait); final_panel_height += export_icon_size as i32; }
        
        if !spirit.b1.is_empty() || !spirit.b2.is_empty() {
            add_gap(&mut final_panel_height, &mut has_previous_section, false, &mut last_section_was_trait);
            let calc_list_height = |items: &[AbilityItem]| -> i32 {
                let mut accumulated_height = 0;
                for (index, list_item) in items.iter().enumerate() {
                    let lines_count = list_item.text.split('\n').count() as i32;
                    accumulated_height += (export_icon_size as i32).max(lines_count * ability_line_height);
                    if index < items.len() - 1 { accumulated_height += icon_gap_y; }
                }
                accumulated_height
            };
            if !spirit.b1.is_empty() { final_panel_height += calc_list_height(&spirit.b1); }
            if !spirit.b1.is_empty() && !spirit.b2.is_empty() { final_panel_height += icon_gap_y; }
            if !spirit.b2.is_empty() { final_panel_height += calc_list_height(&spirit.b2); }
        }
        
        if !spirit.footer.is_empty() { add_gap(&mut final_panel_height, &mut has_previous_section, false, &mut last_section_was_trait); final_panel_height += export_icon_size as i32; }
        final_panel_height += 8 * scale;

        let spirit_rect = Rect::at(padding as i32, card_inner_y).of_size(spirit_panel_width as u32, final_panel_height as u32);
        draw_bottom_rounded_rect_mut(canvas_image, spirit_rect, 8 * scale, Rgba([8, 8, 8, 255]));

        let mut current_y_offset = card_inner_y + 8 * scale;
        let area_item = AbilityItem { icon_id: Some(crate::global::game::img015::ICON_AREA_ATTACK), border_id: None, custom_icon: CustomIcon::None, text: String::new() };
        let area_icon = get_icon_image(&area_item, &cuts_map, &img015_base, &custom_assets, export_icon_size as u32);
        image::imageops::overlay(canvas_image, &area_icon, start_x_absolute as i64, current_y_offset as i64);

        let mut damage_text_y = current_y_offset + list_text_y_offset;
        if damage_total_height < export_icon_size as i32 { damage_text_y += (export_icon_size as i32 - damage_total_height) / 2; }
        for line in damage_lines {
            draw_text_with_superscript(canvas_image, text_white, start_x_absolute + export_icon_size as i32 + list_text_gap_x, damage_text_y, PxScale::from(ABILITY_FONT_SIZE * scale_f), font, line);
            damage_text_y += ability_line_height;
        }
        current_y_offset += (export_icon_size as i32).max(damage_total_height) + icon_gap_y;

        has_previous_section = false;
        last_section_was_trait = false;

        if !spirit.traits.is_empty() { current_y_offset = draw_spirit_icons(canvas_image, &spirit.traits, current_y_offset, start_x_absolute); has_previous_section = true; last_section_was_trait = true; }
        if !spirit.h1.is_empty() { add_gap(&mut current_y_offset, &mut has_previous_section, false, &mut last_section_was_trait); current_y_offset = draw_spirit_icons(canvas_image, &spirit.h1, current_y_offset, start_x_absolute); }
        if !spirit.h2.is_empty() { add_gap(&mut current_y_offset, &mut has_previous_section, false, &mut last_section_was_trait); current_y_offset = draw_spirit_icons(canvas_image, &spirit.h2, current_y_offset, start_x_absolute); }
        
        if !spirit.b1.is_empty() || !spirit.b2.is_empty() {
            add_gap(&mut current_y_offset, &mut has_previous_section, false, &mut last_section_was_trait);
            if !spirit.b1.is_empty() { current_y_offset = draw_spirit_list(canvas_image, &spirit.b1, current_y_offset, start_x_absolute); }
            if !spirit.b1.is_empty() && !spirit.b2.is_empty() { current_y_offset += icon_gap_y; }
            if !spirit.b2.is_empty() { current_y_offset = draw_spirit_list(canvas_image, &spirit.b2, current_y_offset, start_x_absolute); }
        }
        
        if !spirit.footer.is_empty() { add_gap(&mut current_y_offset, &mut has_previous_section, false, &mut last_section_was_trait); draw_spirit_icons(canvas_image, &spirit.footer, current_y_offset, start_x_absolute); }

        card_inner_y + final_panel_height
    };

    let draw_list = |canvas_image: &mut RgbaImage, items: &Vec<AbilityItem>, start_y: i32| -> i32 {
        if items.is_empty() { return start_y; }
        let mut current_y = start_y;
        for (index, item) in items.iter().enumerate() {
            let icon_surface = get_icon_image(item, &cuts_map, &img015_base, &custom_assets, export_icon_size as u32);
            image::imageops::overlay(canvas_image, &icon_surface, padding as i64, current_y as i64);
            
            let text_lines: Vec<&str> = item.text.split('\n').collect();
            let total_text_height = text_lines.len() as i32 * ability_line_height;
            
            let mut current_text_y = current_y + list_text_y_offset;
            if total_text_height < export_icon_size as i32 { current_text_y += (export_icon_size as i32 - total_text_height) / 2; }
            
            for line in text_lines {
                draw_text_with_superscript(canvas_image, text_white, padding + export_icon_size as i32 + list_text_gap_x, current_text_y, PxScale::from(ABILITY_FONT_SIZE * scale_f), font, line);
                current_text_y += ability_line_height;
            }
            
            current_y += (export_icon_size as i32).max(total_text_height);

            if item.icon_id == Some(crate::global::game::img015::ICON_CONJURE) {
                if let Some(spirit) = &data.spirit_data {
                    current_y = draw_spirit_card(canvas_image, spirit, current_y);
                }
            }

            if index < items.len() - 1 { current_y += icon_gap_y; }
        }
        current_y 
    };

    let mut previously_drew_section = false;
    let mut last_main_section_was_trait = false;

    let draw_section_gap = |current_y: &mut i32, has_previous_element: &mut bool, current_is_trait: bool, was_last_element_trait: &mut bool| {
        if *has_previous_element { *current_y += if *was_last_element_trait { trait_gap_y } else { icon_gap_y }; }
        *has_previous_element = true;
        *was_last_element_trait = current_is_trait;
    };

    if !data.traits.is_empty() { 
        current_y_global = draw_icon_row(&mut target_image, &data.traits, current_y_global, padding); 
        previously_drew_section = true;
        last_main_section_was_trait = true;
    }
    if !data.h1.is_empty() { 
        draw_section_gap(&mut current_y_global, &mut previously_drew_section, false, &mut last_main_section_was_trait);
        current_y_global = draw_icon_row(&mut target_image, &data.h1, current_y_global, padding); 
    }
    if !data.h2.is_empty() { 
        draw_section_gap(&mut current_y_global, &mut previously_drew_section, false, &mut last_main_section_was_trait);
        current_y_global = draw_icon_row(&mut target_image, &data.h2, current_y_global, padding); 
    }
    if !data.b1.is_empty() || !data.b2.is_empty() {
        draw_section_gap(&mut current_y_global, &mut previously_drew_section, false, &mut last_main_section_was_trait);
        if !data.b1.is_empty() { current_y_global = draw_list(&mut target_image, &data.b1, current_y_global); }
        if !data.b1.is_empty() && !data.b2.is_empty() { current_y_global += icon_gap_y; }
        if !data.b2.is_empty() { current_y_global = draw_list(&mut target_image, &data.b2, current_y_global); }
    }
    if !data.footer.is_empty() { 
        draw_section_gap(&mut current_y_global, &mut previously_drew_section, false, &mut last_main_section_was_trait);
        current_y_global = draw_icon_row(&mut target_image, &data.footer, current_y_global, padding); 
    }

    let final_height = current_y_global + padding;
    let final_cropped = image::imageops::crop_imm(&target_image, 0, 0, canvas_width as u32, final_height as u32).to_image();
    
    let border_thick = CANVAS_BORDER_THICKNESS * scale;
    let border_pad = CANVAS_BORDER_PADDING * scale;
    let margin = border_thick + border_pad;

    let final_width_with_pad = canvas_width as u32 + (margin * 2) as u32;
    let final_height_with_pad = final_height as u32 + (margin * 2) as u32;
    let mut final_background_layer = RgbaImage::new(final_width_with_pad, final_height_with_pad);
    
    let border_radius = CANVAS_BORDER_RADIUS * scale;
    let inner_border_radius = CANVAS_BORDER_INNER_RADIUS * scale;
    
    if border_thick > 0 {
        draw_rounded_rect_mut(&mut final_background_layer, Rect::at(0, 0).of_size(final_width_with_pad, final_height_with_pad), border_radius, CANVAS_BORDER_COLOR);
        let inner_width = final_width_with_pad - (border_thick * 2) as u32;
        let inner_height = final_height_with_pad - (border_thick * 2) as u32;
        draw_rounded_rect_mut(&mut final_background_layer, Rect::at(border_thick, border_thick).of_size(inner_width, inner_height), inner_border_radius, bg_color);
    } else {
        draw_rounded_rect_mut(&mut final_background_layer, Rect::at(0, 0).of_size(final_width_with_pad, final_height_with_pad), border_radius, bg_color);
    }

    image::imageops::overlay(&mut final_background_layer, &final_cropped, margin as i64, margin as i64);
    
    final_background_layer
}

pub fn generate_and_copy(
    ctx: egui::Context, 
    priority: Vec<String>,
    data: StatblockData,
    cuts_map: HashMap<usize, SpriteCut>,
) {
    let ctx_clone = ctx.clone();
    ctx_clone.data_mut(|d| d.insert_temp(egui::Id::new("is_copying"), true));

    std::thread::spawn(move || {
        let img_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            build_statblock_image(&priority, data, cuts_map)
        }));

        let mut success = false;
        if let Ok(img) = img_result {
            let (width, height) = img.dimensions();
            let img_data = ImageData { width: width as usize, height: height as usize, bytes: Cow::Owned(img.into_raw()) };
            success = Clipboard::new().and_then(|mut c| c.set_image(img_data)).is_ok();
        }

        let current_time = ctx_clone.input(|i| i.time);
        
        ctx_clone.data_mut(|d| {
            d.insert_temp(egui::Id::new("export_copy_time"), current_time);
            d.insert_temp(egui::Id::new("export_copy_res"), success);
            d.insert_temp(egui::Id::new("is_copying"), false);
        });
        ctx_clone.request_repaint();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f32(2.1));
            ctx_clone.request_repaint();
        });
    });
}

pub fn generate_and_save(
    ctx: egui::Context, 
    priority: Vec<String>,
    data: StatblockData,
    cuts_map: HashMap<usize, SpriteCut>,
) {
    let ctx_clone = ctx.clone();
    ctx_clone.data_mut(|d| d.insert_temp(egui::Id::new("is_exporting"), true));

    std::thread::spawn(move || {
        let id_str = data.id_str.clone();
        let val_str = data.top_value.clone();
        let is_cat = data.is_cat;
        
        let img_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            build_statblock_image(&priority, data, cuts_map)
        }));

        let mut success = false;
        if let Ok(img) = img_result {
            let export_dir = Path::new("exports");
            let _ = fs::create_dir_all(export_dir);

            if export_dir.exists() {
                let safe_val_str = val_str.replace(|c: char| !c.is_alphanumeric() && c != '+', "");
                let prefix = if is_cat { "Lv" } else { "Mag" };
                let filename = export_dir.join(format!("{}.{}{}.statblock.png", id_str, prefix, safe_val_str));
                success = img.save(filename).is_ok();
            }
        }
        
        let current_time = ctx_clone.input(|i| i.time);
        
        ctx_clone.data_mut(|d| {
             d.insert_temp(egui::Id::new("export_save_time"), current_time);
             d.insert_temp(egui::Id::new("export_save_res"), success);
             d.insert_temp(egui::Id::new("is_exporting"), false);
        });
        ctx_clone.request_repaint();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f32(2.1));
            ctx_clone.request_repaint();
        });
    });
}