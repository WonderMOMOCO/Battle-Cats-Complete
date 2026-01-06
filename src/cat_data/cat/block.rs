use eframe::egui;
use crate::definitions;
use crate::cat_data::stats::{self, CatRaw};
use crate::cat_data::abilities::{self, AbilityItem};
use crate::cat_data::sprites::SpriteSheet;
use super::utils::text_with_superscript;
use crate::settings::Settings; 

pub fn render_abilities(
    ui: &mut egui::Ui, 
    s: &CatRaw, 
    sheet: &SpriteSheet, 
    multihit_tex: &Option<egui::TextureHandle>, 
    level: i32,
    curve: Option<&stats::CatLevelCurve>,
    cat_id: u32,
    settings: &Settings, 
) {
    ui.spacing_mut().item_spacing.y = 0.0;

    let (grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = abilities::collect_ability_data(s, level, curve, multihit_tex, false);
    

    
    let mut previous_content = false;

    if !grp_hl1.is_empty() { 
        render_icon_row(ui, &grp_hl1, sheet, settings); 
        previous_content = true;
    }
    
    if !grp_hl2.is_empty() { 
        if previous_content { ui.add_space(settings.ability_padding_y); }
        render_icon_row(ui, &grp_hl2, sheet, settings); 
        previous_content = true;
    }

    let has_body = !grp_b1.is_empty() || !grp_b2.is_empty();
    if has_body {
       if previous_content { ui.add_space(settings.ability_padding_y); }
       
       render_list_view(ui, &grp_b1, sheet, multihit_tex, cat_id, level, curve, s, settings);
       render_list_view(ui, &grp_b2, sheet, multihit_tex, cat_id, level, curve, s, settings);
       previous_content = true;
    }

    if !grp_footer.is_empty() {
        let padding_needed = previous_content && !has_body;

        if padding_needed { 
            ui.add_space(settings.ability_padding_y);
        }
        render_icon_row(ui, &grp_footer, sheet, settings); 
    }
}

pub fn render_icon_row(ui: &mut egui::Ui, items: &Vec<AbilityItem>, sheet: &SpriteSheet, settings: &Settings) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(settings.ability_padding_x, settings.ability_padding_y);
        
        for item in items {
            let r = if let Some(tex_id) = item.custom_tex {
                ui.add(egui::Image::new((tex_id, egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE))))
            } else if let Some(sprite) = sheet.get_sprite_by_line(item.icon_id) {
                ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)))
            } else { continue; };
            
            r.on_hover_ui(|ui| { 
                text_with_superscript(ui, &item.text); 
            });
        }
    });
}

pub fn render_list_view(
    ui: &mut egui::Ui, 
    items: &Vec<AbilityItem>, 
    sheet: &SpriteSheet,
    multihit_tex: &Option<egui::TextureHandle>,
    cat_id: u32,
    current_level: i32,
    curve: Option<&stats::CatLevelCurve>,
    s: &CatRaw,
    settings: &Settings, 
) {
    for item in items {
        let is_conjure_item = item.icon_id == definitions::ICON_CONJURE;
        let mut expanded = false;
        let id = ui.make_persistent_id(format!("conjure_expand_{}", cat_id));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            
            let icon_size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
            let (rect, _) = ui.allocate_exact_size(icon_size, egui::Sense::hover());
            
            if let Some(tex_id) = item.custom_tex {
                egui::Image::new((tex_id, icon_size)).paint_at(ui, rect);
            } else if let Some(sprite) = sheet.get_sprite_by_line(item.icon_id) {
                sprite.paint_at(ui, rect);
            }

            if is_conjure_item {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 7.0;

                    expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.expand_spirit_details));
                    text_with_superscript(ui, &item.text);
                    
                    let btn_text = egui::RichText::new("Details").size(11.0);
                    let btn = if expanded {
                        egui::Button::new(btn_text.color(egui::Color32::WHITE))
                            .fill(egui::Color32::from_rgb(0, 100, 200))
                    } else {
                        egui::Button::new(btn_text)
                    };

                    if ui.add(btn).clicked() {
                        expanded = !expanded;
                        ui.data_mut(|d| d.insert_temp(id, expanded));
                    }
                });
            } else {
                text_with_superscript(ui, &item.text);
            }
        }); 

        if is_conjure_item && expanded {
            ui.add_space(3.0);
            
            egui::Frame::none()
                .fill(egui::Color32::from_black_alpha(220)) 
                .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 8.0, se: 8.0 }) 
                .inner_margin(8.0)
                .show(ui, |ui| {
                    
                    if let Some(conjure_stats) = stats::load_from_id(s.conjure_unit_id) {
                        let dmg = curve.as_ref().map_or(
                            conjure_stats.attack_1, 
                            |c| c.calculate_stat(conjure_stats.attack_1, current_level)
                        );

                        let range_txt = format!("Damage: {}\nRange: {}", dmg, conjure_stats.standing_range);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            if let Some(sprite) = sheet.get_sprite_by_line(definitions::ICON_AREA_ATTACK) {
                                ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
                            }
                            ui.label(range_txt);
                        });
                        
                        ui.add_space(settings.ability_padding_y);

                        let (c_hl1, c_hl2, c_b1, c_b2, c_ft) = abilities::collect_ability_data(&conjure_stats, current_level, curve, multihit_tex, true);
                        
                        if !c_hl1.is_empty() { render_icon_row(ui, &c_hl1, sheet, settings); }
                        if !c_hl2.is_empty() { render_icon_row(ui, &c_hl2, sheet, settings); }
                        
                        render_list_view(ui, &c_b1, sheet, multihit_tex, 0, current_level, curve, &conjure_stats, settings);
                        render_list_view(ui, &c_b2, sheet, multihit_tex, 0, current_level, curve, &conjure_stats, settings);
                        
                        if !c_ft.is_empty() {
                            render_icon_row(ui, &c_ft, sheet, settings);
                        }
                    } else {
                        ui.label(egui::RichText::new("Spirit data not found").weak());
                    }
                });
        }
        
        ui.add_space(settings.ability_padding_y);
    }
}