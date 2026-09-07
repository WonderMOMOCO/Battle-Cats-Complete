use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::slice;
use std::fmt;
use std::thread;
use std::time::{Duration, SystemTime};

use iced::alignment::{Horizontal, Vertical};
use iced::font;
use iced::widget::text::Wrapping;
use iced::widget::{image as iced_image, operation, scrollable, tooltip, Id};
use iced::futures::channel::mpsc::unbounded;
use iced::{Color, ContentFit, Element, Length, Size, Task, Theme};
use nyanko::cat::unit::{LevelCurve, TalentCost};
use rayon::prelude::*;
use rustc_hash::FxHasher;
use nyanko::combat::{Entity, REGISTRY};

use kore::common::formats::SpriteSheet as CoreSpriteSheet;
use kore::domains::cat::files as cat_files;
use kore::domains::cat::game::stats as cat_stats;
use kore::domains::cat::game::talents as talent_logic;
use kore::domains::cat::scanner::{self as cat_scanner, CatEntry};
use kore::domains::enemy::scanner::{self as enemy_scanner, EnemyEntry};
use kore::domains::stage::{GlobalMapId, GlobalStageId, StageRegistry};
use nyanko::chapter::Category;
use kore::domains::import;
use kore::domains::mining::{self, changes, enemies, forms, levels, localized, stages, talents, units, Build, Diff, Status};

use kore::domains::settings::{ScannerConfig, Settings};
use kore::systems::combat::registry::{get_display_def, is_trait, AbilityIcon, STAT_RARITY};
use kore::common::context::GlobalContext;
use kore::Vfs;

use crate::app::theme;

mod chrome;
mod foes;
mod gains;
mod kit;
mod maps;
mod roster;
mod shelf;

use chrome::*;
use kit::*;
use maps::*;
use roster::*;
use shelf::*;
use crate::domains::stage::category::CategoryExt;
use crate::common::feedback::{Slot, CONFIRM_LABEL};
use crate::common::fonts;
use crate::common::glyphs::Ruler;
use crate::common::item_icon;
use crate::common::{ability_icon, img015, skill_name, CustomAssets, SpriteSheet};
use iced::widget::image::Handle;
use crate::widget::{fallback_icon, list_row, smooth_scroll, tinted_superscript, uniform_grid};

const SIDEBAR_WIDTH: f32 = 110.0;
const SIDEBAR_PADDING: f32 = 8.0;
const SIDEBAR_SPACING: f32 = 4.0;
const TAB_TEXT_SIZE: f32 = 14.0;
const TAB_PADDING: [u16; 2] = [8, 12];

const PAGE_PADDING: f32 = 20.0;
const SCROLLBAR_GAP: f32 = 8.0;
const SCROLL_ID: &str = "mining_body";
const SCROLLBAR_RESERVE: f32 = 24.0;
const CARD_PADDING: f32 = 12.0;
const UNIT_MIN_WIDTH: f32 = 440.0;
const LEVEL_MIN_WIDTH: f32 = 420.0;
const STAGE_MIN_WIDTH: f32 = 300.0;
const META_MIN_WIDTH: f32 = 340.0;
const NAME_MIN_WIDTH: f32 = 240.0;
const ART_MIN_WIDTH: f32 = 150.0;
const ART_TILE_SIZE: f32 = 96.0;
const ART_CANVAS: u32 = 128;
const FILE_NAME_SIZE: f32 = 13.0;
const NAME_LINE: f32 = FILE_NAME_SIZE * 1.3;
const GROUP_TITLE_SIZE: f32 = 14.0;
const SECTION_SLAB: f32 = 64.0;
const GROUP_SLAB: f32 = 30.0;
const SECTION_HEAD_GAP: f32 = 6.0;
const VIRTUAL_BUFFER: f32 = 240.0;
const TILE_BUDGET: usize = 300;
const SECTION_SPACING: f32 = 18.0;
const CARD_SPACING: f32 = 8.0;
const ULTRA_LEVEL: i32 = 60;
const LEVEL_INPUT_WIDTH: f32 = 44.0;
const LEVEL_INPUT_PADDING: f32 = 2.0;
const IGNORED_LABELS: &[&str] = &["Width"];

const FIRST_FORM: usize = 0;

const PORTRAIT_SIZE: f32 = 56.0;
const PORTRAIT_CANVAS: u32 = PORTRAIT_SIZE as u32 * 2;
const PLACEHOLDER_EDGE: u32 = 8;
const CROWN_GLYPH: &str = "\u{1F732}";
const TALENT_ICON_SIZE: f32 = 30.0;
const NAME_PLATE_HEIGHT: f32 = 26.0;
const UNIT_NAME_SIZE: f32 = 16.0;

