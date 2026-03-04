use eframe::egui;
use std::path::Path;
use std::collections::HashMap;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::DetailTab;
use crate::features::settings::logic::Settings;
use crate::core::utils::autocrop; 
use crate::ui::components::name_box;
use crate::features::cat::paths::{self, AssetType};
use crate::features::cat::data::skilllevel::TalentCost;
use crate::global_data::imgcut::SpriteSheet;

pub const HEADER_NP_ICON_SIZE: f32 = 24.0;
pub const HEADER_NP_TEXT_SIZE: f32 = 20.0;

pub const TALENT_BTN_WIDTH: f32 = 100.0;
pub const TALENT_BTN_HEIGHT: f32 = 23.0;

pub fn render(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    cat: &CatEntry,
    current_form: &mut usize,
    current_tab: &mut DetailTab,
    current_level: &mut i32,
    level_input: &mut String,
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    settings: &Settings,
    talent_levels: &mut HashMap<u8, u8>,
    talent_costs: &HashMap<u8, TalentCost>,
    img022_sheet: &SpriteSheet,
) {
    ui.vertical(|ui| {
        render_form_buttons(ui, cat, current_form, current_tab);
        ui.separator();
        ui.add_space(5.0);

        ui.horizontal_top(|ui| {
            render_cat_icon(ctx, ui, cat, *current_form, current_key, texture_cache);
            ui.add_space(3.0);
            render_info_box(ui, cat, *current_form, level_input, current_level);

            if *current_tab == DetailTab::Talents {
                if let Some(talent_data) = &cat.talent_data {
                    ui.add_space(15.0);
                    
                    let separator_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 85.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, separator_color);
                    
                    ui.add_space(15.0);
                    render_talent_controls(ui, talent_data, talent_levels, talent_costs, img022_sheet, settings);
                }
            }
        });
    });
}

fn render_talent_controls(
    ui: &mut egui::Ui,
    talent_data: &crate::features::cat::data::skillacquisition::TalentRaw,
    talent_levels: &mut HashMap<u8, u8>,
    talent_costs: &HashMap<u8, TalentCost>,
    img022_sheet: &SpriteSheet,
    settings: &Settings,
) {
    ui.vertical(|ui| {
        let total_np = crate::features::cat::logic::talents::get_total_np_cost(talent_data, talent_levels, talent_costs);
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            
            let mut drawn = false;
            if settings.game_language != "--" {
                if let Some(cut) = img022_sheet.cuts_map.get(&crate::global_data::img022::ICON_NP_COST) {
                    if let Some(tex) = &img022_sheet.texture_handle {
                        let aspect = cut.original_size.x / cut.original_size.y;
                        let size = egui::vec2(HEADER_NP_ICON_SIZE * aspect, HEADER_NP_ICON_SIZE);
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates));
                        drawn = true;
                    }
                }
            }
            
            if !drawn {
                ui.label(egui::RichText::new("Total NP").size(HEADER_NP_TEXT_SIZE).strong().color(egui::Color32::WHITE));
            }
            
            ui.label(egui::RichText::new(format!("{}", total_np)).size(HEADER_NP_TEXT_SIZE).strong().color(egui::Color32::WHITE));
        });
        
        ui.spacing_mut().item_spacing.y = 5.0;

        let mut has_normal_enabled = false;
        let mut has_ultra_enabled = false;
        let mut has_ultra_talents = false;

        for (index, group) in talent_data.groups.iter().enumerate() {
            let lvl = *talent_levels.get(&(index as u8)).unwrap_or(&0);
            if group.limit == 1 {
                has_ultra_talents = true;
                if lvl > 0 { has_ultra_enabled = true; }
            } else {
                if lvl > 0 { has_normal_enabled = true; }
            }
        }

        let normal_btn_text = if has_normal_enabled { "No Talents" } else { "All Talents" };
        if ui.add_sized([TALENT_BTN_WIDTH, TALENT_BTN_HEIGHT], egui::Button::new(normal_btn_text)).clicked() {
            for (index, group) in talent_data.groups.iter().enumerate() {
                if group.limit != 1 {
                    let new_lvl = if has_normal_enabled { 0 } else { group.max_level.max(1) };
                    talent_levels.insert(index as u8, new_lvl);
                }
            }
        }

        let ultra_btn_text = if has_ultra_enabled { "No Ultra" } else { "All Ultra" };
        ui.add_enabled_ui(has_ultra_talents, |ui| {
            if ui.add_sized([TALENT_BTN_WIDTH, TALENT_BTN_HEIGHT], egui::Button::new(ultra_btn_text)).clicked() {
                for (index, group) in talent_data.groups.iter().enumerate() {
                    if group.limit == 1 {
                        let new_lvl = if has_ultra_enabled { 0 } else { group.max_level.max(1) };
                        talent_levels.insert(index as u8, new_lvl);
                    }
                }
            }
        });
    });
}

