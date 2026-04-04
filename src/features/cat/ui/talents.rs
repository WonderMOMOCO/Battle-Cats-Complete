use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::features::cat::data::skillacquisition::{TalentRaw, TalentGroupRaw};
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::utils::autocrop;
use crate::features::settings::logic::Settings; 
use crate::features::cat::data::unitid::CatRaw; 
use crate::features::cat::data::unitlevel::CatLevelCurve;
use crate::features::cat::logic::talents;
use crate::features::cat::paths;
use crate::features::cat::data::skilllevel::TalentCost;
use crate::global::ui::shared::render_fallback_icon;
use crate::global::assets::CustomAssets;

pub const TALENT_NP_ICON_SIZE: f32 = 20.0;
pub const TALENT_NP_TEXT_SIZE: f32 = 18.0;

pub const TALENT_SECTION_SPACING: f32 = 2.0;

pub fn render(
    ui: &mut egui::Ui,
    talent_data: &TalentRaw,
    sheets: &[SpriteSheet],
    img022_sheets: &[SpriteSheet],
    name_cache: &mut HashMap<String, egui::TextureHandle>,
    descriptions: Option<&Vec<String>>,
    settings: &Settings, 
    current_stats: Option<&CatRaw>, 
    curve: Option<&CatLevelCurve>,
    unit_level: i32,
    talent_levels: &mut HashMap<u8, u8>, 
    cat_id: u32,                         
    talent_costs: &HashMap<u8, TalentCost>,
    assets: &CustomAssets,
) {
    ui.add_space(5.0);
    
    let sidebar_pad = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("sidebar_visible_width"))).unwrap_or(0.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false]) 
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0); 

                for (index, group) in talent_data.groups.iter().enumerate() {
                    render_talent_group(
                        ui, 
                        cat_id, 
                        index, 
                        group, 
                        sheets,
                        img022_sheets,
                        name_cache, 
                        descriptions, 
                        settings, 
                        current_stats, 
                        curve, 
                        unit_level, 
                        talent_levels, 
                        sidebar_pad,
                        talent_costs,
                        assets
                    );
                }
            });
        });
}

fn render_talent_group(
    ui: &mut egui::Ui,
    cat_id: u32,
    index: usize,
    group: &TalentGroupRaw,
    sheets: &[SpriteSheet],
    img022_sheets: &[SpriteSheet],
    name_cache: &mut HashMap<String, egui::TextureHandle>,
    descriptions: Option<&Vec<String>>,
    settings: &Settings,
    current_stats: Option<&CatRaw>,
    curve: Option<&CatLevelCurve>,
    unit_level: i32,
    talent_levels: &mut HashMap<u8, u8>,
    sidebar_pad: f32,
    talent_costs: &HashMap<u8, TalentCost>,
    assets: &CustomAssets,
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
            let scrollbar_padding = 12.0; 
            
            let target_width = ui.available_width() - sidebar_pad - scrollbar_padding;
            ui.set_width(target_width.max(10.0));

            ui.vertical(|ui| {
                if render_header(ui, group, sheets, name_cache, settings, expanded, assets) {
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
                        unit_level,
                        talent_costs,
                        img022_sheets,
                        settings
                    );
                }
            });
        });
}

fn render_header(
    ui: &mut egui::Ui,
    group: &TalentGroupRaw,
    sheets: &[SpriteSheet],
    name_cache: &mut HashMap<String, egui::TextureHandle>,
    settings: &Settings,
    expanded: bool,
    assets: &CustomAssets,
) -> bool {
    let mut toggle_clicked = false;

    let header_res = ui.horizontal(|ui| {
        ui.set_width(ui.available_width());

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            
            let def_opt = crate::features::cat::registry::get_by_talent_id(group.ability_id);
            
            if let Some(def) = &def_opt {
                let size = egui::vec2(40.0, 40.0);
                
                let mut drawn = false;
                
                match &def.icon {
                    crate::features::cat::registry::AbilityIcon::Custom(custom) => {
                        if let Some(tex) = custom.get_texture(assets) {
                            ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)));
                            drawn = true;
                        }
                    },
                    crate::features::cat::registry::AbilityIcon::Standard(icon_id) => {
                        for sheet in sheets {
                            if let Some(cut) = sheet.cuts_map.get(icon_id) {
                                if let Some(tex) = &sheet.texture_handle {
                                    ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates));
                                    drawn = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                if !drawn {
                    render_fallback_icon(ui, def.fallback, egui::Color32::BLACK);
                }
            } else {
                ui.label(egui::RichText::new("?").weak().size(24.0));
            }

            if let Some(texture) = get_or_load_skill_name(ui, group, settings, name_cache) {
                ui.image((texture.id(), texture.size_vec2()));
            } else {
                let fallback_text = match &def_opt {
                    Some(def) => format!("{}", def.name),
                    None => format!("Unknown Skill (ID: {})", group.ability_id),
                };
                ui.label(
                    egui::RichText::new(fallback_text)
                        .strong()
                        .size(18.0)
                        .color(egui::Color32::WHITE)
                );
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
    talent_costs: &HashMap<u8, TalentCost>,
    img022_sheets: &[SpriteSheet],
    _settings: &Settings,
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

    ui.add_space(TALENT_SECTION_SPACING); 

    let current_lvl_val = *talent_levels.get(&(index as u8)).unwrap_or(&0);
    let np_cost = crate::features::cat::logic::talents::get_talent_np_cost(group.cost_id, current_lvl_val, talent_costs);

    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(100))
        .rounding(4.0)
        .inner_margin(4.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                
                let mut drawn = false;
                for sheet in img022_sheets {
                    if let Some(cut) = sheet.cuts_map.get(&crate::global::game::img022::ICON_NP_COST) {
                        if let Some(tex) = &sheet.texture_handle {
                            let aspect = cut.original_size.x / cut.original_size.y;
                            let size = egui::vec2(TALENT_NP_ICON_SIZE * aspect, TALENT_NP_ICON_SIZE);
                            ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates));
                            drawn = true;
                            break;
                        }
                    }
                }

                if !drawn {
                    ui.label(egui::RichText::new("NP Cost").size(TALENT_NP_TEXT_SIZE).strong().color(egui::Color32::WHITE));
                }
                
                ui.label(egui::RichText::new(format!("{}", np_cost)).size(TALENT_NP_TEXT_SIZE).strong().color(egui::Color32::WHITE));
            });
        });

    ui.add_space(TALENT_SECTION_SPACING);

    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(100))
        .rounding(4.0)
        .inner_margin(4.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            
            ui.vertical(|ui| {
                let effective_max = if group.max_level == 0 { 1 } else { group.max_level };
                let current_level_mut = talent_levels.entry(index as u8).or_insert(0);

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
                        
                        ui.add(egui::Slider::new(current_level_mut, 0..=effective_max)
                            .step_by(1.0)
                            .show_value(false)
                        );
                    });

                    ui.add(egui::DragValue::new(current_level_mut)
                        .speed(0.1)
                        .range(0..=effective_max)
                    );
                });

                if let Some(stats) = current_stats {
                    if let Some(display_text) = talents::calculate_talent_display(group, stats, *current_level_mut, curve, unit_level) {
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
    let dir = Path::new(paths::DIR_SKILL_NAME);
    let base_filename = format!("Skill_name_{:03}.png", image_id);
    crate::global::get(dir, &[base_filename.as_str()], &settings.general.language_priority).into_iter().next()
}