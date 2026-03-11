use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::logic::state::EnemyDetailTab;
use crate::ui::components::name_box;
use image::imageops;

pub const INPUT_SPACING: f32 = 4.0;

#[derive(PartialEq)]
pub enum ExportAction {
    None,
    Copy,
    Save,
}

pub fn render(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    enemy: &EnemyEntry, 
    current_tab: &mut EnemyDetailTab, 
    mag_input: &mut String, 
    magnification: &mut i32, 
    texture_cache: &mut Option<egui::TextureHandle>, 
    current_key: &mut String
) -> ExportAction {
    let mut export_action = ExportAction::None;

    ui.vertical(|ui| {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            ui.horizontal(|ui| {
                let tabs = [
                    (EnemyDetailTab::Abilities, "Abilities"), 
                    (EnemyDetailTab::Details, "Details"), 
                    (EnemyDetailTab::Animation, "Animation")
                ];

                for (tab_enum, label) in tabs {
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

        ui.separator();
        ui.add_space(5.0);

        ui.horizontal_top(|ui| {
            let container_size = egui::vec2(110.0, 85.0);
            let (rect, _) = ui.allocate_exact_size(container_size, egui::Sense::hover());
            
            let expected_path = enemy.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            if *current_key != expected_path {
                *current_key = expected_path.clone();
                *texture_cache = if !expected_path.is_empty() { load_icon_texture(ctx, &expected_path) } else { None };
            }

            if let Some(tex) = texture_cache {
                let icon_size = egui::vec2(85.0, 85.0);
                let x_off = (container_size.x - icon_size.x) / 2.0;
                let icon_rect = egui::Rect::from_min_size(rect.min + egui::vec2(x_off, 0.0), icon_size);
                ui.painter().image(tex.id(), icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            } else { 
                ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(30)); 
            }

            ui.add_space(3.0);

            ui.vertical(|ui| {
                ui.set_width(name_box::NAME_BOX_WIDTH);
                
                let disp_name = enemy.display_name();

                ui.add_space(15.0); 
                name_box::render(ui, &disp_name);
                ui.spacing_mut().item_spacing.y = 0.0;
                
                ui.add_space(10.0);
                ui.label(egui::RichText::new(format!("ID: {:03}-E", enemy.id)).color(egui::Color32::from_gray(100)).size(12.0));
                
                ui.add_space(3.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = INPUT_SPACING; 
                    ui.label("Magnification:");
                    if ui.add(egui::TextEdit::singleline(mag_input).desired_width(40.0)).changed() {
                        *magnification = mag_input.trim().parse::<i32>().unwrap_or(100);
                    }
                    ui.label("%");
                });
            });

            if *current_tab == EnemyDetailTab::Abilities {
                ui.add_space(15.0);
                let separator_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 85.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, separator_color);
                ui.add_space(15.0);

                ui.vertical(|ui| {
                    let btn_h = 24.0;
                    let btn_w = 100.0;
                    let gap = 6.0;
                    
                    ui.add_space(15.5);
                    ui.spacing_mut().item_spacing.y = gap;
                    
                    let current_time = ui.input(|i| i.time);
                    
                    let is_copying = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("is_copying"))).unwrap_or(false);
                    let copy_time = ctx.data(|d| d.get_temp::<f64>(egui::Id::new("export_copy_time"))).unwrap_or(-10.0);
                    let copy_res = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("export_copy_res"))).unwrap_or(false);
                    let in_copy_cooldown = (current_time - copy_time) < 2.0;

                    let is_exporting = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("is_exporting"))).unwrap_or(false);
                    let save_time = ctx.data(|d| d.get_temp::<f64>(egui::Id::new("export_save_time"))).unwrap_or(-10.0);
                    let save_res = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("export_save_res"))).unwrap_or(false);
                    let in_save_cooldown = (current_time - save_time) < 2.0;

                    let default_color = egui::Color32::from_rgb(31, 106, 165);
                    let success_color = egui::Color32::from_rgb(40, 160, 60);
                    let fail_color = egui::Color32::from_rgb(200, 40, 40);
                    let processing_color = egui::Color32::from_rgb(200, 160, 0); 

                    let (copy_text, copy_color) = if is_copying {
                        ("Copying...", processing_color)
                    } else if in_copy_cooldown {
                        if copy_res { ("Copied!", success_color) } else { ("Failed!", fail_color) }
                    } else {
                        ("Copy Image", default_color)
                    };

                    let btn_copy = egui::Button::new(egui::RichText::new(copy_text).size(12.0).strong().color(egui::Color32::WHITE))
                        .fill(copy_color)
                        .rounding(4.0);
                    
                    if ui.add_sized([btn_w, btn_h], btn_copy).on_hover_text("Generate a statblock image and copy it to your clipboard!").clicked() {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new("is_copying"), true));
                        export_action = ExportAction::Copy;
                    }

                    let (save_text, save_color) = if is_exporting {
                        ("Exporting...", processing_color)
                    } else if in_save_cooldown {
                        if save_res { ("Exported!", success_color) } else { ("Failed!", fail_color) }
                    } else {
                        ("Export Image", default_color)
                    };

                    let btn_save = egui::Button::new(egui::RichText::new(save_text).size(12.0).strong().color(egui::Color32::WHITE))
                        .fill(save_color)
                        .rounding(4.0);
                    
                    if ui.add_sized([btn_w, btn_h], btn_save).on_hover_text("Save a statblock image to the exports folder!").clicked() {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new("is_exporting"), true));
                        export_action = ExportAction::Save;
                    }
                });
            }
        });
    });

    export_action
}

fn load_icon_texture(ctx: &egui::Context, path_str: &str) -> Option<egui::TextureHandle> {
    let path = std::path::Path::new(path_str);
    let img = image::open(path).ok()?;
    let rgba = imageops::resize(&img.to_rgba8(), 85, 85, imageops::FilterType::Lanczos3);
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ctx.load_texture("enemy_detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_flat_samples().as_slice()), egui::TextureOptions::LINEAR))
}