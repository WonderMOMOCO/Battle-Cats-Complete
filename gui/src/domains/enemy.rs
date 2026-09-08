mod details;
mod export;
mod filter;
mod list;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use iced::alignment::{Horizontal, Vertical};
use iced::futures::channel::mpsc;
use iced::widget::{
    button, column, container, image as iced_image, row, rule,
    text, text_input, Id, Space,
};
use iced::{Color, Element, Length, Size, Subscription, Task, Theme};
use nyanko::combat::Entity;
use tracing::info;

use kore::common::context::GlobalContext;
use kore::common::formats::SpriteSheet as CoreSpriteSheet;
use kore::systems::combat::registry::{format_stat, Magnification, StatContext, STAT_ATK_CYCLE, STAT_ATTACK, STAT_CASH_DROP, STAT_DPS, STAT_HITPOINTS, STAT_KNOCKBACKS, STAT_RANGE, STAT_SPEED};
use kore::systems::combat::RenderContext;
use kore::domains::enemy::animation as enemy_animation;
use kore::domains::enemy::scanner::{self, EnemyEntry};
use kore::domains::enemy::EnemyDataState;
use kore::domains::settings::Settings;
use kore::{Vfs, Vault};

use crate::systems::animation;
use crate::systems::combat::abilities as combat_abilities;
use crate::app::state::{AnimState, AppState, EnemyListState};
use crate::app::theme;
use crate::common::CustomAssets;
use crate::common::SpriteSheet;
use crate::common::header_icon::{self, HeaderIcon};
use crate::editor;
use crate::widget::{grid_frames, grid_header, grid_value, name_box, popup, roster_list, statblock_export, status};

const HEADER_BUTTON_WIDTH: f32 = 65.0;
const HEADER_BUTTON_HEIGHT: f32 = 26.0;
const HEADER_BUTTON_TOP_PADDING: f32 = 5.0;
const EXPORT_BUTTON_RULE_GAP: f32 = 2.0;
const DETAIL_RULE_GAP: f32 = 15.0;
const DETAIL_RULE_HEIGHT: f32 = 96.0;
const ICON_BOX_WIDTH: f32 = 110.0;
const ICON_BOX_HEIGHT: f32 = 96.0;
const APPEARANCES_TEXT_SIZE: f32 = 12.0;
const EMPTY_CAT_ICON: &str = "uni.png";

fn animation_preload(state: &mut animation::State, enemy: &EnemyEntry, vfs: &Vfs, anim_state: &AnimState) -> Task<Message> {
    let key = enemy_animation::set_id(enemy);

    state.preload(&key, || enemy_animation::clips(enemy, vfs), anim_state).map(Message::Animation)
}

fn typable_magnification(value: &str) -> bool {
    value.chars().all(|glyph| glyph.is_ascii_digit() || matches!(glyph, '/' | '|' | '\\'))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Abilities,
    Details,
    Animation,
}

