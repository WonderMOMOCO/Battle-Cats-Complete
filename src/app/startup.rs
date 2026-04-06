use eframe::egui;
use std::path::Path;
use crate::global::assets;
use crate::global::io::json;
use crate::global::game::param::load_param;
use crate::updater;
use crate::features::settings::logic::{lang, upd::UpdateMode};
use crate::app::BattleCatsApp;
use crate::features::cat::paths as cat_paths;
use crate::features::cat::data::{skilllevel, skilldescriptions};

#[cfg(not(debug_assertions))]
use crate::app::frame::Page;

impl BattleCatsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app: Self = json::load("settings.json").unwrap_or_default();

        #[cfg(not(debug_assertions))]
        if app.current_page == Page::Stages {
            app.current_page = Page::Home;
        }

        lang::ensure_complete_list(&mut app.settings.general.language_priority);

        setup_custom_fonts(&cc.egui_ctx);
        
        app.mod_state.refresh_mods();
        updater::cleanup_temp_files();

        app.param = load_param(Path::new("game/tables"), &app.settings.general.language_priority).unwrap_or_default();

        let mut expected_hash = 0;
        let mut needs_validation = false;
        let priority = &app.settings.general.language_priority;

        if let Some((h, cached_cats)) = crate::global::io::cache::load_with_hash::<Vec<crate::features::cat::logic::scanner::CatEntry>>("cats_cache.bin") {
            expected_hash = h;
            needs_validation = true; 
            let cats_dir = Path::new(cat_paths::DIR_CATS);
            let costs_arc = std::sync::Arc::new(skilllevel::load(cats_dir, priority));
            let descs_arc = std::sync::Arc::new(skilldescriptions::load(cats_dir, priority));

            app.cat_list_state.cats = cached_cats.into_iter().map(|mut cat| {
                cat.talent_costs = std::sync::Arc::clone(&costs_arc);
                cat.skill_descriptions = std::sync::Arc::clone(&descs_arc);
                cat
            }).collect();
            app.cat_list_state.initialized = true;
        } else {
            app.cat_list_state.restart_scan(app.settings.scanner_config());
        }

        if let Some((h, cached_enemies)) = crate::global::io::cache::load_with_hash::<Vec<crate::features::enemy::logic::scanner::EnemyEntry>>("enemies_cache.bin") {
            expected_hash = h;
            needs_validation = true;
            app.enemy_list_state.enemies = cached_enemies;
            app.enemy_list_state.initialized = true;
        } else {
            app.enemy_list_state.restart_scan(app.settings.scanner_config());
        }

        app.stage_list_state.restart_scan(app.settings.scanner_config());

        if needs_validation {
            let (tx, rx) = std::sync::mpsc::channel();
            app.hash_rx = Some(rx);
            let active_mod = crate::global::resolver::get_active_mod();
            
            std::thread::spawn(move || {
                let current_hash = crate::global::io::cache::get_game_hash(active_mod.as_deref());
                let _ = tx.send(current_hash == expected_hash && active_mod.is_none());
            });
        }

        if app.settings.general.update_mode != UpdateMode::Ignore {
            app.updater.check_for_updates(cc.egui_ctx.clone(), false);
        }

        app
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("jp_font".to_owned(), egui::FontData::from_static(assets::FONT_JP));
    fonts.font_data.insert("kr_font".to_owned(), egui::FontData::from_static(assets::FONT_KR));
    fonts.font_data.insert("tc_font".to_owned(), egui::FontData::from_static(assets::FONT_TC));
    fonts.font_data.insert("thai_font".to_owned(), egui::FontData::from_static(assets::FONT_TH));

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