const CHIP_RADIUS: f32 = 5.0;
const CHIP_PADDING: f32 = 6.0;
const CELL_SPACING: f32 = 8.0;
const HEADER_TEXT_SIZE: f32 = 13.0;
const LEDGER_TITLE_SIZE: f32 = 17.0;
const ABILITY_ICON_SIZE: f32 = 32.0;
const ABILITY_ICON_GAP: f32 = CELL_SPACING / 2.0;
const LEDGER_RULE_ALPHA: f32 = 0.35;

const VALUE_TEXT_SIZE: f32 = 13.0;
const BOX_PADDING: f32 = 4.0;
const BOX_RADIUS: f32 = 4.0;
const UNIT_BOX_HEIGHT: f32 = PORTRAIT_SIZE + BOX_PADDING * 2.0;
const HEADER_BOX_HEIGHT: f32 = TALENT_ICON_SIZE + BOX_PADDING * 2.0;
const VALUE_LABEL_GAP: f32 = 8.0;
const EMPTY_TEXT_SIZE: f32 = 17.0;
const TABLE_CELL_WIDTH: f32 = 190.0;
const TABLE_ROW_HEIGHT: f32 = 26.0;
const SECTION_TITLE_SIZE: f32 = 18.0;
const FOLD_TITLE_SIZE: f32 = 24.0;
const FOLD_NOTE_SIZE: f32 = 13.0;
const FOLD_LIMIT: usize = 200;
const DEPLOY_COOLDOWN: (&str, &str) = ("Cooldown", "Deploy Cooldown");
const TALLY_LABEL_WIDTH: f32 = 150.0;
const TALLY_VALUE_WIDTH: f32 = 60.0;
const TALLY_PADDING: f32 = 8.0;
const TALLY_TABLE_WIDTH: f32 = TALLY_LABEL_WIDTH + TALLY_PADDING * 2.0 + TALLY_VALUE_WIDTH;
const STAMP_LABEL_WIDTH: f32 = 80.0;
const STAMP_VALUE_WIDTH: f32 = TALLY_TABLE_WIDTH - STAMP_LABEL_WIDTH - TALLY_PADDING * 2.0;

const REGIONS: &[(&str, &str)] = &[
    ("Global", "battlecatsen"),
    ("Japan", "battlecats"),
    ("Taiwan", "battlecatstw"),
    ("Korea", "battlecatskr"),
];
const META_TEXT_SIZE: f32 = 14.0;
const NOTICE_TEXT_SIZE: f32 = 15.0;
const STRUCK_LABEL: &str = "Diff Created!";
const BARREN_LABEL: &str = "No Diffs!";
const RAISED_LABEL: &str = "Updated Snapshot!";
const FOUNDED_LABEL: &str = "Created Snapshot!";
const STILL_LABEL: &str = "No Updates!";
const ACTION_PADDING: f32 = 16.0;
const ACTION_CLEARANCE: f32 = 72.0;

const ULTRA_TINT: Color = Color { r: 0.47, g: 0.08, b: 0.08, a: 1.0 };
const NORMAL_TINT: Color = Color { r: 0.71, g: 0.55, b: 0.08, a: 1.0 };
const RETUNE_TINT: Color = Color { r: 0.16, g: 0.36, b: 0.55, a: 1.0 };
const ADDITION_TINT: Color = Color { r: 0.13, g: 0.42, b: 0.20, a: 1.0 };
const REMOVAL_TINT: Color = Color { r: 0.47, g: 0.13, b: 0.13, a: 1.0 };
const DARK_BOX_BG: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 };
const CHIP_TEXT: Color = Color::WHITE;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tab {
    Meta,
    Cats,
    Enemies,
    Stages,
    Files,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Meta => "Meta",
            Self::Cats => "Cats",
            Self::Enemies => "Enemies",
            Self::Stages => "Stages",
            Self::Files => "Files",
        }
    }
}

const TABS: &[Tab] = &[Tab::Meta, Tab::Cats, Tab::Enemies, Tab::Stages, Tab::Files];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Chore {
    Snapshot,
    Diff,
}

#[derive(Clone, Copy)]
pub struct Scope<'a> {
    pub cats: &'a [CatEntry],
    pub foes: &'a [EnemyEntry],
    pub registry: &'a StageRegistry,
    pub global: GlobalContext<'a>,
    pub settings: &'a Settings,
}

