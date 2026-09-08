mod menu;
mod registry;
mod figures;
mod prose;
mod target;
mod watch;

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use iced::{Element, Point, Size, Task};
use rustc_hash::FxHashMap;
use tracing::{info, trace, warn};

use kore::domains::cat::animation as cat_animation;
use kore::domains::cat::files as cat_files;
use kore::domains::cat::waiter as cat_waiter;
use kore::domains::enemy::animation as enemy_animation;
use kore::domains::enemy::files as enemy_files;
use kore::domains::enemy::scanner::EnemyEntry;
use kore::common::architecture;
use kore::domains::mods;
use kore::domains::settings::{ContextScope, EditorMode};
use kore::Vfs;
use nyanko::graphics::tools::crash::Side;

use crate::app::{theme, BattleCatsApp, Page};
use crate::domains::cat::DetailTab;
use crate::domains::studio;
use kore::domains::studio as kore_studio;
use crate::domains::enemy::DetailTab as EnemyTab;
use crate::common::dialog;
use crate::widget::popup;
use crate::common::feedback::{Slot, NO_ACTIONS_LABEL, NO_ACTIONS_NOTICE, UNSUPPORTED_NOTICE};

pub(crate) use target::{deflect, suppress, target};
pub(crate) use watch::watch;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    FileRow(usize),
    CatRow(u32),
    EnemyRow(u32),
    CatLevels,
    CatForms,
    CatTalents,
    CatAttributes,
    EnemyAttributes,
    CatAnimation,
    EnemyAnimation,
    AnimPart(usize),
    AnimCurve(usize),
    AnimFields,
    CatIcon,
    EnemyIcon,
    CatExplanation,
    EnemyName,
    EnemyDescription,
}

pub(crate) struct Context {
    enabled: bool,
    values: EditorMode,
    page: Page,
    file: Option<FileTarget>,
    cats: Vec<CatTarget>,
    enemies: Vec<EnemyTarget>,
    icon: Option<IconTarget>,
    banner: Option<BannerTarget>,
    assets: Vec<AssetTarget>,
    prose: Vec<ProseTarget>,
    levels: Vec<LevelTarget>,
    animation: Option<AnimTarget>,
    channels: Option<ChannelTarget>,
    offsets: Option<studio::Offsets>,
}

struct ChannelTarget {
    channels: studio::Channels,
}

struct AnimTarget {
    key: String,
    clip: Option<String>,
    asset: Asset,
    unlocked: bool,
    active_mod: Option<String>,
}

struct LevelTarget {
    subject: figures::Subject,
    asset: Asset,
    label: String,
    address: figures::Address,
    unlocked: bool,
    active_mod: Option<String>,
}

struct AssetTarget {
    asset: Asset,
    unlocked: bool,
    active_mod: Option<String>,
}

const LEVEL_FILES: [(figures::Subject, &str); 2] =
    [(figures::Subject::Buy, cat_files::UNIT_BUY), (figures::Subject::Curve, cat_files::UNIT_LEVEL)];

const FORM_FILES: [(figures::Subject, &str); 1] = [(figures::Subject::Buy, cat_files::UNIT_BUY)];

const TALENT_FILES: [(figures::Subject, &str); 2] = [
    (figures::Subject::Talents, cat_files::SKILL_ACQUISITION),
    (figures::Subject::Costs, cat_files::SKILL_LEVEL),
];

const FIRST_COST: u32 = 1;

struct AssetFile {
    name: String,
    game: Option<PathBuf>,
    mod_copy: Option<PathBuf>,
}

fn asset_files(app: &BattleCatsApp, base: &str) -> Vec<AssetFile> {
    let names = app.vault.vfs.variants(base);

    let copies: Vec<Option<PathBuf>> = {
        let located = mod_copies(app, names.iter().map(String::as_str));

        names.iter().map(|name| located.get(name.as_str()).cloned()).collect()
    };

    names
        .into_iter()
        .zip(copies)
        .map(|(name, mod_copy)| {
            let game = app.vault.vfs.rooted(architecture::GAME, &name);

            AssetFile { name, game, mod_copy }
        })
        .collect()
}

fn mod_copies<'a>(
    app: &BattleCatsApp,
    names: impl IntoIterator<Item = &'a str>,
) -> FxHashMap<&'a str, PathBuf> {
    app.mods_state
        .active_mod()
        .map_or_else(FxHashMap::default, |active| mods::find_all(&app.vault.vfs, &active, names))
}

struct Exception {
    internal: String,
    vanilla: Option<PathBuf>,
    mod_copy: Option<PathBuf>,
    variants: Vec<AssetFile>,
}

struct Scope<'a> {
    name: &'a str,
    source: Option<&'a Path>,
    present: Option<&'a Path>,
}

fn exception(app: &BattleCatsApp, wanted: String) -> Option<Exception> {
    exceptions(app, vec![wanted]).pop()
}

fn exceptions(app: &BattleCatsApp, wanted: Vec<String>) -> Vec<Exception> {
    let visible: Vec<String> = wanted.iter().map(|name| visible_name(app, name)).collect();

    let internal: Vec<String> = visible
        .iter()
        .map(|name| app.vault.vfs.stripped(name).unwrap_or_else(|| name.clone()))
        .collect();

    let groups: Vec<Vec<String>> = internal.iter().map(|name| app.vault.vfs.variants(name)).collect();

    let located: FxHashMap<String, PathBuf> = {
        let names = internal.iter().map(String::as_str).chain(groups.iter().flatten().map(String::as_str));

        mod_copies(app, names).into_iter().map(|(name, path)| (name.to_owned(), path)).collect()
    };

    visible
        .into_iter()
        .zip(internal)
        .zip(groups)
        .map(|((seen, internal), group)| {
            let variants: Vec<AssetFile> = group
                .into_iter()
                .map(|name| AssetFile {
                    game: app.vault.vfs.rooted(architecture::GAME, &name),
                    mod_copy: located.get(&name).cloned(),
                    name,
                })
                .collect();

            let vanilla = app
                .vault
                .vfs
                .rooted(architecture::GAME, &seen)
                .or_else(|| variants.iter().find_map(|file| file.game.clone()));

            Exception { mod_copy: located.get(&internal).cloned(), variants, vanilla, internal }
        })
        .collect()
}

