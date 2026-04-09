use eframe::egui;
use std::collections::HashMap;
use crate::features::stage::registry::Stage;
use crate::features::stage::data::mapstagedata::RewardStructure;
use crate::features::stage::logic::treasure as treasure_logic;
use crate::features::cat::data::unitbuy::UnitBuyRow;
use crate::global::formats::gatyaitembuy::GatyaItemBuy;
use crate::global::formats::gatyaitemname::GatyaItemName;

pub fn center_header(ui: &mut egui::Ui, display_text: &str) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(egui::RichText::new(display_text).strong()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

pub fn center_text(ui: &mut egui::Ui, display_text: impl Into<String>) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(display_text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    egui_context: &egui::Context,
    ui: &mut egui::Ui, 
    stage_data: &Stage, 
    item_buy_registry: &HashMap<u32, GatyaItemBuy>, 
    item_name_registry: &HashMap<usize, GatyaItemName>,
    drop_chara_registry: &HashMap<u32, u32>,
    unit_buy_registry: &HashMap<u32, UnitBuyRow>,
    item_texture_cache: &mut HashMap<u32, egui::TextureHandle>,
    active_language_priority_array: &[String]
) {
    match &stage_data.rewards {
        RewardStructure::Treasure { drop_rule, drops } => {
            let rule_description = treasure_logic::format_treasure_rule(*drop_rule);
            ui.strong(format!("Treasure | {}", rule_description));
            ui.separator();
            
            let valid_drops_array: Vec<_> = drops.iter().filter(|drop_data| drop_data.chance > 0).collect();
            
            if valid_drops_array.is_empty() {
                ui.label("No drops configured.");
                return;
            }

            egui::Grid::new("reward_treasure_grid")
                .striped(true)
                .spacing([15.0, 4.0])
                .min_row_height(32.0) 
                .show(ui, |grid| {
                    center_header(grid, "Chance");
                    center_header(grid, "Item");
                    center_header(grid, "Amount");
                    grid.end_row();

                    for drop_data in valid_drops_array {
                        let resolved_item_name = treasure_logic::resolve_item_name(
                            drop_data.id, 
                            item_buy_registry, 
                            item_name_registry, 
                            drop_chara_registry, 
                            unit_buy_registry, 
                            active_language_priority_array
                        );
                        
                        let chance_display = treasure_logic::format_drop_chance(drop_data.chance, *drop_rule);
                        center_text(grid, chance_display);
                        
                        grid.centered_and_justified(|icon_layout| {
                            let mut has_rendered_icon = false;
                            
                            if let Some(resolved_image_path) = treasure_logic::resolve_item_image_path(
                                drop_data.id, 
                                item_buy_registry, 
                                drop_chara_registry, 
                                unit_buy_registry, 
                                active_language_priority_array
                            ) {
                                if !item_texture_cache.contains_key(&drop_data.id) {
                                    if let Some(processed_color_image) = treasure_logic::process_item_icon_texture(&resolved_image_path) {
                                        let generated_texture_handle = egui_context.load_texture(
                                            format!("treasure_item_icon_{}", drop_data.id), 
                                            processed_color_image, 
                                            egui::TextureOptions::LINEAR
                                        );
                                        item_texture_cache.insert(drop_data.id, generated_texture_handle);
                                    }
                                }

                                if let Some(cached_texture_handle) = item_texture_cache.get(&drop_data.id) {
                                    let image_response = icon_layout.add(egui::Image::new(cached_texture_handle).max_size(egui::vec2(32.0, 32.0)));
                                    image_response.on_hover_text(resolved_item_name.clone());
                                    has_rendered_icon = true;
                                }
                            }

                            if !has_rendered_icon {
                                icon_layout.add(egui::Label::new(resolved_item_name).wrap_mode(egui::TextWrapMode::Extend));
                            }
                        });

                        center_text(grid, drop_data.amount.to_string());
                        grid.end_row();
                    }
                });
        }
        RewardStructure::Timed(timed_scores) => {
            ui.strong("Timed Score Rewards");
            ui.separator();
            
            if timed_scores.is_empty() {
                ui.label("No timed rewards configured.");
                return;
            }

            egui::Grid::new("reward_timed_grid")
                .striped(true)
                .spacing([15.0, 4.0])
                .min_row_height(32.0) 
                .show(ui, |grid| {
                    center_header(grid, "Score Required");
                    center_header(grid, "Item");
                    center_header(grid, "Amount");
                    grid.end_row();

                    for score_data in timed_scores {
                        let resolved_item_name = treasure_logic::resolve_item_name(
                            score_data.id, 
                            item_buy_registry, 
                            item_name_registry, 
                            drop_chara_registry, 
                            unit_buy_registry, 
                            active_language_priority_array
                        );
                        
                        center_text(grid, score_data.score.to_string());
                        
                        grid.centered_and_justified(|icon_layout| {
                            let mut has_rendered_icon = false;
                            
                            if let Some(resolved_image_path) = treasure_logic::resolve_item_image_path(
                                score_data.id, 
                                item_buy_registry, 
                                drop_chara_registry, 
                                unit_buy_registry, 
                                active_language_priority_array
                            ) {
                                if !item_texture_cache.contains_key(&score_data.id) {
                                    if let Some(processed_color_image) = treasure_logic::process_item_icon_texture(&resolved_image_path) {
                                        let generated_texture_handle = egui_context.load_texture(
                                            format!("treasure_item_icon_{}", score_data.id), 
                                            processed_color_image, 
                                            egui::TextureOptions::LINEAR
                                        );
                                        item_texture_cache.insert(score_data.id, generated_texture_handle);
                                    }
                                }

                                if let Some(cached_texture_handle) = item_texture_cache.get(&score_data.id) {
                                    let image_response = icon_layout.add(egui::Image::new(cached_texture_handle).max_size(egui::vec2(32.0, 32.0)));
                                    image_response.on_hover_text(resolved_item_name.clone());
                                    has_rendered_icon = true;
                                }
                            }

                            if !has_rendered_icon {
                                icon_layout.add(egui::Label::new(resolved_item_name).wrap_mode(egui::TextWrapMode::Extend));
                            }
                        });

                        center_text(grid, score_data.amount.to_string());
                        grid.end_row();
                    }
                });
        }
        RewardStructure::None => {
            ui.strong("Rewards");
            ui.separator();
            ui.label("No rewards for this stage.");
        }
    }
}