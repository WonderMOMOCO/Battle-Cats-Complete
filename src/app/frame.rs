use eframe::egui;
use crate::app::BattleCatsApp;

// UI Routing
use crate::features::home;
use crate::features::cat::logic::show as show_cats;
use crate::features::enemy::logic::state::show as show_enemies;
use crate::features::stage::ui::master::show as show_stages;
use crate::features::mods::ui::frame::show as show_mods;
use crate::features::data::ui::manager::show as show_data;
use crate::features::settings::ui::show as show_settings;

#[derive(PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Page {
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

pub const ALL_PAGES: &[Page] = &[
    Page::Home,
    Page::Cats,
    Page::Enemies,
    #[cfg(debug_assertions)] Page::Stages,
    Page::Mods,
    Page::Data,
    Page::Settings,
];

pub fn draw(app: &mut BattleCatsApp, ctx: &egui::Context) {
    let screen_rect = ctx.screen_rect();

    let sidebar_inner_width = 150.0; 
    let sidebar_margin = 15.0;       
    let total_sidebar_width = sidebar_inner_width + (sidebar_margin * 2.0);
    
    let target_open = if app.sidebar_open { 1.0 } else { 0.0 };
    let open_factor = ctx.animate_value_with_time(egui::Id::new("sb_anim"), target_open, 0.35);
    
    let visible_sidebar_width = total_sidebar_width * open_factor;
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("sidebar_visible_width"), visible_sidebar_width));

    if open_factor > 0.0 && open_factor < 1.0 {
        ctx.request_repaint();
    }

    if app.mod_state.needs_rescan {
        app.mod_state.needs_rescan = false;
        app.perform_full_data_reload();
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

    match app.current_page {
        Page::Home => home::show(ctx, &mut app.drag_guard),
        Page::Cats => show_cats(ctx, &mut app.cat_list_state, &mut app.settings, &app.param),
        Page::Enemies => show_enemies(ctx, &mut app.enemy_list_state, &mut app.settings, &app.param),            
        Page::Stages => show_stages(ctx, &mut app.stage_list_state, &mut app.settings),
        Page::Mods => show_mods(ctx, &mut app.mod_state, &mut app.settings),
        Page::Data => {
            egui::CentralPanel::default().show(ctx, |ui| {
                show_data(ui, &mut app.import_state, &mut app.settings); 
            });
        },
        Page::Settings => {
            let refresh_needed = show_settings(ctx, &mut app.settings, &mut app.drag_guard);
            if refresh_needed {
                app.perform_full_data_reload();
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
                                let is_selected = app.current_page == *page_enum;
                                
                                let bg_color = if is_selected {
                                    egui::Color32::from_rgb(31, 106, 165) 
                                } else {
                                    egui::Color32::from_rgb(50, 50, 50)   
                                };

                                let btn = egui::Button::new(btn_text).fill(bg_color).min_size(egui::vec2(0.0, 45.0));
                                if !ui.add_sized([ui.available_width(), 45.0], btn).clicked() {
                                    continue;
                                }
                                
                                if app.current_page == *page_enum {
                                    continue;
                                }
                                
                                app.current_page = *page_enum;
                                app.settings.runtime.show_ip_field = false;
                            }

                        });
                    });
            });
    }

    egui::Area::new("toggle_btn".into())
        .fixed_pos(egui::pos2(button_x, 2.5))
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let arrow = if app.sidebar_open { "▶" } else { "◀" };
            let btn = egui::Button::new(egui::RichText::new(arrow).size(20.0).strong())
                .fill(egui::Color32::from_rgb(31, 106, 165));

            if ui.add_sized([40.0, 40.0], btn).clicked() {
                app.sidebar_open = !app.sidebar_open;
            }
        });
}