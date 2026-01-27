use eframe::egui;
use crate::core::{cat, import, settings};
use crate::updater; // Import the new updater module
use crate::ui::views::main_menu;
use std::path::PathBuf;
use std::process::Command;

#[derive(PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum Page {
    MainMenu,
    ImportData,
    CatData,
    Settings,
}

const PAGES: &[(Page, &str)] = &[
    (Page::MainMenu, "Main Menu"),
    (Page::CatData, "Cat Data"),
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
    import_state: import::ImportState,
    #[serde(skip)]
    updater: updater::Updater,
    
    cat_list_state: cat::CatListState,
    pub settings: settings::Settings,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: Page::MainMenu,
            sidebar_open: false,
            import_state: import::ImportState::default(),
            cat_list_state: cat::CatListState::default(),
            settings: settings::Settings::default(),
            updater: updater::Updater::default(),
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
        app.cat_list_state.restart_scan(&app.settings.game_language);

        if app.settings.check_updates_on_startup {
            app.updater.check_for_updates();
        }

        app
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "jp_font".to_owned(),
        egui::FontData::from_static(include_bytes!("assets/NotoSansJP-Regular.ttf")),
    );
    fonts.font_data.insert(
        "kr_font".to_owned(),
        egui::FontData::from_static(include_bytes!("assets/NotoSansKR-Regular.ttf")),
    );
    fonts.font_data.insert(
        "tc_font".to_owned(),
        egui::FontData::from_static(include_bytes!("assets/NotoSansTC-Regular.ttf")),
    );
    fonts.font_data.insert(
        "thai_font".to_owned(),
        egui::FontData::from_static(include_bytes!("assets/NotoSansThai-Regular.ttf")),
    );

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
        
        // Process Update Messages
        self.updater.update_state();

        // Handle Manual Check Request from Settings
        if self.settings.manual_check_requested {
            self.settings.manual_check_requested = false;
            self.updater.check_for_updates();
        }

        // Update Found Modal
        if let updater::UpdateStatus::UpdateFound(tag, release) = &self.updater.status {
            let tag_clone = tag.clone();
            let release_clone = release.clone();
            let mut close_modal = false;
            let mut start_download = false;
            let mut disable_future = false;

            egui::Window::new("Update Available")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("New Battle Cats Complete update found: {}", tag_clone));
                    ui.add_space(10.0);
                    ui.label("Would you like to download the update now?");
                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            start_download = true;
                        }
                        if ui.button("No").clicked() {
                            close_modal = true;
                        }
                        if ui.button("Never").clicked() {
                            close_modal = true;
                            disable_future = true;
                        }
                    });
                });

            if start_download {
                self.updater.download_and_install(release_clone);
            }
            if disable_future {
                self.settings.check_updates_on_startup = false;
            }
            if close_modal {
                self.updater.status = updater::UpdateStatus::Idle;
            }
        }

        // Downloading Progress
        if let updater::UpdateStatus::Downloading(tag) = &self.updater.status {
             egui::Window::new("Downloading Update")
                .collapsible(false)
                .resizable(false)
                .title_bar(false) 
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("Downloading {}...", tag));
                    });
                    ui.add_space(10.0);
                    // Just a visual indeterminate bar
                    let progress = 0.5; 
                    ui.add(egui::ProgressBar::new(progress).animate(true));
                });
             ctx.request_repaint();
        }

        // Restart Prompt
        if let updater::UpdateStatus::RestartPending(tag) = &self.updater.status {
            let mut should_restart = false;
            let mut close = false;

            egui::Window::new("Update Complete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("{} update complete!", tag));
                    ui.add_space(5.0);
                    ui.label("Would you like to restart and apply the update now?");
                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        if ui.button("Yes (Restart)").clicked() {
                            should_restart = true;
                        }
                        if ui.button("No (Apply Later)").clicked() {
                            close = true;
                        }
                    });
                });
            
            if should_restart {
                // Spawn new process
                let _ = Command::new(std::env::current_exe().unwrap()).spawn();
                // Exit current process
                std::process::exit(0);
            }
            if close {
                self.updater.status = updater::UpdateStatus::Idle;
            }
        }

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

        let mut reload_queue: Vec<PathBuf> = Vec::new();
        if let Some(rx) = &self.cat_list_state.watch_receiver {
            while let Ok(path) = rx.try_recv() {
                reload_queue.push(path);
            }
        }

        for path in reload_queue {
            self.cat_list_state.handle_event(ctx, &path, &self.settings.game_language);
        }

        self.cat_list_state.update_data();
        if self.cat_list_state.scan_receiver.is_some() {
            ctx.request_repaint();
        }

        let import_finished = self.import_state.update(ctx, &mut self.settings);
        if import_finished {
            self.cat_list_state.restart_scan(&self.settings.game_language);
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
            Page::MainMenu => main_menu::show(ctx),
            Page::ImportData => {
                crate::ui::views::import::show(ctx, &mut self.import_state); 
            },
            Page::CatData => {
                crate::core::cat::show(ctx, &mut self.cat_list_state, &self.settings);
            },
            Page::Settings => {
                let mut tabs = vec!["General"];
                for (page_enum, label) in PAGES {
                    if *page_enum != Page::MainMenu && *page_enum != Page::Settings {
                        tabs.push(label);
                    }
                }
                
                let refresh_needed = crate::ui::views::settings::show(ctx, &mut self.settings, &tabs);
                
                if refresh_needed {
                    self.cat_list_state.cat_list.clear_cache();
                    self.cat_list_state.restart_scan(&self.settings.game_language);
                }
            }
        }
        
        let screen_rect = ctx.screen_rect();
        let sidebar_x = screen_rect.width() - visible_sidebar_width;
        let button_gap = 10.0;
        let button_size = 40.0;
        let button_x = sidebar_x - button_gap - button_size;

        if open_factor > 0.0 {
            egui::Area::new("sidebar_area".into())
                .constrain(false)
                .fixed_pos(egui::pos2(sidebar_x, 0.0))
                .order(egui::Order::Foreground)
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
                                        self.current_page = *page_enum;
                                    }
                                }
                            });
                        });
                });
        }

        egui::Area::new("toggle_btn".into())
            .fixed_pos(egui::pos2(button_x, 2.5))
            .order(egui::Order::Tooltip)
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