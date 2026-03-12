use eframe::egui;
use std::collections::HashSet; 
use crate::global::ui::shared; 
use crate::updater;
use crate::features::mainmenu;
use crate::features::import::logic::ImportState;
use crate::features::cat::logic::CatListState;
use crate::features::enemy::logic::state::EnemyListState;
use crate::features::settings::logic::{Settings, upd::UpdateMode};

#[derive(PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum Page {
    MainMenu,
    ImportData,
    CatData,
    EnemyData,
    Settings,
}

const PAGES: &[(Page, &str)] = &[
    (Page::MainMenu, "Main Menu"),
    (Page::CatData, "Cat Data"),
    (Page::EnemyData, "Enemy Data"),
    (Page::ImportData, "Game Data"),
    (Page::Settings, "Settings"),
];

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] 
pub struct BattleCatsApp {
    #[serde(skip)]
    current_page: Page,
    #[serde(skip)]
    sidebar_open: bool,
    #[serde(skip)]
    import_state: ImportState,
    
    #[serde(skip)]
    updater: updater::Updater,
    
    #[serde(skip)]
    drag_guard: shared::DragGuard,
    
    #[serde(skip)]
    global_watcher: Option<crate::global::io::watcher::GlobalWatcher>,
    
    cat_list_state: CatListState,
    enemy_list_state: EnemyListState,
    pub settings: Settings,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: Page::MainMenu,
            sidebar_open: false,
            import_state: ImportState::default(),
            cat_list_state: CatListState::default(),
            enemy_list_state: EnemyListState::default(),
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

        setup_custom_fonts(&cc.egui_ctx);
        
        if app.settings.runtime.rx_lang.is_none() {
            app.settings.runtime.rx_lang = Some(crate::features::settings::logic::lang::start_scan());
        }

        app.cat_list_state.restart_scan(app.settings.scanner_config());
        app.enemy_list_state.restart_scan(app.settings.scanner_config());
        updater::cleanup_temp_files();

