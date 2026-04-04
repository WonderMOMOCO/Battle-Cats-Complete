use eframe::egui;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::Ordering;
use crate::global::ui::shared; 
use crate::updater;
use crate::features::home;
use crate::features::import::logic::ImportState;
use crate::features::cat::logic::CatListState;
use crate::features::enemy::logic::state::EnemyListState;
use crate::features::stage::logic::state::StageListState;
use crate::features::settings::logic::{Settings, upd::UpdateMode};

#[derive(PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum Page {
    Home,
    Cats,
    Enemies,
    Stages,
    Mods,
    Data,
    Settings,
}

impl Page {
    pub fn tab_name(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Cats => "Cats",
            Self::Enemies => "Enemies",
            Self::Stages => "Stages",
            Self::Mods => "Mods",
            Self::Data => "Data",
            Self::Settings => "Settings",
        }
    }
}

const ALL_PAGES: &[Page] = &[
    Page::Home,
    Page::Cats,
    Page::Enemies,
    #[cfg(debug_assertions)] Page::Stages,
    Page::Mods,
    Page::Data,
    Page::Settings,
];

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] 
pub struct BattleCatsApp {
    #[serde(skip)] current_page: Page,
    #[serde(skip)] sidebar_open: bool,
    #[serde(skip)] import_state: ImportState,
    #[serde(skip)] updater: updater::Updater,
    #[serde(skip)] drag_guard: shared::DragGuard,
    #[serde(skip)] global_watcher: Option<crate::global::io::watcher::GlobalWatcher>,
    cat_list_state: CatListState,
    enemy_list_state: EnemyListState,
    stage_list_state: StageListState,
    mod_state: crate::features::mods::logic::state::ModState,
    pub settings: Settings,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: Page::Home,
            sidebar_open: false,
            import_state: ImportState::default(),
            cat_list_state: CatListState::default(),
            enemy_list_state: EnemyListState::default(),
            stage_list_state: StageListState::default(),
            mod_state: crate::features::mods::logic::state::ModState::default(),
            settings: Settings::default(),
            updater: updater::Updater::default(),
            drag_guard: shared::DragGuard::default(),
            global_watcher: None,
        }
    }
}

