use eframe::egui;

use crate::cat_data::scanner::CatEntry;
use crate::cat_data::sprites::SpriteSheet;
use crate::cat_data::stats; 
use crate::definitions; 
use crate::settings::Settings;

pub mod name;
pub mod grid;
pub mod block; 
pub mod utils;

use name::render_name_in_box;
use grid::{grid_cell, grid_cell_custom, render_frames};
use block::render_abilities; 
use utils::autocrop;

pub const NAME_BOX_WIDTH: f32 = 150.0;
pub const NAME_BOX_HEIGHT: f32 = 15.0;

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    current_form: &mut usize,
    level_input: &mut String,   
    current_level: &mut i32,    
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    sprite_sheet: &mut SpriteSheet,
    multihit_texture: &mut Option<egui::TextureHandle>,
    settings: &Settings,
) {
    let base_dir = std::path::Path::new("game/assets");
    
    if !settings.game_language.is_empty() {
        let tex_name = format!("img015/img015_{}.png", settings.game_language);
        let cut_name = format!("img015/img015_{}.imgcut", settings.game_language);
        
        let texture_path = base_dir.join(tex_name);
        let cut_path = base_dir.join(cut_name);
        
        sprite_sheet.load(ctx, &texture_path, &cut_path);
    }

    if multihit_texture.is_none() {
        const MULTIHIT_BYTES: &[u8] = include_bytes!("../../../assets/multihit.png");
        if let Ok(img) = image::load_from_memory(MULTIHIT_BYTES) {
            let rgba = img.to_rgba8();
            *multihit_texture = Some(ctx.load_texture(
                "multihit_icon",
                egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_flat_samples().as_slice()
                ),
                egui::TextureOptions::LINEAR
            ));
        }
    }

    let current_stats = cat.stats.get(*current_form).and_then(|opt| opt.as_ref());

    ui.vertical(|ui| {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            ui.horizontal(|ui| {
                let form_labels = ["Normal", "Evolved", "True", "Ultra"];
                for (index, &exists) in cat.forms.iter().enumerate() {
                    if exists {
                        let label = form_labels[index];
                        let is_selected = *current_form == index;
                        let (fill, stroke, text) = if is_selected {
                            (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                        } else {
                            (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                        };
                        let btn = egui::Button::new(egui::RichText::new(label).color(text))
                            .fill(fill).stroke(stroke).rounding(egui::Rounding::ZERO).min_size(egui::vec2(60.0, 30.0));
                        if ui.add(btn).clicked() { *current_form = index; }
                    }
                }
            });
        });

        ui.separator(); 
        ui.add_space(5.0);

        ui.horizontal_top(|ui| {
            ui.horizontal_top(|ui| {
                let form_char = match *current_form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };
                let expected = format!("game/cats/{:03}/{}/uni{:03}_{}00.png", cat.id, form_char, cat.id, form_char);

                if *current_key != expected {
                    *current_key = expected.clone(); 
                    *texture_cache = None; 
                    
                    let p = std::path::Path::new(&expected);
                    let f = std::path::Path::new("game/cats/uni.png");
                    
                    let path_to_load = if p.exists() { Some(p) } else if f.exists() { Some(f) } else { None };

                    if let Some(path) = path_to_load {
                        if let Ok(img) = image::open(path) {
                            let mut rgba = img.to_rgba8();
                            rgba = autocrop(rgba);
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let pixels = rgba.as_flat_samples();
                            *texture_cache = Some(ctx.load_texture("detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), egui::TextureOptions::LINEAR));
                        }
                    }
                }

                if let Some(tex) = texture_cache { 
                    ui.image(&*tex); 
                } else { 
                    ui.allocate_space(egui::vec2(64.0, 64.0)); 
                }

                ui.add_space(3.0);

                ui.vertical(|ui| {
                    ui.set_width(NAME_BOX_WIDTH);

                    let form_num = *current_form + 1;
                    let raw_name = cat.names.get(*current_form).cloned().unwrap_or_default();
                    let disp_name = if raw_name.is_empty() { format!("{:03}-{}", cat.id, form_num) } else { raw_name };

                    ui.add_space(15.0); 
                    render_name_in_box(ui, &disp_name);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(format!("ID: {:03}-{}", cat.id, form_num)).color(egui::Color32::from_gray(100)).size(12.0));
                    
                    ui.add_space(3.0);

                    ui.horizontal(|ui| {
                        ui.label("Level:");
                        let response = ui.add(egui::TextEdit::singleline(level_input).desired_width(40.0));
                        if response.changed() {
                            let mut sum = 0;
                            let parts = level_input.split('+');
                            for part in parts {
                                if let Ok(val) = part.trim().parse::<i32>() { sum += val; }
                            }
                            if sum <= 0 { *current_level = 1; } 
                            else { *current_level = sum; }
                        }
                    });
                });
            }); 

            ui.add_space(2.0);

            if let Some(s) = current_stats {
                let hp = cat.curve.as_ref().map_or(s.hitpoints, |c| c.calculate_stat(s.hitpoints, *current_level));
                let atk_1 = cat.curve.as_ref().map_or(s.attack_1, |c| c.calculate_stat(s.attack_1, *current_level));
                let atk_2 = cat.curve.as_ref().map_or(s.attack_2, |c| c.calculate_stat(s.attack_2, *current_level));
                let atk_3 = cat.curve.as_ref().map_or(s.attack_3, |c| c.calculate_stat(s.attack_3, *current_level));
                let total_atk = atk_1 + atk_2 + atk_3;
                let total_atk_cycle = s.attack_cycle(cat.atk_anim_frames[*current_form]);
                let dps = if total_atk_cycle > 0 { (total_atk as f32 * 30.0 / total_atk_cycle as f32) as i32 } else { 0 };
                let atk_type = if s.area_attack == 0 { "Single" } else { "Area" };

                let cell_width = 60.0;

                egui::Grid::new("stats_grid_right").min_col_width(cell_width).spacing([4.0, 4.0]).show(ui, |ui| {
                        grid_cell(ui, "Atk", true); grid_cell(ui, "Dps", true); grid_cell(ui, "Range", true); grid_cell(ui, "Atk Cycle", true); grid_cell(ui, "Atk Type", true); ui.end_row();
                        
                        grid_cell(ui, &format!("{}", total_atk), false); 
                        grid_cell(ui, &format!("{}", dps), false); 
                        grid_cell(ui, &format!("{}", s.standing_range), false);
                        
                        grid_cell_custom(ui, false, 
                            Some(Box::new(move |ui| {
                                ui.vertical_centered(|ui| render_frames(ui, total_atk_cycle, f32::INFINITY));
                            })), 
                            |ui| render_frames(ui, total_atk_cycle, cell_width)
                        ); 
                        
                        grid_cell(ui, atk_type, false); 
                        ui.end_row();

                        grid_cell(ui, "Hp", true); grid_cell(ui, "Kb", true); grid_cell(ui, "Speed", true); grid_cell(ui, "Cooldown", true); grid_cell(ui, "Cost", true); ui.end_row();
                        
                        grid_cell(ui, &format!("{}", hp), false); 
                        grid_cell(ui, &format!("{}", s.knockbacks), false); 
                        grid_cell(ui, &format!("{}", s.speed), false);
                        
                        let cd_val = s.effective_cooldown();
                        grid_cell_custom(ui, false, 
                            Some(Box::new(move |ui| {
                                ui.vertical_centered(|ui| render_frames(ui, cd_val, f32::INFINITY));
                            })), 
                            |ui| render_frames(ui, cd_val, cell_width)
                        ); 
                        
                        grid_cell(ui, &format!("{}¢", s.eoc1_cost * 3 / 2), false); 
                        ui.end_row();
                });
            }
        });

        ui.separator(); 
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false, false]) 
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            if let Some(s) = current_stats {
                
                let has_any_trait = 
                    s.target_red > 0 || s.target_floating > 0 || s.target_black > 0 ||
                    s.target_metal > 0 || s.target_angel > 0 || s.target_alien > 0 ||
                    s.target_zombie > 0 || s.target_relic > 0 || s.target_aku > 0 ||
                    s.target_traitless > 0;

                if has_any_trait {
                    ui.add_space(settings.ability_padding_y); 
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(settings.ability_padding_x, settings.ability_padding_y);
                        for &line_num in definitions::UI_TRAIT_ORDER {
                            let has_trait = match line_num {
                                definitions::ICON_TRAIT_RED => s.target_red > 0,
                                definitions::ICON_TRAIT_FLOATING => s.target_floating > 0,
                                definitions::ICON_TRAIT_BLACK => s.target_black > 0,
                                definitions::ICON_TRAIT_METAL => s.target_metal > 0,
                                definitions::ICON_TRAIT_ANGEL => s.target_angel > 0,
                                definitions::ICON_TRAIT_ALIEN => s.target_alien > 0,
                                definitions::ICON_TRAIT_ZOMBIE => s.target_zombie > 0,
                                definitions::ICON_TRAIT_RELIC => s.target_relic > 0,
                                definitions::ICON_TRAIT_AKU => s.target_aku > 0,
                                definitions::ICON_TRAIT_TRAITLESS => s.target_traitless > 0,
                                _ => false,
                            };
                            if has_trait {
                                if let Some(sprite) = sprite_sheet.get_sprite_by_line(line_num) {
                                    let r = ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
                                    let tooltip_text = match line_num {
                                        definitions::ICON_TRAIT_RED => "Targets Red Enemies",
                                        definitions::ICON_TRAIT_FLOATING => "Targets Floating Enemies",
                                        definitions::ICON_TRAIT_BLACK => "Targets Black Enemies",
                                        definitions::ICON_TRAIT_METAL => "Targets Metal Enemies",
                                        definitions::ICON_TRAIT_ANGEL => "Targets Angel Enemies",
                                        definitions::ICON_TRAIT_ALIEN => "Targets Alien Enemies",
                                        definitions::ICON_TRAIT_ZOMBIE => "Targets Zombie Enemies",
                                        definitions::ICON_TRAIT_RELIC => "Targets Relic Enemies",
                                        definitions::ICON_TRAIT_AKU => "Targets Aku Enemies",
                                        definitions::ICON_TRAIT_TRAITLESS => "Targets Traitless Enemies",
                                        _ => "",
                                    };
                                    if !tooltip_text.is_empty() { r.on_hover_text(tooltip_text); }
                                }
                            }
                        }
                    });
                }

                ui.add_space(settings.ability_padding_y);
                render_abilities(ui, s, sprite_sheet, multihit_texture, *current_level, cat.curve.as_ref(), cat.id, settings); 
                ui.add_space(20.0);
            }
        });
}