fn visible_name(app: &BattleCatsApp, wanted: &str) -> String {
    app.vault
        .vfs
        .find(wanted)
        .as_deref()
        .and_then(Path::file_name)
        .map_or_else(|| wanted.to_owned(), |name| name.to_string_lossy().into_owned())
}

impl Exception {
    fn scopes(&self, in_mod: bool) -> Vec<Scope<'_>> {
        if in_mod {
            return vec![Scope { name: &self.internal, source: self.vanilla.as_deref(), present: self.mod_copy.as_deref() }];
        }

        self.variants
            .iter()
            .map(|file| Scope { name: &file.name, source: file.game.as_deref(), present: file.game.as_deref() })
            .collect()
    }
}

enum Asset {
    Variants { key: String, files: Vec<AssetFile> },
    Exception(Exception),
}

impl Asset {
    fn key(&self) -> &str {
        match self {
            Asset::Exception(exception) => &exception.internal,
            Asset::Variants { key, .. } => key,
        }
    }

    fn names(&self) -> Vec<String> {
        match self {
            Asset::Exception(exception) => {
                exception.variants.iter().map(|file| file.name.clone()).collect()
            }
            Asset::Variants { files, .. } => files.iter().map(|file| file.name.clone()).collect(),
        }
    }

    fn scopes(&self, in_mod: bool) -> Vec<Scope<'_>> {
        match self {
            Asset::Exception(exception) => exception.scopes(in_mod),
            Asset::Variants { files, .. } => files
                .iter()
                .map(|file| Scope {
                    name: &file.name,
                    source: file.game.as_deref(),
                    present: if in_mod { file.mod_copy.as_deref() } else { file.game.as_deref() },
                })
                .collect(),
        }
    }
}

struct ProseTarget {
    subject: prose::Subject,
    asset: Asset,
    label: String,
    row: usize,
    unlocked: bool,
    active_mod: Option<String>,
}

struct IconTarget {
    asset: Exception,
    unlocked: bool,
    active_mod: Option<String>,
}

struct BannerTarget {
    key: String,
    forms: Vec<Exception>,
    unlocked: bool,
    active_mod: Option<String>,
}

impl BannerTarget {
    fn scopes(&self, in_mod: bool) -> Vec<Scope<'_>> {
        self.forms.iter().flat_map(|form| form.scopes(in_mod)).collect()
    }
}

const ENEMY_HEADER_ROWS: usize = 2;
const NORMAL_FORM: usize = 0;
const NO_EGGS: (i32, i32) = (-1, -1);

struct EnemyTarget {
    file: String,
    mod_copy: Option<PathBuf>,
    name: Option<String>,
    source: PathBuf,
    row: usize,
    unlocked: bool,
    active_mod: Option<String>,
}

impl EnemyTarget {
    fn title(&self) -> String {
        match self.name.as_deref() {
            Some(name) if !name.is_empty() => [name, self.file.as_str()].join(theme::HEADER_SEPARATOR),
            _ => self.file.clone(),
        }
    }
}

struct CatTarget {
    file: String,
    mod_copy: Option<PathBuf>,
    name: Option<String>,
    source: PathBuf,
    form: usize,
    unlocked: bool,
    active_mod: Option<String>,
}

impl CatTarget {
    fn title(&self) -> String {
        let form = figures::FORMS.get(self.form).copied().unwrap_or("Unknown");

        match self.name.as_deref() {
            Some(name) => [name, form, self.file.as_str()].join(theme::HEADER_SEPARATOR),
            None => [self.file.as_str(), form].join(theme::HEADER_SEPARATOR),
        }
    }
}

struct FileTarget {
    source: PathBuf,
    game: Option<PathBuf>,
    name: String,
    mount: String,
    folder: bool,
    unlocked: bool,
    active_mod: Option<String>,
    mod_copy: Option<PathBuf>,
}

pub(super) type Trail = Vec<usize>;

#[derive(Clone, Debug)]
pub enum Message {
    Opened(Point, Option<Target>),
    Dismissed,
    Invoked(Trail),
    Hovered(Trail),
    ConfirmExpired,
    FailureExpired,
    Replaced(bool),
    Figures(figures::Subject, figures::Message),
    Prose(prose::Subject, prose::Message),
}

struct Item {
    label: String,
    hint: Option<String>,
    action: Option<Action>,
    children: Vec<Item>,
    confirm: bool,
    terse: bool,
}

impl Item {
    fn new(label: impl Into<String>, action: Action) -> Self {
        Self { label: label.into(), hint: None, action: Some(action), children: Vec::new(), confirm: false, terse: false }
    }

    fn disabled(label: impl Into<String>, hint: impl Into<String>) -> Self {
        Self { label: label.into(), hint: Some(hint.into()), action: None, children: Vec::new(), confirm: false, terse: false }
    }

    fn unsupported(label: impl Into<String>) -> Self {
        Self::disabled(label, UNSUPPORTED_NOTICE)
    }

    fn list(label: impl Into<String>, children: Vec<Item>) -> Self {
        let hint = shared_hint(&children);

        Self { label: label.into(), hint, action: None, children, confirm: false, terse: false }
    }

    fn opens(&self) -> bool {
        !self.children.is_empty()
    }

    fn live(&self) -> bool {
        self.action.is_some() || self.children.iter().any(Item::live)
    }