#[derive(Clone)]
pub enum Message {
    AnimationTick,
    SheetsCheck,
    ScanProgress(u64, usize, usize),
    Loaded(u64, Vec<EnemyEntry>, Option<u64>),
    Img015Loaded(u64, usize, Option<CoreSpriteSheet>),
    SearchChanged(String),
    SelectEnemy(u32),
    JumpToEnemyMagnified(u32, String),
    SelectTab(DetailTab),
    ToggleOrigin(bool),
    ToggleParts(bool),
    ToggleWorld(bool),
    MagnificationChanged(String),
    NavigateAppearances(u32),
    List(list::Message),
    Filter(filter::Message),
    Export(statblock_export::Message),
    Animation(animation::Message),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AnimationTick => write!(f, "AnimationTick"),
            Self::SheetsCheck => write!(f, "SheetsCheck"),
            Self::ScanProgress(_, done, total) => write!(f, "ScanProgress({}/{})", done, total),
            Self::Loaded(_, enemies, _) => write!(f, "Loaded({})", enemies.len()),
            Self::Img015Loaded(_, i, _) => write!(f, "Img015Loaded({})", i),
            Self::SearchChanged(s) => write!(f, "SearchChanged({})", s),
            Self::SelectEnemy(id) => write!(f, "SelectEnemy({})", id),
            Self::JumpToEnemyMagnified(id, mag_input) => write!(f, "JumpToEnemyMagnified({}, {})", id, mag_input),
            Self::SelectTab(t) => write!(f, "SelectTab({:?})", t),
            Self::ToggleOrigin(b) => write!(f, "ToggleOrigin({})", b),
            Self::ToggleParts(b) => write!(f, "ToggleParts({})", b),
            Self::ToggleWorld(b) => write!(f, "ToggleWorld({})", b),
            Self::MagnificationChanged(s) => write!(f, "MagnificationChanged({})", s),
            Self::NavigateAppearances(id) => write!(f, "NavigateAppearances({})", id),
            Self::List(msg) => write!(f, "List({:?})", msg),
            Self::Filter(msg) => write!(f, "Filter({:?})", msg),
            Self::Export(msg) => write!(f, "Export({:?})", msg),
            Self::Animation(msg) => write!(f, "Animation({:?})", msg),
        }
    }
}

pub struct EnemyState {
    pub data: EnemyDataState,
    pub selected_enemy: Option<u32>,
    pub selected_tab: DetailTab,
    pub search_query: String,

    pub mag_input: String,
    pub magnification: Magnification,

    img015_sheets: Vec<SpriteSheet>,
    sheet_generation: u64,
    scan_generation: u64,
    custom_assets: CustomAssets,

    dynamic_stats: RefCell<Option<(u32, Option<Entity>)>>,
    header_icon_cache: header_icon::Cache,
    header_icon_dummy: HeaderIcon,

    scan_progress: Option<(usize, usize)>,
    cached_key: Option<u64>,

    list: list::State,
    filter: filter::State,
    abilities: combat_abilities::State,
    export: statblock_export::State,
    animation: animation::State,
}

impl Default for EnemyState {
    fn default() -> Self {
        let header_icon_dummy = HeaderIcon::dummy();

        Self {
            data: EnemyDataState::default(),
            selected_enemy: None,
            selected_tab: DetailTab::Abilities,
            search_query: String::new(),
            mag_input: String::from("100"),
            magnification: Magnification { hitpoints: 100, attack: 100 },

            img015_sheets: Vec::new(),
            sheet_generation: 0,
            scan_generation: 0,
            custom_assets: CustomAssets::new(),

            dynamic_stats: RefCell::new(None),
            header_icon_cache: RefCell::new(HashMap::new()),
            header_icon_dummy,

            scan_progress: None,
            cached_key: None,

            list: list::State::default(),
            filter: filter::State::default(),
            abilities: combat_abilities::State::default(),
            export: statblock_export::State::new("Enemy"),
            animation: animation::State::with_popup(popup::Kind::EnemyAnimationExport),
        }
    }
}

impl EnemyState {
    pub(crate) fn list_scrollable_id() -> Id {
        list::State::scrollable_id()
    }

    pub(crate) fn list_scroll_offset(&self) -> f32 {
        self.list.scroll_offset()
    }

    pub(crate) fn restore_state(&mut self, state: &EnemyListState) {
        self.list.set_scroll_offset(state.list_scroll_offset);
        self.selected_enemy = state.selected_enemy;
        self.search_query = state.search_query.clone();
    }

    pub(crate) fn sync_state(&self, state: &mut EnemyListState) {
        state.list_scroll_offset = self.list.scroll_offset();
        state.selected_enemy = self.selected_enemy;

        if state.search_query != self.search_query {
            state.search_query = self.search_query.clone();
        }
    }

