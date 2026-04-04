use eframe::egui;
use std::collections::HashSet;

use crate::global::formats::imgcut::SpriteSheet;
use crate::global::assets::CustomAssets;
use crate::global::game::abilities::UI_TRAIT_ORDER;
use crate::global::ui::shared::DragGuard;
use crate::features::cat::registry::{CAT_ABILITY_REGISTRY, DisplayGroup, AbilityIcon};
use crate::global::game::img015;
use crate::features::settings::logic::Settings;

pub use crate::features::cat::logic::filter::{CatFilterState, MatchMode, TalentFilterMode};
use crate::features::cat::logic::filter::{get_icon_name, ATTACK_TYPE_ICONS};

pub const WINDOW_WIDTH: f32 = 500.0;
pub const WINDOW_HEIGHT: f32 = 580.0;
pub const TILDE_SPACING: f32 = 5.0; 
pub const BTN_SIZE_RARITY: [f32; 2] = [77.0, 24.0];
pub const BTN_SIZE_FORM: [f32; 2] = [118.0, 24.0];

pub fn show_popup(
    ctx: &egui::Context,
    state: &mut CatFilterState,
    sheets: &mut Vec<SpriteSheet>,
    assets: &CustomAssets,
    settings: &Settings,
    drag_guard: &mut DragGuard,
) {
    if !state.is_open { return; }
    
    img015::ensure_loaded(ctx, sheets, settings);

    let window_id = egui::Id::new("Cat Filter");
    let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);
    
    let mut clear_filters = false;
    let mut is_open_local = state.is_open;
    
    let mut window = egui::Window::new("Advanced Cat Filter")
        .id(window_id)
        .open(&mut is_open_local)
        .collapsible(false)
        .resizable(false)
        .constrain(false)
        .movable(allow_drag)
        .default_pos(ctx.screen_rect().center() - egui::vec2(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0))
        .fixed_size([WINDOW_WIDTH, WINDOW_HEIGHT]);
        
    if let Some(pos) = fixed_pos { window = window.current_pos(pos); }
    
    window.show(ctx, |ui| {
        let max_rect = ui.max_rect(); 
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            
            ui.set_min_width(WINDOW_WIDTH - 20.0);
            
            ui.heading("Attributes");
            ui.add_space(5.0);
            
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                filter_button(ui, &mut state.rarities[0], "Normal", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[1], "Special", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[2], "Rare", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[3], "Super Rare", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[4], "Uber Rare", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[5], "Legend Rare", BTN_SIZE_RARITY);
            });
            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                filter_button(ui, &mut state.forms[0], "Normal Form", BTN_SIZE_FORM);
                filter_button(ui, &mut state.forms[1], "Evolved Form", BTN_SIZE_FORM);
                filter_button(ui, &mut state.forms[2], "True Form", BTN_SIZE_FORM);
                filter_button(ui, &mut state.forms[3], "Ultra Form", BTN_SIZE_FORM);
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(16.0, 4.0);
                
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new("Mode:").strong());
                    
                    egui::ComboBox::from_id_salt("cb_match_mode")
                        .selected_text(if state.match_mode == MatchMode::And { "And" } else { "Or" })
                        .width(55.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.match_mode, MatchMode::And, "And");
                            ui.selectable_value(&mut state.match_mode, MatchMode::Or, "Or");
                        });
                });

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0; 
                    ui.label(egui::RichText::new("Talents:").strong());
                    
                    ui.scope(|ui| {
                        if state.talent_mode == TalentFilterMode::Only {
                            let active_blue = egui::Color32::from_rgb(31, 106, 165);
                            let visuals = ui.visuals_mut();
                            visuals.widgets.inactive.bg_fill = active_blue;
                            visuals.widgets.hovered.bg_fill = active_blue;
                            visuals.widgets.active.bg_fill = active_blue;
                            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                        }
                        
                        egui::ComboBox::from_id_salt("cb_talent_mode")
                            .selected_text(state.talent_mode.label())
                            .width(85.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut state.talent_mode, TalentFilterMode::Ignore, "Ignore");
                                ui.selectable_value(&mut state.talent_mode, TalentFilterMode::Consider, "Consider");
                                ui.selectable_value(&mut state.talent_mode, TalentFilterMode::Only, "Only");
                            });
                    });
                });

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0; 
                    ui.label(egui::RichText::new("Ultra Talents:").strong());
                    
                    ui.scope(|ui| {
                        if state.ultra_talent_mode == TalentFilterMode::Only {
                            let active_blue = egui::Color32::from_rgb(31, 106, 165);
                            let visuals = ui.visuals_mut();
                            visuals.widgets.inactive.bg_fill = active_blue;
                            visuals.widgets.hovered.bg_fill = active_blue;
                            visuals.widgets.active.bg_fill = active_blue;
                            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                        }
                        
                        egui::ComboBox::from_id_salt("cb_ultra_talent_mode")
                            .selected_text(state.ultra_talent_mode.label())
                            .width(85.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut state.ultra_talent_mode, TalentFilterMode::Ignore, "Ignore");
                                ui.selectable_value(&mut state.ultra_talent_mode, TalentFilterMode::Consider, "Consider");
                                ui.selectable_value(&mut state.ultra_talent_mode, TalentFilterMode::Only, "Only");
                            });
                    });
                });
            });
            ui.add_space(15.0);

            // STATS SECTION
            ui.heading("Stats");
            ui.add_space(5.0);
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(egui::RichText::new("Target Level:").strong());
                ui.add_sized(
                    egui::vec2(45.0, 20.0), 
                    egui::TextEdit::singleline(&mut state.level_input)
                        .hint_text(egui::RichText::new("50").color(egui::Color32::from_gray(100)))
                );
            });
            ui.add_space(8.0);

            let stat_keys = ["Attack", "Dps", "Range", "Atk Cycle (f)", "Hitpoints", "Knockbacks", "Speed", "Cooldown (f)", "Cost"];
            
            egui::Grid::new("stat_filter_grid")
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    for (i, &stat) in stat_keys.iter().enumerate() {
                        ui.label(format!("{}:", stat));
                        
                        let range = state.stat_ranges.entry(stat).or_default();
                        
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = TILDE_SPACING;

                            let hint = egui::RichText::new("Any").color(egui::Color32::from_gray(100));
                            
                            ui.add_sized(
                                egui::vec2(45.0, 20.0), 
                                egui::TextEdit::singleline(&mut range.min).hint_text(hint.clone())
                            );
                            
                            ui.label("~");
                            
                            ui.add_sized(
                                egui::vec2(45.0, 20.0), 
                                egui::TextEdit::singleline(&mut range.max).hint_text(hint)
                            );
                        });
                        
                        if (i + 1) % 2 == 0 {
                            ui.end_row();
                        }
                    }
                });
            ui.add_space(15.0);

            ui.heading("Target Traits");
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for &icon_id in UI_TRAIT_ORDER {
                    render_filter_icon(ui, &AbilityIcon::Standard(icon_id), &mut state.active_icons, sheets, assets);
                }
            });
            ui.add_space(15.0);

            ui.heading("Attack Type");
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for icon in ATTACK_TYPE_ICONS {
                    render_filter_icon(ui, icon, &mut state.active_icons, sheets, assets);
                }
            });
            ui.add_space(15.0);

            ui.heading("Abilities");
            ui.add_space(5.0);

            let mut rendered_icons = HashSet::new();

            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Headline1, false, true, sheets, assets);
            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Headline2, false, true, sheets, assets);
            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Body1, true, true, sheets, assets); 
            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Body2, true, true, sheets, assets); 
            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Footer, false, true, sheets, assets);

            let check_talents = state.talent_mode != TalentFilterMode::Ignore || state.ultra_talent_mode != TalentFilterMode::Ignore;
            if check_talents {
                let mut talent_icons = Vec::new();
                for def in CAT_ABILITY_REGISTRY.iter() {
                    let is_trait = if let AbilityIcon::Standard(id) = def.icon { UI_TRAIT_ORDER.contains(&id) } else { false };
                    
                    if !rendered_icons.contains(&def.icon) && !is_trait && !ATTACK_TYPE_ICONS.contains(&def.icon) {
                        if !talent_icons.contains(&def.icon) {
                            talent_icons.push(def.icon.clone());
                        }
                    }
                }

                if !talent_icons.is_empty() {
                    ui.add_space(2.0);
                    ui.heading("Talents");
                    ui.add_space(5.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                        for icon in talent_icons {
                            render_filter_icon(ui, &icon, &mut state.active_icons, sheets, assets);
                        }
                    });
                }
            }
            
            ui.add_space(50.0); 
        });
        
        let btn_size = egui::vec2(160.0, 34.0);
        let btn_rect = egui::Rect::from_center_size(
            max_rect.center_bottom() - egui::vec2(0.0, btn_size.y / 2.0 + 12.0),
            btn_size
        );
        
        let clear_btn = egui::Button::new(
            egui::RichText::new("Clear Filter").color(egui::Color32::WHITE).strong().size(15.0)
        )
        .fill(egui::Color32::from_rgb(210, 50, 50)) 
        .rounding(6.0);
        
        if ui.put(btn_rect, clear_btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            clear_filters = true;
        }
    });
    
    state.is_open = is_open_local;

    if clear_filters {
        *state = CatFilterState { is_open: state.is_open, ..Default::default() };
    }
}

