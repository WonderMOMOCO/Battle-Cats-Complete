use eframe::egui;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::stats::{self, CatRaw};
use crate::features::cat::logic::abilities;
use crate::global::formats::imgcut::SpriteSheet;
use crate::features::settings::logic::Settings;
use crate::ui::components::shared::{render_fallback_icon, text_with_superscript};
use crate::global::game::img015;
use crate::features::cat::data::skillacquisition::TalentRaw;
use std::collections::HashMap;
use crate::global::game::abilities::{ABILITY_X, ABILITY_Y, TRAIT_Y};
use crate::global::game::abilities::AbilityItem;
use crate::global::assets::CustomAssets;

pub fn render(
    ui: &mut egui::Ui, 
    final_stats: &CatRaw, 
    base_stats: &CatRaw,
    cat: &CatEntry, 
    level: i32,
    sheet: &SpriteSheet, 
    assets: &CustomAssets,
    settings: &Settings, 
    talent_data: Option<&TalentRaw>,
    talent_levels: Option<&HashMap<u8, u8>>
) {
    ui.spacing_mut().item_spacing.y = 0.0;

    let curve = cat.curve.as_ref();
    
    let (grp_trait, grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = abilities::collect_ability_data(
        final_stats, base_stats, level, curve, settings, false, talent_data, talent_levels
    );
    
    let mut previous_content = false;
    let mut last_was_trait = false;
    let main_border = egui::Color32::BLACK;

    if !grp_trait.is_empty() {
        render_icon_row(ui, &grp_trait, sheet, settings, main_border, assets);
        previous_content = true;
        last_was_trait = true;
    }

    if !grp_hl1.is_empty() { 
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
        render_icon_row(ui, &grp_hl1, sheet, settings, main_border, assets); 
        previous_content = true;
    }
    
    if !grp_hl2.is_empty() { 
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
        render_icon_row(ui, &grp_hl2, sheet, settings, main_border, assets); 
        previous_content = true;
    }

    let has_body = !grp_b1.is_empty() || !grp_b2.is_empty();
    if has_body {
       if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
       
       render_list_view(ui, &grp_b1, sheet, assets, cat.id, level, curve, final_stats, settings, main_border);
       
       if !grp_b1.is_empty() && !grp_b2.is_empty() { ui.add_space(ABILITY_Y); }

       render_list_view(ui, &grp_b2, sheet, assets, cat.id, level, curve, final_stats, settings, main_border);
       previous_content = true;
    }

    if !grp_footer.is_empty() {
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); }
        render_icon_row(ui, &grp_footer, sheet, settings, main_border, assets); 
    }
}

pub fn render_icon_row(
    ui: &mut egui::Ui, 
    items: &Vec<AbilityItem>, 
    sheet: &SpriteSheet, 
    settings: &Settings, 
    border_color: egui::Color32,
    assets: &CustomAssets,
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(ABILITY_X, ABILITY_Y);
        ui.horizontal_wrapped(|ui| {
            for item in items {
                let r = render_single_icon(ui, item, sheet, settings, border_color, assets);
                r.on_hover_ui(|ui| text_with_superscript(ui, &item.text));
            }
        });
    });
}

fn render_single_icon(
    ui: &mut egui::Ui, 
    item: &AbilityItem, 
    sheet: &SpriteSheet, 
    settings: &Settings, 
    border: egui::Color32,
    assets: &CustomAssets,
) -> egui::Response {
    let size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
    let force_fallback = settings.general.game_language == "--";

    let custom_texture = item.custom_icon.get_texture(assets);

    let response = if !force_fallback && custom_texture.is_some() {
        ui.add(egui::Image::new(egui::load::SizedTexture::new(custom_texture.unwrap().id(), size)))
    } else if !force_fallback && sheet.cuts_map.contains_key(&item.icon_id) {
        let cut = sheet.cuts_map.get(&item.icon_id).unwrap();
        if let Some(tex) = &sheet.texture_handle {
             ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates))
        } else {
             ui.allocate_response(size, egui::Sense::hover())
        }
    } else {
        let alt = crate::features::cat::registry::get_fallback_by_icon(item.icon_id);
        render_fallback_icon(ui, alt, border)
    };

    if !force_fallback {
        if let Some(border_id) = item.border_id {
            if let Some(b_cut) = sheet.cuts_map.get(&border_id) {
                if let Some(tex) = &sheet.texture_handle {
                    ui.put(response.rect, egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(b_cut.uv_coordinates));
                }
            }
        }
    }

    response
}

