use std::path::Path;
use crate::app::BattleCatsApp;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::game::param::load_param;

impl BattleCatsApp {
    pub fn perform_full_data_reload(&mut self) {
        self.cat_list_state.texture_cache_version += 1;
        self.cat_list_state.anim_viewer.loaded_id.clear();
        self.cat_list_state.detail_texture = None;
        self.cat_list_state.detail_key.clear();
        
        self.cat_list_state.img015_sheets.clear();
        self.cat_list_state.img022_sheets.clear();
        self.cat_list_state.sprite_sheet = SpriteSheet::default();
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
        
        self.param = load_param(Path::new("game/tables"), &self.settings.general.language_priority).unwrap_or_default();
    }
}