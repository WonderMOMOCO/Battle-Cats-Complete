use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use nyanko::chapter::Category;

use kore::systems::animation::export::ExportFormat;
use kore::systems::animation::Placement;
use kore::domains::import::ImportSubTab;
use kore::domains::stage::{GlobalMapId, GlobalStageId};

use crate::domains::files::Mode;
use crate::widget::popup::StoredSize;

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppState {
    pub(crate) cat: CatListState,
    pub(crate) enemy: EnemyListState,
    pub(crate) stage: StageListState,
    pub(crate) mods: ModsListState,
    pub(crate) files: FilesState,
    pub(crate) data: GameDataState,
    pub(crate) animation: AnimState,
    pub(crate) studio: StudioState,
    pub(crate) notice: NoticeState,
    pub(crate) help: HelpState,
    pub(crate) popups: BTreeMap<String, StoredSize>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct StudioState {
    pub name: String,
    pub sheet: Option<PathBuf>,
    pub cuts: Option<PathBuf>,
    pub model: Option<PathBuf>,
    pub anims: Vec<PathBuf>,
    pub sealed: bool,
    pub target_mod: Option<String>,
    pub atlas: bool,
    pub clip: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct NoticeState {
    pub acknowledged: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct HelpState {
    pub active_page: usize,
    pub scroll_offset: f32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct ModsListState {
    pub selected_mod: Option<String>,
    pub search_query: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct FilesState {
    pub mount: Option<String>,
    pub mode: Mode,
    pub search_query: String,
    pub selected_file: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub(crate) struct CatListState {
    pub list_scroll_offset: f32,
    pub selected_cat: Option<u32>,
    pub selected_form: usize,
    pub search_query: String,
    pub talent_levels: HashMap<u32, HashMap<u8, u8>>,
    pub talent_history: VecDeque<u32>,
    pub current_level: i32,
    pub level_input: String,
}

impl Default for CatListState {
    fn default() -> Self {
        Self {
            list_scroll_offset: 0.0,
            selected_cat: None,
            selected_form: 0,
            search_query: String::new(),
            talent_levels: HashMap::new(),
            talent_history: VecDeque::new(),
            current_level: 1,
            level_input: String::from("1"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct EnemyListState {
    pub list_scroll_offset: f32,
    pub selected_enemy: Option<u32>,
    pub search_query: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct StageListState {
    pub selected_category: Option<Category>,
    pub selected_map: Option<GlobalMapId>,
    pub selected_stage: Option<GlobalStageId>,
    pub selected_crown: u8,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub(crate) struct GameDataState {
    pub adb_import_type_idx: usize,
    pub adb_region_idx: usize,
    pub selected_import: Option<ImportSubTab>,
}

impl Default for GameDataState {
    fn default() -> Self {
        Self {
            adb_import_type_idx: 0,
            adb_region_idx: 4,
            selected_import: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub(crate) struct AnimState {
    pub last_export_format: ExportFormat,
    pub last_export_quality: Option<i32>,
    pub last_export_compression: Option<i32>,
    pub controls_expanded: bool,
    pub export_background: bool,
    pub include_debug: bool,
    pub placement: Placement,
    pub pan: (f32, f32),
    pub zoom: f32,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            last_export_format: ExportFormat::Gif,
            last_export_quality: None,
            last_export_compression: None,
            controls_expanded: true,
            export_background: false,
            include_debug: false,
            placement: Placement::default(),
            pan: (0.0, 0.0),
            zoom: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_file_from_before_the_shared_camera_still_opens_at_a_usable_zoom() {
        // A zoom of zero would deserialize as a blank viewport rather than a default one.
        let held: AnimState = serde_json::from_str(r#"{"controls_expanded":true}"#).expect("it loads");

        assert_eq!(held.zoom, 1.0);
        assert_eq!(held.pan, (0.0, 0.0));
    }

    #[test]
    fn a_state_file_from_before_the_debug_export_leaves_it_switched_off() {
        // It opts a render into drawing overlays, so it has to default to off for
        // everyone who never asked for it, however old their state file is.
        let held: AnimState = serde_json::from_str(r#"{"controls_expanded":true}"#).expect("it loads");

        assert!(!held.include_debug);

        let held: AnimState = serde_json::from_str(r#"{"include_debug":true}"#).expect("it loads");

        assert!(held.include_debug, "and a deliberate choice round-trips");
    }

    #[test]
    fn the_background_preference_survives_a_restart_transparent_by_default() {
        let held: AnimState = serde_json::from_str(r#"{"controls_expanded":true}"#).expect("it loads");

        assert!(!held.export_background, "a format that can hold alpha still exports with it");

        let held: AnimState = serde_json::from_str(r#"{"export_background":true}"#).expect("it loads");

        assert!(held.export_background);
    }

    #[test]
    fn a_saved_offset_row_is_dropped_so_the_placement_default_wins() {
        // Older builds persisted the bare view as `offset_row: null`, and a null that
        // survives means the viewer keeps showing a placement the engine never draws.
        let held: AnimState = serde_json::from_str(r#"{"offset_row":null}"#).expect("it loads");

        assert_eq!(held.placement, Placement::Row(0));

        let held: AnimState = serde_json::from_str(r#"{"placement":"Bare"}"#).expect("it loads");

        assert_eq!(held.placement, Placement::Bare, "a deliberate choice still round-trips");
    }
}