#[derive(Clone)]
pub enum Message {
    Select(Tab),
    Scrolled(f32),
    CreateBase,
    CreateDiff,
    Mined(bool),
    Rested,
    ClearDiff,
    WipeExpired,
    OpenFile(String),
    Fold(Tab, &'static str, bool),
    TilesLoaded(u64, Vec<(PathBuf, Option<Handle>)>),
    LevelChanged(u32, String),
    OpenTalents(u32, usize),
    OpenForm(u32, usize),
    OpenUnit(u32),
    OpenEnemy(u32),
    OpenCategory(Category),
    OpenMap(GlobalMapId),
    OpenStage(GlobalStageId, Option<u8>),
    Img015Loaded(u64, usize, Option<CoreSpriteSheet>),
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Select(tab) => write!(f, "Select({:?})", tab),
            Self::Scrolled(offset) => write!(f, "Scrolled({})", offset),
            Self::CreateBase => write!(f, "CreateBase"),
            Self::CreateDiff => write!(f, "CreateDiff"),
            Self::Mined(recorded) => write!(f, "Mined({})", recorded),
            Self::Rested => write!(f, "Rested"),
            Self::ClearDiff => write!(f, "ClearDiff"),
            Self::WipeExpired => write!(f, "WipeExpired"),
            Self::OpenFile(name) => write!(f, "OpenFile({})", name),
            Self::Fold(tab, title, open) => write!(f, "Fold({:?}, {}, {})", tab, title, open),
            Self::TilesLoaded(generation, loaded) => write!(f, "TilesLoaded({}, {})", generation, loaded.len()),
            Self::LevelChanged(cat, value) => write!(f, "LevelChanged({}, {})", cat, value),
            Self::OpenTalents(cat, form) => write!(f, "OpenTalents({}, {})", cat, form),
            Self::OpenForm(cat, form) => write!(f, "OpenForm({}, {})", cat, form),
            Self::OpenUnit(cat) => write!(f, "OpenUnit({})", cat),
            Self::OpenEnemy(enemy) => write!(f, "OpenEnemy({})", enemy),
            Self::OpenCategory(category) => write!(f, "OpenCategory({:?})", category),
            Self::OpenMap(map) => write!(f, "OpenMap({:?})", map),
            Self::OpenStage(stage, crown) => write!(f, "OpenStage({:?}, {:?})", stage, crown),
            Self::Img015Loaded(generation, index, _) => write!(f, "Img015Loaded({}, {})", generation, index),
        }
    }
}

pub struct State {
    tab: Tab,
    offsets: HashMap<Tab, f32>,
    diff: Option<Diff>,
    chore: Option<Chore>,
    outcome: Slot<(Chore, bool)>,
    creating: bool,
    wipe: Slot<()>,
    files: Shelves,
    tiles: HashMap<PathBuf, Option<Handle>>,
    decoding: bool,
    report: Option<talents::Report>,
    units: Option<units::Report>,
    forms: Vec<forms::Unlocked>,
    changes: Vec<changes::Changed>,
    foes: Vec<enemies::Changed>,
    fresh_foes: Vec<u32>,
    spoken_cats: Vec<localized::Localized>,
    spoken_foes: Vec<localized::Localized>,
    spoken_lands: Vec<localized::Localized>,
    surfaced: Vec<u32>,
    index: HashMap<u32, usize>,
    foe_index: HashMap<u32, usize>,
    ready: Ready,
    terrain: Terrain,
    snapped: bool,
    diggable: bool,
    folds: HashMap<(Tab, &'static str), bool>,
    tag: u64,
    lands: stages::Report,
    levels_raised: Vec<levels::Raised>,
    promoted: Vec<u32>,
    debuts: HashSet<u32>,
    listed: Vec<u32>,
    spotted: Vec<u32>,
    levels: HashMap<u32, String>,
    img015_sheets: Vec<SpriteSheet>,
    sheet_generation: u64,
    icons: ability_icon::Cache,
    plates: skill_name::Cache,
    portraits: RefCell<HashMap<(PathBuf, bool), Option<Handle>>>,
    art: RefCell<HashMap<PathBuf, Option<Handle>>>,
    assets: CustomAssets,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tab: Tab::Meta,
            offsets: HashMap::new(),
            diff: None,
            chore: None,
            outcome: Slot::default(),
            creating: false,
            wipe: Slot::default(),
            files: Shelves::default(),
            tiles: HashMap::new(),
            decoding: false,
            report: None,
            units: None,
            forms: Vec::new(),
            changes: Vec::new(),
            foes: Vec::new(),
            fresh_foes: Vec::new(),
            spoken_cats: Vec::new(),
            spoken_foes: Vec::new(),
            spoken_lands: Vec::new(),
            surfaced: Vec::new(),
            index: HashMap::new(),
            foe_index: HashMap::new(),
            ready: Ready::default(),
            terrain: Terrain::default(),
            snapped: false,
            diggable: false,
            folds: HashMap::new(),
            tag: 0,
            lands: stages::Report::default(),
            levels_raised: Vec::new(),
            promoted: Vec::new(),
            debuts: HashSet::new(),
            listed: Vec::new(),
            spotted: Vec::new(),
            levels: HashMap::new(),
            img015_sheets: Vec::new(),
            sheet_generation: 0,
            icons: ability_icon::Cache::default(),
            plates: skill_name::Cache::default(),
            portraits: RefCell::new(HashMap::new()),
            art: RefCell::new(HashMap::new()),
            assets: CustomAssets::new(),
        }
    }
}

