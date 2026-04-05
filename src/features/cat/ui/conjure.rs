use eframe::egui;
use crate::features::cat::logic::stats;
use crate::features::cat::logic::abilities;
use crate::global::formats::imgcut::SpriteSheet;
use crate::features::settings::logic::Settings;
use crate::global::ui::shared::{render_fallback_icon, text_with_superscript};
use crate::global::game::img015;
use crate::global::game::abilities::ABILITY_Y;
use crate::features::statblock::logic::builder::SpiritData;
use crate::features::cat::logic::context::CatRenderContext;

pub fn render_conjure_toggle(ui: &mut egui::Ui, text: &str, id: egui::Id, settings: &Settings) {
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

pub fn render_conjure_details(
    ui: &mut egui::Ui,
    ctx: &CatRenderContext,
    sheets: &[SpriteSheet],
) {
    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(220)) 
        .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 8.0, se: 8.0 }) 
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let spirit_border = egui::Color32::WHITE;
            
            let conjure_stats_vec = match stats::load_from_id(ctx.base_stats.conjure_unit_id, &ctx.global.settings.general.language_priority) {
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

            let conjure_final = stats::get_final_stats(
                conjure_stats, ctx.level_curve, ctx.current_level, None, None
            );

            let spirit_ctx = CatRenderContext {
                global: ctx.global,
                base_stats: conjure_stats,
                final_stats: &conjure_final,
                current_level: ctx.current_level,
                level_curve: ctx.level_curve,
                talent_data: None,
                talent_levels: None,
                is_conjure_unit: true,
            };

            let dmg = conjure_final.attack_1;
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let icon = img015::ICON_AREA_ATTACK;
                let size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
                
                let mut drawn = false;
                for sheet in sheets {
                    if let Some(cut) = sheet.cuts_map.get(&icon) {
                        if let Some(tex) = &sheet.texture_handle {
                             ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates));
                             drawn = true;
                             break;
                        }
                    }
                }
                if !drawn {
                    let alt = crate::features::cat::registry::get_fallback_by_icon(crate::features::cat::registry::AbilityIcon::Standard(icon));
                    render_fallback_icon(ui, alt, spirit_border);
                }
                ui.label(format!("Damage: {}\nRange: {}", dmg, conjure_final.standing_range));
            });
            
            ui.add_space(ABILITY_Y);

            let (spirit_traits, spirit_head_1, spirit_head_2, spirit_body_1, spirit_body_2, spirit_footer) = abilities::collect_ability_data(&spirit_ctx);
            
            let mut prev = false;
            let mut last_was_trait = false;

            if !spirit_traits.is_empty() { 
                crate::features::cat::ui::abilities::render_icon_row(ui, &spirit_traits, sheets, &spirit_ctx.global, spirit_border); 
                prev = true;
                last_was_trait = true;
            }

            if !spirit_head_1.is_empty() { 
                if prev { ui.add_space(if last_was_trait { crate::global::game::abilities::TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                crate::features::cat::ui::abilities::render_icon_row(ui, &spirit_head_1, sheets, &spirit_ctx.global, spirit_border); 
                prev = true;
            }

            if !spirit_head_2.is_empty() { 
                if prev { ui.add_space(if last_was_trait { crate::global::game::abilities::TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                crate::features::cat::ui::abilities::render_icon_row(ui, &spirit_head_2, sheets, &spirit_ctx.global, spirit_border); 
                prev = true;
            }
            
            let has_body = !spirit_body_1.is_empty() || !spirit_body_2.is_empty();
            if has_body {
                if prev { ui.add_space(if last_was_trait { crate::global::game::abilities::TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                crate::features::cat::ui::abilities::render_list_view(ui, &spirit_body_1, sheets, 0, &spirit_ctx, spirit_border);
                if !spirit_body_1.is_empty() && !spirit_body_2.is_empty() { ui.add_space(ABILITY_Y); }
                crate::features::cat::ui::abilities::render_list_view(ui, &spirit_body_2, sheets, 0, &spirit_ctx, spirit_border);
                prev = true;
            }
            
            if !spirit_footer.is_empty() {
                if prev { ui.add_space(if last_was_trait { crate::global::game::abilities::TRAIT_Y } else { ABILITY_Y }); }
                crate::features::cat::ui::abilities::render_icon_row(ui, &spirit_footer, sheets, &spirit_ctx.global, spirit_border);
            }
        });
}

pub fn build_spirit_data(
    ctx: &CatRenderContext
) -> Option<SpiritData> {
    if ctx.base_stats.conjure_unit_id > 0 {
        if let Some(c_vec) = stats::load_from_id(ctx.base_stats.conjure_unit_id, &ctx.global.settings.general.language_priority) {
            if let Some(c_stats) = c_vec.first() {
                let conjure_final = stats::get_final_stats(c_stats, ctx.level_curve, ctx.current_level, None, None);
                
                let spirit_ctx = CatRenderContext {
                    global: ctx.global,
                    base_stats: c_stats,
                    final_stats: &conjure_final,
                    current_level: ctx.current_level,
                    level_curve: ctx.level_curve,
                    talent_data: None,
                    talent_levels: None,
                    is_conjure_unit: true,
                };
                
                let (s_traits, s_h1, s_h2, s_b1, s_b2, s_footer) = abilities::collect_ability_data(&spirit_ctx);
                
                return Some(SpiritData {
                    dmg_text: format!("Damage: {}\nRange: {}", conjure_final.attack_1, conjure_final.standing_range),
                    traits: s_traits,
                    h1: s_h1,
                    h2: s_h2,
                    b1: s_b1,
                    b2: s_b2,
                    footer: s_footer,
                });
            }
        }
    }
    None
}