    fn dynamic_stats(&self, id: u32, vault: &Vault, show_invalid: bool) -> Option<Entity> {
        if let Some((cached_id, stats)) = self.dynamic_stats.borrow().as_ref()
            && *cached_id == id
        {
            return stats.clone();
        }

        let stats = scanner::scan_single(id, vault, show_invalid).map(|entry| entry.stats);
        *self.dynamic_stats.borrow_mut() = Some((id, stats.clone()));
        stats
    }

    pub(crate) fn invalidate_assets(&mut self, enemies: &HashSet<u32>, vault: &Vault, show_invalid: bool) {
        self.dynamic_stats.replace(None);
        self.animation.invalidate_paths();

        let mut dropped: Vec<u32> = Vec::new();
        let mut restored: Vec<EnemyEntry> = Vec::new();

        for id in enemies.iter().copied() {
            self.list.forget(id);

            let Some(entry) = self.data.enemies.iter_mut().find(|entry| entry.id == id) else {
                restored.extend(scanner::scan_single(id, vault, show_invalid));
                continue;
            };

            if !scanner::revalidate(&vault.vfs, entry, show_invalid) {
                dropped.push(id);
            }
        }

        if !dropped.is_empty() {
            info!(count = dropped.len(), "Dropping enemies that no longer have a listable icon");
            self.data.enemies.retain(|entry| !dropped.contains(&entry.id));
        }

        for entry in restored {
            info!(id = entry.id, "Restoring an enemy that became listable again");

            let at = self.data.enemies.partition_point(|existing| existing.id < entry.id);
            self.data.enemies.insert(at, entry);
        }

        self.header_icon_cache.borrow_mut().clear();
        self.list.refresh(&self.data.enemies, &self.search_query, &self.filter.filter_state);
    }

    pub(crate) fn reload_selected(&mut self, vault: &Vault, show_invalid: bool) {
        let Some(id) = self.selected_enemy else { return; };
        let Some(index) = self.data.enemies.iter().position(|entry| entry.id == id) else { return; };
        let Some(entry) = scanner::scan_single(id, vault, show_invalid) else { return; };

        self.data.enemies[index] = entry;
    }

    pub fn set_indexing(&mut self) {
        self.scan_progress = Some((0, 0));
    }

    pub(crate) fn clear_indexing(&mut self) {
        self.scan_progress = None;
    }

    pub fn icon_stream(&mut self) -> Task<Message> {
        self.list.result_stream().map(Message::List)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.selected_tab != DetailTab::Animation {
            return Subscription::none();
        }

        iced::time::every(self.animation.pace()).map(|_| Message::AnimationTick)
    }

    pub fn start_load(&mut self, settings: &Settings, vault: &Arc<Vault>, active_mod: Option<String>, cached: bool) -> Task<Message> {
        info!(cached, "Triggering enemy load");
        let config = settings.scanner_config(active_mod);
        let scan_store = Arc::clone(vault);
        let (tx, rx) = mpsc::unbounded();

        self.scan_generation = self.scan_generation.wrapping_add(1);
        let generation = self.scan_generation;

        thread::spawn(move || {
            if cached && let Some((key, enemies)) = scanner::hydrate() {
                let _ = tx.unbounded_send(Message::Loaded(generation, enemies, Some(key)));
                return;
            }

            let scan = scanner::load(config, scan_store, |done, total| {
                let _ = tx.unbounded_send(Message::ScanProgress(generation, done, total));
            });

            let payload = scan.payload;
            let _ = tx.unbounded_send(Message::Loaded(generation, scan.data, scan.key));

            if let Some(bytes) = payload {
                scanner::persist(&bytes);
            }
        });

        Task::batch([Task::stream(rx), self.check_sheets(&vault.vfs)])
    }

    pub fn rescan(&mut self, settings: &Settings, vault: &Arc<Vault>, active_mod: Option<String>) -> Task<Message> {
        info!("Rescanning enemies");
        self.clear_caches();
        self.start_load(settings, vault, active_mod, false)
    }

    pub(crate) fn cached_key(&self) -> Option<u64> {
        self.cached_key
    }

