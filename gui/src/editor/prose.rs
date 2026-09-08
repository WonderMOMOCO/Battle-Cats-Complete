use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use iced::alignment::Horizontal;
use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length, Padding, Size, Task};
use nyanko::combat::Separator;
use nyanko::common;
use tracing::warn;

use kore::common::preview::{self, Stamp};
use kore::Vfs;
use kore::domains::mods;
use kore::domains::stage::names;

use crate::app::{theme, Page};
use crate::common::feedback::{Slot, CONFIRM_LABEL};
use crate::widget::{popup, smooth_scroll};

const FIELD_HEIGHT: f32 = INPUT_SIZE * 1.3 + PADDING * 2.0;
const SYNC_HEIGHT: f32 = INPUT_SIZE * 1.3 + (PADDING + 1.0) * 2.0;

const POPUP_WIDTH: f32 = 420.0;
const NARROW_WIDTH: f32 = POPUP_WIDTH * 0.6;

const EXPLANATION_LABELS: &[&str] = &[
    "Name...",
    "Description Line 1...",
    "Description Line 2...",
    "Description Line 3...",
    "Comment...",
];

const ENEMY_NAME_LABELS: &[&str] = &["Name..."];

const COMBO_NAME_LABELS: &[&str] = &["Combo Name..."];

const MAP_NAME_LABELS: &[&str] = &["Map Name..."];

const ITEM_NAME_LABELS: &[&str] =
    &["Item Name...", "Description Line 1...", "Description Line 2...", "Description Line 3..."];

const STAGE_NAME_LABELS: &[&str] = &["Stage Name..."];

const TALENT_TEXT_LABELS: &[&str] = &["Description Line 1...", "Description Line 2..."];

const TALENT_BREAK: &str = "<br>";

const TALENT_TEXT_NOTICE: &str =
    "Descriptions are shared by every unit; editing one changes it for every talent that references this text id";

const NOTICE_SIZE: f32 = 11.0;
const NOTICE_LINES: f32 = 2.0;
const NOTICE_HEIGHT: f32 = NOTICE_SIZE * 1.3 * NOTICE_LINES;

const ENEMY_DESCRIPTION_LABELS: &[&str] = &[
    "Description Line 1...",
    "Description Line 2...",
    "Description Line 3...",
    "Description Line 4...",
];

const INPUT_SIZE: f32 = 13.0;
const PADDING: f32 = 4.0;
const GAP: f32 = 8.0;
const BODY_PADDING: f32 = 12.0;
const SYNC_WIDTH: f32 = 172.0;

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);

fn next_token() -> u64 {
    NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
}

pub(super) const COUNT: usize = 8;

