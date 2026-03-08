use std::borrow::Cow;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use image::{RgbaImage, Rgba};
use ab_glyph::{FontRef, PxScale};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use arboard::{Clipboard, ImageData};
use eframe::egui;

use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::paths::{self, AssetType};
use crate::core::utils::autocrop;
use crate::global::imgcut::SpriteCut;
use crate::features::cat::logic::abilities::{collect_ability_data, CustomIcon, AbilityItem};
use crate::features::settings::logic::Settings;
use crate::features::cat::ui::abilities::{ABILITY_X, ABILITY_Y, TRAIT_Y};

use super::draw::*;

// --- EXPORT LAYOUT CONSTANTS ---
const NAME_BOX_WIDTH: f32 = 150.0;
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

// --- CANVAS BORDER CONSTANTS ---
const CANVAS_BORDER_THICKNESS: i32 = 5; 
const CANVAS_BORDER_RADIUS: i32 = 8; 
const CANVAS_BORDER_INNER_RADIUS: i32 = 8; 
const CANVAS_BORDER_PADDING: i32 = 4; 
const CANVAS_BORDER_COLOR: Rgba<u8> = Rgba([31, 106, 165, 255]); 

const SPIRIT_PADDING_X: f32 = 8.0;

fn build_statblock_image(
    language: &str,
    cat: &CatEntry,
    form: usize,
    level: i32,
    level_str: &str,
    cuts_map: HashMap<usize, SpriteCut>,
    talent_levels: Option<HashMap<u8, u8>>,
    is_conjure_expanded: bool
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

    let font_data: &[u8] = match language {
        "kr" => include_bytes!("../../../assets/NotoSansKR-Regular.ttf"),
        "tw" => include_bytes!("../../../assets/NotoSansTC-Regular.ttf"),
        "th" => include_bytes!("../../../assets/NotoSansThai-Regular.ttf"),
        _ => include_bytes!("../../../assets/NotoSansJP-Regular.ttf"), 
    };
    let font = FontRef::try_from_slice(font_data).expect("Failed to load font");

    let mut dummy_settings = Settings::default();
    dummy_settings.game_language = language.to_string();

    let base_stats = cat.stats.get(form).and_then(|opt| opt.as_ref()).unwrap();
    let final_stats = crate::features::cat::logic::stats::get_final_stats(
        base_stats, cat.curve.as_ref(), level, cat.talent_data.as_ref(), talent_levels.as_ref()
    );

    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(
        &final_stats, base_stats, level, cat.curve.as_ref(), &dummy_settings, false, 
        cat.talent_data.as_ref(), talent_levels.as_ref()
    );

    let mut max_needed_width: f32 = base_grid_width;

    let check_icon_row_width = |items: &Vec<AbilityItem>| -> f32 {
        if items.is_empty() { return 0.0; }
        8.0 + (items.len() as f32 * (40.0 + ABILITY_X)) - ABILITY_X + 8.0 
    };

    let mut list_max_w = 0.0_f32;
    for item in b1.iter().chain(b2.iter()) {
        let mut max_line_w = 0.0;
        for line in item.text.split('\n') {
            let tw = measure_text_with_superscript(PxScale::from(ABILITY_FONT_SIZE), &font, line); 
            if tw as f32 > max_line_w { max_line_w = tw as f32; }
        }
        let mut w = 8.0 + 40.0 + 8.0 + max_line_w + 8.0; 
        
        if item.icon_id == crate::global::img015::ICON_CONJURE && is_conjure_expanded {
            if let Some(c_vec) = crate::features::cat::logic::stats::load_from_id(base_stats.conjure_unit_id) {
                if let Some(c_stats) = c_vec.first() {
                    let conjure_final = crate::features::cat::logic::stats::get_final_stats(c_stats, cat.curve.as_ref(), level, None, None);
                    
                    let (s_traits, s_h1, s_h2, s_b1, s_b2, s_footer) = collect_ability_data(&conjure_final, c_stats, level, cat.curve.as_ref(), &dummy_settings, true, None, None);
                    let mut spirit_max = 0.0_f32;
                    
                    let dmg_text = format!("Damage: {}\nRange: {}", conjure_final.attack_1, conjure_final.standing_range); 
                    for l in dmg_text.split('\n') {
                        let tw = measure_text_with_superscript(PxScale::from(ABILITY_FONT_SIZE), &font, l);
                        let sw = 8.0 + 40.0 + 8.0 + tw as f32;
                        if sw > spirit_max { spirit_max = sw; }
                    }
                    
                    for s_item in s_b1.iter().chain(s_b2.iter()) {
                        let mut s_line_w = 0.0;
                        for l in s_item.text.split('\n') {
                            let tw = measure_text_with_superscript(PxScale::from(ABILITY_FONT_SIZE), &font, l);
                            if tw as f32 > s_line_w { s_line_w = tw as f32; }
                        }
                        let sw = 8.0 + 40.0 + 8.0 + s_line_w;
                        if sw > spirit_max { spirit_max = sw; }
                    }

                    for s_items in [&s_traits, &s_h1, &s_h2, &s_footer] {
                        if !s_items.is_empty() {
                            let ic_w = 8.0 + (s_items.len() as f32 * (40.0 + ABILITY_X)) - ABILITY_X;
                            if ic_w > spirit_max { spirit_max = ic_w; }
                        }
                    }

                    w = w.max(8.0 + spirit_max + SPIRIT_PADDING_X); 
                }
            }
        }
        if w > list_max_w { list_max_w = w; }
    }
    max_needed_width = max_needed_width.max(list_max_w);

    max_needed_width = max_needed_width.max(check_icon_row_width(&traits));
    max_needed_width = max_needed_width.max(check_icon_row_width(&h1));
    max_needed_width = max_needed_width.max(check_icon_row_width(&h2));
    max_needed_width = max_needed_width.max(check_icon_row_width(&footer));

    let canvas_width = (max_needed_width.ceil() as i32) * scale;
    
    let mut img = RgbaImage::new(canvas_width as u32, 4000 * scale as u32); 
    
    let bg_color = Rgba([33, 33, 33, 255]);
    let separator_color = Rgba([60, 60, 60, 255]);
    let text_white = Rgba([255, 255, 255, 255]);
    let text_weak = Rgba([150, 150, 150, 255]);
    
    let header_bg = Rgba([20, 20, 20, 255]);
    let data_bg = Rgba([60, 60, 60, 255]);

    let img015_folder = crate::global::paths::img015_folder(Path::new(""));
    
    let codes_to_try: Vec<String> = if language == "--" || language.is_empty() {
        crate::core::utils::LANGUAGE_PRIORITY.iter().map(|s| s.to_string()).collect()
    } else {
        vec![language.to_string()]
    };

    let mut img015_base_opt = None;
    for code in codes_to_try {
        let png_filename = if code.is_empty() { "img015.png".to_string() } else { format!("img015_{}.png", code) };
        let full_png_path = img015_folder.join(&png_filename);
        if full_png_path.exists() {
            if let Ok(loaded) = image::open(&full_png_path) {
                img015_base_opt = Some(loaded.to_rgba8());
                break;
            }
        }
    }
    let img015_base = img015_base_opt.unwrap_or_else(|| RgbaImage::new(1024, 1024));
    
    let multihit_base = image::load_from_memory(include_bytes!("../../../assets/multihit.png")).unwrap().to_rgba8();
    let kamikaze_base = image::load_from_memory(include_bytes!("../../../assets/kamikaze.png")).unwrap().to_rgba8();
    let bosswave_base = image::load_from_memory(include_bytes!("../../../assets/boss_wave_immune.png")).unwrap().to_rgba8();

    // === HEADER ===
    let icon_path = paths::image(Path::new(paths::DIR_CATS), AssetType::Icon, cat.id, form, cat.egg_ids);
    if let Some(path) = icon_path {
        if let Ok(icon_img) = image::open(path) {
            let mut rgba = autocrop(icon_img.to_rgba8());
            let target_w = 110 * scale as u32;
            let target_h = 85 * scale as u32;
            if rgba.width() != target_w || rgba.height() != target_h {
                rgba = image::imageops::resize(&rgba, target_w, target_h, image::imageops::FilterType::Lanczos3);
            }
            image::imageops::overlay(&mut img, &rgba, padding as i64, padding as i64);
        }
    }

    let form_num = form + 1;
    let disp_name = if cat.names[form].is_empty() { format!("{:03}-{}", cat.id, form_num) } else { cat.names[form].clone() };

    let text_x = padding + 110 * scale + 12 * scale;
    let shift_y = HEADER_TEXT_Y_SHIFT * scale;
    
    let max_name_width = NAME_BOX_WIDTH * HEADER_CONTENT_SCALE * scale_f;
    let name_box_height = NAME_BOX_HEIGHT * HEADER_CONTENT_SCALE * scale_f; 
    
    let mut name_scale = NAME_BASE_FONT_SIZE * HEADER_CONTENT_SCALE;
    let mut name_lines = wrap_text(&disp_name, &font, PxScale::from(name_scale * scale_f), max_name_width);

    let scaled_line_spacing = (NAME_LINE_SPACING as f32 * HEADER_CONTENT_SCALE).round() as i32;

    while name_lines.len() > 2 && name_scale > 8.0 {
        name_scale -= 1.0;
        name_lines = wrap_text(&disp_name, &font, PxScale::from(name_scale * scale_f), max_name_width);
    }

    let line_height = (name_scale * scale_f) as i32 + (scaled_line_spacing * scale);
    let total_text_h = name_lines.len() as i32 * line_height;

    let base_box_y = padding + 8 * scale + shift_y;
    let scaled_y_offset = (NAME_Y_OFFSET as f32 * HEADER_CONTENT_SCALE).round() as i32;
    let mut current_name_y = base_box_y + ((name_box_height as i32 - total_text_h) / 2).max(0) + (scaled_y_offset * scale);

    for line in &name_lines {
        draw_text_mut(&mut img, text_white, text_x, current_name_y, PxScale::from(name_scale * scale_f), &font, line);
        current_name_y += line_height;
    }

    let final_id_y = padding + (52.0 * HEADER_CONTENT_SCALE).round() as i32 * scale + shift_y;
    let final_level_y = padding + (70.0 * HEADER_CONTENT_SCALE).round() as i32 * scale + shift_y;

    draw_text_mut(&mut img, text_weak, text_x, final_id_y, PxScale::from(14.0 * HEADER_CONTENT_SCALE * scale_f), &font, &format!("ID: {:03}-{}", cat.id, form_num));
    
    // --- STYLIZED EGUI-LIKE LEVEL FIELD ---
    let lvl_prefix = "Level:";
    let lvl_prefix_scale = PxScale::from(16.0 * HEADER_CONTENT_SCALE * scale_f);
    let (prefix_w, _) = text_size(lvl_prefix_scale, &font, lvl_prefix);

    let lvl_val_scale = PxScale::from(15.0 * HEADER_CONTENT_SCALE * scale_f); 
    let (val_w, _) = text_size(lvl_val_scale, &font, level_str);

    let box_pad_x = (8.0 * HEADER_CONTENT_SCALE).round() as i32 * scale; 
    let box_pad_y = (2.0 * HEADER_CONTENT_SCALE).round() as i32 * scale;
    
    let visual_text_h = lvl_val_scale.y as i32;
    let box_h = visual_text_h + box_pad_y * 2;
    let box_w = val_w as i32 + box_pad_x * 2;

    let spacing = (4.0 * HEADER_CONTENT_SCALE).round() as i32 * scale; 
    let box_x = text_x + prefix_w as i32 + spacing;
    
    let box_y = final_level_y + (lvl_prefix_scale.y as i32 - box_h) / 2;

    draw_text_mut(&mut img, text_white, text_x, final_level_y, lvl_prefix_scale, &font, lvl_prefix);

    let input_bg = Rgba([10, 10, 10, 255]); 
    let pill_radius = box_h / 2;
    draw_rounded_rect_mut(&mut img, Rect::at(box_x, box_y).of_size(box_w as u32, box_h as u32), pill_radius, input_bg);

    draw_text_mut(&mut img, text_white, box_x + box_pad_x, box_y + box_pad_y, lvl_val_scale, &font, level_str);

    let lowest_element_y = std::cmp::max(padding + 85 * scale, box_y + box_h);
    
    let mut cursor_y = lowest_element_y + HEADER_PADDING_Y * scale; 
    draw_filled_rect_mut(&mut img, Rect::at(padding, cursor_y).of_size(canvas_width as u32 - (padding * 2) as u32, 1 * scale as u32), separator_color);
    cursor_y += STAT_GRID_PADDING_Y * scale;

    // === STAT GRID ===
    let total_atk = final_stats.attack_1 + final_stats.attack_2 + final_stats.attack_3;
    let cycle = final_stats.attack_cycle(cat.atk_anim_frames[form]);
    let dps = if cycle > 0 { (total_atk as f32 * 30.0 / cycle as f32) as i32 } else { 0 };
    let atk_type = if final_stats.area_attack == 0 { "Single" } else { "Area" };
    let cd_val = final_stats.effective_cooldown();
    
    let stat_headers_1 = ["Atk", "Dps", "Range", "Atk Cycle", "Atk Type"];
    let stat_data_1 = [
        total_atk.to_string(), 
        dps.to_string(), 
        final_stats.standing_range.to_string(), 
        "".to_string(), 
        atk_type.to_string()
    ];
    let stat_headers_2 = ["Hp", "Kb", "Speed", "Cooldown", "Cost"];
    let stat_data_2 = [
        final_stats.hitpoints.to_string(), 
        final_stats.knockbacks.to_string(), 
        final_stats.speed.to_string(), 
        "".to_string(), 
        format!("{}¢", final_stats.eoc1_cost * 3 / 2)
    ];

    let row_h = 24 * scale;
    let cell_radius = 4 * scale;
    
    let r1_hy = cursor_y;
    let r1_dy = cursor_y + row_h + gap;
    let r2_hy = cursor_y + (row_h * 2) + (gap * 2);
    let r2_dy = cursor_y + (row_h * 3) + (gap * 3);

    for col in 0..5 {
        let x = padding + (col * (col_w + gap));
        
        let h1_rect = Rect::at(x, r1_hy).of_size(col_w as u32, row_h as u32);
        draw_rounded_rect_mut(&mut img, h1_rect, cell_radius, header_bg);
        draw_centered_text(&mut img, text_white, h1_rect, PxScale::from(14.0 * STAT_GRID_TEXT_SCALE * scale_f), &font, stat_headers_1[col as usize]);
        
        let d1_rect = Rect::at(x, r1_dy).of_size(col_w as u32, row_h as u32);
        if col == 3 {
            draw_time_cell(&mut img, data_bg, d1_rect, cycle, &font, scale_f, scale, cell_radius, STAT_GRID_TEXT_SCALE);
        } else {
            draw_rounded_rect_mut(&mut img, d1_rect, cell_radius, data_bg);
            draw_centered_text(&mut img, text_white, d1_rect, PxScale::from(15.0 * STAT_GRID_TEXT_SCALE * scale_f), &font, &stat_data_1[col as usize]);
        }
        
        let h2_rect = Rect::at(x, r2_hy).of_size(col_w as u32, row_h as u32);
        draw_rounded_rect_mut(&mut img, h2_rect, cell_radius, header_bg);
        draw_centered_text(&mut img, text_white, h2_rect, PxScale::from(14.0 * STAT_GRID_TEXT_SCALE * scale_f), &font, stat_headers_2[col as usize]);
        
        let d2_rect = Rect::at(x, r2_dy).of_size(col_w as u32, row_h as u32);
        if col == 3 {
            draw_time_cell(&mut img, data_bg, d2_rect, cd_val, &font, scale_f, scale, cell_radius, STAT_GRID_TEXT_SCALE);
        } else {
            draw_rounded_rect_mut(&mut img, d2_rect, cell_radius, data_bg);
            draw_centered_text(&mut img, text_white, d2_rect, PxScale::from(15.0 * STAT_GRID_TEXT_SCALE * scale_f), &font, &stat_data_2[col as usize]);
        }
    }

    cursor_y += (row_h * 4) + (gap * 3) + STAT_GRID_PADDING_Y * scale;
    draw_filled_rect_mut(&mut img, Rect::at(padding, cursor_y).of_size(canvas_width as u32 - (padding * 2) as u32, 1 * scale as u32), separator_color);
    cursor_y += 10 * scale;

    // === ABILITIES ===
    let ability_line_height = (ABILITY_FONT_SIZE * scale_f).round() as i32 + (ABILITY_LINE_SPACING * scale);

    let draw_icon_row = |img: &mut RgbaImage, items: &Vec<AbilityItem>, y: i32, x_start: i32| -> i32 {
        if items.is_empty() { return y; }
        let mut x = x_start;
        let mut cur_y = y;
        
        for item in items {
            if x + export_icon_size > canvas_width - padding {
                x = x_start;
                cur_y += export_icon_size + icon_gap_y;
            }
            let icon = get_icon_image(item, &cuts_map, &img015_base, &multihit_base, &kamikaze_base, &bosswave_base, export_icon_size as u32);
            image::imageops::overlay(img, &icon, x as i64, cur_y as i64);
            x += export_icon_size + icon_gap_x; 
        }
        cur_y + export_icon_size 
    };

    let draw_list = |img: &mut RgbaImage, items: &Vec<AbilityItem>, mut y: i32| -> i32 {
        if items.is_empty() { return y; }

        for (i, item) in items.iter().enumerate() {
            let icon = get_icon_image(item, &cuts_map, &img015_base, &multihit_base, &kamikaze_base, &bosswave_base, export_icon_size as u32);
            image::imageops::overlay(img, &icon, padding as i64, y as i64);
            
            let lines: Vec<&str> = item.text.split('\n').collect();
            let total_text_h = lines.len() as i32 * ability_line_height;
            
            let mut text_y = y;
            if total_text_h < export_icon_size as i32 {
                text_y += (export_icon_size as i32 - total_text_h) / 2; 
            }
            text_y += list_text_y_offset;

            for line in lines {
                draw_text_with_superscript(img, text_white, padding + export_icon_size as i32 + list_text_gap_x, text_y, PxScale::from(ABILITY_FONT_SIZE * scale_f), &font, line);
                text_y += ability_line_height;
            }
            
            y += (export_icon_size as i32).max(total_text_h);

            // --- SPIRIT CARD RENDER BLOCK ---
            if item.icon_id == crate::global::img015::ICON_CONJURE && is_conjure_expanded {
                if let Some(c_vec) = crate::features::cat::logic::stats::load_from_id(base_stats.conjure_unit_id) {
                    if let Some(c_stats) = c_vec.first() {
                        y += icon_gap_y; 

                        let conjure_final = crate::features::cat::logic::stats::get_final_stats(c_stats, cat.curve.as_ref(), level, None, None);

                        let (s_traits, s_h1, s_h2, s_b1, s_b2, s_footer) = collect_ability_data(&conjure_final, c_stats, level, cat.curve.as_ref(), &dummy_settings, true, None, None);
                        
                        let sx = 8 * scale;
                        let mut spirit_content_w = 0;
                        
                        let dmg_text = format!("Damage: {}\nRange: {}", conjure_final.attack_1, conjure_final.standing_range);
                        let dmg_lines: Vec<&str> = dmg_text.split('\n').collect();

                        for l in &dmg_lines {
                            let tw = measure_text_with_superscript(PxScale::from(ABILITY_FONT_SIZE * scale_f), &font, l);
                            spirit_content_w = spirit_content_w.max(sx + export_icon_size as i32 + list_text_gap_x + tw as i32);
                        }
                        
                        for s_item in s_b1.iter().chain(s_b2.iter()) {
                            for l in s_item.text.split('\n') {
                                let tw = measure_text_with_superscript(PxScale::from(ABILITY_FONT_SIZE * scale_f), &font, l);
                                spirit_content_w = spirit_content_w.max(sx + export_icon_size as i32 + list_text_gap_x + tw as i32);
                            }
                        }
                        
                        for s_items in [&s_traits, &s_h1, &s_h2, &s_footer] {
                            if !s_items.is_empty() {
                                let ic_w = sx + (s_items.len() as i32 * (export_icon_size as i32 + icon_gap_x)) - icon_gap_x;
                                spirit_content_w = spirit_content_w.max(ic_w);
                            }
                        }

                        let spirit_w = spirit_content_w + (SPIRIT_PADDING_X * scale_f) as i32;

                        let mut final_h = 8 * scale;
                        let dmg_total_h = dmg_lines.len() as i32 * ability_line_height;
                        final_h += (export_icon_size as i32).max(dmg_total_h) + icon_gap_y;
                        
                        let mut prev = false;
                        let mut last_was_trait_s = false;
                        
                        if !s_traits.is_empty() { final_h += export_icon_size as i32; prev = true; last_was_trait_s = true; }
                        if !s_h1.is_empty() { if prev { final_h += if last_was_trait_s { trait_gap_y } else { icon_gap_y }; last_was_trait_s = false; } final_h += export_icon_size as i32; prev = true; }
                        if !s_h2.is_empty() { if prev { final_h += if last_was_trait_s { trait_gap_y } else { icon_gap_y }; last_was_trait_s = false; } final_h += export_icon_size as i32; prev = true; }
                        if !s_b1.is_empty() || !s_b2.is_empty() {
                            if prev { final_h += if last_was_trait_s { trait_gap_y } else { icon_gap_y }; last_was_trait_s = false; }
                            if !s_b1.is_empty() {
                                for (idx, it) in s_b1.iter().enumerate() {
                                    let th = it.text.split('\n').count() as i32 * ability_line_height;
                                    final_h += (export_icon_size as i32).max(th);
                                    if idx < s_b1.len() - 1 { final_h += icon_gap_y; }
                                }
                            }
                            if !s_b1.is_empty() && !s_b2.is_empty() { final_h += icon_gap_y; }
                            if !s_b2.is_empty() {
                                for (idx, it) in s_b2.iter().enumerate() {
                                    let th = it.text.split('\n').count() as i32 * ability_line_height;
                                    final_h += (export_icon_size as i32).max(th);
                                    if idx < s_b2.len() - 1 { final_h += icon_gap_y; }
                                }
                            }
                            prev = true;
                        }
                        if !s_footer.is_empty() { if prev { final_h += if last_was_trait_s { trait_gap_y } else { icon_gap_y }; } final_h += export_icon_size as i32; }
                        final_h += 8 * scale;

                        let spirit_rect = Rect::at(padding as i32, y).of_size(spirit_w as u32, final_h as u32);
                        draw_bottom_rounded_rect_mut(img, spirit_rect, 8 * scale, Rgba([8, 8, 8, 255]));

                        let mut sy = y + 8 * scale;
                        let sx_abs = padding as i32 + 8 * scale;

                        let area_item = AbilityItem { icon_id: crate::global::img015::ICON_AREA_ATTACK, border_id: None, custom_icon: CustomIcon::None, text: String::new() };
                        let area_icon = get_icon_image(&area_item, &cuts_map, &img015_base, &multihit_base, &kamikaze_base, &bosswave_base, export_icon_size as u32);
                        image::imageops::overlay(img, &area_icon, sx_abs as i64, sy as i64);

                        let mut sty = sy;
                        if dmg_total_h < export_icon_size as i32 {
                            sty += (export_icon_size as i32 - dmg_total_h) / 2; 
                        }
                        sty += list_text_y_offset;

                        for line in dmg_lines {
                            draw_text_with_superscript(img, text_white, sx_abs + export_icon_size as i32 + list_text_gap_x, sty, PxScale::from(ABILITY_FONT_SIZE * scale_f), &font, line);
                            sty += ability_line_height;
                        }
                        sy += (export_icon_size as i32).max(dmg_total_h) + icon_gap_y;

                        let draw_s_icons = |s_img: &mut RgbaImage, s_items: &[AbilityItem], cy: i32| -> i32 {
                            if s_items.is_empty() { return cy; }
                            let mut cx = sx_abs;
                            for it in s_items {
                                let ic = get_icon_image(it, &cuts_map, &img015_base, &multihit_base, &kamikaze_base, &bosswave_base, export_icon_size as u32);
                                image::imageops::overlay(s_img, &ic, cx as i64, cy as i64);
                                cx += export_icon_size as i32 + icon_gap_x;
                            }
                            cy + export_icon_size as i32
                        };

                        let draw_s_list = |s_img: &mut RgbaImage, s_items: &[AbilityItem], mut cy: i32| -> i32 {
                            if s_items.is_empty() { return cy; }
                            for (idx, it) in s_items.iter().enumerate() {
                                let ic = get_icon_image(it, &cuts_map, &img015_base, &multihit_base, &kamikaze_base, &bosswave_base, export_icon_size as u32);
                                image::imageops::overlay(s_img, &ic, sx_abs as i64, cy as i64);
                                
                                let lns: Vec<&str> = it.text.split('\n').collect();
                                let mut t_y = cy;
                                let th = lns.len() as i32 * ability_line_height;
                                if th < export_icon_size as i32 { t_y += (export_icon_size as i32 - th) / 2; }
                                t_y += list_text_y_offset;
                                
                                for ln in lns {
                                    draw_text_with_superscript(s_img, text_white, sx_abs + export_icon_size as i32 + list_text_gap_x, t_y, PxScale::from(ABILITY_FONT_SIZE * scale_f), &font, ln);
                                    t_y += ability_line_height;
                                }
                                cy += (export_icon_size as i32).max(th);
                                if idx < s_items.len() - 1 { cy += icon_gap_y; }
                            }
                            cy
                        };

                        let mut prev_b = false;
                        let mut last_was_trait_b = false;
                        
                        if !s_traits.is_empty() { sy = draw_s_icons(img, &s_traits, sy); prev_b = true; last_was_trait_b = true; }
                        if !s_h1.is_empty() { if prev_b { sy += if last_was_trait_b { trait_gap_y } else { icon_gap_y }; last_was_trait_b = false; } sy = draw_s_icons(img, &s_h1, sy); prev_b = true; }
                        if !s_h2.is_empty() { if prev_b { sy += if last_was_trait_b { trait_gap_y } else { icon_gap_y }; last_was_trait_b = false; } sy = draw_s_icons(img, &s_h2, sy); prev_b = true; }
                        if !s_b1.is_empty() || !s_b2.is_empty() {
                            if prev_b { sy += if last_was_trait_b { trait_gap_y } else { icon_gap_y }; last_was_trait_b = false; }
                            if !s_b1.is_empty() { sy = draw_s_list(img, &s_b1, sy); }
                            if !s_b1.is_empty() && !s_b2.is_empty() { sy += icon_gap_y; }
                            if !s_b2.is_empty() { draw_s_list(img, &s_b2, sy); }
                            prev_b = true;
                        }
                        if !s_footer.is_empty() { if prev_b { sy += if last_was_trait_b { trait_gap_y } else { icon_gap_y }; } draw_s_icons(img, &s_footer, sy); }

                        y += final_h;
                    }
                }
            }

            if i < items.len() - 1 {
                y += icon_gap_y;
            }
        }
        y 
    };

    let mut previously_drew = false;
    let mut last_was_trait_main = false;

    if !traits.is_empty() { 
        cursor_y = draw_icon_row(&mut img, &traits, cursor_y, padding); 
        previously_drew = true;
        last_was_trait_main = true;
    }
    
    if !h1.is_empty() { 
        if previously_drew { cursor_y += if last_was_trait_main { trait_gap_y } else { icon_gap_y }; last_was_trait_main = false; }
        cursor_y = draw_icon_row(&mut img, &h1, cursor_y, padding); 
        previously_drew = true;
    }
    
    if !h2.is_empty() { 
        if previously_drew { cursor_y += if last_was_trait_main { trait_gap_y } else { icon_gap_y }; last_was_trait_main = false; }
        cursor_y = draw_icon_row(&mut img, &h2, cursor_y, padding); 
        previously_drew = true;
    }

    if !b1.is_empty() || !b2.is_empty() {
        if previously_drew { cursor_y += if last_was_trait_main { trait_gap_y } else { icon_gap_y }; last_was_trait_main = false; }
        
        if !b1.is_empty() {
            cursor_y = draw_list(&mut img, &b1, cursor_y);
        }
        if !b1.is_empty() && !b2.is_empty() { 
            cursor_y += icon_gap_y; 
        }
        if !b2.is_empty() {
            cursor_y = draw_list(&mut img, &b2, cursor_y);
        }
        
        previously_drew = true;
    }

    if !footer.is_empty() { 
        if previously_drew { cursor_y += if last_was_trait_main { trait_gap_y } else { icon_gap_y }; } 
        cursor_y = draw_icon_row(&mut img, &footer, cursor_y, padding); 
    }

    let final_height = cursor_y + padding;
    let final_cropped = image::imageops::crop_imm(&img, 0, 0, canvas_width as u32, final_height as u32).to_image();
    
    let border_thick = CANVAS_BORDER_THICKNESS * scale;
    let border_pad = CANVAS_BORDER_PADDING * scale;
    
    let margin = border_thick + border_pad;

    let final_width_with_pad = canvas_width as u32 + (margin * 2) as u32;
    let final_height_with_pad = final_height as u32 + (margin * 2) as u32;

    let mut final_bg = RgbaImage::new(final_width_with_pad, final_height_with_pad);
    
    let border_radius = CANVAS_BORDER_RADIUS * scale;
    let inner_border_radius = CANVAS_BORDER_INNER_RADIUS * scale;
    
    if border_thick > 0 {
        draw_rounded_rect_mut(&mut final_bg, Rect::at(0, 0).of_size(final_width_with_pad, final_height_with_pad), border_radius, CANVAS_BORDER_COLOR);
        draw_rounded_rect_mut(
            &mut final_bg, 
            Rect::at(border_thick, border_thick)
                .of_size(final_width_with_pad - (border_thick * 2) as u32, final_height_with_pad - (border_thick * 2) as u32), 
            inner_border_radius, 
            bg_color
        );
    } else {
        draw_rounded_rect_mut(&mut final_bg, Rect::at(0, 0).of_size(final_width_with_pad, final_height_with_pad), border_radius, bg_color);
    }

    image::imageops::overlay(&mut final_bg, &final_cropped, margin as i64, margin as i64);
    
    final_bg
}