impl BattleCatsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        #[cfg(not(debug_assertions))]
        if app.current_page == Page::Stages {
            app.current_page = Page::Home;
        }

        crate::features::settings::logic::lang::ensure_complete_list(&mut app.settings.general.language_priority);

        setup_custom_fonts(&cc.egui_ctx);
        
        app.cat_list_state.restart_scan(app.settings.scanner_config());
        app.enemy_list_state.restart_scan(app.settings.scanner_config());
        app.stage_list_state.restart_scan(app.settings.scanner_config());
        app.mod_state.refresh_mods();
        updater::cleanup_temp_files();

        if app.settings.general.update_mode != UpdateMode::Ignore {
            app.updater.check_for_updates(cc.egui_ctx.clone(), false);
        }

        app
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("jp_font".to_owned(), egui::FontData::from_static(crate::global::assets::FONT_JP));
    fonts.font_data.insert("kr_font".to_owned(), egui::FontData::from_static(crate::global::assets::FONT_KR));
    fonts.font_data.insert("tc_font".to_owned(), egui::FontData::from_static(crate::global::assets::FONT_TC));
    fonts.font_data.insert("thai_font".to_owned(), egui::FontData::from_static(crate::global::assets::FONT_TH));

    let families = [egui::FontFamily::Proportional, egui::FontFamily::Monospace];
    for family in families {
        let Some(list_ref) = fonts.families.get_mut(&family) else { continue; };
        
        list_ref.push("jp_font".to_owned());
        list_ref.push("kr_font".to_owned());
        list_ref.push("tc_font".to_owned());
        list_ref.push("thai_font".to_owned());
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for BattleCatsApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.updater.update_state(ctx);
        
        let status_str = match self.updater.status {
            updater::UpdateStatus::Checking => "Checking",
            updater::UpdateStatus::UpToDate => "UpToDate",
            updater::UpdateStatus::UpdateFound(..) => "UpdateFound",
            updater::UpdateStatus::CheckFailed => "CheckFailed",
            updater::UpdateStatus::Downloading(_) => "Downloading",
            updater::UpdateStatus::RestartPending(_) => "RestartPending",
            updater::UpdateStatus::Idle => "Idle",
        };
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("updater_status"), status_str));

        if self.settings.runtime.manual_check_requested {
            self.settings.runtime.manual_check_requested = false;
            self.updater.check_for_updates(ctx.clone(), true);
        }

        self.updater.show_ui(ctx, &mut self.settings, &mut self.drag_guard);
        
        let screen_rect = ctx.screen_rect();

        let sidebar_inner_width = 150.0; 
        let sidebar_margin = 15.0;       
        let total_sidebar_width = sidebar_inner_width + (sidebar_margin * 2.0);
        
        let target_open = if self.sidebar_open { 1.0 } else { 0.0 };
        let open_factor = ctx.animate_value_with_time(egui::Id::new("sb_anim"), target_open, 0.35);
        
        let visible_sidebar_width = total_sidebar_width * open_factor;
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("sidebar_visible_width"), visible_sidebar_width));

        if open_factor > 0.0 && open_factor < 1.0 {
            ctx.request_repaint();
        }

        if self.mod_state.needs_rescan {
            self.mod_state.needs_rescan = false;
            self.perform_full_data_reload();
            ctx.request_repaint();
        }

        self.process_file_events(ctx);

        self.cat_list_state.update_data();
        self.enemy_list_state.update_data();
        self.stage_list_state.update_data();

        if self.cat_list_state.scan_receiver.is_some() || self.enemy_list_state.scan_receiver.is_some() || self.stage_list_state.scan_receiver.is_some() {
            ctx.request_repaint();
        }
        
        let import_finished = self.import_state.update(ctx);
        if import_finished {
            self.perform_full_data_reload();
            ctx.request_repaint();
        }

        let mut style = (*ctx.style()).clone();
        style.visuals.window_rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(10.0);
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.visuals.window_fill = egui::Color32::from_rgb(33, 33, 33);
        style.visuals.panel_fill = egui::Color32::from_rgb(33, 33, 33);
        style.visuals.override_text_color = Some(egui::Color32::WHITE);
        ctx.set_style(style);

        match self.current_page {
            Page::Home => home::show(ctx, &mut self.drag_guard),
            Page::Cats => {
                crate::features::cat::logic::show(ctx, &mut self.cat_list_state, &mut self.settings);
            },
            Page::Enemies => {
                crate::features::enemy::logic::state::show(ctx, &mut self.enemy_list_state, &mut self.settings);            
            },
            Page::Stages => {
                crate::features::stage::ui::master::show(ctx, &mut self.stage_list_state, &mut self.settings);
            },
            Page::Mods => {
                crate::features::mods::ui::frame::show(ctx, &mut self.mod_state, &mut self.settings);
            },
            Page::Data => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::features::import::ui::manager::show(ui, &mut self.import_state, &mut self.settings); 
                });
            },
            Page::Settings => {
                let refresh_needed = crate::features::settings::ui::show(ctx, &mut self.settings, &mut self.drag_guard);
                
                if refresh_needed {
                    self.perform_full_data_reload();
                    ctx.request_repaint();
                }
            }
        }
        
        let sidebar_x = screen_rect.width() - visible_sidebar_width;
        let button_gap = 10.0;
        let button_size = 40.0;
        let button_x = sidebar_x - button_gap - button_size;

        if open_factor > 0.0 {
            egui::Area::new("sidebar_area".into())
                .constrain(false)
                .fixed_pos(egui::pos2(sidebar_x, 0.0))
                .order(egui::Order::Middle) 
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(20, 20, 20))
                        .inner_margin(15.0)
                        .rounding(egui::Rounding { nw: 10.0, sw: 10.0, ne: 0.0, se: 0.0 })
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(sidebar_inner_width, screen_rect.height()));
                            ui.vertical_centered_justified(|ui| {
                                
                                for page_enum in ALL_PAGES {
                                    ui.add_space(5.0);
                                    let btn_text = egui::RichText::new(page_enum.tab_name()).size(16.0); 
                                    let is_selected = self.current_page == *page_enum;
                                    
                                    let bg_color = if is_selected {
                                        egui::Color32::from_rgb(31, 106, 165) 
                                    } else {
                                        egui::Color32::from_rgb(50, 50, 50)   
                                    };

                                    let btn = egui::Button::new(btn_text).fill(bg_color).min_size(egui::vec2(0.0, 45.0));
                                    if !ui.add_sized([ui.available_width(), 45.0], btn).clicked() {
                                        continue;
                                    }
                                    
                                    if self.current_page == *page_enum {
                                        continue;
                                    }
                                    
                                    self.current_page = *page_enum;
                                    self.settings.runtime.show_ip_field = false;
                                }

                            });
                        });
                });
        }

        egui::Area::new("toggle_btn".into())
            .fixed_pos(egui::pos2(button_x, 2.5))
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let arrow = if self.sidebar_open { "▶" } else { "◀" };
                let btn = egui::Button::new(egui::RichText::new(arrow).size(20.0).strong())
                    .fill(egui::Color32::from_rgb(31, 106, 165));

                if ui.add_sized([40.0, 40.0], btn).clicked() {
                    self.sidebar_open = !self.sidebar_open;
                }
            });
    }
}