    pub(crate) fn clear_caches(&mut self) {
        self.dynamic_stats.replace(None);
        self.animation.invalidate_paths();
        self.sheet_generation = self.sheet_generation.wrapping_add(1);
        for sheet in &mut self.img015_sheets {
            sheet.mark_stale();
        }
        self.header_icon_cache.borrow_mut().clear();
    }

    pub(crate) fn relocalize(&mut self, vfs: &Vfs) -> Task<Message> {
        self.sheet_generation = self.sheet_generation.wrapping_add(1);

        for sheet in &mut self.img015_sheets {
            sheet.mark_stale();
        }

        self.check_sheets(vfs)
    }

    fn recut(&self) {
        self.abilities.clear_icons();
        self.filter.clear_icons();
    }

    fn check_sheets(&mut self, vfs: &Vfs) -> Task<Message> {
        let generation = self.sheet_generation;

        crate::common::img015::ensure_loaded(&mut self.img015_sheets, vfs, false)
            .map(move |(index, sheet)| Message::Img015Loaded(generation, index, sheet))
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState, global_ctx: GlobalContext<'_>) -> Task<Message> {
        let task = self.update_inner(message, settings, app_state, global_ctx);

        self.list.refresh(&self.data.enemies, &self.search_query, &self.filter.filter_state);

        Task::batch([task, self.list.take_scroll()])
    }

