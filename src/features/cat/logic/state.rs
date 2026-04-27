use eframe::egui;
use std::collections::HashSet;
use std::sync::mpsc::Receiver;
use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use super::scanner::CatEntry;
use super::loader;

use crate::features::cat::ui::list::CatList; 
use crate::features::cat::ui as cat_detail;
use crate::global::formats::imgcut::SpriteSheet; 
use crate::global::formats::mamodel::Model;
use crate::global::assets::CustomAssets;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::features::settings::logic::state::ScannerConfig;
use crate::features::settings::logic::Settings;
use crate::global::ui::shared::DragGuard; 
use crate::global::game::param::Param;

pub const TOP_PANEL_PADDING: f32 = 2.5;
pub const SEARCH_FILTER_GAP: f32 = 5.0;
pub const SPACE_BEFORE_SEPARATOR: f32 = 2.0;
pub const SPACE_AFTER_SEPARATOR: f32 = 2.0;

#[derive(Deserialize, Serialize, PartialEq, Clone, Copy)]
pub enum DetailTab {
    Abilities,
    Details,
    Talents,
    Animation,
}

impl Default for DetailTab {
    fn default() -> Self { Self::Abilities }
}

#[derive(Deserialize, Serialize)]
#[serde(default)] 
pub struct CatListState {
    #[serde(skip)] pub cats: Vec<CatEntry>,           
    #[serde(skip)] pub incoming_cats: Vec<CatEntry>, 
    #[serde(skip)] pub is_cold_scan: bool,
    #[serde(skip)] pub last_update_time: Option<Instant>,
    #[serde(alias = "persistent_id")] pub selected_cat: Option<u32>,
    pub search_query: String, 
    pub selected_form: usize, 
    pub selected_detail_tab: DetailTab,
    pub level_input: String, 
    pub current_level: i32,
    #[serde(skip)] pub cat_list: CatList,
    #[serde(skip)] pub scan_receiver: Option<Receiver<CatEntry>>,
    #[serde(skip)] pub active_scan_ids: HashSet<u32>,
    #[serde(skip)] pub detail_texture: Option<egui::TextureHandle>,
    #[serde(skip)] pub detail_key: String, 
    #[serde(skip)] pub img015_sheets: Vec<SpriteSheet>,   
    #[serde(skip)] pub img022_sheets: Vec<SpriteSheet>, 
    #[serde(skip)] pub sprite_sheet: SpriteSheet, 
    #[serde(skip)] pub model_data: Option<Model>,
    #[serde(skip)] pub anim_viewer: AnimViewer,
    #[serde(skip)] pub custom_assets: Option<CustomAssets>,
    #[serde(skip)] pub talent_name_textures: HashMap<String, egui::TextureHandle>, 
    #[serde(skip)] pub gatya_item_textures: HashMap<i32, Option<egui::TextureHandle>>,
    #[serde(skip)] pub texture_cache_version: u64,
    #[serde(skip)] pub initialized: bool,
    pub talent_levels: HashMap<u32, HashMap<u8, u8>>,
    pub talent_history: VecDeque<u32>, 
    #[serde(skip)] pub filter_state: crate::features::cat::ui::filter::CatFilterState,
    #[serde(skip)] pub drag_guard: DragGuard,
    #[serde(skip)] pub saved_pre_ultra_level: Option<(i32, String)>,
    #[serde(skip)] pub is_in_ultra_state: bool,
}

impl Default for CatListState {
    fn default() -> Self {
        Self {
            cats: Vec::new(),
            incoming_cats: Vec::new(),
            is_cold_scan: false,
            last_update_time: None,
            selected_cat: None,
            cat_list: CatList::default(),
            scan_receiver: None,
            search_query: String::new(),
            selected_form: 0,
            selected_detail_tab: DetailTab::default(),
            level_input: "50".to_string(),
            current_level: 50,
            active_scan_ids: HashSet::new(),
            detail_texture: None,
            detail_key: String::new(),
            img015_sheets: Vec::new(), 
            img022_sheets: Vec::new(),
            sprite_sheet: SpriteSheet::default(), 
            model_data: None,
            anim_viewer: AnimViewer::default(),
            custom_assets: None,
            talent_name_textures: HashMap::new(),
            gatya_item_textures: HashMap::new(), 
            texture_cache_version: 0, 
            initialized: false, 
            talent_levels: HashMap::new(),
            talent_history: VecDeque::new(),
            filter_state: crate::features::cat::ui::filter::CatFilterState::default(),
            drag_guard: DragGuard::default(),
            saved_pre_ultra_level: None,
            is_in_ultra_state: false,
        }
    }
}

impl CatListState {
    pub fn update_data(&mut self) {
        loader::update_data(self);
    }

    pub fn restart_scan(&mut self, config: ScannerConfig) {
        loader::restart_scan(self, config);
    }
}