fn render_display_group(
    ui: &mut egui::Ui,
    state: &mut CatFilterState,
    rendered_icons: &mut HashSet<AbilityIcon>,
    target_group: DisplayGroup,
    is_vertical: bool,
    draw_labels: bool,
    sheets: &[SpriteSheet],
    assets: &CustomAssets,
) {
    let mut icons_in_group = Vec::new();
    
    for def in CAT_ABILITY_REGISTRY.iter() {
        let is_trait = if let AbilityIcon::Standard(id) = def.icon { UI_TRAIT_ORDER.contains(&id) } else { false };

        if def.group == target_group && !is_trait && !ATTACK_TYPE_ICONS.contains(&def.icon) {
            if !icons_in_group.contains(&def.icon) {
                icons_in_group.push(def.icon.clone());
                rendered_icons.insert(def.icon.clone());
            }
        }
    }
    
    if target_group == DisplayGroup::Headline2 {
        let conjure = AbilityIcon::Standard(img015::ICON_CONJURE);
        let kamikaze = AbilityIcon::Custom(crate::global::game::abilities::CustomIcon::Kamikaze);

        if !icons_in_group.contains(&conjure) { 
            icons_in_group.insert(0, conjure.clone()); 
            rendered_icons.insert(conjure);
        }
        if !icons_in_group.contains(&kamikaze) { 
            icons_in_group.push(kamikaze.clone()); 
            rendered_icons.insert(kamikaze);
        }
    }
    
    if !icons_in_group.is_empty() {
        if is_vertical {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
                for icon in icons_in_group {
                    render_filter_icon_row(ui, state, &icon, draw_labels, sheets, assets);
                }
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for icon in icons_in_group {
                    render_filter_icon(ui, &icon, &mut state.active_icons, sheets, assets);
                }
            });
        }
        ui.add_space(8.0); 
    }
}

