use eframe::egui;
use crate::core::definitions;
use crate::core::cat::scanner::CatEntry;
use crate::core::cat::stats::{self, CatRaw};
use crate::core::cat::abilities::{self, AbilityItem};
use crate::core::files::imgcut::SpriteSheet;
use crate::core::settings::Settings;
use crate::ui::components::shared::{render_fallback_icon, text_with_superscript};
use crate::core::files::img015;

pub fn render(
    ui: &mut egui::Ui, 
    s: &CatRaw, 
    cat: &CatEntry, 
    level: i32,
    sheet: &SpriteSheet, 
    multihit_tex: &Option<egui::TextureHandle>, 
    settings: &Settings, 
) {
    if render_traits(ui, s, sheet, settings) {
        ui.add_space(settings.trait_padding_y);
    }

    let curve = cat.curve.as_ref();
    let (grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = abilities::collect_ability_data(s, level, curve, multihit_tex, settings, false);
    
    let mut previous_content = false;
    let main_border = egui::Color32::BLACK;

    if !grp_hl1.is_empty() { 
        render_icon_row(ui, &grp_hl1, sheet, settings, main_border); 
        previous_content = true;
    }
    
    if !grp_hl2.is_empty() { 
        if previous_content { ui.add_space(settings.ability_padding_y); }
        render_icon_row(ui, &grp_hl2, sheet, settings, main_border); 
        previous_content = true;
    }

    let has_body = !grp_b1.is_empty() || !grp_b2.is_empty();
    if has_body {
       if previous_content { ui.add_space(settings.ability_padding_y); }
       
       render_list_view(ui, &grp_b1, sheet, multihit_tex, cat.id, level, curve, s, settings, main_border);
       render_list_view(ui, &grp_b2, sheet, multihit_tex, cat.id, level, curve, s, settings, main_border);
       previous_content = true;
    }

    if !grp_footer.is_empty() {
        if previous_content && !has_body { 
            ui.add_space(settings.ability_padding_y);
        }
        render_icon_row(ui, &grp_footer, sheet, settings, main_border); 
    }
}

fn render_traits(ui: &mut egui::Ui, s: &CatRaw, sheet: &SpriteSheet, settings: &Settings) -> bool {
    let has_any_trait = s.target_red > 0 || s.target_floating > 0 || s.target_black > 0 ||
        s.target_metal > 0 || s.target_angel > 0 || s.target_alien > 0 ||
        s.target_zombie > 0 || s.target_relic > 0 || s.target_aku > 0 ||
        s.target_traitless > 0;

    if !has_any_trait { return false; }

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(settings.ability_padding_x, settings.ability_padding_y);
        for &line_num in definitions::UI_TRAIT_ORDER {
            let has_trait = check_trait(s, line_num);
            if has_trait {
                let tooltip = get_trait_tooltip(line_num);
                let r = if let Some(sprite) = sheet.get_sprite_by_line(line_num) {
                    ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)))
                } else {
                    let alt = img015::img015_alt(line_num);
                    render_fallback_icon(ui, alt, egui::Color32::BLACK)
                };
                if !tooltip.is_empty() { r.on_hover_text(tooltip); }
            }
        }
    });
    
    ui.add_space(settings.ability_padding_y);
    true
}

fn check_trait(s: &CatRaw, line: usize) -> bool {
    match line {
        img015::ICON_TRAIT_RED => s.target_red > 0,
        img015::ICON_TRAIT_FLOATING => s.target_floating > 0,
        img015::ICON_TRAIT_BLACK => s.target_black > 0,
        img015::ICON_TRAIT_METAL => s.target_metal > 0,
        img015::ICON_TRAIT_ANGEL => s.target_angel > 0,
        img015::ICON_TRAIT_ALIEN => s.target_alien > 0,
        img015::ICON_TRAIT_ZOMBIE => s.target_zombie > 0,
        img015::ICON_TRAIT_RELIC => s.target_relic > 0,
        img015::ICON_TRAIT_AKU => s.target_aku > 0,
        img015::ICON_TRAIT_TRAITLESS => s.target_traitless > 0,
        _ => false,
    }
}

