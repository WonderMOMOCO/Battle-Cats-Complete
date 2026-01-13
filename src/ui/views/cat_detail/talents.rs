use eframe::egui;
use std::collections::HashMap;
use std::path::Path;
use crate::core::files::skillacquisition::{self, TalentRaw};
use crate::core::files::imgcut::SpriteSheet;
use crate::core::utils::{self, autocrop};
use crate::core::settings::Settings; 
use crate::core::files::unitid::CatRaw; 
use crate::core::files::unitlevel::CatLevelCurve;
use crate::core::cat::talents;

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
    
    // Retrieve sidebar padding
    let sidebar_pad = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("sidebar_visible_width"))).unwrap_or(0.0);

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0); 

        for (index, group) in talent_data.groups.iter().enumerate() {
            let bg_color = if group.limit == 1 {
                egui::Color32::from_rgb(120, 20, 20) 
            } else {
                egui::Color32::from_rgb(180, 140, 20) 
            };

            // Use ID to ensure unique card/talent states between Cats
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
                        
                        let header_res = ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());

                            // Icon and Text
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

                                let image_id_to_use = if group.name_id > 0 { group.name_id } else { group.ability_id as i16 };
                                if image_id_to_use > 0 {
                                    let mut final_file_name = None;
                                    let base_dir = "game/assets/Skill_name";
                                    if !settings.game_language.is_empty() {
                                        let candidate = format!("Skill_name_{:03}_{}.png", image_id_to_use, settings.game_language);
                                        if Path::new(&format!("{}/{}", base_dir, candidate)).exists() { final_file_name = Some(candidate); }
                                    } else {
                                        for code in utils::LANGUAGE_PRIORITY {
                                            if code.is_empty() { continue; }
                                            let candidate = format!("Skill_name_{:03}_{}.png", image_id_to_use, code);
                                            if Path::new(&format!("{}/{}", base_dir, candidate)).exists() {
                                                final_file_name = Some(candidate);
                                                break; 
                                            }
                                        }
                                    }
                                    if let Some(file_name) = final_file_name {
                                        if !name_cache.contains_key(&file_name) {
                                            let path_str = format!("{}/{}", base_dir, file_name);
                                            if let Ok(img) = image::open(&path_str) {
                                                let rgba = autocrop(img.to_rgba8());
                                                let texture = ui.ctx().load_texture(&file_name, egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR);
                                                name_cache.insert(file_name.clone(), texture);
                                            }
                                        }
                                        if let Some(texture) = name_cache.get(&file_name) { ui.image(&*texture); } 
                                    } 
                                }
                            });

                            // Arrow Button
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let arrow = if expanded { "▲" } else { "▼" };
                                let btn = egui::Button::new(egui::RichText::new(arrow).size(20.0).strong()).fill(egui::Color32::from_black_alpha(100));
                                if ui.add_sized([40.0, 40.0], btn).clicked() {
                                    expanded = !expanded;
                                    ui.data_mut(|d| d.insert_temp(id, expanded));
                                }
                            });
                        }); 

                        if header_res.response.interact(egui::Sense::click()).clicked() {
                            expanded = !expanded;
                            ui.data_mut(|d| d.insert_temp(id, expanded));
                        }

                        if expanded {
                            ui.add_space(6.0);
                            let mut text_to_display = if let Some(desc_list) = descriptions {
                                let tid = group.text_id as usize;
                                desc_list.get(tid).cloned().unwrap_or_else(|| "No skill description found".to_string())
                            } else {
                                "No skill description found".to_string()
                            };
                            if !text_to_display.contains('\n') { text_to_display.push('\n'); }

                            // Description Frame
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
                                                ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_gray(180); 
                                                ui.visuals_mut().widgets.active.bg_fill = egui::Color32::WHITE;            
                                                ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_gray(220);  
                                                ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(50));
                                                ui.visuals_mut().widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(50));
                                                ui.visuals_mut().widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(50));
                                                
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
                                                        .color(egui::Color32::WHITE) // Changed to WHITE
                                                        .size(15.0)   
                                                        .strong()     
                                                );
                                            }
                                        }
                                    });
                                });
                        }
                    });
                });
        }
    });
}