use eframe::egui;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::stats::{self, CatRaw};
use crate::features::cat::logic::abilities;
use crate::global::formats::imgcut::SpriteSheet;
use crate::features::settings::logic::Settings;
use crate::global::ui::shared::{render_fallback_icon, text_with_superscript};
use crate::features::cat::data::skillacquisition::TalentRaw;
use std::collections::HashMap;
use crate::global::game::abilities::{ABILITY_X, ABILITY_Y, TRAIT_Y};
use crate::global::game::abilities::{AbilityItem, CustomIcon};
use crate::global::assets::CustomAssets;
use crate::features::cat::registry::AbilityIcon;
use crate::global::game::img015;

pub fn render(
    ui: &mut egui::Ui, 
    final_stats: &CatRaw, 
    base_stats: &CatRaw,
    cat: &CatEntry, 
    level: i32,
    sheets: &[SpriteSheet], 
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
        render_icon_row(ui, &grp_trait, sheets, settings, main_border, assets);
        previous_content = true;
        last_was_trait = true;
    }

    if !grp_hl1.is_empty() { 
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
        render_icon_row(ui, &grp_hl1, sheets, settings, main_border, assets); 
        previous_content = true;
    }
    
    if !grp_hl2.is_empty() { 
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
        render_icon_row(ui, &grp_hl2, sheets, settings, main_border, assets); 
        previous_content = true;
    }

    let has_body = !grp_b1.is_empty() || !grp_b2.is_empty();
    if has_body {
       if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
       
       render_list_view(ui, &grp_b1, sheets, assets, cat.id, level, curve, final_stats, settings, main_border);
       
       if !grp_b1.is_empty() && !grp_b2.is_empty() { ui.add_space(ABILITY_Y); }

       render_list_view(ui, &grp_b2, sheets, assets, cat.id, level, curve, final_stats, settings, main_border);
       previous_content = true;
    }

    if !grp_footer.is_empty() {
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); }
        render_icon_row(ui, &grp_footer, sheets, settings, main_border, assets); 
    }
}

pub fn render_icon_row(
    ui: &mut egui::Ui, 
    items: &Vec<AbilityItem>, 
    sheets: &[SpriteSheet], 
    settings: &Settings, 
    border_color: egui::Color32,
    assets: &CustomAssets,
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(ABILITY_X, ABILITY_Y);
        ui.horizontal_wrapped(|ui| {
            for item in items {
                let r = render_single_icon(ui, item, sheets, settings, border_color, assets);
                r.on_hover_ui(|ui| text_with_superscript(ui, &item.text));
            }
        });
    });
}

fn render_single_icon(
    ui: &mut egui::Ui, 
    item: &AbilityItem, 
    sheets: &[SpriteSheet], 
    _settings: &Settings, 
    border: egui::Color32,
    assets: &CustomAssets,
) -> egui::Response {
    let size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);

    // Try Custom Icon first
    if let Some(tex) = item.custom_icon.get_texture(assets) {
        return ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)));
    }

    // Cascade through available language sheets for Standard Icons
    if let Some(icon_id) = item.icon_id {
        for sheet in sheets {
            if let Some(cut) = sheet.cuts_map.get(&icon_id) {
                if let Some(tex) = &sheet.texture_handle {
                     let response = ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates));
                     if let Some(border_id) = item.border_id {
                         if let Some(b_cut) = sheet.cuts_map.get(&border_id) {
                             ui.put(response.rect, egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(b_cut.uv_coordinates));
                         }
                     }
                     return response;
                } else if sheet.is_loading_active {
                     return ui.allocate_response(size, egui::Sense::hover());
                }
            }
        }
    }

    let icon_enum = if item.custom_icon != CustomIcon::None {
        AbilityIcon::Custom(item.custom_icon)
    } else {
        AbilityIcon::Standard(item.icon_id.unwrap_or(9999)) 
    };

    let alt = crate::features::cat::registry::get_fallback_by_icon(icon_enum);
    render_fallback_icon(ui, alt, border)
}

pub fn render_list_view(
    ui: &mut egui::Ui, 
    items: &Vec<AbilityItem>, 
    sheets: &[SpriteSheet],
    assets: &CustomAssets,
    cat_id: u32,
    current_level: i32,
    curve: Option<&stats::CatLevelCurve>,
    s: &CatRaw,
    settings: &Settings, 
    border_color: egui::Color32,
) {
    for (i, item) in items.iter().enumerate() {
        let is_conjure = item.icon_id == Some(img015::ICON_CONJURE) && item.custom_icon == CustomIcon::None;
        let id = egui::Id::new(format!("conjure_expand_{}", cat_id));
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0; 
            render_single_icon(ui, item, sheets, settings, border_color, assets); 

            if !is_conjure {
                text_with_superscript(ui, &item.text);
            } else {
                crate::features::cat::ui::conjure::render_conjure_toggle(ui, &item.text, id, settings);
            }
        }); 

        let expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.cat_data.expand_spirit_details));
        if is_conjure && expanded {
            ui.add_space(ABILITY_Y);
            crate::features::cat::ui::conjure::render_conjure_details(ui, s, current_level, curve, sheets, assets, settings);
        }
        
        if i < items.len() - 1 {
            ui.add_space(ABILITY_Y);
        }
    }
}