pub(super) const SUBJECTS: [Subject; COUNT] = [
    Subject::Explanation,
    Subject::EnemyName,
    Subject::EnemyDescription,
    Subject::ComboName,
    Subject::TalentText,
    Subject::MapName,
    Subject::StageName,
    Subject::ItemName,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Subject {
    Explanation,
    EnemyName,
    EnemyDescription,
    ComboName,
    TalentText,
    MapName,
    StageName,
    ItemName,
}

impl Subject {
    pub(super) fn slot(self) -> usize {
        self as usize
    }

    pub(super) fn page(self) -> Page {
        match self {
            Self::Explanation | Self::ComboName | Self::TalentText => Page::Cats,
            Self::EnemyName | Self::EnemyDescription => Page::Enemies,
            Self::MapName | Self::StageName | Self::ItemName => Page::Stages,
        }
    }

    fn labels(self) -> &'static [&'static str] {
        match self {
            Self::Explanation => EXPLANATION_LABELS,
            Self::EnemyName => ENEMY_NAME_LABELS,
            Self::EnemyDescription => ENEMY_DESCRIPTION_LABELS,
            Self::ComboName => COMBO_NAME_LABELS,
            Self::TalentText => TALENT_TEXT_LABELS,
            Self::MapName => MAP_NAME_LABELS,
            Self::StageName => STAGE_NAME_LABELS,
            Self::ItemName => ITEM_NAME_LABELS,
        }
    }

    pub(super) fn kind(self) -> popup::Kind {
        match self {
            Self::Explanation => popup::Kind::Explanation,
            Self::EnemyName => popup::Kind::EnemyName,
            Self::EnemyDescription => popup::Kind::EnemyDescription,
            Self::ComboName => popup::Kind::ComboName,
            Self::TalentText => popup::Kind::TalentText,
            Self::MapName => popup::Kind::MapName,
            Self::StageName => popup::Kind::StageName,
            Self::ItemName => popup::Kind::ItemName,
        }
    }

    fn delimited(self) -> bool {
        !matches!(self, Self::EnemyName | Self::ComboName)
    }

    // The head is only borrowed from a neighbouring line for subjects whose skipped cells
    // are an id the row may leave blank. A name row's head is its siblings' own names, so
    // borrowing one would splice another map's names into this line.
    fn borrows(self) -> bool {
        matches!(self, Self::EnemyDescription | Self::TalentText)
    }

    fn pins(self) -> bool {
        matches!(self, Self::ComboName | Self::TalentText)
    }

    fn wrapped(self) -> Option<&'static str> {
        matches!(self, Self::TalentText).then_some(TALENT_BREAK)
    }

    fn chooses(self) -> f32 {
        f32::from(u8::from(matches!(self, Self::TalentText)))
    }

    fn notice(self) -> Option<&'static str> {
        matches!(self, Self::TalentText).then_some(TALENT_TEXT_NOTICE)
    }

    fn skipped(self) -> usize {
        match self {
            Self::EnemyDescription | Self::TalentText | Self::MapName => 1,
            Self::Explanation
            | Self::EnemyName
            | Self::ComboName
            | Self::StageName
            | Self::ItemName => 0,
        }
    }

    fn width(self) -> f32 {
        match self {
            Self::EnemyName | Self::ComboName | Self::MapName | Self::StageName => NARROW_WIDTH,
            Self::Explanation | Self::EnemyDescription | Self::TalentText | Self::ItemName => POPUP_WIDTH,
        }
    }
}