fn render_form_buttons(ui: &mut egui::Ui, cat: &CatEntry, current_form: &mut usize, current_tab: &mut DetailTab) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0; 
        ui.horizontal(|ui| {
            let form_labels = ["Normal", "Evolved", "True", "Ultra"];
            
            for index in 0..4 {
                let exists = cat.forms.get(index).copied().unwrap_or(false);
                let is_selected = *current_form == index;

                let (fill, stroke, text) = if is_selected {
                    (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                } else if exists {
                    (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                } else {
                    (egui::Color32::from_gray(15), egui::Stroke::new(1.0, egui::Color32::from_gray(50)), egui::Color32::from_gray(120))
                };
                
                let btn = egui::Button::new(egui::RichText::new(form_labels[index]).color(text))
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(egui::Rounding::ZERO)
                    .min_size(egui::vec2(60.0, 30.0));
                
                if ui.add_enabled(exists, btn).clicked() { 
                    *current_form = index; 
                    
                    if index < 2 && *current_tab == DetailTab::Talents {
                        *current_tab = DetailTab::Abilities;
                    }
                }
            }

            ui.add(egui::Separator::default().vertical().spacing(20.0));

            let tabs = [
                (DetailTab::Abilities, "Abilities"),
                (DetailTab::Talents, "Talents"),
                (DetailTab::Details, "Details"),
                (DetailTab::Animation, "Animation"),
            ];

            for (tab_enum, label) in tabs {
                let is_talents = tab_enum == DetailTab::Talents;
                let enabled = if is_talents {
                    *current_form >= 2 && cat.talent_data.is_some()
                } else {
                    true
                };

                let is_selected = *current_tab == tab_enum;
                
                let (fill, stroke, text) = if is_selected {
                    (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                } else if enabled {
                    (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                } else {
                    (egui::Color32::from_gray(15), egui::Stroke::new(1.0, egui::Color32::from_gray(50)), egui::Color32::from_gray(120))
                };

                let btn = egui::Button::new(egui::RichText::new(label).color(text))
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(egui::Rounding::from(5.0)) 
                    .min_size(egui::vec2(60.0, 30.0));

                if ui.add_enabled(enabled, btn).clicked() { *current_tab = tab_enum; }
            }
        });
    });
}

fn render_cat_icon(
    ctx: &egui::Context,
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    form: usize,
    current_key: &mut String,
    texture_cache: &mut Option<egui::TextureHandle>
) {
    let expected_path_str = if let Some(path) = paths::image(
        Path::new(paths::DIR_CATS),
        AssetType::Icon,
        cat.id,
        form,
        cat.egg_ids
    ) {
        path.to_string_lossy().to_string()
    } else {
        String::new() 
    };

    if *current_key != expected_path_str {
        *current_key = expected_path_str.clone();
        *texture_cache = if !expected_path_str.is_empty() {
             load_icon_texture(ctx, &expected_path_str)
        } else {
             None
        };
    }

    if let Some(tex) = texture_cache { 
        ui.image((tex.id(), tex.size_vec2())); 
    } else { 
        ui.allocate_space(egui::vec2(64.0, 64.0)); 
    }
}

fn load_icon_texture(ctx: &egui::Context, path_str: &str) -> Option<egui::TextureHandle> {
    let path = Path::new(path_str);
    let fallback = Path::new(paths::FALLBACK_ICON);
    
    let final_path = if path.exists() { path } else if fallback.exists() { fallback } else { return None };

    let img = image::open(final_path).ok()?;
    let mut rgba = autocrop(img.to_rgba8());
    
    if rgba.width() != 110 || rgba.height() != 85 {
        rgba = image::imageops::resize(&rgba, 110, 85, image::imageops::FilterType::Lanczos3);
    }
    
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.as_flat_samples();
    
    Some(ctx.load_texture("detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), egui::TextureOptions::LINEAR))
}

fn render_info_box(ui: &mut egui::Ui, cat: &CatEntry, form: usize, level_input: &mut String, current_level: &mut i32) {
    ui.vertical(|ui| {
        ui.set_width(name_box::NAME_BOX_WIDTH);

        let form_num = form + 1;
        let raw_name = cat.names.get(form).cloned().unwrap_or_default();
        
        let disp_name = if raw_name.is_empty() { 
            format!("{:03}-{}", cat.id, form_num) 
        } else { 
            raw_name 
        };

        ui.add_space(15.0); 
        name_box::render(ui, &disp_name);
        ui.spacing_mut().item_spacing.y = 0.0;
        
        ui.add_space(10.0);
        ui.label(egui::RichText::new(format!("ID: {:03}-{}", cat.id, form_num)).color(egui::Color32::from_gray(100)).size(12.0));
        ui.add_space(3.0);

        ui.horizontal(|ui| {
            ui.label("Level:");
            if ui.add(egui::TextEdit::singleline(level_input).desired_width(40.0)).changed() {
                let sum: i32 = level_input.split('+').filter_map(|s| s.trim().parse::<i32>().ok()).sum();
                *current_level = if sum <= 0 { 1 } else { sum };
            }
        });
    });
}