    fn relabel(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    fn confirming(mut self) -> Self {
        self.confirm = true;
        self
    }

    fn confirming_terse(mut self) -> Self {
        self.confirm = true;
        self.terse = true;
        self
    }
}

fn shared_hint(children: &[Item]) -> Option<String> {
    if children.iter().any(Item::live) {
        return None;
    }

    let mut hints = children.iter().map(|child| child.hint.as_deref());
    let first = hints.next().flatten()?;

    hints.all(|hint| hint == Some(first)).then(|| first.to_owned())
}

enum Action {
    Add { source: PathBuf, target_mod: String },
    Delete { source: PathBuf },
    AddChannel { part: usize, kind: i32 },
    DropChannel { track: usize },
    AddPart { parent: Option<usize> },
    AddOffset,
    DropOffset { row: usize },
    Locate { part: usize },
    DropPart { part: usize },
    EditAnimation(AnimPlan),
    EditFigures(figures::Plan),
    EditProse(prose::Plan),
    Replace { file: String, target_mod: Option<String>, game: Option<PathBuf> },
    Sync { file: String, target_mod: String, game: PathBuf },
    Open { source: PathBuf },
    Find { source: PathBuf },
}

enum Outcome {
    Done,
    Opened(Page),
    Deferred(Task<Message>),
    Failed,
}

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const PROBE_BYTES: u64 = 1024;

pub(super) enum Format {
    Text,
    Image,
    Binary,
}

pub(super) fn classify(path: &Path) -> Format {
    let Ok(file) = fs::File::open(path) else {
        return Format::Binary;
    };

    let mut probe = Vec::new();

    if file.take(PROBE_BYTES).read_to_end(&mut probe).is_err() {
        return Format::Binary;
    }

    if probe.starts_with(&PNG_MAGIC) {
        return Format::Image;
    }

    if probe.contains(&0) {
        return Format::Binary;
    }

    match std::str::from_utf8(&probe) {
        Ok(_) => Format::Text,
        Err(err) if err.error_len().is_none() => Format::Text,
        Err(_) => Format::Binary,
    }
}

fn pick<'a>(items: &'a [Item], trail: &[usize]) -> Option<&'a Item> {
    let (last, parents) = trail.split_last()?;
    let mut level = items;

    for index in parents {
        level = &level.get(*index)?.children;
    }

    level.get(*last)
}

fn panels<'a>(items: &'a [Item], trail: &[usize]) -> Vec<&'a [Item]> {
    let mut open = Vec::with_capacity(trail.len());
    let mut level = items;

    for index in trail {
        let Some(item) = level.get(*index).filter(|item| item.opens()) else {
            break;
        };

        level = item.children.as_slice();
        open.push(level);
    }

    open
}

#[derive(Default)]
pub(crate) struct State {
    open: Option<Open>,
    opened: Option<Page>,
    pending: Option<Trail>,
    confirm: Slot<Trail>,
    failed: Slot<Trail>,
    figures: [figures::State; figures::COUNT],
    prose: [prose::State; prose::COUNT],
    synced: Option<Key>,
}

struct Snapshot {
    page: Page,
    figures: [Option<figures::Plan>; figures::COUNT],
    prose: [Vec<prose::Plan>; prose::COUNT],
}

#[derive(PartialEq)]
struct Key {
    page: Page,
    values: EditorMode,
    cat: Option<u32>,
    form: usize,
    enemy: Option<u32>,
    unlocked: bool,
    active_mod: Option<String>,
}

pub(crate) struct Update {
    snapshot: Snapshot,
    key: Key,
}

struct Open {
    at: Point,
    items: Vec<Item>,
    trail: Trail,
}

impl State {
    pub(crate) fn open(&mut self, at: Point, context: &Context) {
        let mut items = registry::items(context);

        if items.is_empty() {
            trace!(
                page = ?context.page,
                targeted = context.file.is_some(),
                nightly = context.enabled,
                "Right click produced no context menu actions"
            );

            if context.enabled {
                items.push(Item::disabled(NO_ACTIONS_LABEL, NO_ACTIONS_NOTICE));
            }
        }

        self.confirm.expire();
        self.failed.expire();
        self.pending = None;
        self.open = (!items.is_empty()).then_some(Open { at, items, trail: Trail::new() });
    }

    pub(crate) fn update(&mut self, message: Message, vfs: &Vfs, studio: &mut studio::State) -> Task<Message> {
        match message {
            Message::Invoked(trail) => self.invoke(trail, vfs, studio),
            Message::Hovered(trail) => {
                if let Some(open) = self.open.as_mut() {
                    open.trail = trail;
                }

                Task::none()
            }
            Message::Dismissed => {
                self.open = None;
                self.pending = None;
                self.confirm.expire();
                self.failed.expire();
                Task::none()
            }
            Message::ConfirmExpired => {
                self.confirm.expire();
                Task::none()
            }
            Message::FailureExpired => {
                self.failed.expire();
                Task::none()
            }
            Message::Replaced(placed) => {
                let Some(trail) = self.pending.take() else { return Task::none(); };

                if placed {
                    self.open = None;
                    return Task::none();
                }

                self.failed.set(trail, Message::FailureExpired)
            }
            Message::Prose(subject, msg) => self
                .prose_mut(subject)
                .update(msg, vfs)
                .map(move |inner| Message::Prose(subject, inner)),
            Message::Figures(subject, msg) => self
                .subject_mut(subject)
                .update(msg, vfs)
                .map(move |inner| Message::Figures(subject, inner)),
            Message::Opened(..) => Task::none(),
        }
    }

    pub(crate) fn take_opened(&mut self) -> Option<Page> {
        self.opened.take()
    }