fn spec(subject: Subject) -> popup::Spec {
    let fields = subject.labels().len() as f32 + subject.chooses();
    let height = FIELD_HEIGHT * fields
        + GAP * (fields - 1.0)
        + SYNC_HEIGHT
        + GAP
        + BODY_PADDING * 3.0
        + subject.notice().map_or(0.0, |_| NOTICE_HEIGHT + GAP)
        + popup::CHROME_HEIGHT;

    popup::Spec::new(subject.kind(), Size::new(subject.width(), height))
}

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Aimed(usize),
    Changed(usize, String),
    Sync,
    SyncExpired,
    Persisted(u64, PathBuf, Option<Stamp>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keyed {
    Map(u32),
    Stage(u32),
}

impl Keyed {
    // The line a name sits on is not the same in every localized file -- Map_Name_en.csv
    // holds 1,282 rows and Map_Name_ja.csv 1,290 -- so the address is resolved against the
    // body the draft actually read, never once against whichever file happened to resolve.
    fn locate(self, body: &str, delimiter: char) -> Option<usize> {
        match self {
            Self::Map(id) => names::map_name_rows(body, delimiter).get(&id).copied(),
            Self::Stage(id) => names::stage_name_rows(body, delimiter).get(&id).copied(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Plan {
    subject: Subject,
    row: usize,
    keyed: Option<Keyed>,
    rows: Vec<usize>,
    cell: Option<usize>,
    stated: Option<char>,
    label: String,
    file: String,
    game: PathBuf,
    target_mod: Option<String>,
}

impl Plan {
    pub(super) fn subject(&self) -> Subject {
        self.subject
    }

    pub(super) fn over(self, rows: Vec<usize>) -> Plan {
        Plan { rows, ..self }
    }

    pub(super) fn at(self, cell: usize) -> Plan {
        Plan { cell: Some(cell), ..self }
    }

    pub(super) fn keyed(self, keyed: Keyed) -> Plan {
        Plan { keyed: Some(keyed), ..self }
    }

    // A file the importer strips loses its language suffix on the way into a mod, and the
    // suffix is the only thing the filename rule reads, so a Japanese Map_Name.csv would be
    // split on a bar. The source keeps its suffix either way, so the delimiter comes off that.
    pub(super) fn sourced(self) -> Plan {
        let named = self.game.file_name().map(|name| name.to_string_lossy().into_owned());

        Plan { stated: named.as_deref().map(separator), ..self }
    }

    fn delimiter(&self) -> Option<char> {
        self.subject
            .delimited()
            .then(|| self.stated.unwrap_or_else(|| separator(&self.file)))
    }

    fn skip(&self) -> usize {
        self.cell.unwrap_or_else(|| self.subject.skipped())
    }

    fn matches(&self, other: &Plan) -> bool {
        self.row == other.row
            && self.keyed == other.keyed
            && self.cell == other.cell
            && self.stated == other.stated
            && self.game == other.game
            && self.target_mod == other.target_mod
            && self.label == other.label
    }

    fn source(&self, vfs: &Vfs) -> PathBuf {
        self.target_mod
            .as_deref()
            .and_then(|name| mods::find(vfs, name, Path::new(&self.file)))
            .unwrap_or_else(|| self.game.clone())
    }
}

#[derive(Default)]
pub(super) struct State {
    draft: Option<Draft>,
    frame: popup::State,
    confirm: Slot<()>,
    pinned: Option<usize>,
}

struct Draft {
    plan: Plan,
    row: usize,
    read_from: PathBuf,
    stamp: Stamp,
    delimiter: Option<char>,
    lines: Vec<String>,
    head: Vec<String>,
    fields: Vec<String>,
    tail: Vec<String>,
    failed: bool,
    dirty: bool,
    writing: bool,
    token: u64,
}

impl State {
    pub(super) fn begin(&mut self, plan: Plan, nudge: usize, vfs: &Vfs) {
        self.frame = popup::cascaded(nudge);
        self.confirm.expire();
        self.reload(plan, vfs);
    }

    fn reload(&mut self, plan: Plan, vfs: &Vfs) {
        self.draft = Draft::load(plan, vfs);
        self.pinned = self
            .draft
            .as_ref()
            .filter(|draft| draft.plan.subject.pins())
            .map(|draft| draft.plan.row);
    }

    pub(super) fn flush(&mut self, vfs: &Vfs) -> Task<Message> {
        self.draft.as_mut().map_or(Task::none(), |draft| draft.persist_if_dirty(vfs))
    }

    pub(super) fn flush_now(&mut self, vfs: &Vfs) {
        if let Some(draft) = self.draft.as_mut() {
            draft.persist_now(vfs);
        }
    }

    pub(super) fn drafting(&self) -> bool {
        self.draft.is_some()
    }

    pub(super) fn raised(&self) -> u64 {
        self.frame.raised()
    }

    pub(super) fn drifted(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| preview::stamp(&draft.read_from) != Some(draft.stamp))
    }

    pub(super) fn sync(&mut self, plans: &[Plan], vfs: &Vfs) {
        let Some(current) = self.draft.as_ref() else {
            return;
        };

        if current.dirty || current.writing {
            return;
        }

        let Some(plan) = plans.iter().find(|plan| plan.file == current.plan.file).or_else(|| plans.first()) else {
            self.draft = None;

            return;
        };

        if current.plan.target_mod.is_none() != plan.target_mod.is_none() {
            self.draft = None;

            return;
        }

        let held = self.pinned.filter(|row| plan.rows.is_empty() || plan.rows.contains(row));

        let plan = match held {
            Some(row) => Plan { row, ..plan.clone() },
            None => plan.clone(),
        };

        if !current.plan.matches(&plan) || preview::stamp(&current.read_from) != Some(current.stamp) {
            self.reload(plan, vfs);
        }
    }

    pub(super) fn update(&mut self, message: Message, vfs: &Vfs) -> Task<Message> {
        match message {
            Message::Popup(msg) => {
                let Some(subject) = self.draft.as_ref().map(|draft| draft.plan.subject) else {
                    return Task::none();
                };

                if self.frame.update(msg, spec(subject)) {
                    let task = self.draft.as_mut().map_or(Task::none(), |draft| draft.persist_if_dirty(vfs));

                    self.draft = None;
                    self.confirm.expire();

                    return task;
                }
            }
            Message::Aimed(row) => {
                let Some(draft) = self.draft.as_mut().filter(|draft| draft.plan.row != row) else {
                    return Task::none();
                };

                draft.persist_now(vfs);

                let plan = Plan { row, ..draft.plan.clone() };
                self.reload(plan, vfs);
            }
            Message::Changed(index, value) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.edit(index, value);
                }
            }
            Message::SyncExpired => self.confirm.expire(),
            Message::Sync => {
                if !self.confirm.take(&()) {
                    return self.confirm.set((), Message::SyncExpired);
                }

                if let Some(draft) = self.draft.as_mut() {
                    draft.restore();
                }
            }
            Message::Persisted(token, path, stamp) => {
                if let Some(draft) = self.draft.as_mut()
                    && draft.token == token
                {
                    draft.writing = false;

                    match stamp {
                        Some(stamp) => {
                            draft.read_from = path;
                            draft.stamp = stamp;
                            draft.failed = false;
                        }
                        None => draft.failed = true,
                    }
                }
            }
        }

        self.draft.as_mut().map_or(Task::none(), |draft| draft.persist_if_dirty(vfs))
    }

    pub(super) fn view(&self, window: Size) -> Option<Element<'_, Message>> {
        let draft = self.draft.as_ref()?;
        let armed = self.confirm.is_set();

        Some(self.frame.view(
            &draft.plan.label,
            spec(draft.plan.subject),
            window,
            Message::Popup,
            move || draft.body(armed),
            None,
        ))
    }
}

fn write_now(path: &Path, body: &[u8], stamp: Stamp) -> Option<Stamp> {
    preview::save(path, body, stamp)
        .inspect_err(|err| warn!(path = %path.display(), "Prose editor could not write the file: {}", err))
        .ok()
}

impl Draft {
    fn load(plan: Plan, vfs: &Vfs) -> Option<Draft> {
        let read_from = plan.source(vfs);

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Prose editor could not read the file: {}", err))
            .ok()?;

        let stamp = preview::stamp(&read_from)?;
        let body = common::scrub(&bytes);
        let delimiter = plan.delimiter();
        let lines: Vec<String> = body.lines().map(str::to_owned).collect();
        let row = seat(&plan, &body, delimiter)?;
        let (head, fields, tail) = parse(&body, row, plan.skip(), delimiter, plan.subject);

        Some(Draft {
            plan,
            row,
            read_from,
            stamp,
            delimiter,
            lines,
            head,
            fields,
            tail,
            failed: false,
            dirty: false,
            writing: false,
            token: next_token(),
        })
    }

    fn edit(&mut self, index: usize, value: String) {
        let Some(slot) = self.fields.get_mut(index) else {
            return;
        };

        if *slot == value {
            return;
        }

        *slot = value;
        self.stage();
    }

    fn restore(&mut self) {
        let Ok(bytes) = fs::read(&self.plan.game) else {
            warn!(path = %self.plan.game.display(), "Prose editor could not read the vanilla file");
            self.failed = true;

            return;
        };

        let body = common::scrub(&bytes);
        let delimiter = self.plan.delimiter();
        let Some(row) = seat(&self.plan, &body, delimiter) else {
            warn!(path = %self.plan.game.display(), "Prose editor could not find the row in the vanilla file");
            self.failed = true;

            return;
        };

        let (head, fields, tail) = parse(&body, row, self.plan.skip(), delimiter, self.plan.subject);
        self.head = head;
        self.fields = fields;
        self.tail = tail;
        self.stage();
    }

    fn destination(&self, vfs: &Vfs) -> Option<(PathBuf, Stamp)> {
        let Some(name) = self.plan.target_mod.as_deref() else {
            return Some((self.plan.game.clone(), self.stamp));
        };

        if self.read_from != self.plan.game {
            return Some((self.read_from.clone(), self.stamp));
        }

        let path = mods::ensure_as(vfs, name, &self.plan.game, &self.plan.file)
            .inspect_err(|err| warn!(source = %self.plan.game.display(), "Prose editor could not stage the file: {}", err))
            .ok()?;

        preview::stamp(&path).map(|stamp| (path, stamp))
    }

    fn stage(&mut self) {
        while self.lines.len() <= self.row {
            self.lines.push(String::new());
        }

        let joined =
            join(&self.head, &self.fields, &self.tail, self.delimiter, self.plan.subject.wrapped());
        let Some(slot) = self.lines.get_mut(self.row) else {
            self.failed = true;

            return;
        };

        *slot = joined;
        self.dirty = true;
    }

    fn persist_if_dirty(&mut self, vfs: &Vfs) -> Task<Message> {
        if !self.dirty || self.writing {
            return Task::none();
        }

        self.persist(vfs)
    }

    fn prepare_write(&mut self, vfs: &Vfs) -> Option<(PathBuf, Vec<u8>, Stamp, u64)> {
        let Some((path, stamp)) = self.destination(vfs) else {
            self.failed = true;

            return None;
        };

        let mut body = self.lines.join("\n");
        body.push('\n');

        self.dirty = false;
        self.writing = true;

        Some((path, body.into_bytes(), stamp, self.token))
    }

    fn persist(&mut self, vfs: &Vfs) -> Task<Message> {
        let Some((path, body, stamp, token)) = self.prepare_write(vfs) else {
            return Task::none();
        };

        let reported = path.clone();

        Task::perform(
            smol::unblock(move || write_now(&path, &body, stamp)),
            move |stamp| Message::Persisted(token, reported.clone(), stamp),
        )
    }

    fn persist_now(&mut self, vfs: &Vfs) {
        if !self.dirty && !self.writing {
            return;
        }

        let Some((path, body, stamp, _)) = self.prepare_write(vfs) else {
            return;
        };

        match write_now(&path, &body, stamp) {
            Some(stamp) => {
                self.read_from = path;
                self.stamp = stamp;
                self.failed = false;
            }
            None => self.failed = true,
        }

        self.writing = false;
    }

    fn picker(&self) -> Option<Element<'_, Message>> {
        if self.plan.rows.len() < 2 {
            return None;
        }

        let held: Vec<Aim> = self.plan.rows.iter().map(|row| self.aim(*row)).collect();
        let current = held.iter().find(|aim| aim.row == self.row).cloned()?;

        Some(
            pick_list(held, Some(current), |aim: Aim| Message::Aimed(aim.row))
                .width(Length::Fill)
                .padding(PADDING)
                .text_size(INPUT_SIZE)
                .style(theme::combo_box)
                .menu_style(theme::combo_box_menu)
                .into(),
        )
    }

    fn aim(&self, row: usize) -> Aim {
        let spoken = self
            .lines
            .get(row)
            .and_then(|line| line.split(self.delimiter.unwrap_or(PIPE)).nth(self.plan.subject.skipped()))
            .map(|text| text.replace(TALENT_BREAK, " "))
            .filter(|text| !text.trim().is_empty());

        Aim { row, label: spoken.unwrap_or_else(|| format!("Text {row}")) }
    }

    fn body(&self, armed: bool) -> Element<'_, Message> {
        let mut rows = column![].spacing(GAP);

        if let Some(notice) = self.plan.subject.notice() {
            rows = rows.push(
                text(notice)
                    .size(NOTICE_SIZE)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill)
                    .style(text::secondary),
            );
        }

        if let Some(picker) = self.picker() {
            rows = rows.push(picker);
        }

        for (index, label) in self.plan.subject.labels().iter().enumerate() {
            let field = text_input(label, self.fields.get(index).map_or("", String::as_str))
                .on_input(move |value| Message::Changed(index, value))
                .size(INPUT_SIZE)
                .padding(PADDING)
                .width(Length::Fill)
                .style(theme::rounded_input);

            rows = rows.push(field);
        }

        let label = if armed { CONFIRM_LABEL } else { "Sync With \"game\"" };

        let sync = button(theme::centered_text(label).size(INPUT_SIZE).wrapping(iced::widget::text::Wrapping::None))
            .width(Length::Fixed(SYNC_WIDTH))
            .padding([PADDING + 1.0, 10.0])
            .style(theme::danger_button)
            .on_press(Message::Sync);

        column![
            smooth_scroll(
                scrollable(container(rows).padding(BODY_PADDING).width(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
            ),
            container(sync)
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(Padding::ZERO.top(GAP).left(BODY_PADDING).right(BODY_PADDING).bottom(BODY_PADDING)),
        ]
        .height(Length::Fill)
        .into()
    }
}