struct Sifted {
    listed: Vec<u32>,
    sighted: Vec<u32>,
}

struct Vetted {
    cats: HashSet<u32>,
    foes: HashSet<u32>,
}

fn sift(cats: &[CatEntry], foes: &[EnemyEntry], vfs: &Vfs, settings: &Settings) -> Sifted {
    let strict = strict_config(settings);

    let listed = cats
        .par_iter()
        .filter(|entry| cat_scanner::listable(vfs, entry, &strict))
        .map(|entry| entry.id)
        .collect();

    let sighted = foes
        .par_iter()
        .filter(|entry| enemy_scanner::listable(vfs, entry.id, &strict))
        .map(|entry| entry.id)
        .collect();

    Sifted { listed, sighted }
}

impl State {
    pub(crate) fn refresh(&mut self, scope: Scope<'_>, window: Size) -> Task<Message> {
        let vfs = &scope.global.vault.vfs;

        if self.current_tag(scope) == self.tag {
            self.restock(scope);

            return Task::none();
        }

        self.reread(scope.cats, scope.foes, scope.registry, scope.global, scope.settings);

        Task::batch([self.check_sheets(vfs), self.ensure_tiles(window)])
    }

    pub(crate) fn reload(&mut self, scope: Scope<'_>, window: Size) -> Task<Message> {
        let vfs = &scope.global.vault.vfs;

        self.clear_caches();
        self.reread(scope.cats, scope.foes, scope.registry, scope.global, scope.settings);

        Task::batch([self.check_sheets(vfs), self.ensure_tiles(window)])
    }

    fn reconcile(&mut self, sifted: Sifted) {
        self.promoted = mining::reconcile(&sifted.listed);
        self.surfaced = mining::reconcile_foes(&sifted.sighted);

        if !sifted.listed.is_empty() {
            self.listed = sifted.listed;
        }

        if !sifted.sighted.is_empty() {
            self.spotted = sifted.sighted;
        }
    }

    fn reread(
        &mut self,
        cats: &[CatEntry],
        foes: &[EnemyEntry],
        registry: &StageRegistry,
        global: GlobalContext<'_>,
        settings: &Settings,
    ) {
        let vfs = &global.vault.vfs;

        self.diff = mining::load();
        self.files = self.diff.as_ref().map_or_else(Shelves::default, |held| shelve(held, vfs));
        self.tiles.clear();
        self.folds.clear();
        self.report = self
            .diff
            .as_ref()
            .and_then(|held| held.file(cat_files::SKILL_ACQUISITION))
            .map(talents::read);

        self.units = self
            .diff
            .as_ref()
            .and_then(|held| held.file(cat_files::UNIT_BUY))
            .map(units::read);

        self.debuts = self.diff.as_ref().map_or_else(HashSet::new, |held| {
            held.files
                .iter()
                .filter(|delta| delta.status == Status::Baseline)
                .filter_map(|delta| cat_files::stats_id(&delta.file))
                .collect()
        });

        self.forms = self
            .diff
            .as_ref()
            .and_then(|held| held.file(cat_files::UNIT_BUY))
            .map_or_else(Vec::new, forms::read);

        self.changes = self
            .diff
            .as_ref()
            .map_or_else(Vec::new, |held| held.files.iter().flat_map(changes::read).collect());

        self.foes = self
            .diff
            .as_ref()
            .map_or_else(Vec::new, |held| held.files.iter().flat_map(enemies::read).collect());

        self.fresh_foes = self
            .diff
            .as_ref()
            .map_or_else(Vec::new, |held| held.files.iter().flat_map(enemies::fresh).collect());

        self.spoken_cats = self.diff.as_ref().map_or_else(Vec::new, |held| localized::cats(held, vfs));
        self.spoken_foes = self.diff.as_ref().map_or_else(Vec::new, |held| localized::enemies(held, vfs));
        self.spoken_lands = self.diff.as_ref().map_or_else(Vec::new, |held| localized::stages(held, vfs));

        self.lands = self.diff.as_ref().map_or_else(stages::Report::default, |ore| {
            ore.files.iter().map(stages::read).fold(stages::Report::default(), |mut all, part| {
                all.fresh_maps.extend(part.fresh_maps);
                all.fresh_stages.extend(part.fresh_stages);
                all.changed_stages.extend(part.changed_stages);
                all.crowned.extend(part.crowned);

                all
            })
        });

        self.levels_raised = self
            .diff
            .as_ref()
            .and_then(|held| held.file(cat_files::UNIT_BUY))
            .map_or_else(Vec::new, levels::read);

        let scope = Scope { cats, foes, registry, global, settings };

        self.restock(scope);

        self.tag = self.current_tag(scope);
    }

