pub mod desktop;
pub mod lang;
pub mod nightly;
pub mod pem;

use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use md5::{Digest, Md5};
use nyanko::common::Region;
use nyanko::graphics::tools::crash::Side;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::common::io::json;
use crate::common::keys::sanitize;
use crate::systems::animation::Loop;

const EXPECTED_HASHES: [(&str, &str); 4] = [
    ("bac299d3cf278544782427ff7c71ef58", "6910fae125547fd957a505c67e1c72bd"),
    ("b9e48b02312e5b3dd60194a03157d70c", "45cad482726268e341f5759230ce8cff"),
    ("264a0ffd5f69d257284b93ae881ce2b6", "213cecb58af008964303ecb2cf0f5373"),
    ("3d22eafdcc4fc2a1379b103970b36217", "4cacdb0839634116caaf0b966638865b"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UpdateMode {
    #[default]
    Prompt,
    Ignore,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Debug)]
pub enum SidebarBehavior {
    #[default]
    Cover,
    Push,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct StageDataSettings {
    pub sidebar_behavior: SidebarBehavior,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Debug)]
pub enum ExportBehavior {
    #[default]
    Automatic,
    Create,
    Update,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub cat_data: CatDataSettings,
    pub enemy_data: EnemyDataSettings,
    pub game_data: GameDataSettings,
    pub animation: AnimSettings,
    pub studio: StudioSettings,
    pub mods: ModsSettings,
    pub stages: StageDataSettings,
    pub files: FilesSettings,
    pub utilities: UtilitiesSettings,
    pub window: WindowSettings,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum FrameCount {
    #[default]
    Automatic,
    Continuous,
}

impl FrameCount {
    pub const ALL: [FrameCount; 2] = [FrameCount::Automatic, FrameCount::Continuous];

    pub fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Automatic",
            Self::Continuous => "Continuous",
        }
    }

    pub fn looping(self) -> Loop {
        match self {
            Self::Automatic => Loop::Auto,
            Self::Continuous => Loop::Continuous,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct UtilitiesSettings {
    pub frame_count: FrameCount,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(default)]
pub struct WindowSettings {
    pub width: f32,
    pub height: f32,
    pub fullscreen: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self { width: 800.0, height: 600.0, fullscreen: false }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FilesSettings {
    pub unlock_game_mount: bool,
    pub utf8_mode: Utf8Mode,
    pub context_scope: ContextScope,
    pub editor_mode: EditorMode,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ContextScope {
    #[default]
    Broad,
    Specific,
}

impl ContextScope {
    pub const ALL: [Self; 2] = [Self::Broad, Self::Specific];

    pub fn hint(self) -> &'static str {
        match self {
            Self::Specific => "Only allows the editing of the assets and files related to areas you right click on",
            Self::Broad => "Allows you to edit any file reachable from the current loaded context from right clicking anywhere",
        }
    }
}

impl std::fmt::Display for ContextScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Specific => "Specific",
            Self::Broad => "Broad",
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum EditorMode {
    #[default]
    Resolved,
    Raw,
}

impl EditorMode {
    pub const ALL: [Self; 2] = [Self::Resolved, Self::Raw];

    pub fn hint(self) -> &'static str {
        match self {
            Self::Resolved => "Presents each attribute the way the game means it, converting the columns the engine stores at a different scale",
            Self::Raw => "Presents every attribute exactly as it is written in the file, which is the only way to edit a value the conversion cannot represent",
        }
    }
}

impl std::fmt::Display for EditorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Resolved => "Resolved",
            Self::Raw => "Raw",
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Utf8Mode {
    #[default]
    Fill,
    Virtual,
}

impl Utf8Mode {
    pub const ALL: [Self; 2] = [Self::Fill, Self::Virtual];

    pub fn hint(self) -> &'static str {
        match self {
            Self::Fill => "Loads the whole file at once, so you can select and copy all of it, but very long files may stutter",
            Self::Virtual => "Only keeps the visible part of the file loaded, so long files stay smooth, but selecting and copying is limited to what is on screen",
        }
    }
}

impl std::fmt::Display for Utf8Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Fill => "Fill",
            Self::Virtual => "Virtual",
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ModsSettings {
    pub export_behavior: ExportBehavior,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralSettings {
    #[serde(default = "lang::default_priority")]
    pub language_priority: Vec<String>,
    pub update_mode: UpdateMode,
    pub enable_nightly: bool,
    pub enable_logging: bool,
    pub ignore_conflict_errors: bool,
    pub ignore_watcher_failure: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language_priority: lang::default_priority(),
            update_mode: UpdateMode::default(),
            enable_nightly: false,
            enable_logging: true,
            ignore_conflict_errors: false,
            ignore_watcher_failure: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CatDataSettings {
    pub preferred_banner_form: usize,
    pub show_invalid_cats: bool,
    pub expand_spirit_details: bool,
    pub default_level: i32,
    pub auto_level_calculations: bool,
    pub bump_ultra_60: bool,
}

impl Default for CatDataSettings {
    fn default() -> Self {
        Self {
            preferred_banner_form: 3,
            show_invalid_cats: false,
            expand_spirit_details: false,
            default_level: 50,
            auto_level_calculations: true,
            bump_ultra_60: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EnemyDataSettings {
    pub show_invalid_enemies: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ImportStructure {
    #[default]
    Bcc,
    Flat,
}

impl ImportStructure {
    pub const ALL: [Self; 2] = [Self::Bcc, Self::Flat];

    pub fn hint(self) -> &'static str {
        match self {
            Self::Bcc => "Import into an understandable file structure where assets are easy to discover",
            Self::Flat => "Import all files into the root of the \"game\" folder for faster routing speeds",
        }
    }
}

impl std::fmt::Display for ImportStructure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Bcc => "BCC",
            Self::Flat => "Flat",
        };
        write!(f, "{}", label)
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GameDataSettings {
    pub enforce_key_validation: bool,
    pub ignore_modified_app: bool,
    pub import_structure: ImportStructure,
}

impl Default for GameDataSettings {
    fn default() -> Self {
        Self {
            enforce_key_validation: true,
            ignore_modified_app: false,
            import_structure: ImportStructure::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum Tier {
    #[default]
    None,
    Line,
    Bold,
}

impl Tier {
    pub const ALL: [Tier; 3] = [Tier::Bold, Tier::Line, Tier::None];

    pub fn label(self) -> &'static str {
        match self {
            Tier::None => "None",
            Tier::Line => "Line",
            Tier::Bold => "Bold",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Tier::None => 0,
            Tier::Line => 1,
            Tier::Bold => 2,
        }
    }

    pub fn of(on: bool) -> Tier {
        match on {
            true => Tier::Line,
            false => Tier::None,
        }
    }

    pub fn on(self) -> bool {
        self != Tier::None
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Scope {
    #[default]
    Rig,
    Hierarchy,
    Selected,
    None,
}

impl Scope {
    pub const ALL: [Scope; 4] = [Scope::Rig, Scope::Hierarchy, Scope::Selected, Scope::None];

    pub fn label(self) -> &'static str {
        match self {
            Scope::Rig => "Rig",
            Scope::Hierarchy => "Hierarchy",
            Scope::Selected => "Selected",
            Scope::None => "None",
        }
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Shown {
    Visible,
    #[default]
    Invisible,
}

impl Shown {
    pub const ALL: [Shown; 2] = [Shown::Visible, Shown::Invisible];

    pub fn label(self) -> &'static str {
        match self {
            Shown::Visible => "Visible",
            Shown::Invisible => "Invisible",
        }
    }

    pub fn of(on: bool) -> Shown {
        match on {
            true => Shown::Visible,
            false => Shown::Invisible,
        }
    }

    pub fn on(self) -> bool {
        self == Shown::Visible
    }
}

impl std::fmt::Display for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Faults {
    None,
    Attack,
    #[default]
    Either,
    Cat,
    Enemy,
}

impl Faults {
    pub const ALL: [Faults; 5] =
        [Faults::None, Faults::Attack, Faults::Either, Faults::Cat, Faults::Enemy];

    pub fn label(self) -> &'static str {
        match self {
            Faults::None => "None",
            Faults::Attack => "Attack",
            Faults::Either => "Either",
            Faults::Cat => "Cat",
            Faults::Enemy => "Enemy",
        }
    }

    pub fn side(self) -> Option<Side> {
        match self {
            Faults::None => None,
            Faults::Attack | Faults::Either => Some(Side::Either),
            Faults::Cat => Some(Side::Cat),
            Faults::Enemy => Some(Side::Enemy),
        }
    }

    pub fn attacking(self) -> bool {
        self == Faults::Attack
    }

    pub fn of(side: Side) -> Faults {
        match side {
            Side::Either => Faults::Either,
            Side::Cat => Faults::Cat,
            Side::Enemy => Faults::Enemy,
        }
    }
}

impl std::fmt::Display for Faults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

pub const ONION_LIFE: i32 = 15;
pub const ONION_ALPHA: i32 = 100;

const TINT_GAIN: f32 = 2.0;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Switch {
    Enabled,
    #[default]
    Disabled,
}

impl Switch {
    pub const ALL: [Switch; 2] = [Switch::Enabled, Switch::Disabled];

    pub fn label(self) -> &'static str {
        match self {
            Switch::Enabled => "Enabled",
            Switch::Disabled => "Disabled",
        }
    }

    pub fn on(self) -> bool {
        self == Switch::Enabled
    }
}

impl std::fmt::Display for Switch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

pub fn washed(color: &str) -> [f32; 4] {
    inked(color).map_or([0.0, 0.0, 0.0, 0.0], |[red, green, blue]| {
        [red * TINT_GAIN, green * TINT_GAIN, blue * TINT_GAIN, 1.0]
    })
}

pub fn inked(color: &str) -> Option<[f32; 3]> {
    let held = color.trim().trim_start_matches('#');

    if held.is_empty() || held.len() > 6 || !held.chars().all(|glyph| glyph.is_ascii_hexdigit()) {
        return None;
    }

    let widened = format!("{:0<6}", held);

    let channel = |at: usize| u8::from_str_radix(widened.get(at..at + 2)?, 16).ok();

    Some([
        f32::from(channel(0)?) / 255.0,
        f32::from(channel(2)?) / 255.0,
        f32::from(channel(4)?) / 255.0,
    ])
}
pub const ONION_GAP: i32 = 5;
pub const ONION_MOST: i32 = 8;

#[derive(Clone, Copy, Default)]
pub struct Overlays {
    pub rig: Tier,
    pub selected: Tier,
    pub hierarchy: Tier,
    pub origin: Shown,
    pub world: Shown,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScrubBehavior {
    #[default]
    Pause,
    Retain,
}

impl ScrubBehavior {
    pub const ALL: [ScrubBehavior; 2] = [ScrubBehavior::Pause, ScrubBehavior::Retain];

    pub fn label(self) -> &'static str {
        match self {
            ScrubBehavior::Pause => "Pause",
            ScrubBehavior::Retain => "Retain",
        }
    }
}

impl std::fmt::Display for FrameCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::fmt::Display for ScrubBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct StudioSettings {
    pub gizmo: crate::systems::animation::posing::Hand,
    pub entity: Scope,
    pub origin: Shown,
    pub rig: Tier,
    pub selected: Tier,
    pub hierarchy: Tier,
    pub world: Shown,
    pub onion: Switch,
    pub onion_before_life: String,
    pub onion_after_life: String,
    pub onion_before_color: String,
    pub onion_after_color: String,
    pub onion_gap: String,
    pub onion_alpha: String,
    pub faults: Faults,
    pub auto_faults: bool,
    pub scrub: ScrubBehavior,
    pub frame_count: FrameCount,
}

impl Default for StudioSettings {
    fn default() -> Self {
        Self {
            gizmo: crate::systems::animation::posing::Hand::default(),
            entity: Scope::default(),
            origin: Shown::default(),
            rig: Tier::default(),
            selected: Tier::default(),
            hierarchy: Tier::default(),
            world: Shown::default(),
            onion: Switch::default(),
            onion_before_life: String::new(),
            onion_after_life: String::new(),
            onion_before_color: String::new(),
            onion_after_color: String::new(),
            onion_gap: String::new(),
            onion_alpha: String::new(),
            faults: Faults::default(),
            auto_faults: true,
            scrub: ScrubBehavior::default(),
            frame_count: FrameCount::default(),
        }
    }
}

impl StudioSettings {
    pub fn overlays(&self) -> Overlays {
        Overlays {
            rig: self.rig,
            selected: self.selected,
            hierarchy: self.hierarchy,
            origin: self.origin,
            world: self.world,
        }
    }

    pub fn onion_behind(&self) -> Option<i32> {
        counted(&self.onion_before_life)
    }

    pub fn onion_ahead(&self) -> Option<i32> {
        counted(&self.onion_after_life)
    }

    pub fn onion_step(&self) -> Option<i32> {
        counted(&self.onion_gap)
    }

    pub fn onion_skins(&self, life: i32) -> i32 {
        self.onion_step().map_or(0, |gap| ((life.max(0) + gap - 1) / gap).min(ONION_MOST))
    }

    pub fn onion_alpha(&self) -> f32 {
        counted(&self.onion_alpha).unwrap_or(ONION_ALPHA).min(ONION_ALPHA) as f32 / ONION_ALPHA as f32
    }

    pub fn onion_fade(&self, aged: f32, life: i32) -> f32 {
        if life <= 0 {
            return 0.0;
        }

        let left = 1.0 - aged / life as f32;

        (left * self.onion_alpha()).max(0.0)
    }

    pub fn onion_before_wash(&self) -> [f32; 4] {
        washed(&self.onion_before_color)
    }

    pub fn onion_after_wash(&self) -> [f32; 4] {
        washed(&self.onion_after_color)
    }

    pub fn onion_on(&self) -> bool {
        let skinned = self.onion_behind().is_some() || self.onion_ahead().is_some();

        self.onion.on() && skinned && self.onion_step().is_some()
    }

    pub fn onion_arm(&mut self, live: bool) {
        self.onion = match live {
            true => Switch::Enabled,
            false => Switch::Disabled,
        };

        if !live {
            return;
        }

        if counted(&self.onion_before_life).is_none() && counted(&self.onion_after_life).is_none() {
            self.onion_before_life = ONION_LIFE.to_string();
        }

        for (slot, fallback) in [(&mut self.onion_gap, ONION_GAP), (&mut self.onion_alpha, ONION_ALPHA)]
        {
            if counted(slot).is_none() {
                *slot = fallback.to_string();
            }
        }
    }
}

fn counted(text: &str) -> Option<i32> {
    text.trim().parse::<i32>().ok().filter(|held| *held >= 1)
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AnimSettings {
    pub auto_set_camera_region: bool,
    pub bounds_cull: i32,
    pub default_showcase_walk: i32,
    pub default_showcase_idle: i32,
    pub default_showcase_kb: i32,
    pub show_rig: bool,
    pub show_origin: bool,
    pub show_world: bool,
}

impl Default for AnimSettings {
    fn default() -> Self {
        Self {
            auto_set_camera_region: false,
            bounds_cull: 100,
            default_showcase_walk: 90,
            default_showcase_idle: 90,
            default_showcase_kb: 60,
            show_rig: false,
            show_origin: false,
            show_world: false,
        }
    }
}

impl AnimSettings {
    pub fn overlays(&self) -> Overlays {
        Overlays {
            rig: Tier::of(self.show_rig),
            selected: Tier::None,
            hierarchy: Tier::None,
            origin: Shown::of(self.show_origin),
            world: Shown::of(self.show_world),
        }
    }
}

#[derive(Clone, Debug, Hash)]
pub struct ScannerConfig {
    pub language_priority: Vec<String>,
    pub active_mod: Option<String>,
    pub preferred_form: usize,
    pub show_invalid_cats: bool,
    pub show_invalid_enemies: bool,
    pub pristine: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ImportConfig {
    pub structure: ImportStructure,
    pub enforce_validation: bool,
    pub ignore_modified_app: bool,
}

impl Settings {
    pub fn scanner_config(&self, active_mod: Option<String>) -> ScannerConfig {
        ScannerConfig {
            language_priority: self.general.language_priority.clone(),
            active_mod,
            preferred_form: self.cat_data.preferred_banner_form,
            show_invalid_cats: self.cat_data.show_invalid_cats,
            show_invalid_enemies: self.enemy_data.show_invalid_enemies,
            pristine: false,
        }
    }

    pub fn show_invalid_enemies(&self) -> bool {
        self.enemy_data.show_invalid_enemies
    }

    pub fn import_config(&self) -> ImportConfig {
        ImportConfig {
            structure: self.game_data.import_structure,
            enforce_validation: self.game_data.enforce_key_validation,
            ignore_modified_app: self.game_data.ignore_modified_app,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum RuleHandling {
    Include,
    Only,
    Ignore,
}

impl RuleHandling {
    pub fn all() -> [Self; 3] {
        [Self::Include, Self::Only, Self::Ignore]
    }

}

impl std::fmt::Display for RuleHandling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Include => "Include",
            Self::Only => "Only",
            Self::Ignore => "Ignore",
        };
        write!(f, "{}", label)
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleSource {
    #[default]
    Default,
    Custom,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionRule {
    pub pattern: String,
    pub extension: String,
    pub handling: RuleHandling,
    pub languages: IndexMap<String, bool>,
}

impl Default for ExceptionRule {
    fn default() -> Self {
        let mut languages = IndexMap::new();
        for lang in ["en", "ja", "tw", "ko", "es", "de", "fr", "it", "th"] {
            languages.insert(lang.to_string(), false);
        }
        Self {
            pattern: String::new(),
            extension: String::new(),
            handling: RuleHandling::Include,
            languages,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionList {
    #[serde(default)]
    pub source: RuleSource,
    pub rules: Vec<ExceptionRule>,
}

impl Default for ExceptionList {
    fn default() -> Self {
        let default_json = include_str!("settings/exceptions.json");
        serde_json::from_str(default_json).unwrap_or_else(|_| ExceptionList {
            source: RuleSource::Default,
            rules: vec![ExceptionRule::default()],
        })
    }
}

impl ExceptionList {
    pub fn save(&mut self) {
        self.source = RuleSource::Custom;
        if let Err(err) = json::save("exceptions.json", self) {
            warn!("Failed to save exceptions.json: {}", err);
        }
    }

    pub fn load_or_default() -> Self {
        json::load("exceptions.json").unwrap_or_default()
    }

    pub fn save_to_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        self.source = RuleSource::Custom;
        let json_string = serde_json::to_string_pretty(self)?;
        fs::write(path, json_string)
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn sync_on_boot() {
        let disk_list = json::load::<ExceptionList>("exceptions.json");

        let needs_overwrite = disk_list.is_none_or(|list| list.source == RuleSource::Default);

        if needs_overwrite {
            info!("Syncing default exceptions.json to disk...");
            let default_list = Self::default();
            if let Err(err) = json::save("exceptions.json", &default_list) {
                warn!("Failed to save exceptions.json: {}", err);
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct RegionKey {
    pub key: String,
    pub iv: String,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct UserKeys {
    pub ja: RegionKey,
    pub en: RegionKey,
    pub tw: RegionKey,
    pub ko: RegionKey,
}

impl UserKeys {
    pub fn load() -> Self {
        json::load("keys.json").unwrap_or_default()
    }

    pub fn save(&self) {
        if let Err(err) = json::save("keys.json", self) {
            warn!("Failed to save keys.json: {}", err);
        }
    }

    pub fn is_empty(&self) -> bool {
        ![Region::Ja, Region::En, Region::Tw, Region::Ko]
            .iter()
            .any(|&region| self.has_key_for(region))
    }

    fn has_key_for(&self, region: Region) -> bool {
        match region {
            Region::Ja => !self.ja.key.is_empty() && !self.ja.iv.is_empty(),
            Region::En => !self.en.key.is_empty() && !self.en.iv.is_empty(),
            Region::Tw => !self.tw.key.is_empty() && !self.tw.iv.is_empty(),
            Region::Ko => !self.ko.key.is_empty() && !self.ko.iv.is_empty(),
        }
    }

    pub(crate) fn as_tuples(&self) -> Vec<(String, String, Region)> {
        let mut key_tuples = Vec::new();
        if self.has_key_for(Region::Ja) { key_tuples.push((sanitize(&self.ja.key), sanitize(&self.ja.iv), Region::Ja)); }
        if self.has_key_for(Region::En) { key_tuples.push((sanitize(&self.en.key), sanitize(&self.en.iv), Region::En)); }
        if self.has_key_for(Region::Tw) { key_tuples.push((sanitize(&self.tw.key), sanitize(&self.tw.iv), Region::Tw)); }
        if self.has_key_for(Region::Ko) { key_tuples.push((sanitize(&self.ko.key), sanitize(&self.ko.iv), Region::Ko)); }
        key_tuples
    }

    pub fn validate(&self) -> [(bool, bool); 4] {
        let check_hash = |input_value: &str, expected_hash: &str| -> bool {
            if expected_hash.is_empty() { return true; }
            let clean_value = sanitize(input_value);
            if clean_value.is_empty() { return false; }

            let hash_result = format!("{:x}", Md5::digest(clean_value.as_bytes()));
            hash_result == expected_hash
        };

        [
            (check_hash(&self.ja.key, EXPECTED_HASHES[0].0), check_hash(&self.ja.iv, EXPECTED_HASHES[0].1)),
            (check_hash(&self.en.key, EXPECTED_HASHES[1].0), check_hash(&self.en.iv, EXPECTED_HASHES[1].1)),
            (check_hash(&self.tw.key, EXPECTED_HASHES[2].0), check_hash(&self.tw.iv, EXPECTED_HASHES[2].1)),
            (check_hash(&self.ko.key, EXPECTED_HASHES[3].0), check_hash(&self.ko.iv, EXPECTED_HASHES[3].1)),
        ]
    }
}
#[cfg(test)]
mod onion_tests {
    use super::*;

    fn seeded(before: &str, after: &str) -> StudioSettings {
        StudioSettings {
            onion: Switch::Enabled,
            onion_before_life: before.to_owned(),
            onion_after_life: after.to_owned(),
            onion_gap: ONION_GAP.to_string(),
            onion_alpha: ONION_ALPHA.to_string(),
            ..StudioSettings::default()
        }
    }

    #[test]
    fn the_switch_gates_everything_even_with_valid_numbers() {
        let mut held = seeded("3", "");

        assert!(held.onion_on());

        held.onion = Switch::Disabled;

        assert!(!held.onion_on(), "the combo is the on/off, not the fields");
    }

    #[test]
    fn arming_seeds_only_what_is_missing_and_disarming_keeps_it() {
        let mut held = StudioSettings::default();

        held.onion_arm(true);

        assert!(held.onion_on());
        assert_eq!(held.onion_behind(), Some(ONION_LIFE));
        assert_eq!(held.onion_ahead(), None, "the trailing direction is the default");
        assert_eq!(held.onion_step(), Some(ONION_GAP));

        held.onion_before_life = "7".to_owned();
        held.onion_arm(false);

        assert!(!held.onion_on());
        assert_eq!(held.onion_before_life, "7", "disarming never clears what was typed");

        held.onion_arm(true);

        assert_eq!(held.onion_before_life, "7", "and arming does not overwrite it");
    }

    #[test]
    fn a_skin_fades_over_its_duration_and_is_gone_at_the_end_of_it() {
        // Duration is a lifetime in frames, and the age handed in is how far the playhead
        // has walked since the skin was laid down — not a fixed multiple of the delay.
        let held = seeded("20", "");

        assert_eq!(held.onion_fade(0.0, 20), 1.0, "a skin is born at full opacity");
        assert_eq!(held.onion_fade(5.0, 20), 0.75);
        assert_eq!(held.onion_fade(10.0, 20), 0.5);
        assert_eq!(held.onion_fade(20.0, 20), 0.0, "exactly at the end of its life");
        assert_eq!(held.onion_fade(40.0, 20), 0.0, "and it never goes negative");
    }

    #[test]
    fn how_many_skins_fit_is_derived_from_the_duration_and_the_delay() {
        // Count is not a knob any more: a direction shows however many delays fit inside
        // its own duration, and the cap is a cost limit rather than a setting.
        let held = seeded("20", "3");

        assert_eq!(held.onion_step(), Some(ONION_GAP), "delay is 5");
        assert_eq!(held.onion_skins(20), 4);
        assert_eq!(held.onion_skins(3), 1, "a duration under one delay still lays one skin");
        assert_eq!(held.onion_skins(0), 0);
        assert_eq!(held.onion_skins(10_000), ONION_MOST, "and the cost cap holds");
    }

    #[test]
    fn the_two_directions_carry_their_own_duration() {
        let held = seeded("30", "5");

        assert_eq!(held.onion_behind(), Some(30));
        assert_eq!(held.onion_ahead(), Some(5));
        assert_eq!(held.onion_skins(30), 6, "a long tail behind");
        assert_eq!(held.onion_skins(5), 1, "and a hint ahead");
    }

    #[test]
    fn either_direction_alone_is_enough_to_draw() {
        assert!(seeded("3", "").onion_on());
        assert!(seeded("", "3").onion_on());
        assert!(!seeded("", "").onion_on());
        assert!(!seeded("0", "0").onion_on());
    }

    #[test]
    fn opacity_reads_as_a_share() {
        assert_eq!(seeded("3", "").onion_alpha(), 1.0);

        let half = StudioSettings { onion_alpha: "50".to_owned(), ..StudioSettings::default() };

        assert_eq!(half.onion_alpha(), 0.5);
    }

    #[test]
    fn a_half_typed_colour_pads_with_zeros_so_it_shows_at_once() {
        assert_eq!(inked("#ff0000"), Some([1.0, 0.0, 0.0]));
        assert_eq!(inked("ff0000"), Some([1.0, 0.0, 0.0]));
        assert_eq!(inked("  0000ff "), Some([0.0, 0.0, 1.0]));

        // Every prefix of a red is already some colour, so typing recolours as you go.
        assert_eq!(inked("f"), Some([240.0 / 255.0, 0.0, 0.0]), "one digit already reads bright");
        assert_eq!(inked("ff"), Some([1.0, 0.0, 0.0]));
        assert_eq!(inked("ff00"), Some([1.0, 0.0, 0.0]));

        assert_eq!(inked(""), None);
        assert_eq!(inked("nope"), None);
        assert_eq!(inked("#gggggg"), None);
        assert_eq!(inked("ff00000"), None, "seven digits is not a colour");
    }

    #[test]
    fn a_colour_becomes_a_tint_at_double_gain() {
        assert_eq!(washed("#ff0000"), [2.0, 0.0, 0.0, 1.0]);
        let grey = 128.0 / 255.0 * 2.0;

        assert_eq!(washed("#808080"), [grey, grey, grey, 1.0]);

        // Alpha is a presence flag, so only a missing colour reads as untinted.
        assert_eq!(washed(""), [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(washed("nope"), [0.0, 0.0, 0.0, 0.0]);
    }
}