    pub(crate) fn popup_view<'a>(
        &'a self,
        app: &'a BattleCatsApp,
        window: Size,
    ) -> Vec<(u64, popup::Kind, Element<'a, Message>)> {
        let mut views: Vec<(u64, popup::Kind, Element<'_, Message>)> = Vec::new();
        let cap = level_cap(app);
        let used = talent_costs(app);

        for subject in prose::SUBJECTS {
            if subject.page() != app.current_page || !prose_tab(app, subject) {
                continue;
            }

            let slot = &self.prose[subject.slot()];

            if let Some(view) = slot.view(window) {
                views.push((slot.raised(), subject.kind(), view.map(move |inner| Message::Prose(subject, inner))));
            }
        }

        for subject in figures::SUBJECTS {
            if subject.page() != app.current_page || !figures_tab(app, subject) {
                continue;
            }

            let slot = &self.figures[subject.slot()];

            if let Some(view) = slot.view(window, cap, used, &app.vault) {
                views.push((slot.raised(), figures::kind(subject), view.map(move |inner| Message::Figures(subject, inner))));
            }
        }

        views
    }

    #[cfg(test)]
    pub(crate) fn kinds() -> Vec<popup::Kind> {
        prose::SUBJECTS
            .into_iter()
            .map(prose::Subject::kind)
            .chain(figures::SUBJECTS.into_iter().map(figures::kind))
            .collect()
    }

    pub(crate) fn restore_scroll<M: Send + 'static>(&self) -> Task<M> {
        let tasks: Vec<Task<M>> =
            figures::SUBJECTS.iter().filter_map(|subject| self.figures[subject.slot()].restore_scroll()).collect();

        Task::batch(tasks)
    }

    pub(crate) fn relocalize(&self) {
        for slot in &self.figures {
            slot.relocalize();
        }
    }

    pub(crate) fn flush_now(&mut self, vfs: &Vfs) {
        for slot in &mut self.figures {
            slot.flush_now(vfs);
        }

        for slot in &mut self.prose {
            slot.flush_now(vfs);
        }
    }

    pub(crate) fn flush_drafts(&mut self, vfs: &Vfs) -> Task<Message> {
        let figures = figures::SUBJECTS
            .into_iter()
            .map(|subject| self.figures[subject.slot()].flush(vfs).map(move |inner| Message::Figures(subject, inner)));

        let prose = prose::SUBJECTS
            .into_iter()
            .map(|subject| self.prose[subject.slot()].flush(vfs).map(move |inner| Message::Prose(subject, inner)));

        Task::batch(figures.chain(prose))
    }

    fn drafting(&self) -> bool {
        self.figures.iter().any(figures::State::drafting)
            || self.prose.iter().any(prose::State::drafting)
    }

    fn figures_drafting(&self, subject: figures::Subject) -> bool {
        self.figures[subject.slot()].drafting()
    }

    fn prose_drafting(&self, subject: prose::Subject) -> bool {
        self.prose[subject.slot()].drafting()
    }

    fn stale(&self, key: &Key) -> bool {
        self.synced.as_ref() != Some(key) || self.drifted()
    }

    fn drifted(&self) -> bool {
        self.figures.iter().any(figures::State::drifted)
            || self.prose.iter().any(prose::State::drifted)
    }

    pub(crate) fn apply(&mut self, update: Update, vfs: &Vfs) {
        let Update { snapshot, key } = update;
        self.synced = Some(key);

        for subject in prose::SUBJECTS {
            if subject.page() != snapshot.page {
                continue;
            }

            self.prose[subject.slot()].sync(&snapshot.prose[subject.slot()], vfs);
        }

        for (slot, plan) in snapshot.figures.into_iter().enumerate() {
            if figures::SUBJECTS[slot].page() != snapshot.page {
                continue;
            }

            self.figures[slot].sync(plan, vfs);
        }
    }

    fn stacked(&self, page: Page) -> usize {
        let prose = prose::SUBJECTS
            .into_iter()
            .filter(|subject| subject.page() == page && self.prose[subject.slot()].drafting())
            .count();

        let figures = figures::SUBJECTS
            .into_iter()
            .filter(|subject| subject.page() == page && self.figures[subject.slot()].drafting())
            .count();

        prose + figures
    }

    fn subject_mut(&mut self, subject: figures::Subject) -> &mut figures::State {
        &mut self.figures[subject.slot()]
    }

    fn prose_mut(&mut self, subject: prose::Subject) -> &mut prose::State {
        &mut self.prose[subject.slot()]
    }

    fn perform(&mut self, action: &Action, vfs: &Vfs, studio: &mut studio::State) -> Outcome {
        match action {
            Action::Add { source, target_mod } => match mods::adopt(target_mod, source) {
                Ok(path) => {
                    info!(path = %path.display(), "Added a file to a mod");

                    Outcome::Done
                }
                Err(err) => {
                    warn!(source = %source.display(), "Failed to add the file to the mod: {}", err);

                    Outcome::Failed
                }
            },
            Action::Delete { source } => match fs::remove_file(source) {
                Ok(()) => {
                    info!(path = %source.display(), "Deleted a mod file");

                    Outcome::Done
                }
                Err(err) => {
                    warn!(path = %source.display(), "Failed to delete the file: {}", err);

                    Outcome::Failed
                }
            },
            Action::Open { source } => match open::that(source) {
                Ok(()) => {
                    info!(path = %source.display(), "Opened a file in an external program");

                    Outcome::Done
                }
                Err(err) => {
                    warn!(path = %source.display(), "Failed to open the file: {}", err);

                    Outcome::Failed
                }
            },
            Action::Find { source } => {
                let Some(folder) = source.parent() else {
                    warn!(path = %source.display(), "The file has no containing folder to open");

                    return Outcome::Failed;
                };

                match open::that(folder) {
                    Ok(()) => {
                        info!(path = %folder.display(), "Opened a containing folder");

                        Outcome::Done
                    }
                    Err(err) => {
                        warn!(path = %folder.display(), "Failed to open the containing folder: {}", err);

                        Outcome::Failed
                    }
                }
            }
            Action::AddChannel { part, kind } => match studio.add_channel(*part, *kind) {
                true => Outcome::Done,
                false => Outcome::Failed,
            },
            Action::DropChannel { track } => match studio.drop_channel(*track) {
                true => Outcome::Done,
                false => Outcome::Failed,
            },
            Action::AddPart { parent } => match studio.add_part(*parent) {
                true => Outcome::Done,
                false => Outcome::Failed,
            },
            Action::AddOffset => match studio.add_offset() {
                true => Outcome::Done,
                false => Outcome::Failed,
            },
            Action::DropOffset { row } => match studio.drop_offset(*row) {
                true => Outcome::Done,
                false => Outcome::Failed,
            },
            Action::Locate { part } => match studio.locate(*part) {
                true => Outcome::Done,
                false => Outcome::Failed,
            },
            Action::DropPart { part } => match studio.drop_part(*part) {
                true => Outcome::Done,
                false => Outcome::Failed,
            },
            Action::EditAnimation(plan) => match adopt_set(plan, vfs) {
                Some((set, copied, side)) => {
                    studio.adopt(set, plan.target_mod.clone(), plan.clip.clone(), copied, side);

                    Outcome::Opened(Page::Studio)
                }
                None => Outcome::Failed,
            },
            Action::EditFigures(plan) => {
                let subject = plan.subject();
                let already = self.figures[subject.slot()].drafting();
                let nudge = self.stacked(subject.page()) - usize::from(already);

                self.subject_mut(subject).begin(plan.clone(), nudge, vfs);

                Outcome::Done
            }
            Action::EditProse(plan) => {
                let subject = plan.subject();
                let already = self.prose[subject.slot()].drafting();
                let nudge = self.stacked(subject.page()) - usize::from(already);

                self.prose_mut(subject).begin(plan.clone(), nudge, vfs);

                Outcome::Done
            }
            Action::Replace { file, target_mod, game } => {
                replace_icon(file, target_mod.as_deref(), game.as_deref())
            }
            Action::Sync { file, target_mod, game } => match mods::place(target_mod, game, file) {
                Ok(path) => {
                    info!(path = %path.display(), "Synced a mod file with game");

                    Outcome::Done
                }
                Err(err) => {
                    warn!(file, "Failed to sync the file with game: {}", err);

                    Outcome::Failed
                }
            },
        }
    }

    fn invoke(&mut self, trail: Trail, vfs: &Vfs, studio: &mut studio::State) -> Task<Message> {
        let Some((actionable, confirms)) = self
            .open
            .as_ref()
            .and_then(|open| pick(&open.items, &trail))
            .map(|item| (item.action.is_some(), item.confirm))
        else {
            return Task::none();
        };

        if !actionable {
            return Task::none();
        }

        if confirms && !self.confirm.armed_for(&trail) {
            return self.confirm.set(trail, Message::ConfirmExpired);
        }

        self.confirm.expire();
        self.failed.expire();

        let Some(open) = self.open.take() else {
            return Task::none();
        };

        let outcome = pick(&open.items, &trail)
            .and_then(|item| item.action.as_ref())
            .map_or(Outcome::Done, |action| self.perform(action, vfs, studio));

        match outcome {
            Outcome::Done => Task::none(),
            Outcome::Opened(page) => {
                self.opened = Some(page);

                Task::none()
            }
            Outcome::Deferred(task) => {
                self.open = Some(open);
                self.pending = Some(trail);

                task
            }
            Outcome::Failed => {
                self.open = Some(open);

                self.failed.set(trail, Message::FailureExpired)
            }
        }
    }
}