    fn current_tag(&self, scope: Scope<'_>) -> u64 {
        let mut hasher = FxHasher::default();

        mining::stamp().hash(&mut hasher);
        mining::snapped_at().hash(&mut hasher);
        scope.global.vault.vfs.fingerprint().hash(&mut hasher);
        scope.cats.len().hash(&mut hasher);
        scope.foes.len().hash(&mut hasher);
        scope.registry.maps.len().hash(&mut hasher);
        self.promoted.len().hash(&mut hasher);
        self.surfaced.len().hash(&mut hasher);
        strict_config(scope.settings).hash(&mut hasher);

        hasher.finish()
    }

    pub(crate) fn restock(&mut self, scope: Scope<'_>) {
        let sifted = sift(scope.cats, scope.foes, &scope.global.vault.vfs, scope.settings);
        let vetted = Vetted {
            cats: sifted.listed.iter().copied().collect(),
            foes: sifted.sighted.iter().copied().collect(),
        };

        self.reconcile(sifted);

        self.index = scope.cats.iter().enumerate().map(|(slot, entry)| (entry.id, slot)).collect();
        self.foe_index = scope.foes.iter().enumerate().map(|(slot, entry)| (entry.id, slot)).collect();

        self.ready = self.derive(&vetted, scope.cats, scope.foes, scope.global, scope.settings);
        self.terrain = self.survey(scope.registry, &scope.global.vault.vfs);
        self.snapped = mining::has_snapshot();
        self.diggable = mining::capturable();

        if !self.enabled(self.tab) {
            self.tab = Tab::Meta;
        }
    }

