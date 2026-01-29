use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::data::cat::skillacquisition::{self, TalentRaw, TalentGroupRaw};
use crate::data::global::imgcut::SpriteSheet;
use crate::core::utils::{self, autocrop};
use crate::core::settings::Settings; 
use crate::data::cat::unitid::CatRaw; 
use crate::data::cat::unitlevel::CatLevelCurve;
use crate::core::cat::talents;
use crate::paths::cat; 

pub fn render(
    ui: &mut egui::Ui,
    talent_data: &TalentRaw,
    sheet: &SpriteSheet,
    name_cache: &mut HashMap<String, egui::TextureHandle>,
    descriptions: Option<&Vec<String>>,
    settings: &Settings, 
    current_stats: Option<&CatRaw>, 
    curve: Option<&CatLevelCurve>,
    unit_level: i32,
    talent_levels: &mut HashMap<u8, u8>, 
    cat_id: u32,                         
) {
    ui.add_space(5.0);
    
    let sidebar_pad = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("sidebar_visible_width"))).unwrap_or(0.0);

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0); 

        for (index, group) in talent_data.groups.iter().enumerate() {
            render_talent_group(
                ui, 
                cat_id, 
                index, 
                group, 
                sheet, 
                name_cache, 
                descriptions, 
                settings, 
                current_stats, 
                curve, 
                unit_level, 
                talent_levels, 
                sidebar_pad
            );
        }
    });
}

fn render_talent_group(
    ui: &mut egui::Ui,
    cat_id: u32,
    index: usize,
    group: &TalentGroupRaw,
    sheet: &SpriteSheet,
    name_cache: &mut HashMap<String, egui::TextureHandle>,
    descriptions: Option<&Vec<String>>,
    settings: &Settings,
    current_stats: Option<&CatRaw>,
    curve: Option<&CatLevelCurve>,
    unit_level: i32,
    talent_levels: &mut HashMap<u8, u8>,
    sidebar_pad: f32,
) {
    let bg_color = if group.limit == 1 {
        egui::Color32::from_rgb(120, 20, 20) 
    } else {
        egui::Color32::from_rgb(180, 140, 20) 
    };

    let id = ui.make_persistent_id(format!("cat_{}_talent_group_{}", cat_id, index));
    let mut expanded = ui.data(|d| d.get_temp(id).unwrap_or(false)); 

    egui::Frame::none()
        .fill(bg_color)
        .rounding(5.0)
        .inner_margin(6.0)
        .show(ui, |ui| {
            let target_width = ui.available_width() - sidebar_pad;
            ui.set_width(target_width.max(10.0));

            ui.vertical(|ui| {
                if render_header(ui, group, sheet, name_cache, settings, expanded) {
                    expanded = !expanded;
                    ui.data_mut(|d| d.insert_temp(id, expanded));
                }

                if expanded {
                    render_body(
                        ui, 
                        index, 
                        group, 
                        descriptions, 
                        talent_levels, 
                        current_stats, 
                        curve, 
                        unit_level
                    );
                }
            });
        });
}

fn render_header(
    ui: &mut egui::Ui,
    group: &TalentGroupRaw,
    sheet: &SpriteSheet,
    name_cache: &mut HashMap<String, egui::TextureHandle>,
    settings: &Settings,
    expanded: bool
) -> bool {
    let mut toggle_clicked = false;

    let header_res = ui.horizontal(|ui| {
        ui.set_width(ui.available_width());

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            
            if let Some(icon_id) = skillacquisition::map_ability_to_icon(group.ability_id) {
                if let Some(sprite) = sheet.get_sprite_by_line(icon_id) {
                    ui.add(sprite.fit_to_exact_size(egui::vec2(40.0, 40.0)));
                } else {
                    ui.label(egui::RichText::new("?").strong());
                }
            } else {
                ui.label(egui::RichText::new("?").weak());
            }

            if let Some(texture) = get_or_load_skill_name(ui, group, settings, name_cache) {
                ui.image((texture.id(), texture.size_vec2()));
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let arrow = if expanded { "▲" } else { "▼" };
            let btn = egui::Button::new(egui::RichText::new(arrow).size(20.0).strong())
                .fill(egui::Color32::from_black_alpha(100));
            
            if ui.add_sized([40.0, 40.0], btn).clicked() {
                toggle_clicked = true;
            }
        });
    });

    if header_res.response.interact(egui::Sense::click()).clicked() {
        toggle_clicked = true;
    }

    toggle_clicked
}