const PIPE: char = '|';

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Aim {
    row: usize,
    label: String,
}

impl std::fmt::Display for Aim {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}  {}", self.row, fitted(&self.label))
    }
}

fn fitted(label: &str) -> String {
    const ROOM: usize = 48;

    if label.chars().count() <= ROOM {
        return label.to_owned();
    }

    label.chars().take(ROOM).chain("...".chars()).collect()
}

const JAPANESE: &str = "ja";

pub(super) fn separator(name: &str) -> char {
    localized(name).char()
}

fn localized(name: &str) -> Separator {
    let japanese = name.rsplit_once('_').is_some_and(|(_, tail)| tail.starts_with(JAPANESE));

    if japanese { Separator::Comma } else { Separator::Pipe }
}

// A localized file need not carry every row: Map_Name_ja.csv holds maps 1097-1101 and
// 34034 that no other language ships, and two StageName files are the same shape. The menu
// therefore asks whether the address lands before it offers an Edit, so a variant that has
// no line for this map shows its file actions without a dead Edit above them.
pub(super) fn seated(plan: &Plan) -> bool {
    let Some(keyed) = plan.keyed else {
        return true;
    };

    let Some(delimiter) = plan.delimiter() else {
        return true;
    };

    fs::read(&plan.game)
        .ok()
        .is_some_and(|bytes| keyed.locate(&common::scrub(&bytes), delimiter).is_some())
}

