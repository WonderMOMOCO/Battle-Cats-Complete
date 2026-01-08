use eframe::egui;
use std::path::Path;
use crate::core::cat::scanner::CatEntry;
use crate::core::settings::Settings;
use crate::core::utils::autocrop; 
use crate::ui::components::name_box;

pub fn render(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    cat: &CatEntry,
    current_form: &mut usize,
    current_level: &mut i32,
    level_input: &mut String,
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    _settings: &Settings,
) {
    ui.vertical(|ui| {
        render_form_buttons(ui, cat, current_form);
        ui.separator();
        ui.add_space(5.0);

        ui.horizontal_top(|ui| {
            render_cat_icon(ctx, ui, cat, *current_form, current_key, texture_cache);
            ui.add_space(3.0);
            render_info_box(ui, cat, *current_form, level_input, current_level);
        });
    });
}

fn render_form_buttons(ui: &mut egui::Ui, cat: &CatEntry, current_form: &mut usize) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0; 
        ui.horizontal(|ui| {
            let form_labels = ["Normal", "Evolved", "True", "Ultra"];
            for (index, &exists) in cat.forms.iter().enumerate() {
                if !exists { continue; } 
                
                let is_selected = *current_form == index;
                let (fill, stroke, text) = if is_selected {
                    (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                } else {
                    (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                };
                
                let btn = egui::Button::new(egui::RichText::new(form_labels[index]).color(text))
                    .fill(fill).stroke(stroke).rounding(egui::Rounding::ZERO).min_size(egui::vec2(60.0, 30.0));
                
                if ui.add(btn).clicked() { *current_form = index; }
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
    let expected_path = get_icon_path(cat, form);

    if *current_key != expected_path {
        *current_key = expected_path.clone();
        *texture_cache = load_icon_texture(ctx, &expected_path);
    }

    if let Some(tex) = texture_cache { 
        ui.image(&*tex); 
    } else { 
        ui.allocate_space(egui::vec2(64.0, 64.0)); 
    }
}

fn get_icon_path(cat: &CatEntry, form: usize) -> String {
    let (egg_norm, egg_evol) = cat.egg_ids;
    if form == 0 && egg_norm != -1 {
        return format!("game/cats/egg_{:03}/f/uni{:03}_m00.png", egg_norm, egg_norm);
    }
    if form == 1 && egg_evol != -1 {
        return format!("game/cats/egg_{:03}/c/uni{:03}_m01.png", egg_evol, egg_evol);
    }
    
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };
    format!("game/cats/{:03}/{}/uni{:03}_{}00.png", cat.id, form_char, cat.id, form_char)
}

fn load_icon_texture(ctx: &egui::Context, path_str: &str) -> Option<egui::TextureHandle> {
    let path = Path::new(path_str);
    let fallback = Path::new("game/cats/uni.png");
    
    let final_path = if path.exists() { path } else if fallback.exists() { fallback } else { return None };

    let img = image::open(final_path).ok()?;
    let rgba = autocrop(img.to_rgba8());
    
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.as_flat_samples();
    
    Some(ctx.load_texture("detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), egui::TextureOptions::LINEAR))
}

fn render_info_box(ui: &mut egui::Ui, cat: &CatEntry, form: usize, level_input: &mut String, current_level: &mut i32) {
    ui.vertical(|ui| {
        ui.set_width(name_box::NAME_BOX_WIDTH);

        let form_num = form + 1;
        let raw_name = cat.names.get(form).cloned().unwrap_or_default();
        
        let prev_name = if form > 0 { cat.names.get(form - 1).map(|s| s.as_str()).unwrap_or("") } else { "" };
        let is_placeholder = !raw_name.is_empty() && raw_name == prev_name;

        let disp_name = if raw_name.is_empty() || is_placeholder { 
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