    fn update_inner(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState, global_ctx: GlobalContext<'_>) -> Task<Message> {
        match message {
            Message::SheetsCheck => self.check_sheets(&global_ctx.vault.vfs),
            Message::Img015Loaded(generation, index, sheet) => {
                if generation == self.sheet_generation
                    && let Some(slot) = self.img015_sheets.get_mut(index)
                {
                    slot.apply(sheet);
                    self.recut();
                }
                Task::none()
            }
            Message::ScanProgress(generation, done, total) => {
                if generation != self.scan_generation {
                    return Task::none();
                }

                if self.scan_progress.is_none_or(|(prev, _)| done > prev) {
                    self.scan_progress = Some((done, total));
                }
                Task::none()
            }
            Message::Loaded(generation, enemies, key) => {
                if generation != self.scan_generation {
                    return Task::none();
                }

                info!("Enemy load finished with {} entries", enemies.len());
                self.cached_key = key;
                self.scan_progress = None;
                self.list.invalidate();
                self.header_icon_cache.borrow_mut().clear();
                self.data.enemies = enemies;
                let filter_task = self.filter.refresh_available(&self.data.enemies).map(Message::Filter);
                let preload_task = match self.selected_enemy.and_then(|id| self.data.enemies.iter().find(|e| e.id == id)) {
                    Some(enemy) => {
                        if self.selected_tab != DetailTab::Animation {
                            self.animation.clear();
                        }
                        animation_preload(&mut self.animation, enemy, &global_ctx.vault.vfs, &app_state.animation)
                    }
                    None => Task::none(),
                };

                Task::batch([filter_task, preload_task])
            }
            Message::AnimationTick => {
                if let Some(enemy) = self.selected_enemy.and_then(|id| self.data.enemies.iter().find(|e| e.id == id)) {
                    let vfs = &global_ctx.vault.vfs;
                    let key = enemy_animation::set_id(enemy);
                    self.animation.sync(&key, || enemy_animation::clips(enemy, vfs), settings, &app_state.animation);
                }
                self.animation.tick();
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::SelectEnemy(id) => {
                if self.selected_enemy == Some(id) {
                    return Task::none();
                }

                self.selected_enemy = Some(id);
                self.mag_input = String::from("100");
                self.magnification = Magnification { hitpoints: 100, attack: 100 };
                self.animation.reset_playhead();

                info!("Selected enemy ID: {}", id);
                if self.selected_tab != DetailTab::Animation {
                    self.animation.clear();
                }
                match self.data.enemies.iter().find(|e| e.id == id) {
                    Some(enemy) => animation_preload(&mut self.animation, enemy, &global_ctx.vault.vfs, &app_state.animation),
                    None => Task::none(),
                }
            }
            Message::JumpToEnemyMagnified(id, mag_input) => {
                let select = self.update(Message::SelectEnemy(id), settings, app_state, global_ctx);
                let magnified = self.update(Message::MagnificationChanged(mag_input), settings, app_state, global_ctx);
                Task::batch([select, magnified])
            }
            Message::SelectTab(tab) => {
                self.selected_tab = tab;

                if tab == DetailTab::Animation {
                    return Task::batch([Task::done(Message::AnimationTick), self.export_scroll_task()]);
                }

                Task::none()
            }
            Message::ToggleOrigin(val) => {
                settings.animation.show_origin = val;
                Task::none()
            }
            Message::ToggleParts(val) => {
                settings.animation.show_rig = val;
                Task::none()
            }
            Message::ToggleWorld(val) => {
                settings.animation.show_world = val;
                Task::none()
            }
            Message::MagnificationChanged(input) => {
                if !typable_magnification(&input) {
                    return Task::none();
                }

                self.mag_input = input;
                let trimmed = self.mag_input.trim();
                let parts: Vec<&str> = trimmed.split(['/', '|', '\\']).collect();

                self.magnification = if parts.len() >= 2 {
                    let hitpoints = parts[0].trim().parse::<i32>().unwrap_or(100);
                    let attack = parts[1].trim().parse::<i32>().unwrap_or(hitpoints);
                    Magnification { hitpoints, attack }
                } else {
                    let mag = trimmed.parse::<i32>().unwrap_or(100);
                    Magnification { hitpoints: mag, attack: mag }
                };
                Task::none()
            }
            Message::NavigateAppearances(_) => Task::none(),
            Message::List(msg) => {
                if let list::Message::Select(id) = msg {
                    return self.update(Message::SelectEnemy(id), settings, app_state, global_ctx);
                }

                self.list.update(msg);
                Task::none()
            }
            Message::Filter(msg) => {
                self.filter.update(msg);
                Task::none()
            }
            Message::Export(msg) => {
                let enemy = self.selected_enemy.and_then(|id| self.data.enemies.iter().find(|e| e.id == id));
                let ctx = enemy.map(|enemy| export::Ctx {
                    enemy,
                    magnification: self.magnification,
                    sheets: &self.img015_sheets,
                    global: global_ctx,
                    settings,
                    vault: global_ctx.vault,
                });

                self.export.update(msg, || ctx.and_then(export::request)).map(Message::Export)
            }
            Message::Animation(msg) => self.animation.update(msg, settings, &mut app_state.animation).map(Message::Animation),
        }
    }

    pub(crate) fn animation_expanded(&self) -> bool {
        self.animation.is_expanded()
    }

    pub(crate) fn animation_clip(&self) -> Option<String> {
        self.animation.selected_label()
    }

    pub fn expanded_animation_view<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState) -> Option<Element<'a, Message>> {
        self.animation.expanded_view(settings, &app_state.animation).map(|view| view.map(Message::Animation))
    }

    pub fn export_popup_open(&self) -> bool {
        self.animation.export_popup_open()
    }

    pub fn export_popup_visible(&self) -> bool {
        self.selected_tab == DetailTab::Animation
    }

    pub fn filter_popup_open(&self) -> bool {
        self.filter.filter_state.is_open
    }

    pub(crate) fn filter_scroll_task<M: 'static>(&self) -> Task<M> {
        self.filter.restore_scroll()
    }

    pub fn filter_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.filter
            .filter_state
            .is_open
            .then(|| self.filter.view(&self.img015_sheets, &self.custom_assets, window).map(Message::Filter))
    }

    pub(crate) fn export_scroll_task<M: 'static>(&self) -> Task<M> {
        self.animation.export_scroll_task()
    }

    pub fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.animation.export_popup_view(window).map(|view| view.map(Message::Animation))
    }

    pub fn view<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let sidebar = self.view_sidebar();
        let main_content = self.view_main_content(settings, app_state, global_ctx);

        let base_layout = row![sidebar, main_content]
            .width(Length::Fill)
            .height(Length::Fill);

        base_layout.into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        const FILTER_SEARCH_GAP: f32 = 4.0;
        const SEARCH_LIST_GAP: f32 = 8.0;

        let search_input = text_input("Search Enemy...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(4)
            .size(13)
            .width(Length::Fill)
            .style(theme::rounded_input);

        let filter_button = button(theme::button_label("Filter").size(13))
            .on_press(Message::Filter(filter::Message::Toggle))
            .padding([4, 8])
            .width(Length::Fill)
            .style(move |t: &Theme, status| theme::toggle_button(t, status, self.filter.filter_state.is_active()));

        let enemy_list = self.list.view(&self.data.enemies, self.selected_enemy, self.is_indexing()).map(Message::List);

        let mut sidebar = column![
            filter_button,
            Space::new().height(Length::Fixed(FILTER_SEARCH_GAP)),
            search_input,
            Space::new().height(Length::Fixed(SEARCH_LIST_GAP)),
        ];

        sidebar = sidebar.push(enemy_list);

        container(
            sidebar
                .spacing(0)
                .height(Length::Fill)
        )
            .width(Length::Fixed(roster_list::LIST_WIDTH + 16.0))
            .height(Length::Fill)
            .padding(8)
            .style(theme::list_panel_container)
            .into()
    }

    fn is_indexing(&self) -> bool {
        self.scan_progress.is_some() && self.data.enemies.is_empty()
    }

    fn scan_status(&self) -> Option<Element<'_, Message>> {
        if !self.is_indexing() {
            return None;
        }

        let (done, total) = self.scan_progress?;

        if total == 0 {
            return Some(status("Indexing File System...", None));
        }

        Some(status("Scanning Enemies...", Some(format!("{} / {}", done, total))))
    }

    fn view_main_content<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        if let Some(progress) = self.scan_status() {
            return progress;
        }

        let Some(selected_id) = self.selected_enemy else {
            return container(text("Select a Unit").size(24))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let Some(enemy) = self.data.enemies.iter().find(|e| e.id == selected_id) else {
            return container(text("No Enemy Data"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        };

        let header = self.view_header(enemy, &global_ctx.vault.vfs, settings);

        let content = match self.selected_tab {
            DetailTab::Abilities => self.view_abilities(enemy, settings, global_ctx),
            DetailTab::Details => details::view(enemy),
            DetailTab::Animation => editor::target(
                self.animation.view(settings, &app_state.animation).map(Message::Animation),
                editor::Target::EnemyAnimation,
            ),
        };

        column![
            header,
            Space::new().height(Length::Fixed(8.0)),
            rule::horizontal(1),
            Space::new().height(Length::Fixed(8.0)),
            content
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding { top: 4.0, right: 16.0, bottom: 16.0, left: 16.0 })
            .into()
    }

    fn view_header<'a>(&'a self, enemy: &'a EnemyEntry, vfs: &Vfs, settings: &'a Settings) -> Element<'a, Message> {
        let mut tab_row = row![].spacing(4);
        let tabs = [
            (DetailTab::Abilities, "Abilities"),
            (DetailTab::Details, "Details"),
            (DetailTab::Animation, "Animation"),
        ];

        for (tab_enum, label) in tabs {
            let is_selected = self.selected_tab == tab_enum;

            let btn = button(theme::centered_text(label).size(12))
                .width(Length::Fixed(HEADER_BUTTON_WIDTH))
                .height(Length::Fixed(HEADER_BUTTON_HEIGHT))
                .on_press(Message::SelectTab(tab_enum))
                .style(move |t: &Theme, status| theme::header_toggle_button(t, status, is_selected, true));

            tab_row = tab_row.push(btn);
        }

        let mut detail_row = row![
            self.view_enemy_icon(enemy, vfs),
            self.view_info_box(enemy),
        ].spacing(12).align_y(Vertical::Center);

        match self.selected_tab {
            DetailTab::Abilities => {
                detail_row = detail_row.push(Space::new().width(Length::Fixed(DETAIL_RULE_GAP)));
                detail_row = detail_row.push(container(rule::vertical(1)).height(Length::Fixed(DETAIL_RULE_HEIGHT)));
                detail_row = detail_row.push(Space::new().width(Length::Fixed(EXPORT_BUTTON_RULE_GAP)));
                detail_row = detail_row.push(self.export.view().map(Message::Export));
            }
            DetailTab::Details => {
                detail_row = detail_row.push(Space::new().width(Length::Fixed(DETAIL_RULE_GAP)));
                detail_row = detail_row.push(container(rule::vertical(1)).height(Length::Fixed(DETAIL_RULE_HEIGHT)));
                detail_row = detail_row.push(Space::new().width(Length::Fixed(EXPORT_BUTTON_RULE_GAP)));
                detail_row = detail_row.push(self.view_appearances_button(enemy.id));
            }
            DetailTab::Animation => {
                detail_row = detail_row.push(Space::new().width(Length::Fixed(DETAIL_RULE_GAP)));
                detail_row = detail_row.push(container(rule::vertical(1)).height(Length::Fixed(DETAIL_RULE_HEIGHT)));
                detail_row = detail_row.push(Space::new().width(Length::Fixed(EXPORT_BUTTON_RULE_GAP)));
                detail_row = detail_row.push(animation::debug_toggles(settings, Message::ToggleOrigin, Message::ToggleParts, Message::ToggleWorld));
            }
        }

        column![
            Space::new().height(Length::Fixed(HEADER_BUTTON_TOP_PADDING)),
            tab_row,
            Space::new().height(Length::Fixed(8.0)),
            rule::horizontal(1),
            Space::new().height(Length::Fixed(8.0)),
            detail_row,
        ].into()
    }

    fn view_appearances_button(&self, enemy_id: u32) -> Element<'_, Message> {
        let label = theme::centered_text("Appearances")
            .size(APPEARANCES_TEXT_SIZE)
            .width(Length::Fill)
            .height(Length::Fill);

        button(label)
            .width(Length::Fixed(statblock_export::ACTIONS_WIDTH))
            .height(Length::Fixed(statblock_export::ACTIONS_HEIGHT))
            .padding(0)
            .on_press(Message::NavigateAppearances(enemy_id))
            .style(theme::primary_button)
            .into()
    }

    fn view_enemy_icon(&self, enemy: &EnemyEntry, vfs: &Vfs) -> Element<'_, Message> {
        let icon = self.enemy_icon(enemy.icon_path.as_ref(), vfs);
        let (width, height) = icon.scale(ICON_BOX_WIDTH, ICON_BOX_HEIGHT);

        editor::target(
            container(
                iced_image(icon.handle)
                    .width(Length::Fixed(width))
                    .height(Length::Fixed(height))
            )
                .width(Length::Fixed(ICON_BOX_WIDTH))
                .height(Length::Fixed(ICON_BOX_HEIGHT))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Bottom),
            editor::Target::EnemyIcon,
        )
    }

    fn enemy_icon(&self, path: Option<&PathBuf>, vfs: &Vfs) -> HeaderIcon {
        if let Some(path) = path
            && let Some(icon) = header_icon::load(&self.header_icon_cache, path)
        {
            return icon;
        }

        vfs.find(EMPTY_CAT_ICON)
            .and_then(|fallback| header_icon::load(&self.header_icon_cache, &fallback))
            .unwrap_or_else(|| self.header_icon_dummy.clone())
    }


    fn view_info_box<'a>(&'a self, enemy: &'a EnemyEntry) -> Element<'a, Message> {
        let disp_name = enemy.display_name();

        let id_text = text(format!("ID: {:03}-E", enemy.id))
            .size(11)
            .style(|theme: &Theme| text::Style { color: Some(Color { a: 0.4, ..theme.palette().text }) });

        let mag_row = row![
            text("Magnify:").size(11).align_y(Vertical::Center),
            text_input("100", &self.mag_input)
                .on_input(Message::MagnificationChanged)
                .size(11)
                .padding(3)
                .width(Length::Fixed(45.0))
                .style(theme::rounded_input),
            text("%").size(11).align_y(Vertical::Center),
        ].spacing(6).align_y(Vertical::Center);

        column![
            editor::target(name_box(disp_name, 123.0, 56.0, 145.0), editor::Target::EnemyName),
            id_text,
            mag_row,
        ].spacing(0).into()
    }

    fn view_abilities<'a>(&'a self, enemy: &'a EnemyEntry, settings: &'a Settings, global_ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let dynamic_entry = self.dynamic_stats(enemy.id, global_ctx.vault, settings.show_invalid_enemies());
        let stats = dynamic_entry.as_ref().unwrap_or(&enemy.stats);

        let enemy_ctx = RenderContext::enemy(global_ctx, stats, self.magnification);

        editor::target(
            column![
                self.view_stats(enemy, stats),
                Space::new().height(Length::Fixed(8.0)),
                self.abilities.view(&enemy_ctx, &self.img015_sheets, &self.custom_assets, |items, layout| {
                    self.abilities.ability_list(items, &self.img015_sheets, &self.custom_assets, layout)
                })
            ]
                .width(Length::Fill)
                .height(Length::Fill),
            editor::Target::EnemyAttributes,
        )
    }

    fn view_stats(&self, enemy: &EnemyEntry, stats: &Entity) -> Element<'_, Message> {
        let stat_ctx = StatContext::enemy(stats, enemy.atk_anim_frames, self.magnification);

        let atk_str = format_stat(&STAT_ATTACK, &stat_ctx);
        let dps_str = format_stat(&STAT_DPS, &stat_ctx);
        let range_str = format_stat(&STAT_RANGE, &stat_ctx);
        let cash_str = format_stat(&STAT_CASH_DROP, &stat_ctx);
        let hp_str = format_stat(&STAT_HITPOINTS, &stat_ctx);
        let kb_str = format_stat(&STAT_KNOCKBACKS, &stat_ctx);
        let speed_str = format_stat(&STAT_SPEED, &stat_ctx);

        let cycle = (STAT_ATK_CYCLE.get_value)(&stat_ctx);

        let header_row = row![
            grid_header(STAT_ATTACK.display_name),
            grid_header(STAT_DPS.display_name),
            grid_header(STAT_RANGE.display_name),
            grid_header(STAT_ATK_CYCLE.display_name),
        ].spacing(4);

        let value_row = row![
            grid_value(STAT_ATTACK.display_name, &atk_str),
            grid_value(STAT_DPS.display_name, &dps_str),
            grid_value(STAT_RANGE.display_name, &range_str),
            grid_frames(STAT_ATK_CYCLE.display_name, cycle),
        ].spacing(4);

        let header_row2 = row![
            grid_header(STAT_HITPOINTS.display_name),
            grid_header(STAT_KNOCKBACKS.display_name),
            grid_header(STAT_SPEED.display_name),
            grid_header(STAT_CASH_DROP.display_name),
        ].spacing(4);

        let value_row2 = row![
            grid_value(STAT_HITPOINTS.display_name, &hp_str),
            grid_value(STAT_KNOCKBACKS.display_name, &kb_str),
            grid_value(STAT_SPEED.display_name, &speed_str),
            grid_value(STAT_CASH_DROP.display_name, &cash_str),
        ].spacing(4);

        column![header_row, value_row, header_row2, value_row2].spacing(4).into()
    }
}