fn seat(plan: &Plan, body: &str, delimiter: Option<char>) -> Option<usize> {
    let Some(keyed) = plan.keyed else {
        return Some(plan.row);
    };

    keyed.locate(body, delimiter.unwrap_or(Separator::Comma.char()))
}

fn parse(
    body: &str,
    index: usize,
    skip: usize,
    delimiter: Option<char>,
    subject: Subject,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let source = body.lines().nth(index).unwrap_or_default();
    let (head, fields, tail) = row(source, delimiter, skip, subject.labels().len(), subject.wrapped());

    if !subject.borrows() || filled(&head) {
        return (head, fields, tail);
    }

    let borrowed = body
        .lines()
        .map(|line| row(line, delimiter, skip, 0, None).0)
        .find(|candidate| filled(candidate));

    (borrowed.unwrap_or(head), fields, tail)
}

fn filled(head: &[String]) -> bool {
    head.iter().any(|field| !field.trim().is_empty())
}

fn row(
    line: &str,
    delimiter: Option<char>,
    skip: usize,
    count: usize,
    wrapped: Option<&str>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let Some(delimiter) = delimiter else {
        return (Vec::new(), vec![line.trim().to_owned()], Vec::new());
    };

    let mut parts = line.split(delimiter);
    let head = (0..skip).map(|_| parts.next().unwrap_or_default().to_owned()).collect();

    let Some(marker) = wrapped else {
        let fields = (0..count).map(|_| parts.next().unwrap_or_default().trim().to_owned()).collect();

        return (head, fields, parts.map(str::to_owned).collect());
    };

    let held: Vec<&str> = parts.collect();
    let whole = held.join(&delimiter.to_string());
    let mut broken = whole.split(marker);
    let fields = (0..count).map(|_| broken.next().unwrap_or_default().trim().to_owned()).collect();

    (head, fields, Vec::new())
}

