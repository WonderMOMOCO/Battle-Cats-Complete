use eframe::egui;
use std::collections::HashMap;
use std::path::Path;
use image::GenericImageView; 
use crate::data::cat::unitbuy::UnitBuyRow;
use crate::paths::global;

pub fn render(ui: &mut egui::Ui, description: &[String]) {
    ui.add_space(10.0);
    ui.vertical_centered(|ui| {
        ui.heading(egui::RichText::new("Description").size(20.0).strong());
    });
    ui.add_space(8.0);

    ui.vertical_centered(|ui| {
        if description.is_empty() {
            ui.label(egui::RichText::new("No description available").weak().italics());
            return;
        }

        for line in description {
            if line.trim().is_empty() {
                ui.label(" "); 
            } else {
                ui.add(egui::Label::new(egui::RichText::new(line).size(15.0)).wrap());
            }
        }
    });
}

pub fn render_evolve(
    ui: &mut egui::Ui, 
    ctx: &egui::Context,
    unit_buy: &UnitBuyRow, 
    evolution_text: &[String],
    current_form: usize,
    texture_cache: &mut HashMap<i32, Option<egui::TextureHandle>>,
    cache_version: u64,
) {
    ui.add_space(15.0);
    ui.separator(); 
    ui.add_space(10.0);

    let (materials, xp_cost) = match current_form {
        2 => (&unit_buy.true_form_materials, unit_buy.true_form_xp_cost),
        3 => (&unit_buy.ultra_form_materials, unit_buy.ultra_form_xp_cost),
        _ => return, 
    };

    let has_text = evolution_text.iter().any(|s| !s.trim().is_empty());
    let has_mats = !materials.is_empty();
    let has_xp = xp_cost > 0;

    if !has_mats && !has_text && !has_xp { return; }

    ui.vertical_centered(|ui| {
        ui.heading(egui::RichText::new("Evolve").size(20.0).strong());
    });
    ui.add_space(8.0);

    if has_text {
        ui.vertical_centered(|ui| {
            for line in evolution_text {
                if line.trim().is_empty() {
                    ui.label(" "); 
                } else {
                    ui.add(egui::Label::new(egui::RichText::new(line).size(15.0)).wrap());
                }
            }
        });
        ui.add_space(2.0);
    }

    // Materials
    if has_mats {
        let icon_size = 64.0;
        let spacing = 5.0;
        let count = materials.len() as f32;
        
        let total_width = (count * icon_size) + ((count - 1.0).max(0.0) * spacing);
        let available_width = ui.available_width();
        let left_padding = (available_width - total_width) / 2.0;

        ui.horizontal(|ui| {
            if left_padding > 0.0 {
                ui.add_space(left_padding);
            }

            ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
            
            for (item_id, amount) in materials {
                let texture_handle_opt = texture_cache.entry(*item_id).or_insert_with(|| {
                    load_material_icon_legacy(ctx, *item_id, cache_version)
                });

                let rect_size = egui::vec2(icon_size, icon_size);
                let (rect, _) = ui.allocate_exact_size(rect_size, egui::Sense::hover());

                if let Some(texture) = texture_handle_opt {
                    let mut mesh = egui::Mesh::with_texture(texture.id());
                    mesh.add_rect_with_uv(rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                    ui.painter().add(mesh);
                } else {
                    ui.painter().rect_filled(rect, egui::Rounding::same(4.0), egui::Color32::from_gray(50));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("ID {}", item_id),
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE,
                    );
                }

                // Amount
                let text = format!("×{}", amount);
                let font_id = egui::FontId::proportional(13.0);
                let text_color = egui::Color32::WHITE;
                
                let galley = ui.painter().layout_no_wrap(text, font_id, text_color);
                let padding = egui::vec2(4.0, 1.0); 
                let bg_size = galley.size() + padding * 2.0;
                
                let bg_rect = egui::Rect::from_min_size(
                    rect.max - bg_size, 
                    bg_size
                );

                ui.painter().rect_filled(
                    bg_rect, 
                    egui::Rounding::same(4.0), 
                    egui::Color32::from_black_alpha(160)
                );

                ui.painter().galley(bg_rect.min + padding, galley, egui::Color32::WHITE);
            }
        });
    }

    // XP
    if has_xp {
        ui.add_space(2.0); 

        let xp_horizontal_padding = 5.0;
        let xp_icon_height = 32.0;

        let xp_str = format!("{}", xp_cost);
        let font_id = egui::FontId::proportional(18.0);
        let text_galley = ctx.fonts(|f| {
            f.layout_no_wrap(xp_str.clone(), font_id.clone(), egui::Color32::WHITE)
        });
        let text_width = text_galley.size().x;
        let text_height = text_galley.size().y;

        let xp_icon_id = 6;
        let texture_handle_opt = texture_cache.entry(xp_icon_id).or_insert_with(|| {
            load_xp_icon_trimmed(ctx, xp_icon_id, cache_version)
        });

        let display_width = if let Some(tex) = texture_handle_opt {
             let tex_size = tex.size_vec2();
             let aspect = tex_size.x / tex_size.y;
             xp_icon_height * aspect
        } else {
             xp_icon_height 
        };
        
        let display_size = egui::vec2(display_width, xp_icon_height);

        let total_width = display_width + xp_horizontal_padding + text_width;
        let available_width = ui.available_width();
        let left_padding = (available_width - total_width) / 2.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = xp_horizontal_padding;
            
            if left_padding > 0.0 {
                ui.add_space(left_padding);
            }

            if let Some(texture) = texture_handle_opt {
                ui.image((texture.id(), display_size));
            } else {
                let (rect, _) = ui.allocate_exact_size(display_size, egui::Sense::hover());
                ui.painter().rect_filled(rect, egui::Rounding::same(2.0), egui::Color32::from_gray(50));
                 ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "XP",
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
            }
            
            let vertical_offset = (xp_icon_height - text_height) / 2.0;
            if vertical_offset > 0.0 {
                ui.vertical(|ui| {
                    ui.add_space(vertical_offset);
                    ui.label(egui::RichText::new(xp_str).strong().size(18.0));
                });
            } else {
                ui.label(egui::RichText::new(xp_str).strong().size(18.0));
            }
        });
    }
}

