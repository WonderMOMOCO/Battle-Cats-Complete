use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::logic::abilities;
use crate::global::formats::imgcut::SpriteSheet;
use crate::features::settings::logic::Settings;
use crate::global::ui::shared::{render_fallback_icon, text_with_superscript};
use crate::global::game::img015;
use crate::global::assets::CustomAssets;
use crate::global::game::abilities::{AbilityItem, CustomIcon};
use crate::features::enemy::registry;

pub const ABILITY_X: f32 = 3.0;
pub const ABILITY_Y: f32 = 5.0;
pub const TRAIT_Y: f32 = 7.0;

pub fn render(
    ui: &mut egui::Ui, 
    enemy: &EnemyEntry, 
    sheet: &SpriteSheet, 
    assets: &CustomAssets,
    settings: &Settings,
    magnification: i32,
) {
    ui.spacing_mut().item_spacing.y = 0.0;

    let (grp_trait, grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = abilities::collect_ability_data(
        &enemy.stats, settings, magnification,
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
       
       render_list_view(ui, &grp_b1, sheet, assets, settings, main_border);
       
       if !grp_b1.is_empty() && !grp_b2.is_empty() { ui.add_space(ABILITY_Y); }

       render_list_view(ui, &grp_b2, sheet, assets, settings, main_border);
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
    _settings: &Settings, 
    border: egui::Color32,
    assets: &CustomAssets,
) -> egui::Response {
    let size = egui::vec2(40.0, 40.0);

    let custom_texture = match item.icon_id {
        img015::ICON_DOJO => Some(&assets.dojo),
        img015::ICON_STARRED_ALIEN => Some(&assets.starred_alien),
        img015::ICON_BURROW => Some(&assets.burrow),
        img015::ICON_REVIVE => Some(&assets.revive),
        _ => match item.custom_icon {
            CustomIcon::Multihit => Some(&assets.multihit),
            CustomIcon::Kamikaze => Some(&assets.kamikaze),
            _ => None,
        }
    };

    let response = if custom_texture.is_some() {
        ui.add(egui::Image::new(egui::load::SizedTexture::new(custom_texture.unwrap().id(), size)))
    } else if sheet.cuts_map.contains_key(&item.icon_id) {
        let cut = sheet.cuts_map.get(&item.icon_id).unwrap();
        if let Some(tex) = &sheet.texture_handle {
             ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates))
        } else {
             ui.allocate_response(size, egui::Sense::hover())
        }
    } else {
        let alt = registry::get_fallback_by_icon(item.icon_id);
        render_fallback_icon(ui, alt, border)
    };

    if let Some(border_id) = item.border_id {
        if let Some(b_cut) = sheet.cuts_map.get(&border_id) {
            if let Some(tex) = &sheet.texture_handle {
                ui.put(response.rect, egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(b_cut.uv_coordinates));
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
    settings: &Settings, 
    border_color: egui::Color32,
) {
    for (i, item) in items.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0; 
            render_single_icon(ui, item, sheet, settings, border_color, assets); 
            text_with_superscript(ui, &item.text);
        }); 

        if i < items.len() - 1 {
            ui.add_space(ABILITY_Y);
        }
    }
}