fn filter_button(ui: &mut egui::Ui, active: &mut bool, label: &str, size: [f32; 2]) -> egui::Response {
    let mut btn = egui::Button::new(label);
    if *active {
        btn = btn.fill(egui::Color32::from_rgb(31, 106, 165));
    }
    let response = ui.add_sized(size, btn);
    if response.clicked() {
        *active = !*active;
    }
    response
}

fn render_filter_icon_row(
    ui: &mut egui::Ui, 
    state: &mut CatFilterState,
    icon: &AbilityIcon, 
    draw_labels: bool,
    sheets: &[SpriteSheet],
    assets: &CustomAssets,
) {
    let is_active = state.active_icons.contains(icon);
    let name = get_icon_name(icon);
    
    let ability_def = CAT_ABILITY_REGISTRY.iter().find(|d| &d.icon == icon);
    let schema = ability_def.map(|d| d.schema).unwrap_or(&[]);
    let has_adv = !schema.is_empty();

    let bg_fill = if is_active && has_adv { egui::Color32::from_black_alpha(150) } else { egui::Color32::TRANSPARENT };
    let margin = if is_active && has_adv { egui::Margin::symmetric(8.0, 8.0) } else { egui::Margin::same(0.0) };

    egui::Frame::none()
        .fill(bg_fill)
        .rounding(6.0)
        .inner_margin(margin)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    render_filter_icon(ui, icon, &mut state.active_icons, sheets, assets);
                    
                    if draw_labels {
                        ui.add_space(10.0); 
                        let color = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(120) };
                        if ui.add(egui::Label::new(egui::RichText::new(&name).color(color)).sense(egui::Sense::click())).clicked() {
                            if is_active { state.active_icons.remove(icon); } 
                            else { state.active_icons.insert(icon.clone()); }
                        }
                    }
                });

                if is_active && has_adv {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(3.0); 
                        
                        egui::Grid::new(format!("adv_grid_{}", name))
                            .spacing([8.0, 6.0]) 
                            .show(ui, |ui| {
                                for &(attr, _) in schema {
                                    ui.label(format!("{}:", attr));
                                    
                                    let range = state.adv_ranges
                                        .entry(icon.clone())
                                        .or_default()
                                        .entry(attr)
                                        .or_default();
                                    
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = TILDE_SPACING;

                                        let hint = egui::RichText::new("Any").color(egui::Color32::from_gray(100));
                                        
                                        ui.add_sized(
                                            egui::vec2(45.0, 20.0), 
                                            egui::TextEdit::singleline(&mut range.min).hint_text(hint.clone())
                                        );
                                        
                                        ui.label("~");
                                        
                                        ui.add_sized(
                                            egui::vec2(45.0, 20.0), 
                                            egui::TextEdit::singleline(&mut range.max).hint_text(hint)
                                        );
                                    });
                                    ui.end_row();
                                }
                            });
                    });
                }
            });
        });
}