impl BattleCatsApp {
    pub fn perform_full_data_reload(&mut self) {
        self.cat_list_state.texture_cache_version += 1;
        self.cat_list_state.anim_viewer.loaded_id.clear();
        self.cat_list_state.detail_texture = None;
        self.cat_list_state.detail_key.clear();
        
        self.cat_list_state.img015_sheets.clear();
        self.cat_list_state.img022_sheets.clear();
        self.cat_list_state.sprite_sheet = crate::global::formats::imgcut::SpriteSheet::default();
        self.cat_list_state.gatya_item_textures.clear();
        
        self.enemy_list_state.anim_viewer.loaded_id.clear();
        self.enemy_list_state.detail_texture = None;
        self.enemy_list_state.detail_key.clear();
        self.enemy_list_state.img015_sheets.clear();

        let viewers = [
            &mut self.cat_list_state.anim_viewer,
            &mut self.enemy_list_state.anim_viewer,
        ];

        for viewer in viewers {
            viewer.loaded_id.clear();
            viewer.held_model = None;
            viewer.held_sheet = None;
            viewer.current_anim = None;
            viewer.staging_model = None;
            viewer.staging_sheet = None;
            viewer.current_frame = 0.0;
            viewer.texture_version += 1;
        }
        
        let config = self.settings.scanner_config();
        self.cat_list_state.cat_list.clear_cache();
        self.cat_list_state.restart_scan(config.clone());
        
        self.enemy_list_state.enemy_list.clear_cache();
        self.enemy_list_state.restart_scan(config.clone());

        self.stage_list_state.registry.clear_cache();
        self.stage_list_state.restart_scan(config);
    }

    fn process_file_events(&mut self, ctx: &egui::Context) {
        if self.global_watcher.is_none() {
            self.global_watcher = crate::global::io::watcher::GlobalWatcher::new(ctx.clone());
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
        
        crate::global::resolver::set_active_mod(active_mod.clone());

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
                self.cat_list_state.sprite_sheet = crate::global::formats::imgcut::SpriteSheet::default(); 
                self.cat_list_state.texture_cache_version += 1; 
            }

            if path_str.contains("tables") {
                global_cat_refresh = true;
                global_enemy_refresh = true;
                global_stage_refresh = true;
            }

            let is_cat_global_file = crate::features::cat::patterns::CAT_UNIVERSAL_FILES.contains(&file_name);
            
            if is_cat_global_file {
                global_cat_refresh = true;
            } else if file_name == crate::features::cat::paths::UNIT_BUY {
                global_cat_refresh = true;
            } else if path_str.contains(crate::features::cat::paths::DIR_UNIT_EVOLVE) || path_str.contains("unitevolve") {
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
            crate::features::cat::logic::loader::resync_scan(&mut self.cat_list_state, self.settings.scanner_config());
        } else {
            for id in cat_ids_to_refresh {
                self.cat_list_state.cat_list.flush_icon(id);
                if self.cat_list_state.selected_cat == Some(id) {
                    self.cat_list_state.detail_texture = None; 
                    self.cat_list_state.detail_key.clear();
                    self.cat_list_state.texture_cache_version += 1; 
                }
                crate::features::cat::logic::loader::refresh_cat(&mut self.cat_list_state, id, self.settings.scanner_config());
            }
        }

        if enemy_ids_to_refresh.len() > mass_threshold {
            self.enemy_list_state.detail_texture = None;
            self.enemy_list_state.detail_key.clear();
            crate::features::enemy::logic::loader::resync_scan(&mut self.enemy_list_state, self.settings.scanner_config());
        } else {
            for id in enemy_ids_to_refresh {
                self.enemy_list_state.enemy_list.flush_icon(id);
                if self.enemy_list_state.selected_enemy == Some(id) {
                    self.enemy_list_state.detail_texture = None;
                    self.enemy_list_state.detail_key.clear();
                }
                crate::features::enemy::logic::loader::refresh_enemy(&mut self.enemy_list_state, id, &self.settings.scanner_config());
            }
        }

        ctx.request_repaint();
    }

    fn check_if_active_mod_changed(path: &Path, active_mod: Option<&str>) -> bool {
        let Some(active) = active_mod else { return false; };
        let components: Vec<_> = path.components().map(|comp| comp.as_os_str().to_string_lossy().to_lowercase()).collect();
        
        let Some(mods_idx) = components.iter().position(|comp| comp == "mods") else { return false; };
        let Some(mod_folder) = components.get(mods_idx + 1) else { return false; };
        
        mod_folder == active
    }

    fn process_cat_path(&mut self, path: &Path, cat_ids_to_refresh: &mut HashSet<u32>) -> bool {
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
    
    fn process_enemy_path(path: &Path, enemy_ids_to_refresh: &mut HashSet<u32>) -> bool {
        let components: Vec<_> = path.components().map(|comp| comp.as_os_str().to_string_lossy()).collect();
        
        let Some(enemies_idx) = components.iter().position(|comp| comp == "enemies") else { return false; };
        let Some(folder_name) = components.get(enemies_idx + 1) else { return false; };
        
        let Ok(id) = folder_name.parse::<u32>() else { return true; };
        
        enemy_ids_to_refresh.insert(id);
        false
    }
}