fn channel_target(app: &BattleCatsApp, target: Option<Target>) -> Option<ChannelTarget> {
    let part = match target {
        Some(Target::AnimPart(part)) => Some(part),
        Some(Target::AnimCurve(track)) => app.studio_state.holder(track),
        _ => None,
    };

    Some(ChannelTarget { channels: app.studio_state.channels(part)? })
}

fn expanded(app: &BattleCatsApp) -> bool {
    match app.current_page {
        Page::Cats => app.cat_state.animation_expanded(),
        Page::Enemies => app.enemy_state.animation_expanded(),
        Page::Utilities => app.utilities_state.animation_expanded(),
        Page::Studio => app.studio_state.expanded(),
        _ => false,
    }
}

pub(crate) fn context(app: &BattleCatsApp, target: Option<Target>) -> Context {
    let broad = app.settings.files.context_scope == ContextScope::Broad;
    let reached = |wanted: Target| target == Some(wanted) || broad;

    if app.current_page == Page::Studio || expanded(app) {
        return Context {
            enabled: app.settings.general.enable_nightly,
            values: app.settings.files.editor_mode,
            page: app.current_page,
            file: None,
            cats: Vec::new(),
            enemies: Vec::new(),
            icon: None,
            banner: None,
            assets: Vec::new(),
            prose: Vec::new(),
            levels: Vec::new(),
            animation: None,
            channels: channel_target(app, target),
            offsets: matches!(target, Some(Target::AnimFields))
                .then(|| app.studio_state.offsets())
                .flatten(),
        };
    }

    Context {
        enabled: app.settings.general.enable_nightly,
        values: app.settings.files.editor_mode,
        page: app.current_page,
        file: file_target(app, target),
        cats: cat_payloads(app, reached(Target::CatAttributes)),
        enemies: enemy_payloads(app, reached(Target::EnemyAttributes)),
        icon: icon_target(app, icon_subject(app, target, broad)),
        banner: banner_subject(app, target, broad).and_then(|id| banner_target(app, id)),
        assets: talent_assets(app, reached(Target::CatTalents)),
        prose: prose_payloads(app, target, broad),
        levels: level_payloads(app, reached(Target::CatLevels), reached(Target::CatForms))
            .into_iter()
            .chain(talent_payloads(app, reached(Target::CatTalents)))
            .collect(),
        animation: anim_target(app, reached(Target::CatAnimation), reached(Target::EnemyAnimation)),
        channels: None,
        offsets: None,
    }
}

fn named_files(app: &BattleCatsApp, names: Vec<String>) -> Vec<AssetFile> {
    let copies: Vec<Option<PathBuf>> = {
        let located = mod_copies(app, names.iter().map(String::as_str));

        names.iter().map(|name| located.get(name.as_str()).cloned()).collect()
    };

    names
        .into_iter()
        .zip(copies)
        .map(|(name, mod_copy)| {
            let game = app.vault.vfs.rooted(architecture::GAME, &name);

            AssetFile { name, game, mod_copy }
        })
        .collect()
}

