use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::time::Instant;
use std::collections::HashSet;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::settings::logic::state::Settings;
use crate::features::settings::logic::state::ScannerConfig;
use crate::features::enemy::ui::list::EnemyList;
use crate::features::enemy::ui::master;
use crate::global::formats::mamodel::Model;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::global::assets::CustomAssets;
use crate::global::ui::shared::DragGuard;
use crate::features::enemy::ui::filter::EnemyFilterState; 
use super::loader;

pub const TOP_PANEL_PADDING: f32 = 2.5;
pub const SEARCH_FILTER_GAP: f32 = 5.0;
pub const SPACE_BEFORE_SEPARATOR: f32 = 2.0;
pub const SPACE_AFTER_SEPARATOR: f32 = 2.0;

#[derive(Deserialize, Serialize, PartialEq, Clone, Copy)]
pub enum EnemyDetailTab {
    Abilities,
    Details,
    Animation,
}

impl Default for EnemyDetailTab {
    fn default() -> Self { Self::Abilities }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct EnemyListState {
    #[serde(skip)] pub enemies: Vec<EnemyEntry>, 
    #[serde(skip)] pub incoming_enemies: Vec<EnemyEntry>,
    #[serde(skip)] pub is_cold_scan: bool,
    #[serde(skip)] pub last_update_time: Option<Instant>,
    pub selected_enemy: Option<u32>,
    pub search_query: String,
    pub selected_tab: EnemyDetailTab,
    pub mag_input: String,
    pub magnification: i32,
    #[serde(skip)] pub enemy_list: EnemyList, 
    #[serde(skip)] pub initialized: bool,
    #[serde(skip)] pub active_scan_ids: HashSet<u32>,
    #[serde(skip)] pub detail_texture: Option<egui::TextureHandle>,
    #[serde(skip)] pub detail_key: String,
    #[serde(skip)] pub icon_sheet: crate::global::formats::imgcut::SpriteSheet,   
    #[serde(skip)] pub anim_sheet: crate::global::formats::imgcut::SpriteSheet,
    #[serde(skip)] pub model_data: Option<Model>,
    #[serde(skip)] pub anim_viewer: AnimViewer,
    #[serde(skip)] pub custom_assets: Option<CustomAssets>,
    #[serde(skip)] pub scan_receiver: Option<Receiver<EnemyEntry>>,
    #[serde(skip)] pub filter_state: EnemyFilterState,
    #[serde(skip)] pub drag_guard: DragGuard,
}

impl Default for EnemyListState {
    fn default() -> Self {
        Self {
            enemies: Vec::new(),
            incoming_enemies: Vec::new(),
            is_cold_scan: false,
            last_update_time: None,
            selected_enemy: None,
            search_query: String::new(),
            selected_tab: EnemyDetailTab::default(),
            mag_input: "100".to_string(),
            magnification: 100,
            enemy_list: EnemyList::default(),
            initialized: false,
            active_scan_ids: HashSet::new(),
            detail_texture: None,
            detail_key: String::new(),
            icon_sheet: crate::global::formats::imgcut::SpriteSheet::default(), 
            anim_sheet: crate::global::formats::imgcut::SpriteSheet::default(), 
            model_data: None,
            anim_viewer: AnimViewer::default(),
            custom_assets: None, 
            scan_receiver: None,
            filter_state: EnemyFilterState::default(),
            drag_guard: DragGuard::default(),
        }
    }
}

impl EnemyListState {
    pub fn update_data(&mut self) {
        loader::update_data(self);
    }

    pub fn restart_scan(&mut self, config: ScannerConfig) {
        loader::restart_scan(self, config);
    }
}

pub fn show(ctx: &egui::Context, state: &mut EnemyListState, settings: &mut Settings) {
    if state.custom_assets.is_none() {
        state.custom_assets = Some(CustomAssets::new(ctx));
    }
    let assets = state.custom_assets.as_ref().unwrap().clone();

    if !state.initialized {
        state.initialized = true;
        
        if !settings.cat_data.unit_persistence {
            state.selected_enemy = None;
            state.enemy_list.reset_scroll();
        }
    }

    if state.scan_receiver.is_some() {
        state.update_data();
        ctx.request_repaint(); 
    }

    let old_selection_id = state.selected_enemy;

    egui::SidePanel::left("enemy_list_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(TOP_PANEL_PADDING);
                
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let search_response = ui.add(egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text(egui::RichText::new("Search Enemy...").color(egui::Color32::GRAY))
                        .desired_width(140.0));
                    
                    ui.add_space(SEARCH_FILTER_GAP); 
                    
                    let btn_size = egui::vec2(140.0, search_response.rect.height());
                    let filter_active = state.filter_state.is_active();
                    
                    let mut filter_btn = egui::Button::new("Filter");
                    if filter_active {
                        filter_btn = filter_btn.fill(egui::Color32::from_rgb(31, 106, 165));
                    }
                    if ui.add_sized(btn_size, filter_btn).clicked() {
                        state.filter_state.is_open = !state.filter_state.is_open;
                    }
                });
                
                ui.add_space(SPACE_BEFORE_SEPARATOR);
                ui.separator();
                ui.add_space(SPACE_AFTER_SEPARATOR);
            });

            if !state.enemies.is_empty() {
                state.enemy_list.show(ctx, ui, &state.enemies, &mut state.selected_enemy, &state.search_query, &state.filter_state);
            }
        });

    if state.selected_enemy != old_selection_id {
        state.detail_texture = None;
        state.detail_key.clear();
        state.anim_sheet = crate::global::formats::imgcut::SpriteSheet::default(); 
        state.model_data = None; 
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        if state.enemies.is_empty() {
             ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.4);
                    ui.set_max_width(400.0);
                    if state.scan_receiver.is_some() {
                        ui.spinner();
                        ui.add_space(10.0);
                        ui.label("Loading Unit Data...");
                    } else {
                        ui.heading("No Data Found");
                        ui.label(egui::RichText::new("Could not find any units in game/enemies").color(ui.visuals().weak_text_color()));
                        ui.add_space(5.0);
                        if ui.button("Retry Scan").clicked() {
                            state.restart_scan(settings.scanner_config());
                            ui.ctx().request_repaint();
                        }
                    }
                });
            });
            return;
        }

        let Some(selected_id) = state.selected_enemy else {
            ui.centered_and_justified(|ui| { ui.label("Select an Enemy"); });
            return;
        };

        let Some(enemy_entry) = state.enemies.iter().find(|e| e.id == selected_id) else {
            ui.centered_and_justified(|ui| { ui.spinner(); });
            return; 
        };

        master::show(
            ctx, ui, enemy_entry, &mut state.selected_tab, &mut state.mag_input,
            &mut state.magnification, settings, &mut state.icon_sheet,
            &mut state.anim_sheet, &mut state.model_data, &mut state.anim_viewer,
            &assets, &mut state.detail_texture, &mut state.detail_key,
        );
    });

    crate::features::enemy::ui::filter::show_popup(
        ctx, &mut state.filter_state, &mut state.icon_sheet,
        &assets, settings, &mut state.drag_guard,
    );
}