use eframe::egui;
use std::path::Path;
use crate::global::assets;
use crate::global::io::json;
use crate::global::game::param::load_param;
use crate::updater;
use crate::features::settings::logic::{lang, upd::UpdateMode};
use crate::app::BattleCatsApp;

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
        
        app.cat_list_state.restart_scan(app.settings.scanner_config());
        app.enemy_list_state.restart_scan(app.settings.scanner_config());
        app.stage_list_state.restart_scan(app.settings.scanner_config());
        app.mod_state.refresh_mods();
        updater::cleanup_temp_files();

        app.param = load_param(Path::new("game/tables"), &app.settings.general.language_priority).unwrap_or_default();

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