fn join(
    head: &[String],
    fields: &[String],
    tail: &[String],
    delimiter: Option<char>,
    wrapped: Option<&str>,
) -> String {
    let Some(delimiter) = delimiter else {
        return fields.first().cloned().unwrap_or_default();
    };

    let Some(marker) = wrapped else {
        let parts: Vec<&str> = head.iter().chain(fields).chain(tail).map(String::as_str).collect();

        return parts.join(&delimiter.to_string());
    };

    let mut written: Vec<&str> = fields.iter().map(String::as_str).collect();

    while written.last().is_some_and(|last| last.is_empty()) {
        written.pop();
    }

    let body = written.join(marker);
    let parts: Vec<&str> = head.iter().map(String::as_str).chain(std::iter::once(body.as_str())).collect();

    parts.join(&delimiter.to_string())
}

pub(super) fn plan(
    subject: Subject,
    row: usize,
    label: String,
    file: String,
    game: &Path,
    target_mod: Option<String>,
) -> Plan {
    Plan {
        subject,
        row,
        keyed: None,
        rows: Vec::new(),
        cell: None,
        stated: None,
        label,
        file,
        game: game.to_path_buf(),
        target_mod,
    }
}

#[cfg(test)]
mod tests {
    use super::{join, row, Subject, TALENT_BREAK};