fn load_material_icon_legacy(ctx: &egui::Context, id: i32, version: u64) -> Option<egui::TextureHandle> {
    let path_opt = global::gatya_item_icon(Path::new(""), id);

    let mut final_image = egui::ColorImage::new([128, 128], egui::Color32::TRANSPARENT);
    let mut loaded = false;

    if let Some(path) = path_opt {
        if let Ok(mut img) = image::open(path) {
            let (w, h) = img.dimensions();
            
            let mut min_x = w;
            let mut min_y = h;
            let mut max_x = 0;
            let mut max_y = 0;
            let mut has_pixels = false;
            
            let rgba = img.to_rgba8();
            for (x, y, pixel) in rgba.enumerate_pixels() {
                if pixel[3] > 0 { 
                    if x < min_x { min_x = x; }
                    if x > max_x { max_x = x; }
                    if y < min_y { min_y = y; }
                    if y > max_y { max_y = y; }
                    has_pixels = true;
                }
            }

            if has_pixels {
                let crop_w = max_x - min_x + 1;
                let crop_h = max_y - min_y + 1;
                
                if crop_w > 128 || crop_h > 128 {
                    let sub_img = img.crop(min_x, min_y, crop_w, crop_h);
                    let resized = sub_img.resize(128, 128, image::imageops::FilterType::Lanczos3);
                    let (r_w, r_h) = resized.dimensions();
                    
                    let target_x = (128 - r_w) / 2;
                    let target_y = (128 - r_h) / 2;
                    
                    let r_rgba = resized.to_rgba8();
                    for y in 0..r_h {
                        for x in 0..r_w {
                            let src_pixel = r_rgba.get_pixel(x, y);
                            let color = egui::Color32::from_rgba_unmultiplied(
                                src_pixel[0], src_pixel[1], src_pixel[2], src_pixel[3]
                            );
                            final_image[( (target_x + x) as usize, (target_y + y) as usize )] = color;
                        }
                    }
                } else {
                    let target_x = (128 - crop_w) / 2;
                    let target_y = (128 - crop_h) / 2;

                    for y in 0..crop_h {
                        for x in 0..crop_w {
                            let src_pixel = rgba.get_pixel(min_x + x, min_y + y);
                            let color = egui::Color32::from_rgba_unmultiplied(
                                src_pixel[0], src_pixel[1], src_pixel[2], src_pixel[3]
                            );
                            if target_x + x < 128 && target_y + y < 128 {
                                final_image[( (target_x + x) as usize, (target_y + y) as usize )] = color;
                            }
                        }
                    }
                }
                loaded = true;
            }
        }
    }
    
    if loaded {
        Some(ctx.load_texture(format!("gatya_item_legacy_{}_{}", id, version), final_image, egui::TextureOptions::LINEAR))
    } else {
        None
    }
}

fn load_xp_icon_trimmed(ctx: &egui::Context, id: i32, version: u64) -> Option<egui::TextureHandle> {
    let path_opt = global::gatya_item_icon(Path::new(""), id);

    let mut final_image = egui::ColorImage::new([1, 1], egui::Color32::TRANSPARENT);
    let mut loaded = false;

    if let Some(path) = path_opt {
         if let Ok(img) = image::open(path) {
            let (w, h) = img.dimensions();
            let rgba = img.to_rgba8();
            
            let mut min_x = w;
            let mut min_y = h;
            let mut max_x = 0;
            let mut max_y = 0;
            let mut has_pixels = false;

            for (x, y, pixel) in rgba.enumerate_pixels() {
                if pixel[3] > 0 { 
                    if x < min_x { min_x = x; }
                    if x > max_x { max_x = x; }
                    if y < min_y { min_y = y; }
                    if y > max_y { max_y = y; }
                    has_pixels = true;
                }
            }

            if has_pixels {
                let crop_w = max_x - min_x + 1;
                let crop_h = max_y - min_y + 1;
                
                final_image = egui::ColorImage::new(
                    [crop_w as usize, crop_h as usize], 
                    egui::Color32::TRANSPARENT
                );

                for y in 0..crop_h {
                    for x in 0..crop_w {
                        let src_pixel = rgba.get_pixel(min_x + x, min_y + y);
                        let color = egui::Color32::from_rgba_unmultiplied(
                            src_pixel[0], src_pixel[1], src_pixel[2], src_pixel[3]
                        );
                        final_image[(x as usize, y as usize)] = color;
                    }
                }
                loaded = true;
            }
        }
    }
    
    if loaded {
        Some(ctx.load_texture(format!("gatya_item_trimmed_{}_{}", id, version), final_image, egui::TextureOptions::LINEAR))
    } else {
        None
    }
}