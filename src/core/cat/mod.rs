use eframe::egui;
use std::sync::mpsc::Receiver;
use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

pub mod scanner;
pub mod stats;
pub mod abilities; 
pub mod talents; 
pub mod watcher;
pub mod loader;

use crate::ui::views::cat_data::list::CatList; 
use crate::ui::views::cat_data as cat_detail;

use scanner::CatEntry;
use crate::data::global::imgcut::SpriteSheet; 
use crate::data::cat::skilldescriptions; 

use crate::data::cat::unitlevel::CatLevelCurve;
use crate::data::cat::unitbuy::UnitBuyRow;
use crate::data::cat::skillacquisition::TalentRaw;
use crate::core::settings::handle::ScannerConfig;

#[derive(Deserialize, Serialize, PartialEq, Clone, Copy)]
pub enum DetailTab {
    Abilities,
    Details,
    Talents,
}

impl Default for DetailTab {
    fn default() -> Self { Self::Abilities }
}

#[derive(Deserialize, Serialize)]
#[serde(default)] 
pub struct CatListState {
    #[serde(skip)] 
    pub cats: Vec<CatEntry>,          
    #[serde(skip)]
    pub incoming_cats: Vec<CatEntry>, 
    
    #[serde(skip)]
    pub is_cold_scan: bool,

    #[serde(skip)]
    pub last_update_time: Option<Instant>,

    #[serde(skip)]
    pub cached_level_curves: Option<Vec<CatLevelCurve>>,
    #[serde(skip)]
    pub cached_unit_buy: Option<HashMap<u32, UnitBuyRow>>,
    #[serde(skip)]
    pub cached_talents: Option<HashMap<u16, TalentRaw>>,
    #[serde(skip)]
    pub cached_evolve_text: Option<HashMap<u32, [Vec<String>; 4]>>,

    #[serde(alias = "persistent_id")] 
    pub selected_cat: Option<u32>,
    
    pub search_query: String, 
    pub selected_form: usize, 
    
    pub selected_detail_tab: DetailTab,

    pub level_input: String, 
    pub current_level: i32, 

    #[serde(skip)] 
    pub cat_list: CatList,
    
    #[serde(skip)]
    pub scan_receiver: Option<Receiver<CatEntry>>,
    
    #[serde(skip)]
    pub watchers: Option<watcher::CatWatchers>,
    #[serde(skip)]
    pub watch_receiver: Option<Receiver<PathBuf>>,
    
    #[serde(skip)]
    pub detail_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub detail_key: String, 
    
    #[serde(skip)]
    pub sprite_sheet: SpriteSheet, 
    #[serde(skip)]
    pub multihit_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub kamikaze_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub boss_wave_immune_texture: Option<egui::TextureHandle>,
    
    #[serde(skip)]
    pub talent_name_textures: HashMap<String, egui::TextureHandle>, 
    
    #[serde(skip)]
    pub gatya_item_textures: HashMap<i32, Option<egui::TextureHandle>>,

    #[serde(skip)]
    pub texture_cache_version: u64,

    #[serde(skip)]
    pub skill_descriptions: Option<Vec<String>>,

    #[serde(skip)]
    pub initialized: bool,

    pub talent_levels: HashMap<u32, HashMap<u8, u8>>,
    pub talent_history: VecDeque<u32>, 
}

impl Default for CatListState {
    fn default() -> Self {
        Self {
            cats: Vec::new(),
            incoming_cats: Vec::new(),
            is_cold_scan: false,
            last_update_time: None,
            cached_level_curves: None,
            cached_unit_buy: None,
            cached_talents: None,
            cached_evolve_text: None,
            selected_cat: None,
            cat_list: CatList::default(),
            scan_receiver: None,
            watchers: None,
            watch_receiver: None,
            search_query: String::new(),
            selected_form: 0,
            selected_detail_tab: DetailTab::default(),
            level_input: "50".to_string(),
            current_level: 50,
            detail_texture: None,
            detail_key: String::new(),
            sprite_sheet: SpriteSheet::default(), 
            multihit_texture: None,
            kamikaze_texture: None,
            boss_wave_immune_texture: None,
            talent_name_textures: HashMap::new(),
            gatya_item_textures: HashMap::new(), 
            texture_cache_version: 0, 
            skill_descriptions: None, 
            initialized: false, 
            talent_levels: HashMap::new(),
            talent_history: VecDeque::new(),
        }
    }
}

impl CatListState {
    pub fn init_watcher(&mut self, ctx: &egui::Context) {
        watcher::init(self, ctx);
    }

    pub fn handle_event(&mut self, ctx: &egui::Context, path: &PathBuf, config: ScannerConfig) {
        watcher::handle_event(self, ctx, path, config);
    }

    pub fn update_data(&mut self) {
        loader::update_data(self);
    }

    pub fn restart_scan(&mut self, config: ScannerConfig) {
        loader::restart_scan(self, config);
    }
}

pub fn show(ctx: &egui::Context, state: &mut CatListState, settings: &crate::core::settings::Settings) {
    if !state.initialized {
        state.initialized = true;
        state.init_watcher(ctx); 
        
        if !settings.unit_persistence {
            state.selected_cat = None;
            state.cat_list.reset_scroll();
        }
    }

    if state.scan_receiver.is_some() {
        state.update_data();
        ctx.request_repaint(); 
    }

    if state.skill_descriptions.is_none() {
        let path = std::path::Path::new("game/cats");
        state.skill_descriptions = Some(skilldescriptions::load(path, &settings.game_language));
    }

    egui::SidePanel::left("cat_list_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.add_space(12.0); 
            ui.vertical_centered(|ui| {
                ui.add(egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text("Search Cat...")
                    .desired_width(140.0));
            });
            ui.add_space(5.81); 
            ui.separator();

            let old_selection_id = state.selected_cat;
            
            if !state.cats.is_empty() {
                state.cat_list.show(ctx, ui, &state.cats, &mut state.selected_cat, &state.search_query, settings.high_banner_quality);
            } else if state.scan_receiver.is_some() {
                ui.centered_and_justified(|ui| { ui.spinner(); });
            }
            
            if state.selected_cat != old_selection_id {
                state.detail_texture = None; 
                state.detail_key.clear();

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
                        ui.label(egui::RichText::new("Could not find any units in game/cats.").color(ui.visuals().weak_text_color()));
                        ui.add_space(5.0);
                        ui.label("Check that 'unitbuy.csv' exists and unit folders are present.");
                        ui.add_space(15.0);
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
            if state.scan_receiver.is_some() {
                 ui.centered_and_justified(|ui| { ui.spinner(); });
            } else {
                 ui.centered_and_justified(|ui| { ui.label("Select a Unit"); });
            }
            return;
        };

        let Some(cat_entry) = state.cats.iter().find(|cat| cat.id == selected_id) else {
            ui.centered_and_justified(|ui| { ui.spinner(); }); 
            return;
        };
        
        let talent_map = state.talent_levels.entry(selected_id).or_default();

        cat_detail::show(
            ctx, 
            ui, 
            cat_entry, 
            &mut state.selected_form,
            &mut state.selected_detail_tab,
            &mut state.level_input,   
            &mut state.current_level, 
            &mut state.detail_texture, 
            &mut state.detail_key,
            &mut state.sprite_sheet,
            &mut state.multihit_texture,
            &mut state.kamikaze_texture,
            &mut state.boss_wave_immune_texture,
            &mut state.talent_name_textures,
            &mut state.gatya_item_textures, 
            state.skill_descriptions.as_ref(), 
            settings,
            talent_map,
            state.texture_cache_version
        );
    });
}