fn render_body(
    ui: &mut egui::Ui,
    index: usize,
    group: &TalentGroupRaw,
    descriptions: Option<&Vec<String>>,
    talent_levels: &mut HashMap<u8, u8>,
    current_stats: Option<&CatRaw>,
    curve: Option<&CatLevelCurve>,
    unit_level: i32,
) {
    ui.add_space(6.0);

    let mut text_to_display = if let Some(desc_list) = descriptions {
        let tid = group.text_id as usize;
        desc_list.get(tid).cloned().unwrap_or_else(|| "No skill description found".to_string())
    } else {
        "No skill description found".to_string()
    };
    if !text_to_display.contains('\n') { text_to_display.push('\n'); }

    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(100)) 
        .rounding(4.0)
        .inner_margin(4.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(text_to_display).color(egui::Color32::WHITE).size(13.0));
        });

    ui.add_space(0.0); 

    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(100))
        .rounding(4.0)
        .inner_margin(4.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            
            ui.vertical(|ui| {
                let effective_max = if group.max_level == 0 { 1 } else { group.max_level };
                let current_level = talent_levels.entry(index as u8).or_insert(0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.label(egui::RichText::new("Level:").strong());
                    
                    ui.scope(|ui| {
                        let vis = ui.visuals_mut();
                        vis.widgets.inactive.bg_fill = egui::Color32::from_gray(180); 
                        vis.widgets.active.bg_fill = egui::Color32::WHITE;            
                        vis.widgets.hovered.bg_fill = egui::Color32::from_gray(220);  
                        vis.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(50));
                        vis.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(50));
                        vis.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(50));
                        
                        ui.add(egui::Slider::new(current_level, 0..=effective_max)
                            .step_by(1.0)
                            .show_value(false)
                        );
                    });

                    ui.add(egui::DragValue::new(current_level)
                        .speed(0.1)
                        .range(0..=effective_max)
                    );
                });

                if let Some(stats) = current_stats {
                    if let Some(display_text) = talents::calculate_talent_display(group, stats, *current_level, curve, unit_level) {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(display_text)
                                .color(egui::Color32::WHITE)
                                .size(15.0)   
                                .strong()     
                        );
                    }
                }
            });
        });
}

fn get_or_load_skill_name(
    ui: &mut egui::Ui,
    group: &TalentGroupRaw,
    settings: &Settings,
    name_cache: &mut HashMap<String, egui::TextureHandle>
) -> Option<egui::TextureHandle> {
    let image_id = if group.name_id > 0 { group.name_id } else { group.ability_id as i16 };
    if image_id <= 0 { return None; }

    let path = find_skill_image_path(image_id, settings)?;
    let file_name = path.file_name()?.to_string_lossy().to_string();

    if !name_cache.contains_key(&file_name) {
        if let Ok(img) = image::open(&path) {
            let rgba = autocrop(img.to_rgba8());
            let texture = ui.ctx().load_texture(
                &file_name, 
                egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize], 
                    rgba.as_flat_samples().as_slice()
                ), 
                egui::TextureOptions::LINEAR
            );
            name_cache.insert(file_name.clone(), texture);
        }
    }

    name_cache.get(&file_name).cloned()
}

fn find_skill_image_path(image_id: i16, settings: &Settings) -> Option<PathBuf> {
    let root = Path::new(""); 

    if !settings.game_language.is_empty() {
        let candidate = cat::skill_icon(root, image_id, &settings.game_language);
        if candidate.exists() { return Some(candidate); }
    }
    
    for code in utils::LANGUAGE_PRIORITY {
        let candidate = cat::skill_icon(root, image_id, code);
        if candidate.exists() { return Some(candidate); }
    }

    None
}