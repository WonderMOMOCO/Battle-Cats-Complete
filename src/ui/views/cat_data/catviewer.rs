use eframe::egui;
use std::path::{Path, PathBuf};

use crate::core::cat::scanner::CatEntry;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::ui::components::anim::viewer::AnimViewer;
use crate::core::settings::Settings;
use crate::paths::cat::{self, AnimType};
use crate::ui::components::anim::controls::{
    IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_SPIRIT, IDX_MODEL, IDX_BURROW, IDX_SURFACE, IDX_NONE
};

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cat_entry: &CatEntry,
    current_form: usize,
    anim_viewer: &mut AnimViewer,
    model_data: &mut Option<Model>,
    anim_sheet: &mut SpriteSheet,
    settings: &Settings,
) {
    let root = Path::new(cat::DIR_CATS);
    let egg_ids = cat_entry.egg_ids;
    
    // Calculate Availabilty
    let mut available_anims = Vec::new();
    let anim_defs = [
        (IDX_WALK, "Walk"), 
        (IDX_IDLE, "Idle"), 
        (IDX_ATTACK, "Attack"), 
        (IDX_KB, "Knockback"), 
        (IDX_BURROW, "Burrow"), 
        (IDX_SURFACE, "Surface")
    ];
    
    for (idx, label) in anim_defs {
        let path = cat::maanim(root, cat_entry.id, current_form, egg_ids, idx);
        if path.exists() { available_anims.push((idx, label, path)); }
    }

    let std_png = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Png);
    let std_cut = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Imgcut);
    let std_model = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Mamodel);
    let base_assets_available = std_png.exists() && std_cut.exists() && std_model.exists();

    // Spirit Logic
    let conjure_id = if let Some(Some(stats)) = cat_entry.stats.get(current_form) {
        if stats.conjure_unit_id > 0 { Some(stats.conjure_unit_id as u32) } else { None }
    } else { None };

    let mut spirit_pack = None;
    let mut spirit_available = false;
    let spirit_sheet_id = if let Some(s_id) = conjure_id { format!("spirit_{}_{}", s_id, anim_viewer.texture_version) } else { String::new() };

    if let Some(s_id) = conjure_id {
        let s_png = cat::anim(root, s_id, 0, (-1, -1), AnimType::Png);
        let s_cut = cat::anim(root, s_id, 0, (-1, -1), AnimType::Imgcut);
        let s_model = cat::anim(root, s_id, 0, (-1, -1), AnimType::Mamodel);
        let s_atk = cat::maanim(root, s_id, 0, (-1, -1), 2); 

        if s_png.exists() && s_cut.exists() && s_model.exists() && s_atk.exists() {
            spirit_pack = Some((s_png, s_cut, s_model, s_atk));
            spirit_available = true;
        }
    }

    // Validate / Sanitize / Fallback
    let current_idx = anim_viewer.loaded_anim_index;
    let mut valid_idx = current_idx;

    let is_current_valid = if current_idx == IDX_NONE {
        false 
    } else if current_idx == IDX_SPIRIT {
        spirit_available
    } else if current_idx == IDX_MODEL {
        base_assets_available
    } else {
        base_assets_available && available_anims.iter().any(|(i, _, _)| *i == current_idx)
    };

    if !is_current_valid {
        valid_idx = IDX_NONE; 

        if base_assets_available {
            let priority_list = [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB, IDX_BURROW, IDX_SURFACE];
            for check_idx in priority_list {
                if available_anims.iter().any(|(i, _, _)| *i == check_idx) {
                    valid_idx = check_idx;
                    break;
                }
            }
        }

        if valid_idx == IDX_NONE && spirit_available {
            valid_idx = IDX_SPIRIT;
        }

        if valid_idx == IDX_NONE && base_assets_available {
            valid_idx = IDX_MODEL;
        }
    }

    if valid_idx != current_idx {
        anim_viewer.loaded_anim_index = valid_idx;
        // If switching to None, clear data immediately
        if valid_idx == IDX_NONE {
            anim_viewer.current_anim = None;
            anim_viewer.held_model = None;
            anim_viewer.held_sheet = None; 
            *model_data = None;
            *anim_sheet = SpriteSheet::default();
        }
        // If recovering from None, force reload
        if current_idx == IDX_NONE && valid_idx != IDX_NONE {
            anim_viewer.loaded_id.clear();
        }
    }
    
    // Calculate Loading State
    let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
    let id_str = format!("{:03}", cat_entry.id);
    let form_viewer_id = format!("{}_{}_{}", id_str, form_char, anim_viewer.texture_version);

    let target_viewer_id = if anim_viewer.loaded_anim_index == IDX_SPIRIT {
        spirit_sheet_id.clone()
    } else {
        form_viewer_id.clone()
    };

    let is_stable = anim_viewer.loaded_id == target_viewer_id;
        
    let is_loading_new = !is_stable && (anim_viewer.staging_model.is_some() || anim_viewer.staging_sheet.is_some());
    let is_first_launch = anim_viewer.held_model.is_none() && model_data.is_none();
    let mut just_swapped = false;

    // If we are None, we are stable "empty"
    if valid_idx == IDX_NONE && !is_stable {
        anim_viewer.loaded_id = target_viewer_id.clone();
    }

    if is_stable {
        if let Some(m) = model_data {
            anim_viewer.held_model = Some(m.clone());
        }
        anim_viewer.held_sheet = Some((*anim_sheet).clone());
    }

    // Start Transition
    if !is_stable && !is_loading_new && !is_first_launch && valid_idx != IDX_NONE {
        let (resolved_png, resolved_cut, resolved_model, _) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);
        
        let mut load_success = false;
        if let (Some(png), Some(cut), Some(model_path)) = (resolved_png, resolved_cut, resolved_model) {
            let mut new_sheet = SpriteSheet::default();
            new_sheet.load(ctx, png, cut, target_viewer_id.clone());
            
            if let Some(loaded_model) = Model::load(model_path) {
                anim_viewer.staging_sheet = Some(new_sheet);
                anim_viewer.staging_model = Some(loaded_model);
                load_success = true;
            }
        }
        
        // If load fails, force stability
        if !load_success {
            anim_viewer.loaded_id = target_viewer_id.clone();
            anim_viewer.held_model = None;
            anim_viewer.held_sheet = None;
        }
    }

    // First Launch
    if is_first_launch && valid_idx != IDX_NONE {
        let (resolved_png, resolved_cut, resolved_model, resolved_anim) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);

        let mut load_success = false;
        if let (Some(png), Some(cut), Some(model_path)) = (resolved_png, resolved_cut, resolved_model) {
             anim_sheet.image_data = None; 
             anim_sheet.load(ctx, png, cut, target_viewer_id.clone());
             if let Some(loaded_model) = Model::load(model_path) {
                 anim_viewer.held_model = Some(loaded_model.clone());
                 anim_viewer.held_sheet = Some((*anim_sheet).clone());
                 *model_data = Some(loaded_model);
                 
                 anim_viewer.loaded_id = target_viewer_id.clone();
                 anim_viewer.pending_initial_center = true; 
                 load_success = true;
             }
        }
        
        // Same stability fix for first launch
        if !load_success {
            anim_viewer.loaded_id = target_viewer_id.clone();
        } else {
            if let Some(anim_path) = resolved_anim { 
                anim_viewer.load_anim(anim_path); 
            } else { 
                anim_viewer.current_anim = None; 
            }
        }
    }

    // Completion
    if is_loading_new {
        if let Some(staging_sheet) = &mut anim_viewer.staging_sheet {
            staging_sheet.update(ctx);

            let texture_is_ready = staging_sheet.sheet_name == target_viewer_id 
                                && !staging_sheet.is_loading_active 
                                && staging_sheet.image_data.is_some();

            if texture_is_ready {
                if let (Some(new_model), Some(new_sheet)) = (anim_viewer.staging_model.take(), anim_viewer.staging_sheet.take()) {
                    anim_viewer.held_model = Some(new_model.clone());
                    anim_viewer.held_sheet = Some(new_sheet.clone());
                    *model_data = Some(new_model);
                    *anim_sheet = new_sheet; 
                    anim_viewer.loaded_id = target_viewer_id.clone();
                    
                    let (_, _, _, resolved_anim) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);
                    
                    if let Some(anim_path) = resolved_anim { 
                        anim_viewer.load_anim(anim_path); 
                    } else { 
                        anim_viewer.current_anim = None; 
                    }
                    
                    anim_viewer.pending_initial_center = true;
                    just_swapped = true;
                    ctx.request_repaint();
                }
            }
        }
    } else {
        anim_sheet.update(ctx);
    }

    // Render
    let allow_texture_update = !is_loading_new || just_swapped;

    if anim_viewer.is_expanded {
        egui::Area::new("expanded_anim_viewer_area".into())
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Middle) 
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                egui::Frame::window(&ctx.style())
                    .inner_margin(0.0)
                    .shadow(egui::epaint::Shadow::NONE)
                    .show(ui, |ui| {
                        ui.set_min_size(screen_rect.size());
                        ui.set_max_size(screen_rect.size());
                        let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                        ui.put(rect, |ui: &mut egui::Ui| {
                            anim_viewer.render(
                                ui,
                                settings.animation_interpolation, settings.animation_debug, settings.centering_behavior,
                                allow_texture_update,
                                &available_anims,
                                spirit_available,
                                base_assets_available,
                                is_loading_new,
                                &spirit_sheet_id,
                                &form_viewer_id,
                                &spirit_pack,
                                settings.native_fps, 
                                settings.auto_set_camera_region, // PASSED HERE
                            );
                            ui.allocate_rect(rect, egui::Sense::hover())
                        });
                    });
            });

        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(egui::RichText::new("Animation Expanded").size(16.0).weak());
            if ui.button("Restore View").clicked() {
                anim_viewer.is_expanded = false;
            }
        });

    } else {
        ui.vertical(|ui| {
            let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
            ui.put(rect, |ui: &mut egui::Ui| {
                anim_viewer.render(
                    ui,
                    settings.animation_interpolation, settings.animation_debug, settings.centering_behavior,
                    allow_texture_update,
                    &available_anims,
                    spirit_available,
                    base_assets_available,
                    is_loading_new,
                    &spirit_sheet_id,
                    &form_viewer_id,
                    &spirit_pack,
                    settings.native_fps, 
                    settings.auto_set_camera_region, // AND HERE
                );
                ui.allocate_rect(rect, egui::Sense::hover())
            });
        });
    }
}

fn resolve_paths<'a>(
    idx: usize,
    png_path_base: &'a PathBuf,
    cut_path_base: &'a PathBuf,
    model_path_base: &'a PathBuf,
    spirit_pack: &'a Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    anims: &'a Vec<(usize, &str, PathBuf)>
) -> (Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>) {
    
    if idx == IDX_SPIRIT {
        if let Some((s_png, s_cut, s_model, s_anim)) = spirit_pack {
            return (Some(s_png), Some(s_cut), Some(s_model), Some(s_anim));
        }
    } else {
        let anim_path = anims.iter().find(|(i, _, _)| *i == idx).map(|(_, _, p)| p);
        return (Some(png_path_base), Some(cut_path_base), Some(model_path_base), anim_path);
    }
    (None, None, None, None)
}