    fn entry<'a>(&self, cats: &'a [CatEntry], cat_id: u32) -> Option<&'a CatEntry> {
        cats.get(*self.index.get(&cat_id)?).filter(|entry| entry.id == cat_id)
    }

    fn foe<'a>(&self, foes: &'a [EnemyEntry], enemy_id: u32) -> Option<&'a EnemyEntry> {
        foes.get(*self.foe_index.get(&enemy_id)?).filter(|entry| entry.id == enemy_id)
    }

    fn derive(
        &self,
        vetted: &Vetted,
        cats: &[CatEntry],
        foes: &[EnemyEntry],
        global: GlobalContext<'_>,
        settings: &Settings,
    ) -> Ready {
        let vfs = &global.vault.vfs;
        let strict = strict_config(settings);
        let Vetted { cats: listable, foes: sighted } = vetted;

        let spirits = conjured(cats);
        let mut seen: HashSet<u32> = HashSet::new();

        let units = self.units.as_ref().map_or(&[][..], |units| units.fresh.as_slice());

        let mut fresh: Vec<u32> = units
            .iter()
            .chain(self.risen(&self.promoted))
            .chain(self.debuts.iter())
            .filter(|id| !spirits.contains(*id) && seen.insert(**id))
            .filter(|id| listable.contains(id))
            .copied()
            .collect();

        fresh.sort_unstable();

        let spoken: Vec<usize> = self
            .spoken_cats
            .iter()
            .enumerate()
            .filter(|(_, held)| !self.debuted(held.id) && listable.contains(&held.id))
            .filter(|(_, held)| {
                held.form.is_none_or(|form| {
                    !self.unlocks(held.id, form)
                        && self
                            .entry(cats, held.id)
                            .is_some_and(|entry| entry.forms.get(form).copied().unwrap_or(false))
                })
            })
            .map(|(slot, _)| slot)
            .collect();

        let changed: Vec<(usize, forms::Diff)> = self
            .changes
            .iter()
            .enumerate()
            .filter(|(_, changed)| !self.arrived(changed.cat_id) && !self.unlocks(changed.cat_id, changed.form))
            .filter(|(_, changed)| {
                listable.contains(&changed.cat_id)
                    && self
                        .entry(cats, changed.cat_id)
                        .is_some_and(|entry| entry.forms.get(changed.form).copied().unwrap_or(false))
            })
            .map(|(slot, changed)| (slot, changed_diff(changed, self.entry(cats, changed.cat_id), global, settings)))
            .filter(|(_, diff)| !diff.is_empty())
            .collect();

        let unlocked: Vec<(usize, forms::Diff)> = self
            .forms
            .iter()
            .enumerate()
            .filter(|(_, held)| !self.arrived(held.cat_id) && listable.contains(&held.cat_id))
            .map(|(slot, held)| {
                let diff = self
                    .entry(cats, held.cat_id)
                    .map_or_else(forms::Diff::default, |entry| form_diff(entry, held.form, global, settings));

                (slot, diff)
            })
            .collect();

        let mut talents: Vec<usize> = Vec::new();

        if let Some(report) = self.report.as_ref().filter(|report| report.status == Status::Changed) {
            talents = (0..report.finds.len()).collect();
        }

        let raised: Vec<usize> = self
            .levels_raised
            .iter()
            .enumerate()
            .filter(|(_, held)| !self.arrived(held.cat_id) && listable.contains(&held.cat_id))
            .map(|(slot, _)| slot)
            .collect();

        let mut foes_new: Vec<u32> = self.arrivals().into_iter().filter(|id| sighted.contains(id)).collect();

        foes_new.sort_unstable();

        let foes_changed: Vec<(usize, forms::Diff)> = self
            .foes
            .iter()
            .enumerate()
            .filter(|(_, foe)| !self.surged(foe.enemy_id) && sighted.contains(&foe.enemy_id))
            .map(|(slot, foe)| {
                let frames = self.foe(foes, foe.enemy_id).map_or((0, 0), |held| (held.atk_anim_frames, held.atk_anim_frames));

                let diff = forms::compare(&forms::Subject {
                    global,
                    previous: &foe.previous,
                    current: &foe.current,
                    curve: None,
                    level: 1,
                    frames,
                });

                (slot, diff)
            })
            .collect();

        let foes_spoken: Vec<usize> = self
            .spoken_foes
            .iter()
            .enumerate()
            .filter(|(_, held)| !self.surged(held.id) && sighted.contains(&held.id))
            .map(|(slot, _)| slot)
            .collect();

        let mut shots: Vec<(u32, usize)> = fresh.iter().map(|id| (*id, FIRST_FORM)).collect();

        shots.extend(spoken.iter().filter_map(|slot| self.spoken_cats.get(*slot)).map(|held| {
            (held.id, self.spoken_form(cats, held))
        }));

        shots.extend(changed.iter().filter_map(|(slot, _)| self.changes.get(*slot)).map(|held| (held.cat_id, held.form)));
        shots.extend(unlocked.iter().filter_map(|(slot, _)| self.forms.get(*slot)).map(|held| (held.cat_id, held.form)));

        shots.extend(raised.iter().filter_map(|slot| self.levels_raised.get(*slot)).map(|held| {
            (held.cat_id, self.entry(cats, held.cat_id).map_or(FIRST_FORM, top_form))
        }));

        if let Some(report) = &self.report {
            shots.extend(talents.iter().filter_map(|slot| report.finds.get(*slot)).map(|find| {
                let form = self
                    .entry(cats, find.cat_id)
                    .map_or(FIRST_FORM, |entry| portrait_form(entry, find.has_ultra()));

                (find.cat_id, form)
            }));
        }

        let art: HashMap<(u32, usize), PathBuf> = shots
            .into_iter()
            .filter_map(|(id, form)| {
                let entry = self.entry(cats, id)?;

                cat_scanner::icon(vfs, entry, form, &strict).map(|path| ((id, form), path))
            })
            .collect();

        let foe_art: HashMap<u32, PathBuf> = foes_new
            .iter()
            .copied()
            .chain(foes_changed.iter().filter_map(|(slot, _)| self.foes.get(*slot)).map(|foe| foe.enemy_id))
            .chain(foes_spoken.iter().filter_map(|slot| self.spoken_foes.get(*slot)).map(|held| held.id))
            .filter_map(|id| enemy_scanner::icon(vfs, id, &strict).map(|path| (id, path)))
            .collect();

        Ready {
            fresh,
            spoken,
            changed,
            unlocked,
            talents,
            raised,
            art,
            foe_art,
            foes_new,
            foes_changed,
            foes_spoken,
        }
    }

    fn survey(&self, registry: &StageRegistry, vfs: &Vfs) -> Terrain {
        let atlas: HashMap<u32, GlobalMapId> = registry
            .maps
            .keys()
            .filter_map(|key| key.category.global_map_id(key.map).map(|global| (global, key.clone())))
            .collect();

        let moves: HashMap<GlobalMapId, (u8, u8)> = self
            .lands
            .crowned
            .iter()
            .filter_map(|found| atlas.get(&found.global_map).map(|key| (key.clone(), (found.before, found.after))))
            .collect();

        let tongues: HashMap<GlobalMapId, Vec<String>> = self
            .spoken_lands
            .iter()
            .filter_map(|held| atlas.get(&held.id).map(|key| (key.clone(), held.languages.clone())))
            .collect();

        let fresh = grouped(self.lands.fresh_maps.iter().chain(self.lands.fresh_stages.iter()), registry);
        let spoken = gather(tongues.keys().cloned().map(|key| (key, None)), registry);
        let moved = grouped(self.lands.changed_stages.iter(), registry);
        let crowned = gather(moves.keys().cloned().map(|key| (key, None)), registry);

        let mut banners: HashMap<GlobalMapId, PathBuf> = HashMap::new();
        let mut plates: HashMap<GlobalStageId, PathBuf> = HashMap::new();

        for shown in [&fresh, &spoken, &moved, &crowned] {
            for (category, maps) in shown {
                let art = category.image_prefix();

                for (map, listed) in maps {
                    let key = GlobalMapId { category: category.clone(), map: *map };

                    if !banners.contains_key(&key)
                        && let Some(path) = vfs.pristine(&map_art(*map, &art))
                    {
                        banners.insert(key, path);
                    }

                    for stage in listed {
                        let chip = GlobalStageId { category: category.clone(), map: *map, stage: *stage };

                        if !plates.contains_key(&chip)
                            && let Some(path) = vfs.pristine(&stage_art(*map, *stage, &art))
                        {
                            plates.insert(chip, path);
                        }
                    }
                }
            }
        }

        Terrain {
            fresh,
            spoken,
            opened: grouped(self.lands.fresh_maps.iter(), registry),
            added: grouped(self.lands.fresh_stages.iter(), registry),
            moved,
            crowned,
            banners,
            plates,
            tongues,
            moves,
        }
    }

    pub(crate) fn enter(&mut self, scope: Scope<'_>, window: Size) -> Task<Message> {
        let vfs = &scope.global.vault.vfs;

        if self.current_tag(scope) != self.tag {
            self.reread(scope.cats, scope.foes, scope.registry, scope.global, scope.settings);
        }

        Task::batch([self.check_sheets(vfs), self.ensure_tiles(window), self.restore()])
    }

    pub(crate) fn relocalize(&mut self, vfs: &Vfs, window: Size) -> Task<Message> {
        self.clear_caches();

        Task::batch([self.check_sheets(vfs), self.ensure_tiles(window)])
    }

    pub(crate) fn clear_caches(&mut self) {
        self.sheet_generation = self.sheet_generation.wrapping_add(1);
        self.icons.clear();
        self.plates.borrow_mut().clear();
        self.portraits.borrow_mut().clear();
        self.art.borrow_mut().clear();
        self.tiles.clear();

        for sheet in &mut self.img015_sheets {
            sheet.mark_stale();
        }
    }

    pub(crate) fn wipe(&mut self, scope: Scope<'_>, window: Size) -> Task<Message> {
        if !self.wipe.is_set() {
            return self.wipe.set((), Message::WipeExpired);
        }

        self.wipe.clear();
        mining::discard();

        self.reload(scope, window)
    }

    pub(crate) fn begin(&mut self, chore: Chore) -> Task<Message> {
        if self.chore.is_some() {
            return Task::none();
        }

        self.chore = Some(chore);
        self.creating = !self.snapped;

        let (tx, rx) = unbounded();
        let listed = self.listed.clone();
        let spotted = self.spotted.clone();

        thread::spawn(move || {
            let recorded = match chore {
                Chore::Snapshot => mining::capture(listed, spotted),
                Chore::Diff => mining::craft(&listed, &spotted),
            };

            let _ = tx.unbounded_send(Message::Mined(recorded));
        });

        Task::stream(rx)
    }

    pub(crate) fn settle(&mut self, scope: Scope<'_>, recorded: bool, window: Size) -> Task<Message> {
        let chore = self.chore.take();

        self.reread(scope.cats, scope.foes, scope.registry, scope.global, scope.settings);

        let rested = chore.map_or_else(Task::none, |held| self.outcome.set((held, recorded), Message::Rested));

        Task::batch([rested, self.ensure_tiles(window)])
    }

    pub fn update(&mut self, message: Message, window: Size) -> Task<Message> {
        match message {
            Message::Select(tab) => {
                if self.enabled(tab) && self.tab != tab {
                    self.tab = tab;

                    return Task::batch([self.ensure_tiles(window), self.restore()]);
                }
            }
            Message::Rested => {
                self.outcome.expire();
            }
            Message::WipeExpired => {
                self.wipe.expire();
            }
            Message::Fold(tab, title, open) => {
                self.folds.insert((tab, title), open);
            }
            Message::TilesLoaded(generation, loaded) => {
                self.decoding = false;

                if generation != self.sheet_generation {
                    return Task::none();
                }

                for (path, handle) in loaded {
                    self.tiles.insert(path, handle);
                }

                return self.ensure_tiles(window);
            }
            Message::Scrolled(offset) => {
                self.offsets.insert(self.tab, offset);

                return self.ensure_tiles(window);
            }
            Message::LevelChanged(cat_id, value) => {
                if value.chars().all(|glyph| glyph.is_ascii_digit()) {
                    self.levels.insert(cat_id, value);
                }
            }
            Message::CreateBase
            | Message::CreateDiff
            | Message::Mined(..)
            | Message::ClearDiff
            | Message::OpenTalents(..)
            | Message::OpenForm(..)
            | Message::OpenUnit(..)
            | Message::OpenEnemy(..)
            | Message::OpenCategory(..)
            | Message::OpenMap(..)
            | Message::OpenStage(..)
            | Message::OpenFile(..) => {}
            Message::Img015Loaded(generation, index, sheet) => {
                if generation == self.sheet_generation
                    && let Some(slot) = self.img015_sheets.get_mut(index)
                {
                    slot.apply(sheet);
                    self.icons.clear();
                }
            }
        }

        Task::none()
    }

    fn restore(&self) -> Task<Message> {
        let offset = self.offsets.get(&self.tab).copied().unwrap_or_default();

        operation::scroll_to(Id::new(SCROLL_ID), scrollable::AbsoluteOffset { x: 0.0, y: offset })
    }

    fn check_sheets(&mut self, vfs: &Vfs) -> Task<Message> {
        let generation = self.sheet_generation;

        img015::ensure_loaded(&mut self.img015_sheets, vfs, true)
            .map(move |(index, sheet)| Message::Img015Loaded(generation, index, sheet))
    }

    fn has_diff(&self) -> bool {
        TABS.iter().any(|tab| *tab != Tab::Meta && self.enabled(*tab))
    }

    pub(crate) fn enabled_levels(&self, cat_id: u32) -> Option<HashMap<u8, u8>> {
        self.report
            .as_ref()?
            .finds
            .iter()
            .find(|find| find.cat_id == cat_id)
            .map(talents::Find::enabled_levels)
    }

    fn unlocks(&self, cat_id: u32, form: usize) -> bool {
        self.forms.iter().any(|unlocked| unlocked.cat_id == cat_id && unlocked.form == form)
    }

    fn arrived(&self, cat_id: u32) -> bool {
        self.promoted.contains(&cat_id)
    }

    fn has_finds(&self) -> bool {
        !self.ready.fresh.is_empty()
            || !self.ready.spoken.is_empty()
            || !self.ready.changed.is_empty()
            || !self.ready.unlocked.is_empty()
            || !self.ready.talents.is_empty()
            || !self.ready.raised.is_empty()
    }

    fn surged(&self, enemy_id: u32) -> bool {
        self.fresh_foes.contains(&enemy_id) || self.surfaced.contains(&enemy_id)
    }

    fn arrivals(&self) -> Vec<u32> {
        let mut seen: HashSet<u32> = HashSet::new();

        self.fresh_foes.iter().chain(self.risen(&self.surfaced)).filter(|id| seen.insert(**id)).copied().collect()
    }

    fn risen<'a>(&self, arrivals: &'a [u32]) -> slice::Iter<'a, u32> {
        if self.diff.is_some() { arrivals.iter() } else { [].iter() }
    }

    fn debuted(&self, cat_id: u32) -> bool {
        self.arrived(cat_id)
            || self.debuts.contains(&cat_id)
            || self.units.as_ref().is_some_and(|units| units.fresh.contains(&cat_id))
    }

    fn portrait(&self, path: &Path) -> Option<Handle> {
        self.cached_icon(path, false)
    }

    fn thumbnail(&self, path: &Path) -> Option<Handle> {
        self.cached_icon(path, true)
    }

    fn plate(&self, path: &Path) -> Option<Handle> {
        if let Some(cached) = self.art.borrow().get(path) {
            return cached.clone();
        }

        let handle = item_icon::load_cropped(path)
            .and_then(|(handle, width, height)| {
                (width >= PLACEHOLDER_EDGE && height >= PLACEHOLDER_EDGE).then_some(handle)
            });

        self.art.borrow_mut().insert(path.to_path_buf(), handle.clone());

        handle
    }

    fn cached_icon(&self, path: &Path, fill: bool) -> Option<Handle> {
        let key = (path.to_path_buf(), fill);

        if let Some(cached) = self.portraits.borrow().get(&key) {
            return cached.clone();
        }

        let handle = if fill {
            item_icon::load_scaled(path, PORTRAIT_CANVAS)
        } else {
            item_icon::load_boxed(path, PORTRAIT_CANVAS)
        };

        self.portraits.borrow_mut().insert(key, handle.clone());

        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The scan feeds mining::reconcile, which compares it against the stored roster as a
    // slice; a reordered scan would read as the whole roster arriving at once.
    #[test]
    fn the_parallel_scan_keeps_entry_order() {
        let ids: Vec<u32> = (0..5_000).collect();

        let scanned: Vec<u32> = ids.par_iter().filter(|id| **id % 3 != 0).copied().collect();
        let expected: Vec<u32> = ids.iter().filter(|id| **id % 3 != 0).copied().collect();

        assert_eq!(scanned, expected);
    }
}
