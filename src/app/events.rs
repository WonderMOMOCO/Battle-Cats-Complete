use eframe::egui;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::Ordering;

use crate::app::BattleCatsApp;
use crate::global::io::watcher::GlobalWatcher;
use crate::global::resolver;
use crate::global::formats::imgcut::SpriteSheet;

use crate::features::cat::logic::loader as cat_loader;
use crate::features::cat::{paths as cat_paths, patterns as cat_patterns};
use crate::features::enemy::logic::loader as enemy_loader;

impl BattleCatsApp {
    pub fn process_file_events(&mut self, ctx: &egui::Context) {
        if self.global_watcher.is_none() {
            self.global_watcher = GlobalWatcher::new(ctx.clone());
        }

        let Some(watcher) = &self.global_watcher else { return; };

        let mut paths = Vec::new();
        while let Ok(path) = watcher.rx.try_recv() {
            paths.push(path);
        }

        if paths.is_empty() { return; }
        
        if self.import_state.import_rx.is_some() || self.import_state.import_job_status.load(Ordering::Relaxed) == 1 { return; }
        if self.import_state.export_rx.is_some() || self.import_state.export_job_status.load(Ordering::Relaxed) == 1 { return; }

        let mut cat_ids_to_refresh = HashSet::new();
        let mut enemy_ids_to_refresh = HashSet::new(); 
        let mut global_cat_refresh = false;
        let mut global_enemy_refresh = false;
        let mut global_stage_refresh = false;
        let mut mods_refresh = false;
        let mut active_mod_file_changed = false;

        let active_mod = self.mod_state.loaded_mods.iter()
            .find(|mod_item| mod_item.enabled)
            .map(|mod_item| mod_item.folder_name.to_lowercase());
        
        resolver::set_active_mod(active_mod.clone());

        for path in paths {
            let path_str = path.to_string_lossy().to_lowercase();
            let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
            
            let is_mod_path = path_str.contains("mods") && !path_str.contains("packages");
            if is_mod_path {
                mods_refresh = true;
                if Self::check_if_active_mod_changed(&path, active_mod.as_deref()) {
                    active_mod_file_changed = true;
                }
            }

            if path_str.contains("img015") || path_str.contains("img022") {
                self.cat_list_state.img015_sheets.clear();
                self.cat_list_state.img022_sheets.clear();
                self.enemy_list_state.img015_sheets.clear();
            }

            if path_str.contains("ui") || path_str.contains("gatyaitem") || path_str.contains("sheets") {
                self.cat_list_state.gatya_item_textures.clear();
                self.cat_list_state.sprite_sheet = SpriteSheet::default(); 
                self.cat_list_state.texture_cache_version += 1; 
            }

            if path_str.contains("tables") {
                global_cat_refresh = true;
                global_enemy_refresh = true;
                global_stage_refresh = true;
            }

            let is_cat_global_file = cat_patterns::CAT_UNIVERSAL_FILES.contains(&file_name);
            
            if is_cat_global_file {
                global_cat_refresh = true;
            } else if file_name == cat_paths::UNIT_BUY {
                global_cat_refresh = true;
            } else if path_str.contains(cat_paths::DIR_UNIT_EVOLVE) || path_str.contains("unitevolve") {
                global_cat_refresh = true;
            } else if path_str.contains("cats") && self.process_cat_path(&path, &mut cat_ids_to_refresh) {
                global_cat_refresh = true;
            }

            let is_enemy_global_file = file_name.contains("t_unit") || file_name.contains("enemyname") || file_name.contains("enemypicturebook");
            if is_enemy_global_file {
                global_enemy_refresh = true;
            }

            let is_in_enemies_dir = path_str.contains("enemies");
            if is_in_enemies_dir && Self::process_enemy_path(&path, &mut enemy_ids_to_refresh) {
                global_enemy_refresh = true;
            }

            if path_str.contains("stages") {
                global_stage_refresh = true;
            }
        }

        if mods_refresh {
            self.mod_state.refresh_mods();
        }

        if active_mod_file_changed || global_cat_refresh || global_enemy_refresh || global_stage_refresh {
            self.perform_full_data_reload();
            ctx.request_repaint();
            return; 
        }

        let mass_threshold = 5;

        if cat_ids_to_refresh.len() > mass_threshold {
            self.cat_list_state.detail_texture = None;
            self.cat_list_state.detail_key.clear();
            self.cat_list_state.texture_cache_version += 1;
            self.cat_list_state.anim_viewer.loaded_id.clear();
            cat_loader::resync_scan(&mut self.cat_list_state, self.settings.scanner_config());
        } else {
            for &id in &cat_ids_to_refresh {
                self.cat_list_state.cat_list.flush_icon(id);
                if self.cat_list_state.selected_cat == Some(id) {
                    self.cat_list_state.detail_texture = None; 
                    self.cat_list_state.detail_key.clear();
                    self.cat_list_state.texture_cache_version += 1; 
                }
                cat_loader::refresh_cat(&mut self.cat_list_state, id, self.settings.scanner_config());
            }
        }

        if enemy_ids_to_refresh.len() > mass_threshold {
            self.enemy_list_state.detail_texture = None;
            self.enemy_list_state.detail_key.clear();
            enemy_loader::resync_scan(&mut self.enemy_list_state, self.settings.scanner_config());
        } else {
            for &id in &enemy_ids_to_refresh {
                self.enemy_list_state.enemy_list.flush_icon(id);
                if self.enemy_list_state.selected_enemy == Some(id) {
                    self.enemy_list_state.detail_texture = None;
                    self.enemy_list_state.detail_key.clear();
                }
                enemy_loader::refresh_enemy(&mut self.enemy_list_state, id, &self.settings.scanner_config());
            }
        }

        if (!cat_ids_to_refresh.is_empty() || !enemy_ids_to_refresh.is_empty() || global_cat_refresh || global_enemy_refresh || global_stage_refresh) 
            && !crate::global::resolver::is_mod_active() {
            
            let cats = self.cat_list_state.cats.clone();
            let enemies = self.enemy_list_state.enemies.clone();
            
            std::thread::spawn(move || {
                let hash = crate::global::io::cache::get_game_hash(None);
                crate::global::io::cache::save("cats_cache.bin", hash, &cats);
                crate::global::io::cache::save("enemies_cache.bin", hash, &enemies);
            });
        }

        ctx.request_repaint();
    }