pub fn generate_and_copy_statblock(
    ctx: egui::Context, 
    language: String,
    cat: CatEntry,
    form: usize,
    level: i32,
    level_str: String,
    cuts_map: HashMap<usize, SpriteCut>,
    talent_levels: Option<HashMap<u8, u8>>,
    is_conjure_expanded: bool,
) {
    let ctx_clone = ctx.clone();

    std::thread::spawn(move || {
        let img_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            build_statblock_image(&language, &cat, form, level, &level_str, cuts_map, talent_levels, is_conjure_expanded)
        }));

        let mut success = false;

        if let Ok(img) = img_result {
            let (width, height) = img.dimensions();
            let raw_pixels = img.into_raw();
            let img_data = ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(raw_pixels),
            };

            success = match Clipboard::new() {
                Ok(mut clipboard) => clipboard.set_image(img_data).is_ok(),
                Err(_) => false,
            };
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

pub fn generate_and_save_statblock(
    ctx: egui::Context, 
    language: String,
    cat: CatEntry,
    form: usize,
    level: i32,
    level_str: String,
    cuts_map: HashMap<usize, SpriteCut>,
    talent_levels: Option<HashMap<u8, u8>>,
    is_conjure_expanded: bool,
) {
    let ctx_clone = ctx.clone();

    std::thread::spawn(move || {
        let img_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            build_statblock_image(&language, &cat, form, level, &level_str, cuts_map, talent_levels, is_conjure_expanded)
        }));

        let mut success = false;

        if let Ok(img) = img_result {
            let export_dir = Path::new("exports");
            success = true;

            if !export_dir.exists() {
                if fs::create_dir_all(export_dir).is_err() {
                    success = false;
                }
            }

            if success {
                let safe_level_str = level_str.replace(|c: char| !c.is_alphanumeric() && c != '+', "_");
                let filename = export_dir.join(format!("{:03}-{}.Lv{}.statblock.png", cat.id, form + 1, safe_level_str));
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