    const PIPE: Option<char> = Some('|');

    // textID|text, with at most one <br> in the text across every shipped locale.
    // The id is skipped so it cannot be typed over, and the break becomes the second field.
    #[test]
    fn a_description_splits_its_id_off_and_its_break_into_a_second_line() {
        let line = "12|Gain the \"Weaken\" ability.<br>Level up to Weaken enemies for even longer!";
        let (head, fields, tail) = row(line, PIPE, 1, 2, Some(TALENT_BREAK));

        assert_eq!(head, ["12"]);
        assert_eq!(fields[0], "Gain the \"Weaken\" ability.");
        assert_eq!(fields[1], "Level up to Weaken enemies for even longer!");
        assert!(tail.is_empty());

        assert_eq!(join(&head, &fields, &tail, PIPE, Some(TALENT_BREAK)), line);
    }

    #[test]
    fn a_description_with_one_line_does_not_grow_a_break() {
        let line = "4|Gain the \"Only Attacks\" ability.";
        let (head, fields, tail) = row(line, PIPE, 1, 2, Some(TALENT_BREAK));

        assert_eq!(fields[1], "", "the second line is empty, not the whole text again");
        assert_eq!(join(&head, &fields, &tail, PIPE, Some(TALENT_BREAK)), line, "and no <br> is added");
    }

    // The file is roster-wide: one text id is referenced by talents on unrelated units,
    // so the popup has to say so the way the cost curves do.
    #[test]
    fn only_the_roster_wide_subject_carries_a_notice() {
        assert!(Subject::TalentText.notice().is_some());

        for subject in [Subject::Explanation, Subject::EnemyName, Subject::EnemyDescription, Subject::ComboName] {
            assert_eq!(subject.notice(), None, "{subject:?} edits a row that belongs to what it names");
        }
    }

    #[test]
    fn only_the_talent_text_is_broken_on_a_marker() {
        assert_eq!(Subject::TalentText.wrapped(), Some(TALENT_BREAK));

        for subject in [Subject::Explanation, Subject::EnemyName, Subject::EnemyDescription, Subject::ComboName] {
            assert_eq!(subject.wrapped(), None, "{subject:?} has no inner break");
        }
    }
}