    pub fn check_if_active_mod_changed(path: &Path, active_mod: Option<&str>) -> bool {
        let Some(active) = active_mod else { return false; };
        let components: Vec<_> = path.components().map(|comp| comp.as_os_str().to_string_lossy().to_lowercase()).collect();
        
        let Some(mods_idx) = components.iter().position(|comp| comp == "mods") else { return false; };
        let Some(mod_folder) = components.get(mods_idx + 1) else { return false; };
        
        mod_folder == active
    }

    pub fn process_cat_path(&mut self, path: &Path, cat_ids_to_refresh: &mut HashSet<u32>) -> bool {
        let components: Vec<_> = path.components().map(|comp| comp.as_os_str().to_string_lossy()).collect();
        
        let Some(cats_idx) = components.iter().position(|comp| comp == "cats") else { return false; };
        let Some(folder_name) = components.get(cats_idx + 1) else { return false; };

        let parsed_id = if let Ok(id) = folder_name.parse::<u32>() {
            Some(id)
        } else if folder_name.starts_with("egg_") {
            folder_name[4..].parse::<u32>().ok()
        } else {
            None
        };

        let Some(id) = parsed_id else { return true; };

        let is_anim = components.get(cats_idx + 3).map(|string_val| string_val.as_ref()) == Some("anim");
        if !is_anim || self.cat_list_state.selected_cat != Some(id) {
            cat_ids_to_refresh.insert(id);
            return false;
        }

        let form_char = components.get(cats_idx + 2).map(|string_val| string_val.to_string()).unwrap_or_else(|| "f".to_string());
        let marker = format!("_{}_", form_char);
        
        let loaded = &mut self.cat_list_state.anim_viewer.loaded_id;
        if loaded.is_empty() || loaded.contains(&marker) {
            loaded.clear();
            self.cat_list_state.anim_viewer.texture_version += 1; 
        }
        
        false
    }
    
    pub fn process_enemy_path(path: &Path, enemy_ids_to_refresh: &mut HashSet<u32>) -> bool {
        let components: Vec<_> = path.components().map(|comp| comp.as_os_str().to_string_lossy()).collect();
        
        let Some(enemies_idx) = components.iter().position(|comp| comp == "enemies") else { return false; };
        let Some(folder_name) = components.get(enemies_idx + 1) else { return false; };
        
        let Ok(id) = folder_name.parse::<u32>() else { return true; };
        
        enemy_ids_to_refresh.insert(id);
        false
    }
}