        if app.settings.general.update_mode != UpdateMode::Ignore {
            app.updater.check_for_updates();
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
        if let Some(list) = fonts.families.get_mut(&family) {
            list.push("jp_font".to_owned());
            list.push("kr_font".to_owned());
            list.push("tc_font".to_owned());
            list.push("thai_font".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for BattleCatsApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.settings.runtime.rx_lang.is_some() {
            self.settings.update_language_list();
            ctx.request_repaint(); 
        }

        self.updater.update_state();

        if self.settings.runtime.manual_check_requested {
            self.settings.runtime.manual_check_requested = false;
            self.updater.check_for_updates();
        }

        self.updater.show_ui(ctx, &mut self.settings, &mut self.drag_guard);
        
        let screen_rect = ctx.screen_rect();

        let sidebar_inner_width = 150.0; 
        let sidebar_margin = 15.0;       
        let total_sidebar_width = sidebar_inner_width + (sidebar_margin * 2.0);
        
        let target_open = if self.sidebar_open { 1.0 } else { 0.0 };
        let open_factor = ctx.animate_value_with_time(egui::Id::new("sb_anim"), target_open, 0.35);
        
        let visible_sidebar_width = total_sidebar_width * open_factor;
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("sidebar_visible_width"), visible_sidebar_width));

        if open_factor > 0.0 && open_factor < 1.0 {
            ctx.request_repaint();
        }

        self.process_file_events(ctx);

        self.cat_list_state.update_data();
        self.enemy_list_state.update_data();

        if self.cat_list_state.scan_receiver.is_some() || self.enemy_list_state.scan_receiver.is_some() {
            ctx.request_repaint();
        }
        
        self.enemy_list_state.update_data();
        if self.enemy_list_state.scan_receiver.is_some() {
            ctx.request_repaint();
        }

        let import_finished = self.import_state.update(ctx, &mut self.settings);
        if import_finished {
            self.cat_list_state.restart_scan(self.settings.scanner_config());
            self.enemy_list_state.restart_scan(self.settings.scanner_config());
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
            Page::MainMenu => mainmenu::show(ctx, &mut self.drag_guard),
            Page::ImportData => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::features::import::ui::manager::show(ui, &mut self.import_state, &mut self.settings); 
                });
            }
            Page::CatData => {
                crate::features::cat::logic::show(ctx, &mut self.cat_list_state, &mut self.settings);
            },
            Page::EnemyData => {
                crate::features::enemy::logic::state::show(ctx, &mut self.enemy_list_state, &mut self.settings);            
            },
            Page::Settings => {
                let refresh_needed = crate::features::settings::ui::show(ctx, &mut self.settings, &mut self.drag_guard);
                
                if refresh_needed {
                    self.cat_list_state.cat_list.clear_cache();
                    self.cat_list_state.restart_scan(self.settings.scanner_config());
                    
                    self.enemy_list_state.enemy_list.clear_cache();
                    self.enemy_list_state.restart_scan(self.settings.scanner_config());
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
                                for (page_enum, label) in PAGES {
                                    ui.add_space(5.0);
                                    let btn_text = egui::RichText::new(*label).size(16.0); 
                                    let is_selected = self.current_page == *page_enum;
                                    let bg_color = if is_selected {
                                        egui::Color32::from_rgb(31, 106, 165) 
                                    } else {
                                        egui::Color32::from_rgb(50, 50, 50)   
                                    };

                                    let btn = egui::Button::new(btn_text).fill(bg_color).min_size(egui::vec2(0.0, 45.0));
                                    if ui.add_sized([ui.available_width(), 45.0], btn).clicked() {
                                        if self.current_page != *page_enum {
                                            self.current_page = *page_enum;
                                            self.settings.runtime.show_ip_field = false;
                                        }
                                    }
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
    fn process_file_events(&mut self, ctx: &egui::Context) {
        if self.global_watcher.is_none() {
            self.global_watcher = crate::global::io::watcher::GlobalWatcher::new(ctx.clone());
        }

        let watcher = match &self.global_watcher {
            Some(w) => w,
            None => return,
        };

        let mut paths = Vec::new();
        while let Ok(path) = watcher.rx.try_recv() {
            paths.push(path);
        }

        if paths.is_empty() {
            return;
        }

        if self.import_state.rx.is_some() || self.import_state.is_adb_busy {
            return;
        }

        let mut cat_ids_to_refresh = HashSet::new();
        let mut enemy_ids_to_refresh = HashSet::new(); 
        let mut global_cat_refresh = false;
        let mut global_enemy_refresh = false;

        for path in paths {
            let path_str = path.to_string_lossy().to_lowercase();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            
            if path_str.contains("img015") || path_str.contains("img022") {
                self.cat_list_state.icon_sheet = crate::global::formats::imgcut::SpriteSheet::default();
                self.cat_list_state.img022_sheet = crate::global::formats::imgcut::SpriteSheet::default();
                self.enemy_list_state.icon_sheet = crate::global::formats::imgcut::SpriteSheet::default();
            }

            if path_str.contains("assets") || path_str.contains("gatyaitem") {
                self.cat_list_state.gatya_item_textures.clear();
                self.cat_list_state.sprite_sheet = crate::global::formats::imgcut::SpriteSheet::default(); 
                self.cat_list_state.texture_cache_version += 1; 
            }

            let is_cat_global = crate::features::cat::patterns::CAT_UNIVERSAL_FILES.contains(&file_name) 
                             || crate::global::io::patterns::CHECK_LINE_FILES.contains(&file_name);
            
            if is_cat_global {
                global_cat_refresh = true;
            } else if file_name == crate::features::cat::paths::UNIT_BUY {
                self.cat_list_state.cached_unit_buy = None;
                global_cat_refresh = true;
            } else if path_str.contains(crate::features::cat::paths::DIR_UNIT_EVOLVE) || path_str.contains("unitevolve") {
                self.cat_list_state.cached_evolve_text = None; 
                global_cat_refresh = true;
            } else if path_str.contains("cats") {
                let components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy()).collect();
                if let Some(cats_idx) = components.iter().position(|c| c == "cats") {
                    if let Some(id_str) = components.get(cats_idx + 1) {
                        if let Ok(id) = id_str.parse::<u32>() {
                            
                            if let Some(anim_seg) = components.get(cats_idx + 3) {
                                if anim_seg == "anim" && self.cat_list_state.selected_cat == Some(id) {
                                    let form_char = components.get(cats_idx + 2).map(|s| s.to_string()).unwrap_or_else(|| "f".to_string());
                                    let marker = format!("_{}_", form_char);
                                    if self.cat_list_state.anim_viewer.loaded_id.is_empty() || self.cat_list_state.anim_viewer.loaded_id.contains(&marker) {
                                        self.cat_list_state.anim_viewer.loaded_id.clear();
                                        self.cat_list_state.anim_viewer.texture_version += 1; 
                                    }
                                    continue; 
                                }
                            }
                            cat_ids_to_refresh.insert(id);
                        }
                    }
                }
            }

            let is_enemy_global = file_name.contains("t_unit") || file_name.contains("enemyname") || file_name.contains("enemypicturebook");
            if is_enemy_global {
                global_enemy_refresh = true;
            }

            if path_str.contains("enemies") {
                if let Some(parent) = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
                    if parent.len() == 3 && parent.chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(id) = parent.parse::<u32>() {
                            enemy_ids_to_refresh.insert(id);
                        }
                    }
                }
            }
        }

        let mass_threshold = 5;

        if global_cat_refresh || cat_ids_to_refresh.len() > mass_threshold {
            self.cat_list_state.detail_texture = None;
            self.cat_list_state.detail_key.clear();
            self.cat_list_state.texture_cache_version += 1;
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

        if global_enemy_refresh || enemy_ids_to_refresh.len() > mass_threshold {
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
}