fn get_trait_tooltip(line: usize) -> &'static str {
    match line {
        img015::ICON_TRAIT_RED => "Targets Red Enemies",
        img015::ICON_TRAIT_FLOATING => "Targets Floating Enemies",
        img015::ICON_TRAIT_BLACK => "Targets Black Enemies",
        img015::ICON_TRAIT_METAL => "Targets Metal Enemies",
        img015::ICON_TRAIT_ANGEL => "Targets Angel Enemies",
        img015::ICON_TRAIT_ALIEN => "Targets Alien Enemies",
        img015::ICON_TRAIT_ZOMBIE => "Targets Zombie Enemies",
        img015::ICON_TRAIT_RELIC => "Targets Relic Enemies",
        img015::ICON_TRAIT_AKU => "Targets Aku Enemies",
        img015::ICON_TRAIT_TRAITLESS => "Targets Traitless Enemies",
        _ => "",
    }
}

pub fn render_icon_row(ui: &mut egui::Ui, items: &Vec<AbilityItem>, sheet: &SpriteSheet, settings: &Settings, border_color: egui::Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(settings.ability_padding_x, settings.ability_padding_y);
        for item in items {
            let r = render_single_icon(ui, item, sheet, border_color);
            r.on_hover_ui(|ui| text_with_superscript(ui, &item.text));
        }
    });
}

fn render_single_icon(ui: &mut egui::Ui, item: &AbilityItem, sheet: &SpriteSheet, border: egui::Color32) -> egui::Response {
    let size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
    if let Some(tex_id) = item.custom_tex {
        ui.add(egui::Image::new((tex_id, size)))
    } else if let Some(sprite) = sheet.get_sprite_by_line(item.icon_id) {
        ui.add(sprite.fit_to_exact_size(size))
    } else {
        let alt = img015::img015_alt(item.icon_id);
        render_fallback_icon(ui, alt, border)
    }
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
    border_color: egui::Color32,
) {
    for item in items {
        let is_conjure = item.icon_id == img015::ICON_CONJURE;
        let id = ui.make_persistent_id(format!("conjure_expand_{}", cat_id));
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            render_single_icon(ui, item, sheet, border_color); 

            if !is_conjure {
                text_with_superscript(ui, &item.text);
            } else {
                render_conjure_toggle(ui, &item.text, id, settings);
            }
        }); 

        let expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.expand_spirit_details));
        if is_conjure && expanded {
            ui.add_space(3.0);
            render_conjure_details(ui, s, current_level, curve, sheet, multihit_tex, settings);
        }
        
        ui.add_space(settings.ability_padding_y);
    }
}

fn render_conjure_toggle(ui: &mut egui::Ui, text: &str, id: egui::Id, settings: &Settings) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 7.0;
        
        let mut expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.expand_spirit_details));
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
    multihit_tex: &Option<egui::TextureHandle>,
    settings: &Settings
) {
    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(220)) 
        .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 8.0, se: 8.0 }) 
        .inner_margin(8.0)
        .show(ui, |ui| {
            let spirit_border = egui::Color32::WHITE;
            
            let conjure_stats = match stats::load_from_id(parent_stats.conjure_unit_id) {
                Some(s) => s,
                None => {
                    ui.label(egui::RichText::new("Spirit data not found").weak());
                    return;
                }
            };

            let dmg = curve.as_ref().map_or(
                conjure_stats.attack_1, 
                |c| c.calculate_stat(conjure_stats.attack_1, level)
            );
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let icon = img015::ICON_AREA_ATTACK;
                
                let size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
                if let Some(sprite) = sheet.get_sprite_by_line(icon) {
                    ui.add(sprite.fit_to_exact_size(size));
                } else {
                    let alt = img015::img015_alt(icon);
                    render_fallback_icon(ui, alt, spirit_border);
                }
                
                ui.label(format!("Damage: {}\nRange: {}", dmg, conjure_stats.standing_range));
            });
            
            ui.add_space(settings.ability_padding_y);

            let (c_hl1, c_hl2, c_b1, c_b2, c_ft) = abilities::collect_ability_data(&conjure_stats, level, curve, multihit_tex, settings, true);
            
            if !c_hl1.is_empty() { render_icon_row(ui, &c_hl1, sheet, settings, spirit_border); }
            if !c_hl2.is_empty() { render_icon_row(ui, &c_hl2, sheet, settings, spirit_border); }
            
            render_list_view(ui, &c_b1, sheet, multihit_tex, 0, level, curve, &conjure_stats, settings, spirit_border);
            render_list_view(ui, &c_b2, sheet, multihit_tex, 0, level, curve, &conjure_stats, settings, spirit_border);
            
            if !c_ft.is_empty() {
                render_icon_row(ui, &c_ft, sheet, settings, spirit_border);
            }
        });
}