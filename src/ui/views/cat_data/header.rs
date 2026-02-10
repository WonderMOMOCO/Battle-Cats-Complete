use eframe::egui;
use std::path::Path;
use crate::core::cat::scanner::CatEntry;
use crate::core::cat::DetailTab;
use crate::core::settings::Settings;
use crate::core::utils::autocrop; 
use crate::ui::components::name_box;
use crate::paths::cat::{self, AssetType};

pub fn render(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    cat: &CatEntry,
    current_form: &mut usize,
    current_tab: &mut DetailTab,
    current_level: &mut i32,
    level_input: &mut String,
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    _settings: &Settings,
) {
    ui.vertical(|ui| {
        render_form_buttons(ui, cat, current_form, current_tab);
        ui.separator();
        ui.add_space(5.0);

        ui.horizontal_top(|ui| {
            render_cat_icon(ctx, ui, cat, *current_form, current_key, texture_cache);
            ui.add_space(3.0);
            render_info_box(ui, cat, *current_form, level_input, current_level);
        });
    });
}

fn render_form_buttons(ui: &mut egui::Ui, cat: &CatEntry, current_form: &mut usize, current_tab: &mut DetailTab) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0; 
        ui.horizontal(|ui| {
            let form_labels = ["Normal", "Evolved", "True", "Ultra"];
            
            for index in 0..4 {
                let exists = cat.forms.get(index).copied().unwrap_or(false);

                if exists { 
                    let is_selected = *current_form == index;
                    let (fill, stroke, text) = if is_selected {
                        (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                    } else {
                        (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                    };
                    
                    let btn = egui::Button::new(egui::RichText::new(form_labels[index]).color(text))
                        .fill(fill)
                        .stroke(stroke)
                        .rounding(egui::Rounding::ZERO)
                        .min_size(egui::vec2(60.0, 30.0));
                    
                    if ui.add(btn).clicked() { 
                        *current_form = index; 
                        
                        // Switch back to abilities if talents aren't available for this form
                        if index < 2 && *current_tab == DetailTab::Talents {
                            *current_tab = DetailTab::Abilities;
                        }
                    }
                } else {
                    ui.allocate_space(egui::vec2(60.0, 30.0)); 
                } 
            }

            ui.add(egui::Separator::default().vertical().spacing(20.0));

            let tabs = [
                (DetailTab::Abilities, "Abilities"),
                (DetailTab::Talents, "Talents"),
                (DetailTab::Details, "Details"),
                (DetailTab::Animation, "Animation"),
            ];

            for (tab_enum, label) in tabs {
                // Hide talents tab if not applicable
                if tab_enum == DetailTab::Talents && (*current_form < 2 || cat.talent_data.is_none()) {
                    continue;
                }

                let is_selected = *current_tab == tab_enum;
                let (fill, stroke, text) = if is_selected {
                    (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                } else {
                    (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                };

                let btn = egui::Button::new(egui::RichText::new(label).color(text))
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(egui::Rounding::from(5.0)) 
                    .min_size(egui::vec2(60.0, 30.0));

                if ui.add(btn).clicked() { *current_tab = tab_enum; }
            }
        });
    });
}

fn render_cat_icon(
    ctx: &egui::Context,
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    form: usize,
    current_key: &mut String,
    texture_cache: &mut Option<egui::TextureHandle>
) {
    let expected_path_str = if let Some(path) = cat::image(
        Path::new(cat::DIR_CATS),
        AssetType::Icon,
        cat.id,
        form,
        cat.egg_ids
    ) {
        path.to_string_lossy().to_string()
    } else {
        String::new() 
    };

    if *current_key != expected_path_str {
        *current_key = expected_path_str.clone();
        *texture_cache = if !expected_path_str.is_empty() {
             load_icon_texture(ctx, &expected_path_str)
        } else {
             None
        };
    }

    if let Some(tex) = texture_cache { 
        ui.image((tex.id(), tex.size_vec2())); 
    } else { 
        ui.allocate_space(egui::vec2(64.0, 64.0)); 
    }
}

fn load_icon_texture(ctx: &egui::Context, path_str: &str) -> Option<egui::TextureHandle> {
    let path = Path::new(path_str);
    let fallback = Path::new(cat::FALLBACK_ICON);
    
    let final_path = if path.exists() { path } else if fallback.exists() { fallback } else { return None };

    let img = image::open(final_path).ok()?;
    let mut rgba = autocrop(img.to_rgba8());
    
    if rgba.width() != 110 || rgba.height() != 85 {
        rgba = image::imageops::resize(&rgba, 110, 85, image::imageops::FilterType::Lanczos3);
    }
    
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.as_flat_samples();
    
    Some(ctx.load_texture("detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), egui::TextureOptions::LINEAR))
}

fn render_info_box(ui: &mut egui::Ui, cat: &CatEntry, form: usize, level_input: &mut String, current_level: &mut i32) {
    ui.vertical(|ui| {
        ui.set_width(name_box::NAME_BOX_WIDTH);

        let form_num = form + 1;
        let raw_name = cat.names.get(form).cloned().unwrap_or_default();
        
        let disp_name = if raw_name.is_empty() { 
            format!("{:03}-{}", cat.id, form_num) 
        } else { 
            raw_name 
        };

        ui.add_space(15.0); 
        name_box::render(ui, &disp_name);
        ui.spacing_mut().item_spacing.y = 0.0;
        
        ui.add_space(10.0);
        ui.label(egui::RichText::new(format!("ID: {:03}-{}", cat.id, form_num)).color(egui::Color32::from_gray(100)).size(12.0));
        ui.add_space(3.0);

        ui.horizontal(|ui| {
            ui.label("Level:");
            if ui.add(egui::TextEdit::singleline(level_input).desired_width(40.0)).changed() {
                let sum: i32 = level_input.split('+').filter_map(|s| s.trim().parse::<i32>().ok()).sum();
                *current_level = if sum <= 0 { 1 } else { sum };
            }
        });
    });
}