pub fn show(ctx: &egui::Context, state: &mut CatListState, settings: &mut Settings, param: &Param) {
    if state.custom_assets.is_none() {
        state.custom_assets = Some(CustomAssets::new(ctx));
    }
    let assets = state.custom_assets.as_ref().unwrap().clone();

    if !state.initialized {
        state.initialized = true;
    }

    if state.scan_receiver.is_some() {
        state.update_data();
        ctx.request_repaint(); 
    }

    egui::SidePanel::left("cat_list_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(TOP_PANEL_PADDING); 
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let search_response = ui.add(egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text(egui::RichText::new("Search Cat...").color(egui::Color32::GRAY))
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

            let old_selection_id = state.selected_cat;
            if !state.cats.is_empty() {
                state.cat_list.show(
                    ctx, ui, &state.cats, &mut state.selected_cat, 
                    &state.search_query, &state.filter_state, 
                    settings.cat_data.high_banner_quality
                );
            }
            
            if state.selected_cat != old_selection_id {
                state.detail_texture = None; 
                state.detail_key.clear();
                state.sprite_sheet = SpriteSheet::default();
                state.model_data = None;
                state.saved_pre_ultra_level = None;
                state.is_in_ultra_state = false;

                if let Some(new_id) = state.selected_cat {
                    if let Some(pos) = state.talent_history.iter().position(|&id| id == new_id) {
                        state.talent_history.remove(pos);
                    }
                    state.talent_history.push_back(new_id);
                    while state.talent_history.len() > 3 {
                        if let Some(popped_id) = state.talent_history.pop_front() {
                            state.talent_levels.remove(&popped_id);
                        }
                    }

                    if let Some(new_cat) = state.cats.iter().find(|c| c.id == new_id) {
                        let mut max_form_index = 0;
                        for (i, &exists) in new_cat.forms.iter().enumerate() {
                            if exists { max_form_index = i; }
                        }
                        if state.selected_form > max_form_index || !new_cat.forms[state.selected_form] {
                            state.selected_form = max_form_index;
                        }
                        if state.selected_detail_tab == DetailTab::Talents {
                            let form_valid = state.selected_form >= 2;
                            let has_data = new_cat.talent_data.is_some();
                            if !form_valid || !has_data {
                                state.selected_detail_tab = DetailTab::Abilities;
                            }
                        }

                        if settings.cat_data.auto_level_calculations {
                            let base_max = new_cat.unit_buy.level_cap_catseye;
                            let plus_max = new_cat.unit_buy.level_cap_plus;
                            let is_legend_rare = new_cat.unit_buy.rarity == 5;
                            let is_normal_rare = new_cat.unit_buy.rarity == 0;
                            
                            if is_legend_rare {
                                state.current_level = 50;
                                state.level_input = "50".to_string();
                            } else if base_max == 1 || (plus_max >= 5 && plus_max <= 65) || is_normal_rare {
                                state.current_level = base_max + plus_max;
                                if plus_max > 0 {
                                    state.level_input = format!("{}+{}", base_max, plus_max);
                                } else {
                                    state.level_input = base_max.to_string();
                                }
                            } else if base_max > 50 {
                                state.current_level = 50;
                                state.level_input = "50".to_string();
                            } else {
                                state.current_level = base_max;
                                state.level_input = base_max.to_string();
                            }
                        } else {
                            state.current_level = settings.cat_data.default_level;
                            state.level_input = settings.cat_data.default_level.to_string();
                        }
                    }
                }
            }
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        if state.cats.is_empty() {
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
                        ui.label(egui::RichText::new("Could not find any units in game/cats").color(ui.visuals().weak_text_color()));
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

        let Some(selected_id) = state.selected_cat else {
             ui.centered_and_justified(|ui| { ui.label("Select a Unit"); });
             return;
        };

        let Some(cat_entry) = state.cats.iter().find(|cat| cat.id == selected_id) else {
            ui.centered_and_justified(|ui| { ui.spinner(); }); 
            return;
        };
        
        let talent_map = state.talent_levels.entry(selected_id).or_default();
        let prev_form = state.selected_form;

        cat_detail::show(
            ctx, ui, cat_entry, 
            &mut state.selected_form, &mut state.selected_detail_tab,
            &mut state.level_input, &mut state.current_level, 
            &mut state.detail_texture, &mut state.detail_key,
            &mut state.img015_sheets, &mut state.img022_sheets, &mut state.sprite_sheet,
            &mut state.model_data, &mut state.anim_viewer,
            &assets,
            &mut state.talent_name_textures, &mut state.gatya_item_textures, 
            Some(cat_entry.skill_descriptions.as_ref()), settings, talent_map,
            cat_entry.talent_costs.as_ref(),
            state.texture_cache_version,
            param,
        );

        let mut current_ultra_state = state.selected_form == 3;
        if state.selected_form >= 2 {
            if let Some(levels) = state.talent_levels.get(&selected_id) {
                if let Some(t_data) = &cat_entry.talent_data {
                    for (idx, group) in t_data.groups.iter().enumerate() {
                        if group.limit == 1 { 
                            if let Some(&lvl) = levels.get(&(idx as u8)) {
                                if lvl > 0 { current_ultra_state = true; break; }
                            }
                        }
                    }
                } else if levels.iter().any(|(&idx, &lvl)| idx >= 5 && lvl > 0) {
                    current_ultra_state = true;
                }
            }
        }

        if settings.cat_data.bump_ultra_60 {
            if !state.is_in_ultra_state && current_ultra_state {
                state.saved_pre_ultra_level = Some((state.current_level, state.level_input.clone()));
                if state.current_level < 60 {
                    state.current_level = 60;
                    state.level_input = "60".to_string();
                }
            } else if state.is_in_ultra_state && !current_ultra_state {
                if let Some((saved_lvl, saved_str)) = state.saved_pre_ultra_level.take() {
                    let expected_ultra_level = if saved_lvl < 60 { 60 } else { saved_lvl };
                    if state.current_level == expected_ultra_level {
                        state.current_level = saved_lvl;
                        state.level_input = saved_str;
                    }
                }
            }
            state.is_in_ultra_state = current_ultra_state;
        } else {
            state.is_in_ultra_state = current_ultra_state;
            state.saved_pre_ultra_level = None;
        }

        if state.selected_form != prev_form {
            state.sprite_sheet = SpriteSheet::default();
            state.model_data = None;
        }
    });
    
    crate::features::cat::ui::filter::show_popup(
        ctx, &mut state.filter_state, &mut state.img015_sheets,
        &assets,
        settings, &mut state.drag_guard,
    );
}