fn render_filter_icon(
    ui: &mut egui::Ui, 
    icon: &AbilityIcon, 
    active_icons: &mut HashSet<AbilityIcon>,
    sheets: &[SpriteSheet],
    assets: &crate::global::assets::CustomAssets,
) {
    let is_active = active_icons.contains(icon);
    let tint = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(80) };
    
    let mut drawn = false;
    
    match icon {
        AbilityIcon::Custom(custom_variant) => {
            if let Some(tex) = custom_variant.get_texture(assets) {
                let img = egui::Image::new(tex).fit_to_exact_size(egui::vec2(32.0, 32.0)).tint(tint);
                let response = ui.add(egui::ImageButton::new(img).frame(false));
                if response.clicked() {
                    if is_active { active_icons.remove(icon); } 
                    else { active_icons.insert(icon.clone()); }
                }
                response.on_hover_text(get_icon_name(icon));
                drawn = true;
            }
        },
        AbilityIcon::Standard(icon_id) => {
            for sheet in sheets {
                if let Some(cut) = sheet.cuts_map.get(icon_id) {
                    if let Some(tex) = &sheet.texture_handle {
                        let img = egui::Image::new(egui::load::SizedTexture::new(tex.id(), egui::vec2(32.0, 32.0)))
                            .uv(cut.uv_coordinates)
                            .tint(tint);
                        let response = ui.add(egui::ImageButton::new(img).frame(false));
                        if response.clicked() {
                            if is_active { active_icons.remove(icon); } 
                            else { active_icons.insert(icon.clone()); }
                        }
                        response.on_hover_text(get_icon_name(icon));
                        drawn = true;
                        break;
                    }
                }
            }
        }
    }

    if !drawn {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
        if ui.is_rect_visible(rect) {
            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_black_alpha(100));
            let text_color = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(100) };
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(20.0), text_color);
        }
        if response.clicked() {
            if is_active { active_icons.remove(icon); } 
            else { active_icons.insert(icon.clone()); }
        }
    }
}