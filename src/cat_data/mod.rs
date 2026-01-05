use eframe::egui;
use std::sync::mpsc::Receiver;
use crate::functions::SoftReset;

pub mod scanner;
pub mod list;
pub mod cat; 
pub mod sprites;
pub mod stats;
pub mod abilities; 

use scanner::CatEntry;
use list::CatList;
use sprites::SpriteSheet;

pub struct CatListState {
    pub cats: Vec<CatEntry>,
    pub selected_cat: Option<u32>,
    pub cat_list: CatList,
    pub scan_receiver: Option<Receiver<CatEntry>>,
    pub search_query: String,
    pub selected_form: usize,
    
    pub level_input: String,
    pub current_level: i32,
    
    pub detail_texture: Option<egui::TextureHandle>,
    pub detail_key: String, 
    
    pub sprite_sheet: SpriteSheet, 
    pub multihit_texture: Option<egui::TextureHandle>,
}

impl Default for CatListState {
    fn default() -> Self {
        Self {
            cats: Vec::new(),
            selected_cat: None,
            cat_list: CatList::default(),
            scan_receiver: None,
            search_query: String::new(),
            selected_form: 0,
            
            level_input: "50".to_string(),
            current_level: 50,
            
            detail_texture: None,
            detail_key: String::new(),
            
            sprite_sheet: SpriteSheet::default(), 
            multihit_texture: None,
        }
    }
}

impl SoftReset for CatListState {
    fn reset(&mut self) {
        self.cats.clear();
        self.cat_list.clear_cache();
        self.detail_texture = None;
        self.detail_key.clear();
        self.selected_cat = None;
        self.selected_form = 0; 
        self.search_query.clear(); 
        self.level_input = "50".to_string();
        self.current_level = 50;
        self.sprite_sheet = SpriteSheet::default(); 
        self.multihit_texture = None; 
        self.scan_receiver = None;
    }
}

impl CatListState {
    pub fn update_data(&mut self) {
        // Guard: Exit if no active scanner
        let Some(rx) = &self.scan_receiver else { return };

        let mut new_data = false;
        while let Ok(entry) = rx.try_recv() {
            self.cats.push(entry);
            new_data = true;
        }

        if !new_data { return; }

        self.cats.sort_by_key(|c| c.id);

        if self.selected_cat.is_none() {
            self.selected_cat = Some(0);
        }

        let target_id = self.selected_cat.unwrap();
        let exists = self.cats.iter().any(|c| c.id == target_id);

        if !exists {
            return;
        }
        
    }

    pub fn restart_scan(&mut self, language: &str) {
        let current_selection = self.selected_cat.or(Some(0));
        
        self.reset();
        
        self.selected_cat = current_selection;
        
        self.scan_receiver = Some(scanner::start_scan(language.to_string()));
    }
}

pub fn show(ctx: &egui::Context, state: &mut CatListState, settings: &crate::settings::Settings) {
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
            ui.add_space(6.0); 
            ui.separator();

            let old_selection = state.selected_cat;
            
            state.cat_list.show(ctx, ui, &state.cats, &mut state.selected_cat, &state.search_query, settings.high_banner_quality);
            
            if state.selected_cat != old_selection {
                state.selected_form = 0; 
                state.detail_texture = None; 
                state.detail_key.clear();
            }
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        let Some(selected_id) = state.selected_cat else {
            ui.centered_and_justified(|ui| { ui.label("No Data Found"); });
            return;
        };

        let Some(cat) = state.cats.iter().find(|c| c.id == selected_id) else {
            ui.centered_and_justified(|ui| { ui.spinner(); });
            return;
        };
        
        cat::show(
            ctx, 
            ui, 
            cat, 
            &mut state.selected_form,
            &mut state.level_input,   
            &mut state.current_level, 
            &mut state.detail_texture, 
            &mut state.detail_key,
            &mut state.sprite_sheet,
            &mut state.multihit_texture,
            settings
        );
    });
}