fn anim_target(app: &BattleCatsApp, cats: bool, enemies: bool) -> Option<AnimTarget> {
    let vfs = &app.vault.vfs;

    let (files, clip) = match app.current_page {
        Page::Cats if cats => {
            let id = app.app_state.cat.selected_cat?;
            let form = app.app_state.cat.selected_form;
            let cat = app.cat_state.data.cats.iter().find(|cat| cat.id == id)?;

            (cat_animation::rig_files(cat, form, vfs)?, app.cat_state.animation_clip())
        }
        Page::Enemies if enemies => {
            let id = app.app_state.enemy.selected_enemy?;
            let enemy = app.enemy_state.data.enemies.iter().find(|enemy| enemy.id == id)?;

            (enemy_animation::rig_files(enemy, vfs)?, app.enemy_state.animation_clip())
        }
        _ => return None,
    };

    let key = files.key.clone();
    let asset = Asset::Variants { key: files.key.clone(), files: named_files(app, files.names()) };

    Some(AnimTarget {
        key,
        clip,
        asset,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

#[derive(Clone)]
pub(crate) struct AnimPlan {
    key: String,
    files: Vec<String>,
    target_mod: Option<String>,
    unlocked: bool,
    clip: Option<String>,
}

fn anim_plan(target: &AnimTarget, target_mod: Option<String>) -> AnimPlan {
    AnimPlan {
        key: target.key.clone(),
        files: target.asset.names(),
        target_mod,
        unlocked: target.unlocked,
        clip: target.clip.clone(),
    }
}
fn adopt_set(plan: &AnimPlan, vfs: &Vfs) -> Option<(studio::Set, bool, Option<Side>)> {
    let mut set = studio::Set { name: plan.key.clone(), ..studio::Set::default() };

    for file in &plan.files {
        let seated = match plan.target_mod.as_deref() {
            Some(name) => stage_into(vfs, name, file),
            None => vfs.rooted(architecture::GAME, file),
        };

        let Some(seated) = seated else {
            continue;
        };

        match seated.extension().and_then(|ext| ext.to_str()) {
            Some("png") => set.sheet = Some(seated),
            Some("imgcut") => set.cuts = Some(seated),
            Some("mamodel") => set.model = Some(seated),
            Some("maanim") => set.anims.push(seated),
            _ => {}
        }
    }

    if !set.rigged() {
        return None;
    }

    let side = plan
        .files
        .iter()
        .find(|file| file.ends_with(".mamodel"))
        .and_then(|file| Path::new(file).file_stem()?.to_str())
        .and_then(kore_studio::stem_side);

    if plan.target_mod.is_some() || plan.unlocked {
        return Some((set, false, side));
    }

    let name = kore_studio::vacant(&plan.key);

    match kore_studio::adopt(&name, &set) {
        Ok(copied) => Some((copied, true, side)),
        Err(err) => {
            warn!(name, "Failed to copy a locked rig into studio: {}", err);

            None
        }
    }
}

fn stage_into(vfs: &Vfs, name: &str, file: &str) -> Option<PathBuf> {
    if let Some(held) = vfs.rooted(name, file) {
        return Some(held);
    }

    let game = vfs.rooted(architecture::GAME, file)?;
    let staged = mods::ensure_as(vfs, name, &game, file)
        .inspect_err(|err| warn!(file, "Failed to copy a rig file into the mod: {}", err))
        .ok()?;

    if let Err(err) = vfs.create((name, staged.as_path())) {
        warn!(path = %staged.display(), "Failed to index a staged rig file: {}", err);
    }

    Some(staged)
}

fn figures_tab(app: &BattleCatsApp, subject: figures::Subject) -> bool {
    match subject {
        figures::Subject::Cat => app.cat_state.selected_tab == DetailTab::Abilities,
        figures::Subject::Enemy => app.enemy_state.selected_tab == EnemyTab::Abilities,
        figures::Subject::Buy
        | figures::Subject::Curve
        | figures::Subject::Talents
        | figures::Subject::Costs => true,
    }
}

enum IconSubject {
    Cat(u32),
    Enemy(u32),
}

fn icon_subject(app: &BattleCatsApp, target: Option<Target>, broad: bool) -> Option<IconSubject> {
    match target {
        Some(Target::EnemyRow(id)) => Some(IconSubject::Enemy(id)),
        Some(Target::CatIcon) => app.app_state.cat.selected_cat.map(IconSubject::Cat),
        Some(Target::EnemyIcon) => app.app_state.enemy.selected_enemy.map(IconSubject::Enemy),
        _ if !broad => None,
        _ => match app.current_page {
            Page::Cats => app.app_state.cat.selected_cat.map(IconSubject::Cat),
            Page::Enemies => app.app_state.enemy.selected_enemy.map(IconSubject::Enemy),
            _ => None,
        },
    }
}

fn prose_payloads(app: &BattleCatsApp, target: Option<Target>, broad: bool) -> Vec<ProseTarget> {
    if !broad {
        let Some(subject) = prose_subject(target) else {
            return Vec::new();
        };

        return prose_targets(app, subject);
    }

    prose::SUBJECTS
        .into_iter()
        .filter(|subject| subject.page() == app.current_page && prose_tab(app, *subject))
        .flat_map(|subject| prose_targets(app, subject))
        .collect()
}

fn level_payloads(app: &BattleCatsApp, levels: bool, forms: bool) -> Vec<LevelTarget> {
    let files: &[(figures::Subject, &str)] = match (levels, forms) {
        (true, _) => &LEVEL_FILES,
        (false, true) => &FORM_FILES,
        (false, false) => return Vec::new(),
    };

    roster_payloads(app, files)
}

fn talent_payloads(app: &BattleCatsApp, reached: bool) -> Vec<LevelTarget> {
    if !reached || !talented(app) {
        return Vec::new();
    }

    roster_payloads(app, &TALENT_FILES)
}

fn talented(app: &BattleCatsApp) -> bool {
    app.app_state.cat.selected_cat.is_some()
}

fn address(app: &BattleCatsApp, subject: figures::Subject, id: u32) -> figures::Address {
    match subject {
        figures::Subject::Talents => figures::Address::Keyed(id),
        figures::Subject::Costs => {
            figures::Address::Keyed(talent_costs(app).first().map_or(FIRST_COST, |cost| u32::from(*cost)))
        }
        _ => figures::Address::Line(id as usize),
    }
}

fn talent_costs(app: &BattleCatsApp) -> figures::Marks {
    let mut used = figures::Marks::default();

    let Some(id) = app.app_state.cat.selected_cat else {
        return used;
    };

    let Some(talents) =
        app.cat_state.data.cats.iter().find(|cat| cat.id == id).and_then(|cat| cat.talent_data.as_ref())
    else {
        return used;
    };

    let mut costs: Vec<u8> = talents
        .groups
        .iter()
        .filter(|group| group.ability_id != 0)
        .map(|group| group.cost_id)
        .collect();

    costs.sort_unstable();
    costs.dedup();

    for (slot, cost) in used.iter_mut().zip(costs) {
        *slot = cost;
    }

    used
}

fn roster_payloads(app: &BattleCatsApp, files: &[(figures::Subject, &str)]) -> Vec<LevelTarget> {
    if app.current_page != Page::Cats {
        return Vec::new();
    }

    let Some(id) = app.app_state.cat.selected_cat else {
        return Vec::new();
    };

    let label = cat_label(app, id);

    files
        .iter()
        .copied()
        .filter_map(|(subject, name)| {
            let files = asset_files(app, name);

            if files.is_empty() {
                return None;
            }

            Some(LevelTarget {
                subject,
                asset: Asset::Variants { key: name.to_owned(), files },
                label: [label.as_str(), name].join(theme::HEADER_SEPARATOR),
                address: address(app, subject, id),
                unlocked: app.settings.files.unlock_game_mount,
                active_mod: app.mods_state.active_mod(),
            })
        })
        .collect()
}

fn talent_assets(app: &BattleCatsApp, reached: bool) -> Vec<AssetTarget> {
    if !reached || app.current_page != Page::Cats || !talented(app) {
        return Vec::new();
    }

    let unlocked = app.settings.files.unlock_game_mount;
    let active_mod = app.mods_state.active_mod();

    exception(app, cat_files::SKILL_DESCRIPTIONS.to_owned())
        .map(|described| AssetTarget { asset: Asset::Exception(described), unlocked, active_mod })
        .into_iter()
        .collect()
}

fn level_cap(app: &BattleCatsApp) -> Option<i32> {
    let id = app.app_state.cat.selected_cat?;
    let cat = app.cat_state.data.cats.iter().find(|cat| cat.id == id)?;

    Some(cat.unitbuy.level_cap_catseye + cat.unitbuy.level_cap_plus)
}

fn cat_label(app: &BattleCatsApp, id: u32) -> String {
    let form = app.app_state.cat.selected_form;

    app.cat_state
        .data
        .cats
        .iter()
        .find(|cat| cat.id == id)
        .and_then(|cat| cat.names.get(form).cloned().flatten())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("{id:03}-C"))
}


fn cat_payloads(app: &BattleCatsApp, reached: bool) -> Vec<CatTarget> {
    if !reached || !figures_tab(app, figures::Subject::Cat) {
        return Vec::new();
    }

    cat_subject(app).into_iter().collect()
}

fn enemy_payloads(app: &BattleCatsApp, reached: bool) -> Vec<EnemyTarget> {
    if !reached || !figures_tab(app, figures::Subject::Enemy) {
        return Vec::new();
    }

    enemy_subject(app).into_iter().collect()
}

fn prose_targets(app: &BattleCatsApp, subject: prose::Subject) -> Vec<ProseTarget> {
    prose_target(app, subject).into_iter().collect()
}

fn prose_tab(app: &BattleCatsApp, subject: prose::Subject) -> bool {
    match subject {
        prose::Subject::EnemyDescription => app.enemy_state.selected_tab == EnemyTab::Details,
        prose::Subject::Explanation | prose::Subject::EnemyName => true,
    }
}

fn prose_subject(target: Option<Target>) -> Option<prose::Subject> {
    match target? {
        Target::CatExplanation => Some(prose::Subject::Explanation),
        Target::EnemyName => Some(prose::Subject::EnemyName),
        Target::EnemyDescription => Some(prose::Subject::EnemyDescription),
        _ => None,
    }
}

fn prose_target(app: &BattleCatsApp, subject: prose::Subject) -> Option<ProseTarget> {
    match subject {
        prose::Subject::Explanation => explanation_target(app, app.app_state.cat.selected_cat?),
        prose::Subject::EnemyName => enemy_name_target(app),
        prose::Subject::EnemyDescription => enemy_description_target(app),
    }
}

fn mod_copy(app: &BattleCatsApp, file: &str) -> Option<PathBuf> {
    mods::find(&app.vault.vfs, app.mods_state.active_mod().as_deref()?, Path::new(file))
}

fn enemy_name_target(app: &BattleCatsApp) -> Option<ProseTarget> {
    let id = app.app_state.enemy.selected_enemy?;

    Some(ProseTarget {
        subject: prose::Subject::EnemyName,
        asset: Asset::Exception(exception(app, enemy_files::NAMES.to_owned())?),
        label: enemy_label(app, id),
        row: id as usize,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn enemy_description_target(app: &BattleCatsApp) -> Option<ProseTarget> {
    let id = app.app_state.enemy.selected_enemy?;
    let files = asset_files(app, enemy_files::PICTURE_BOOK);

    if files.is_empty() {
        return None;
    }

    Some(ProseTarget {
        subject: prose::Subject::EnemyDescription,
        asset: Asset::Variants { key: enemy_files::PICTURE_BOOK.to_owned(), files },
        label: enemy_label(app, id),
        row: id as usize,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn enemy_label(app: &BattleCatsApp, id: u32) -> String {
    app.enemy_state
        .data
        .enemies
        .iter()
        .find(|enemy| enemy.id == id)
        .map_or_else(|| format!("{id:03}-E"), EnemyEntry::display_name)
}

fn explanation_target(app: &BattleCatsApp, id: u32) -> Option<ProseTarget> {
    let form = app.app_state.cat.selected_form;

    cat_waiter::unitexplanation_source(&app.vault.vfs, id, form)?;

    let key = cat_files::explanation_file(id);
    let files = asset_files(app, &key);

    if files.is_empty() {
        return None;
    }

    let name = app
        .cat_state
        .data
        .cats
        .iter()
        .find(|cat| cat.id == id)
        .and_then(|cat| cat.names.get(form).cloned().flatten());

    let form_label = figures::FORMS.get(form).copied().unwrap_or("Unknown");

    let label = match name.as_deref() {
        Some(name) if !name.is_empty() => [name, form_label].join(theme::HEADER_SEPARATOR),
        _ => form_label.to_owned(),
    };

    Some(ProseTarget {
        subject: prose::Subject::Explanation,
        asset: Asset::Variants { key, files },
        label,
        row: form,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn icon_target(app: &BattleCatsApp, subject: Option<IconSubject>) -> Option<IconTarget> {
    let resolved = match subject? {
        IconSubject::Cat(id) => {
            let cat = app.cat_state.data.cats.iter().find(|cat| cat.id == id)?;

            cat.deploy_icon_paths[app.app_state.cat.selected_form].as_ref()?
        }
        IconSubject::Enemy(id) => {
            let enemy = app.enemy_state.data.enemies.iter().find(|enemy| enemy.id == id)?;

            enemy.icon_path.as_ref()?
        }
    };

    let name = resolved.file_name()?.to_string_lossy().into_owned();

    Some(IconTarget {
        asset: exception(app, name)?,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn banner_subject(app: &BattleCatsApp, target: Option<Target>, broad: bool) -> Option<u32> {
    match target {
        Some(Target::CatRow(id)) => Some(id),
        _ if !broad => None,
        _ => app.app_state.cat.selected_cat,
    }
}

fn banner_target(app: &BattleCatsApp, id: u32) -> Option<BannerTarget> {
    if app.current_page != Page::Cats {
        return None;
    }

    let cat = app.cat_state.data.cats.iter().find(|cat| cat.id == id)?;
    let egg_ids = cat.egg_ids.unwrap_or(NO_EGGS);

    let wanted: Vec<String> =
        (0..cat.forms.len()).map(|form| cat_files::banner_file(id, form, egg_ids)).collect();

    let forms: Vec<Exception> = exceptions(app, wanted)
        .into_iter()
        .enumerate()
        .filter_map(|(form, asset)| {
            let offered = form == NORMAL_FORM || cat.forms[form] || !asset.variants.is_empty();

            offered.then_some(asset)
        })
        .collect();

    Some(BannerTarget {
        key: cat_files::banner_base(id),
        forms,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn replace_icon(file: &str, target_mod: Option<&str>, game: Option<&Path>) -> Outcome {
    let file = file.to_string();
    let target_mod = target_mod.map(str::to_string);
    let game = game.map(Path::to_path_buf);

    Outcome::Deferred(Task::perform(
        async move {
            let Some(source) = dialog::file("PNG Image", &["png"]).await else { return true; };

            place_icon(&file, target_mod.as_deref(), game.as_deref(), &source)
        },
        Message::Replaced,
    ))
}

fn place_icon(file: &str, target_mod: Option<&str>, game: Option<&Path>, source: &Path) -> bool {
    let placed = match target_mod {
        Some(name) => mods::place(name, source, file),
        None => match game {
            Some(path) => fs::copy(source, path).map(|_| path.to_path_buf()),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "the vanilla icon is missing")),
        },
    };

    placed
        .inspect(|path| info!(path = %path.display(), "Replaced an icon"))
        .inspect_err(|err| warn!(file, "Failed to replace the icon: {}", err))
        .is_ok()
}

fn cat_subject(app: &BattleCatsApp) -> Option<CatTarget> {
    cat_target(app, app.app_state.cat.selected_cat?)
}

fn cat_target(app: &BattleCatsApp, id: u32) -> Option<CatTarget> {
    if app.current_page != Page::Cats {
        return None;
    }

    let file = cat_files::stats_file(id);
    let source = app.vault.vfs.rooted(architecture::GAME, &file)?;
    let form = app.app_state.cat.selected_form;

    let mod_copy = mod_copy(app, &file);

    let name = app
        .cat_state
        .data
        .cats
        .iter()
        .find(|cat| cat.id == id)
        .and_then(|cat| cat.names.get(form).cloned().flatten());

    Some(CatTarget {
        file,
        mod_copy,
        name,
        source,
        form,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn enemy_subject(app: &BattleCatsApp) -> Option<EnemyTarget> {
    if app.current_page != Page::Enemies {
        return None;
    }

    let id = app.app_state.enemy.selected_enemy?;
    let file = enemy_files::STATS.to_owned();
    let source = app.vault.vfs.rooted(architecture::GAME, &file)?;

    let mod_copy = mod_copy(app, &file);

    let name = app
        .enemy_state
        .data
        .enemies
        .iter()
        .find(|enemy| enemy.id == id)
        .map(|enemy| enemy.name.clone());

    Some(EnemyTarget {
        file,
        mod_copy,
        name,
        source,
        row: ENEMY_HEADER_ROWS + id as usize,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

pub(crate) fn refresh(app: &BattleCatsApp, editor: &State, force: bool) -> Option<Update> {
    if !editor.drafting() {
        return None;
    }

    let key = key(app);

    if !force && !editor.stale(&key) {
        return None;
    }

    Some(Update { snapshot: snapshot(app, editor), key })
}

fn key(app: &BattleCatsApp) -> Key {
    Key {
        page: app.current_page,
        values: app.settings.files.editor_mode,
        cat: app.app_state.cat.selected_cat,
        form: app.app_state.cat.selected_form,
        enemy: app.app_state.enemy.selected_enemy,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    }
}

fn snapshot(app: &BattleCatsApp, editor: &State) -> Snapshot {
    Snapshot {
        page: app.current_page,
        figures: figures::SUBJECTS.map(|subject| {
            if subject.page() != app.current_page || !editor.figures_drafting(subject) {
                return None;
            }

            current_plan(app, subject)
        }),
        prose: prose::SUBJECTS.map(|subject| {
            if subject.page() != app.current_page || !editor.prose_drafting(subject) {
                return Vec::new();
            }

            prose_target(app, subject).map(|target| registry::prose_plans(&target)).unwrap_or_default()
        }),
    }
}

fn current_plan(app: &BattleCatsApp, subject: figures::Subject) -> Option<figures::Plan> {
    let values = app.settings.files.editor_mode;

    match subject {
        figures::Subject::Cat => {
            let cat = cat_subject(app)?;
            let active = cat.active_mod.clone();

            Some(registry::cat_plan(&cat, active, values))
        }
        figures::Subject::Enemy => {
            let enemy = enemy_subject(app)?;
            let active = enemy.active_mod.clone();

            Some(registry::enemy_plan(&enemy, active, values))
        }
        figures::Subject::Buy
        | figures::Subject::Curve
        | figures::Subject::Talents
        | figures::Subject::Costs => {
            let sources = match subject {
                figures::Subject::Talents | figures::Subject::Costs => talent_payloads(app, true),
                _ => level_payloads(app, true, false),
            };

            let target = sources.into_iter().find(|level| level.subject == subject)?;
            let mount = target.active_mod.clone();
            let scopes = target.asset.scopes(mount.is_some());

            scopes.first().and_then(|scope| registry::level_plan(&target, scope, mount, values))
        }
    }
}

fn file_target(app: &BattleCatsApp, target: Option<Target>) -> Option<FileTarget> {
    let Some(Target::FileRow(index)) = target else {
        return None;
    };

    let mount = app.files_state.mount()?;
    let (folder, relative) = app.files_state.entry_at(&app.vault.vfs, index)?;
    let source = app.vault.vfs.root(mount)?.join(&relative);
    let name = relative.file_name()?.to_string_lossy().into_owned();
    let active_mod = app.mods_state.active_mod();

    let mod_copy = active_mod.as_deref().and_then(|active| mods::find(&app.vault.vfs, active, Path::new(&name)));

    let game = app.vault.vfs.rooted(architecture::GAME, &name);

    Some(FileTarget {
        source,
        game,
        name,
        mount: mount.to_owned(),
        folder,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod,
        mod_copy,
    })
}