pub fn render_list_view(
    ui: &mut egui::Ui, 
    items: &Vec<AbilityItem>, 
    sheet: &SpriteSheet,
    assets: &CustomAssets,
    cat_id: u32,
    current_level: i32,
    curve: Option<&stats::CatLevelCurve>,
    s: &CatRaw,
    settings: &Settings, 
    border_color: egui::Color32,
) {
    for (i, item) in items.iter().enumerate() {
        let is_conjure = item.icon_id == img015::ICON_CONJURE;
        let id = egui::Id::new(format!("conjure_expand_{}", cat_id));
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0; 
            render_single_icon(ui, item, sheet, settings, border_color, assets); 

            if !is_conjure {
                text_with_superscript(ui, &item.text);
            } else {
                render_conjure_toggle(ui, &item.text, id, settings);
            }
        }); 

        let expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.cat_data.expand_spirit_details));
        if is_conjure && expanded {
            ui.add_space(ABILITY_Y);
            render_conjure_details(ui, s, current_level, curve, sheet, assets, settings);
        }
        
        if i < items.len() - 1 {
            ui.add_space(ABILITY_Y);
        }
    }
}

fn render_conjure_toggle(ui: &mut egui::Ui, text: &str, id: egui::Id, settings: &Settings) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 7.0;
        let mut expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.cat_data.expand_spirit_details));
        text_with_superscript(ui, text);
        let btn_text = egui::RichText::new("Details").size(11.0);
        let btn = if expanded {
            egui::Button::new(btn_text.color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(0, 100, 200))
        } else {
            egui::Button::new(btn_text)
        };

        if ui.add(btn).clicked() {
            expanded = !expanded;
            ui.data_mut(|d| d.insert_temp(id, expanded));
        }
    });
}

fn render_conjure_details(
    ui: &mut egui::Ui,
    parent_stats: &CatRaw,
    level: i32,
    curve: Option<&stats::CatLevelCurve>,
    sheet: &SpriteSheet,
    assets: &CustomAssets,
    settings: &Settings
) {
    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(220)) 
        .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 8.0, se: 8.0 }) 
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let spirit_border = egui::Color32::WHITE;
            
            let conjure_stats_vec = match stats::load_from_id(parent_stats.conjure_unit_id) {
                Some(s) => s,
                None => {
                    ui.label(egui::RichText::new("Spirit data not found").weak());
                    return;
                }
            };

            let conjure_stats = match conjure_stats_vec.first() {
                Some(s) => s,
                None => return,
            };

            let conjure_final = crate::features::cat::logic::stats::get_final_stats(
                conjure_stats, curve, level, None, None
            );

            let dmg = conjure_final.attack_1;
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let icon = img015::ICON_AREA_ATTACK;
                let size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
                let force_fallback = settings.general.game_language == "--";
                
                if !force_fallback && sheet.cuts_map.contains_key(&icon) {
                    let cut = sheet.cuts_map.get(&icon).unwrap();
                    if let Some(tex) = &sheet.texture_handle {
                         ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates));
                    }
                } else {
                    let alt = crate::features::cat::registry::get_fallback_by_icon(icon);
                    render_fallback_icon(ui, alt, spirit_border);
                }
                ui.label(format!("Damage: {}\nRange: {}", dmg, conjure_final.standing_range));
            });
            
            ui.add_space(ABILITY_Y);

            let (spirit_traits, spirit_head_1, spirit_head_2, spirit_body_1, spirit_body_2, spirit_footer) = abilities::collect_ability_data(
                &conjure_final, conjure_stats, level, curve, settings, true, None, None  
            );
            
            let mut prev = false;
            let mut last_was_trait = false;

            if !spirit_traits.is_empty() { 
                render_icon_row(ui, &spirit_traits, sheet, settings, spirit_border, assets); 
                prev = true;
                last_was_trait = true;
            }

            if !spirit_head_1.is_empty() { 
                if prev { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                render_icon_row(ui, &spirit_head_1, sheet, settings, spirit_border, assets); 
                prev = true;
            }

            if !spirit_head_2.is_empty() { 
                if prev { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                render_icon_row(ui, &spirit_head_2, sheet, settings, spirit_border, assets); 
                prev = true;
            }
            
            let has_body = !spirit_body_1.is_empty() || !spirit_body_2.is_empty();
            if has_body {
                if prev { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                render_list_view(ui, &spirit_body_1, sheet, assets, 0, level, curve, &conjure_final, settings, spirit_border);
                if !spirit_body_1.is_empty() && !spirit_body_2.is_empty() { ui.add_space(ABILITY_Y); }
                render_list_view(ui, &spirit_body_2, sheet, assets, 0, level, curve, &conjure_final, settings, spirit_border);
                prev = true;
            }
            
            if !spirit_footer.is_empty() {
                if prev { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); }
                render_icon_row(ui, &spirit_footer, sheet, settings, spirit_border, assets);
            }
        });
}