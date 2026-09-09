use std::fs;
use std::path::{Path, PathBuf};

use nyanko::chapter::stage::{Battleground, BattlegroundEntry};
use nyanko::common::Column;
use nyanko::combat::{Scale, Separator};
use nyanko::common::FromColumn;
use nyanko::common;
use tracing::warn;

use kore::common::preview::{self, Stamp};
use kore::domains::mods;
use kore::domains::settings::EditorMode;
use kore::Vfs;

use super::figures::resolved::Rule;

const COMMENT: &str = "//";
const TERMINATOR: u32 = 0;
const HEADER_MAX: usize = 7;
const HEADER_PROBE: usize = 6;

pub(super) const ENEMY_ID: usize = 0;
pub(super) const AMOUNT: usize = 1;
pub(super) const MAGNIFICATION: usize = 9;
pub(super) const ATK_MAGNIFICATION: usize = 11;

const WIDTH: usize = 14;
const ENEMY_OFFSET: i32 = -2;

const LABELS: [&str; WIDTH] = [
    "Enemy",
    "Count",
    "Spawn",
    "Resp Min",
    "Resp Max",
    "Base",
    "Layer Min",
    "Layer Max",
    "Boss",
    "Mag",
    "Score",
    "Atk Mag",
    "Time",
    "Kills",
];

const HEAD_LABELS: [&str; 2] = ["Base ID", "No Continues"];

const SETUP_LABELS: [&str; 10] = [
    "Width",
    "Base HP",
    "Spawn Min",
    "Spawn Max",
    "Background",
    "Max Enemies",
    "Anim Base ID",
    "Time Limit",
    "Indestructible",
    "Unknown",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Head,
    Setup,
    Spawn,
}

impl Kind {
    fn labels(self) -> &'static [&'static str] {
        match self {
            Kind::Head => &HEAD_LABELS,
            Kind::Setup => &SETUP_LABELS,
            Kind::Spawn => &LABELS,
        }
    }

    fn width(self) -> usize {
        self.labels().len()
    }
}

pub(super) fn label(kind: Kind, index: usize) -> String {
    kind.labels()
        .get(index)
        .map_or_else(|| format!("Column {}", index + 1), |held| (*held).to_owned())
}

fn found<T>(table: &'static [Column<T>], index: usize) -> Option<&'static Column<T>> {
    table.iter().find(|column| column.index == index)
}

pub(super) fn fallback(kind: Kind, index: usize) -> i32 {
    let declared = match kind {
        Kind::Head => found(Battleground::HEADER_COLUMNS, index).map(|held| held.default),
        Kind::Setup => found(Battleground::CONFIG_COLUMNS, index).map(|held| held.default),
        Kind::Spawn => found(BattlegroundEntry::COLUMNS, index).map(|held| held.default),
    };

    declared.and_then(i32::from_column).unwrap_or_default()
}

fn head_rule(index: usize) -> Rule {
    match index {
        0 => Rule::Floor(-1),
        1 => Rule::Flag,
        _ => Rule::Plain,
    }
}

fn setup_rule(index: usize) -> Rule {
    match index {
        6 => Rule::Offset(ENEMY_OFFSET),
        0..=7 => Rule::Floor(0),
        8 => Rule::Flag,
        _ => Rule::Plain,
    }
}

fn seed(index: usize, shown: i32) -> i32 {
    let rule = rule(Kind::Spawn, index);

    to_raw(Kind::Spawn, index, rule.to_raw(shown, EditorMode::Resolved), EditorMode::Resolved)
}

pub(super) fn hint(kind: Kind, index: usize, values: EditorMode) -> String {
    let held = fallback(kind, index);
    let shown = rule(kind, index).to_display(to_display(kind, index, held, values), values);

    if shown < 0 {
        return String::new();
    }

    shown.to_string()
}

fn scale(kind: Kind, index: usize) -> Scale {
    match kind {
        Kind::Head => found(Battleground::HEADER_COLUMNS, index).map_or(Scale::Raw, |held| held.scale),
        Kind::Setup => found(Battleground::CONFIG_COLUMNS, index).map_or(Scale::Raw, |held| held.scale),
        Kind::Spawn => found(BattlegroundEntry::COLUMNS, index).map_or(Scale::Raw, |held| held.scale),
    }
}

pub(super) fn to_display(kind: Kind, index: usize, raw: i32, values: EditorMode) -> i32 {
    if values == EditorMode::Raw {
        return raw;
    }

    scale(kind, index).apply(raw)
}

pub(super) fn to_raw(kind: Kind, index: usize, display: i32, values: EditorMode) -> i32 {
    if values == EditorMode::Raw {
        return display;
    }

    match scale(kind, index) {
        Scale::Double => display / 2,
        Scale::Quarter => display * 4,
        _ => display,
    }
}

pub(super) fn rule(kind: Kind, index: usize) -> Rule {
    match kind {
        Kind::Head => return head_rule(index),
        Kind::Setup => return setup_rule(index),
        Kind::Spawn => {}
    }

    match index {
        ENEMY_ID => Rule::Offset(ENEMY_OFFSET),
        AMOUNT | 5 | 6 | 7 | 8 | 12 => Rule::Plain,
        2 | 3 | 4 | MAGNIFICATION | 10 | ATK_MAGNIFICATION | 13 => Rule::Floor(0),
        _ => Rule::Opaque,
    }
}

pub(super) struct Row {
    pub(super) cells: Vec<i32>,
    written: Vec<String>,
    stored: usize,
    suffix: String,
}

impl Row {
    pub(super) fn rebuild(&self, touched: usize, delimiter: char) -> String {
        let width = self.stored.max(touched);
        let mut line = self.written[..width.min(self.written.len())].join(&delimiter.to_string());

        line.push_str(&self.suffix);

        line
    }
}

pub(super) fn split(line: &str, delimiter: char) -> Row {
    let (head, suffix) = line.split_once(COMMENT).map_or((line, ""), |(before, _)| {
        (before, &line[before.len()..])
    });

    let written: Vec<String> = head.split(delimiter).map(str::to_owned).collect();
    let cells = written.iter().map(|cell| cell.trim().parse::<i32>().unwrap_or_default()).collect();

    Row { cells, stored: written.len(), written, suffix: suffix.to_owned() }
}

pub(super) struct Sheet {
    pub(super) head: Option<usize>,
    pub(super) config: usize,
    pub(super) spawns: Vec<usize>,
}

fn cells(line: &str, delimiter: char) -> Vec<&str> {
    line.split(delimiter).collect()
}

fn body(line: &str) -> &str {
    line.split_once(COMMENT).map_or(line, |(before, _)| before).trim()
}

fn leading(line: &str, delimiter: char) -> u32 {
    cells(body(line), delimiter).first().and_then(|cell| cell.trim().parse().ok()).unwrap_or(TERMINATOR)
}

fn heads(parts: &[&str]) -> bool {
    parts.len() <= HEADER_MAX || parts.get(HEADER_PROBE).is_some_and(|cell| cell.trim().is_empty())
}

pub(super) fn scan(lines: &[String], delimiter: char) -> Option<Sheet> {
    let mut live = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !body(line).is_empty())
        .map(|(index, line)| (index, body(line)));

    let (first, opening) = live.next()?;

    let (head, config) = match heads(&cells(opening, delimiter)) {
        true => (Some(first), live.next()?.0),
        false => (None, first),
    };

    let spawns = live
        .map(|(index, _)| index)
        .take_while(|index| {
            lines.get(*index).is_some_and(|line| leading(line, delimiter) != TERMINATOR)
        })
        .collect();

    Some(Sheet { head, config, spawns })
}

#[derive(Debug, Clone)]
pub enum Message {
    Popup(crate::widget::popup::Message),
    Changed(usize, usize, String),
    Picked(usize, usize, i32),
    Focused(Option<usize>),
    Added,
    Dropped(usize),
    Grabbed(usize),
    Hauled(iced::Point),
    Released,
    Hunted(String),
    Confirmed(u32),
    Picking(Option<usize>),
    Pick(crate::widget::popup::Message),
    Scrolled(f32),
    Sync,
    SyncExpired,
    Persisted(u64, PathBuf, Option<Stamp>),
}

#[derive(Clone)]
pub(crate) struct Plan {
    label: String,
    game: PathBuf,
    target_mod: Option<String>,
    values: EditorMode,
}

impl Plan {
    fn matches(&self, other: &Plan) -> bool {
        self.game == other.game && self.target_mod == other.target_mod && self.values == other.values
    }

    fn source(&self, vfs: &Vfs) -> PathBuf {
        self.target_mod
            .as_deref()
            .and_then(|name| mods::find(vfs, name, &self.game))
            .unwrap_or_else(|| self.game.clone())
    }
}

pub(super) struct Draft {
    plan: Plan,
    read_from: PathBuf,
    stamp: Stamp,
    delimiter: char,
    lines: Vec<String>,
    seats: Vec<usize>,
    kinds: Vec<Kind>,
    chrome: usize,
    rows: Vec<Row>,
    inputs: Vec<Vec<String>>,
    touched: Vec<usize>,
    buffer: Option<(usize, usize)>,
    failed: bool,
    dirty: bool,
    writing: bool,
    token: u64,
}

impl Draft {
    fn load(plan: Plan, vfs: &Vfs) -> Option<Draft> {
        let read_from = plan.source(vfs);

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Stage editor could not read the file: {}", err))
            .ok()?;

        let stamp = preview::stamp(&read_from)?;
        let body = common::scrub(&bytes);
        let delimiter = Separator::detect(&body).unwrap_or(Separator::Comma).char();
        let lines: Vec<String> = body.lines().map(str::to_owned).collect();

        let sheet = scan(&lines, delimiter)?;
        let (seats, kinds, chrome) = seating(&sheet);

        let rows: Vec<Row> = seats
            .iter()
            .filter_map(|index| lines.get(*index).map(|line| split(line, delimiter)))
            .collect();

        let inputs = rows
            .iter()
            .zip(&kinds)
            .map(|(row, kind)| shown_row(*kind, row, plan.values))
            .collect();
        let touched = vec![0; rows.len()];

        Some(Draft {
            plan,
            read_from,
            stamp,
            delimiter,
            lines,
            seats,
            kinds,
            chrome,
            rows,
            inputs,
            touched,
            buffer: None,
            failed: false,
            dirty: false,
            writing: false,
            token: next_token(),
        })
    }

    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }

    pub(super) fn values(&self) -> EditorMode {
        self.plan.values
    }

    pub(super) fn width(&self, row: usize) -> usize {
        self.rows.get(row).map_or(0, |held| held.cells.len())
    }

    pub(super) fn kind(&self, row: usize) -> Kind {
        self.kinds.get(row).copied().unwrap_or(Kind::Spawn)
    }

    pub(super) fn chrome(&self) -> usize {
        self.chrome
    }

    pub(super) fn reads(&self, row: usize, column: usize) -> i32 {
        self.rows.get(row).and_then(|held| held.cells.get(column)).copied().unwrap_or_default()
    }

    pub(super) fn input(&self, row: usize, column: usize) -> &str {
        self.inputs.get(row).and_then(|held| held.get(column)).map_or("", String::as_str)
    }

    fn edit(&mut self, row: usize, column: usize, typed: &str) {
        let values = self.plan.values;

        if typed.starts_with(BUFFER_MARK) {
            if !typable(typed, true) {
                return;
            }

            self.hold(row, column, typed);
            self.buffer = Some((row, column));

            return;
        }

        if self.buffer == Some((row, column)) {
            self.buffer = None;
        }

        if column == MAGNIFICATION {
            self.spread(row, typed);

            return;
        }

        let kind = self.kind(row);
        let rule = rule(kind, column);

        if !typable(typed, rule.signed(values)) {
            return;
        }

        let raw = match typed.is_empty() {
            true => fallback(kind, column),
            false => match typed.parse::<i32>() {
                Ok(display) => {
                    to_raw(kind, column, rule.to_raw(rule.clamp(display, values), values), values)
                }
                Err(_) => {
                    self.hold(row, column, typed);

                    return;
                }
            },
        };

        self.write(row, column, raw);
        self.stage(row);
    }

    fn spread(&mut self, row: usize, typed: &str) {
        let values = self.plan.values;
        let (health, attack) = magnifications(typed);

        for (column, part) in [(MAGNIFICATION, health), (ATK_MAGNIFICATION, attack)] {
            let Some(part) = part else { continue };

            if !typable(part, false) {
                return;
            }

            let raw = part.parse::<i32>().map_or_else(
                |_| fallback(Kind::Spawn, column),
                |shown| to_raw(Kind::Spawn, column, shown.max(0), values),
            );

            self.write(row, column, raw);
        }

        if let Some(held) = self.inputs.get_mut(row).and_then(|held| held.get_mut(MAGNIFICATION)) {
            *held = typed.to_owned();
        }

        self.stage(row);
    }

    pub(super) fn buffering(&self, row: usize, column: usize) -> bool {
        self.buffer == Some((row, column))
    }

    fn resolve_buffer(&mut self) {
        let Some((row, column)) = self.buffer.take() else {
            return;
        };

        let Some(typed) = self
            .inputs
            .get(row)
            .and_then(|held| held.get(column))
            .and_then(|held| held.strip_prefix(BUFFER_MARK))
        else {
            return;
        };

        let typed = typed.to_owned();

        self.edit(row, column, &typed);
    }

    fn hold(&mut self, row: usize, column: usize, typed: &str) {
        if let Some(held) = self.inputs.get_mut(row).and_then(|held| held.get_mut(column)) {
            *held = typed.to_owned();
        }
    }

    fn write(&mut self, row: usize, column: usize, raw: i32) {
        let values = self.plan.values;

        let Some(held) = self.rows.get_mut(row) else {
            return;
        };

        while held.cells.len() <= column {
            held.cells.push(0);
            held.written.push("0".to_owned());
        }

        held.cells[column] = raw;
        held.written[column] = raw.to_string();

        if let Some(mark) = self.touched.get_mut(row) {
            *mark = (*mark).max(column + 1);
        }

        let kind = self.kind(row);
        let shown = shown_cell(kind, self.rows[row].cells[column], column, values);

        if let Some(field) = self.inputs.get_mut(row).and_then(|held| held.get_mut(column)) {
            *field = shown;
        }
    }

    fn set(&mut self, row: usize, column: usize, raw: i32) {
        if self.reads(row, column) == raw {
            return;
        }

        self.write(row, column, raw);
        self.stage(row);
    }

    fn stage(&mut self, row: usize) {
        let (Some(held), Some(line), Some(touched)) =
            (self.rows.get(row), self.seats.get(row).copied(), self.touched.get(row).copied())
        else {
            return;
        };

        let rebuilt = held.rebuild(touched, self.delimiter);

        let Some(slot) = self.lines.get_mut(line) else {
            self.failed = true;

            return;
        };

        *slot = rebuilt;
        self.dirty = true;
    }

    fn restock(&mut self) {
        let Some(sheet) = scan(&self.lines, self.delimiter) else {
            self.failed = true;

            return;
        };

        let (seats, kinds, chrome) = seating(&sheet);

        self.seats = seats;
        self.kinds = kinds;
        self.chrome = chrome;
        self.rows = self
            .seats
            .iter()
            .filter_map(|index| self.lines.get(*index).map(|line| split(line, self.delimiter)))
            .collect();
        self.touched = vec![0; self.rows.len()];
        self.refresh();
        self.dirty = true;
    }

    fn crowded(&self) -> bool {
        if self.plan.values == EditorMode::Raw {
            return false;
        }

        (self.chrome..self.rows.len())
            .map(|row| self.reads(row, ENEMY_ID))
            .collect::<std::collections::BTreeSet<i32>>()
            .len()
            >= ENEMY_CEILING
    }

    fn shift(&mut self, row: usize, onto: usize) {
        if row == onto || row < self.chrome || onto < self.chrome {
            return;
        }

        let (Some(from), Some(to)) = (self.seats.get(row).copied(), self.seats.get(onto).copied())
        else {
            return;
        };

        if from >= self.lines.len() || to >= self.lines.len() {
            return;
        }

        self.lines.swap(from, to);
        self.restock();
    }

    fn add(&mut self) {
        if self.crowded() {
            return;
        }

        let width = self
            .rows
            .iter()
            .skip(self.chrome)
            .map(|held| held.stored)
            .max()
            .unwrap_or(COMMON_WIDTH)
            .max(COMMON_WIDTH);

        let seeded = |column: usize| {
            SEEDS
                .iter()
                .find(|(held, _)| *held == column)
                .map_or_else(|| fallback(Kind::Spawn, column), |(_, shown)| seed(column, *shown))
        };

        let fields: Vec<String> =
            (0..width).map(|column| seeded(column).to_string()).collect();

        let at = self.seats.last().copied().unwrap_or_default() + 1;
        self.lines.insert(at.min(self.lines.len()), fields.join(&self.delimiter.to_string()));

        self.restock();
    }

    fn drop(&mut self, row: usize) {
        if row < self.chrome {
            return;
        }

        let Some(line) = self.seats.get(row).copied() else {
            return;
        };

        if line >= self.lines.len() {
            return;
        }

        self.lines.remove(line);
        self.restock();
    }

    fn refresh(&mut self) {
        self.inputs = self
            .rows
            .iter()
            .zip(&self.kinds)
            .map(|(row, kind)| shown_row(*kind, row, self.plan.values))
            .collect();
    }

    fn sync(&mut self) {
        let Ok(bytes) = fs::read(&self.plan.game) else {
            self.failed = true;

            return;
        };

        let body = common::scrub(&bytes);
        let delimiter = Separator::detect(&body).unwrap_or(Separator::Comma).char();
        let vanilla: Vec<String> = body.lines().map(str::to_owned).collect();

        let Some(sheet) = scan(&vanilla, delimiter) else {
            self.failed = true;

            return;
        };

        self.lines = vanilla;
        self.delimiter = delimiter;

        let (seats, kinds, chrome) = seating(&sheet);
        self.seats = seats;
        self.kinds = kinds;
        self.chrome = chrome;
        self.rows = self
            .seats
            .iter()
            .filter_map(|index| self.lines.get(*index).map(|line| split(line, delimiter)))
            .collect();
        self.touched = vec![0; self.rows.len()];
        self.refresh();
        self.dirty = true;
    }

    fn destination(&self, vfs: &Vfs) -> Option<(PathBuf, Stamp)> {
        let Some(name) = self.plan.target_mod.as_deref() else {
            return Some((self.plan.game.clone(), self.stamp));
        };

        if self.read_from != self.plan.game {
            return Some((self.read_from.clone(), self.stamp));
        }

        let path = mods::ensure(vfs, name, &self.plan.game)
            .inspect_err(|err| warn!(source = %self.plan.game.display(), "Stage editor could not stage the file: {}", err))
            .ok()?;

        let stamp = preview::stamp(&path)?;

        Some((path, stamp))
    }

    fn prepare(&mut self, vfs: &Vfs) -> Option<(PathBuf, Vec<u8>, Stamp, u64)> {
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

    fn persist_now(&mut self, vfs: &Vfs) {
        if !self.dirty && !self.writing {
            return;
        }

        let Some((path, body, stamp, _)) = self.prepare(vfs) else {
            return;
        };

        match preview::save(&path, &body, stamp) {
            Ok(fresh) => {
                self.read_from = path;
                self.stamp = fresh;
                self.failed = false;
            }
            Err(err) => {
                warn!(path = %path.display(), "Stage editor could not write the file: {}", err);
                self.failed = true;
            }
        }

        self.writing = false;
    }
}

fn seating(sheet: &Sheet) -> (Vec<usize>, Vec<Kind>, usize) {
    let mut seats = Vec::new();
    let mut kinds = Vec::new();

    if let Some(head) = sheet.head {
        seats.push(head);
        kinds.push(Kind::Head);
    }

    seats.push(sheet.config);
    kinds.push(Kind::Setup);

    let chrome = seats.len();

    seats.extend(sheet.spawns.iter().copied());
    kinds.extend(std::iter::repeat_n(Kind::Spawn, sheet.spawns.len()));

    (seats, kinds, chrome)
}

fn magnifications(typed: &str) -> (Option<&str>, Option<&str>) {
    let mut parts = typed.split(['/', '\\', '|']).map(str::trim);
    let health = parts.next();
    let attack = parts.next();

    (health, attack.or(health))
}

fn shown_row(kind: Kind, row: &Row, values: EditorMode) -> Vec<String> {
    (0..kind.width().max(row.cells.len()))
        .map(|column| shown_cell(kind, row.cells.get(column).copied().unwrap_or_default(), column, values))
        .collect()
}

fn shown_cell(kind: Kind, raw: i32, column: usize, values: EditorMode) -> String {
    if raw == fallback(kind, column) {
        return String::new();
    }

    rule(kind, column).to_display(to_display(kind, column, raw, values), values).to_string()
}

fn typable(value: &str, signed: bool) -> bool {
    typable_digits(value.strip_prefix(BUFFER_MARK).unwrap_or(value), signed)
}

fn typable_digits(value: &str, signed: bool) -> bool {
    let mut chars = value.chars();

    match chars.next() {
        None => true,
        Some('-') => signed && chars.all(|digit| digit.is_ascii_digit()),
        Some(first) if first.is_ascii_digit() => chars.all(|digit| digit.is_ascii_digit()),
        Some(_) => false,
    }
}

static NEXT_TOKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_token() -> u64 {
    NEXT_TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

const TABLE_WIDTH: f32 = 720.0;
const POPUP_SIZE: iced::Size =
    iced::Size::new(TABLE_WIDTH + BODY_PADDING * 2.0 + FRAME_ALLOWANCE, 460.0);
const FRAME_ALLOWANCE: f32 = 22.0;
const GROUND_POPUP: crate::widget::popup::Spec =
    crate::widget::popup::Spec::new(crate::widget::popup::Kind::Battleground, POPUP_SIZE);

const ICON_SIZE: f32 = 32.0;
const CELL_SIZE: f32 = 12.0;
const CELL_INSET: f32 = 2.0;
const ROW_GAP: f32 = 2.0;
const COLUMN_GAP: f32 = 6.0;
const CELL_PADDING: [f32; 2] = [4.0, 6.0];
const SYNC_WIDTH: f32 = 172.0;
const BODY_PADDING: f32 = 10.0;
const RANGE_GAP: f32 = 2.0;

const ENEMY_SPAN: u16 = 4;
const COUNT_SPAN: u16 = 5;
const MAG_SPAN: u16 = 9;
const BASE_SPAN: u16 = 6;
const SPAWN_SPAN: u16 = 6;
const RESPAWN_SPAN: u16 = 9;
const LAYER_SPAN: u16 = 6;
const BOSS_SPAN: u16 = 8;
const SCORE_SPAN: u16 = 5;
const KILLS_SPAN: u16 = 5;

const RANGE_MARK: &str = "~";
const BUFFER_MARK: char = '!';
const SETTING_BAND: usize = 6;
const CHROME_GAP: f32 = 6.0;
const CHROME_RULE: f32 = 1.0;
const CLOSE_MARK: &str = "\u{00d7}";
const CONFIRM_MARK: &str = "?";
const DROP_WIDTH: f32 = 18.0;
const GRIP_WIDTH: f32 = 14.0;
const GRIP_MARK: &str = "\u{2630}";
const ROW_PITCH: f32 = ICON_SIZE + CELL_PADDING[0] * 2.0 + ROW_GAP;
const RAW_PITCH: f32 = 30.0;
const CARRIED_TINT: f32 = 0.25;
const DRAG_SLACK: i64 = 1;
const RAW_CELL_WIDTH: f32 = 72.0;
const ADD_LABEL: &str = "Add Enemy";
const ENEMY_CEILING: usize = 11;
const CROWDED_LABEL: &str = "Too Many Enemy IDs!";
const CROWDED_TIP: &str = concat!(
    "There are too many Enemy IDs present on the battlefield!\n",
    "The game faults when 11 or more Enemy IDs are defined",
);
const PICK_TITLE: &str = "Enemy";
const PICK_HINT: &str = "Enter Name or ID...";
const PICK_MISSING: &str = "No enemy by that name or id";
const PICK_CONFIRM: &str = "Confirm";
const PICK_SIZE: iced::Size = iced::Size::new(288.0, 168.0);
const PICK_POPUP: crate::widget::popup::Spec =
    crate::widget::popup::Spec::new(crate::widget::popup::Kind::EnemyPick, PICK_SIZE);
const PICK_STEP: f32 = 9.0;
const PICK_LABEL: f32 = 13.0;
const PICK_SEAT: f32 = 216.0;
const ENEMY_MARKS: [&str; 4] = ["_e", "_E", "-e", "-E"];
const PICK_JOINT: &str = "::";
const PICK_JOINT_GAP: f32 = 5.0;
const TIP_PADDING: f32 = 6.0;
const COMMON_WIDTH: usize = 10;
const FIRST_ENEMY: i32 = 0;

const SEEDS: [(usize, i32); 8] = [
    (ENEMY_ID, FIRST_ENEMY),
    (AMOUNT, 1),
    (SPAWN_COLUMN, 2),
    (RESPAWN_MIN, 2),
    (RESPAWN_MAX, 2),
    (BASE_COLUMN, 100),
    (LAYER_MIN, 9),
    (LAYER_MAX, 9),
];
const DOJO_FLOOR: i32 = 100;

const SPAWN_COLUMN: usize = 2;
const RESPAWN_MIN: usize = 3;
const RESPAWN_MAX: usize = 4;
const BASE_COLUMN: usize = 5;
const LAYER_MIN: usize = 6;
const LAYER_MAX: usize = 7;
const BOSS_COLUMN: usize = 8;
const SCORE_COLUMN: usize = 10;
const KILLS_COLUMN: usize = 13;

enum Cell {
    Enemy,
    One(usize),
    Boss(usize),
    Range(usize, usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Sync,
    Drop(usize),
}

#[derive(Default, Clone, Copy)]
enum Drag {
    #[default]
    Idle,
    Pressed {
        row: usize,
    },
    Moving {
        seat: usize,
        origin: usize,
        anchor: f32,
    },
}

impl Drag {
    fn row(self) -> Option<usize> {
        match self {
            Drag::Idle => None,
            Drag::Pressed { row } => Some(row),
            Drag::Moving { seat, .. } => Some(seat),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Treatment {
    raw: i32,
    label: &'static str,
}

impl std::fmt::Display for Treatment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label)
    }
}

const FLAGS: [Treatment; 2] =
    [Treatment { raw: 0, label: "No" }, Treatment { raw: 1, label: "Yes" }];

const TREATMENTS: [Treatment; 3] = [
    Treatment { raw: 0, label: "No" },
    Treatment { raw: 1, label: "Yes" },
    Treatment { raw: 2, label: "Yes (Shake)" },
];

struct Shape {
    dojo: bool,
    split: bool,
    scored: bool,
}

impl Shape {
    fn of(draft: &Draft) -> Shape {
        let any = |test: &dyn Fn(usize) -> bool| (0..draft.len()).any(test);

        Shape {
            dojo: any(&|row| draft.reads(row, BASE_COLUMN) > DOJO_FLOOR),
            split: any(&|row| {
                let attack = draft.reads(row, ATK_MAGNIFICATION);

                attack != 0 && attack != draft.reads(row, MAGNIFICATION)
            }),
            scored: any(&|row| draft.reads(row, SCORE_COLUMN) > 0),
        }
    }

    fn columns(&self) -> Vec<(String, u16, Cell)> {
        let magnification = match self.split {
            true => "Magnification %\n(HP% / ATK%)",
            false => "Magnification %",
        };

        let base = match self.dojo {
            true => "Dmg #",
            false => "Base %",
        };

        let mut held = vec![
            ("Enemy".to_owned(), ENEMY_SPAN, Cell::Enemy),
            ("Count".to_owned(), COUNT_SPAN, Cell::One(AMOUNT)),
            (magnification.to_owned(), MAG_SPAN, Cell::One(MAGNIFICATION)),
            (base.to_owned(), BASE_SPAN, Cell::One(BASE_COLUMN)),
            ("Spawn".to_owned(), SPAWN_SPAN, Cell::One(SPAWN_COLUMN)),
            ("Respawn".to_owned(), RESPAWN_SPAN, Cell::Range(RESPAWN_MIN, RESPAWN_MAX)),
            ("Layer".to_owned(), LAYER_SPAN, Cell::Range(LAYER_MIN, LAYER_MAX)),
            ("Boss".to_owned(), BOSS_SPAN, Cell::Boss(BOSS_COLUMN)),
        ];

        if self.scored {
            held.push(("Score".to_owned(), SCORE_SPAN, Cell::One(SCORE_COLUMN)));
        }

        held.push(("Kills".to_owned(), KILLS_SPAN, Cell::One(KILLS_COLUMN)));

        held
    }
}

fn raw_columns(draft: &Draft) -> Vec<(String, u16, Cell)> {
    let widest = (0..draft.len()).map(|row| draft.width(row)).max().unwrap_or(WIDTH).max(WIDTH);

    (0..widest).map(|column| (label(Kind::Spawn, column), 1, Cell::One(column))).collect()
}

pub(super) fn kind() -> crate::widget::popup::Kind {
    GROUND_POPUP.kind()
}

pub(super) fn pick_kind() -> crate::widget::popup::Kind {
    PICK_POPUP.kind()
}

fn resolve(
    typed: &str,
    enemies: &std::collections::HashMap<u32, kore::domains::enemy::scanner::EnemyEntry>,
) -> Option<(u32, String)> {
    let typed = typed.trim();

    if typed.is_empty() {
        return None;
    }

    let head = ENEMY_MARKS
        .iter()
        .find_map(|mark| typed.strip_suffix(mark))
        .map_or(typed, str::trim_end);

    if let Ok(id) = head.parse::<u32>() {
        return enemies.get(&id).map(|held| (id, held.name.clone()));
    }

    enemies
        .values()
        .find(|held| held.name.trim().eq_ignore_ascii_case(typed))
        .map(|held| (held.id, held.name.clone()))
}

#[derive(Default)]
pub(super) struct State {
    draft: Option<Draft>,
    frame: crate::widget::popup::State,
    confirm: crate::common::feedback::Slot<Intent>,
    typing: Option<usize>,
    picking: Option<usize>,
    hunt: String,
    pick_frame: crate::widget::popup::State,
    drag: Drag,
    offset: f32,
    icons: std::cell::RefCell<std::collections::HashMap<u32, iced::widget::image::Handle>>,
}

impl State {
    pub(super) fn begin(&mut self, plan: Plan, nudge: usize, vfs: &Vfs) {
        self.frame = crate::widget::popup::cascaded(nudge);
        self.offset = 0.0;
        self.typing = None;
        self.drag = Drag::Idle;
        self.draft = Draft::load(plan, vfs);
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

    pub(super) fn flush_now(&mut self, vfs: &Vfs) {
        if let Some(draft) = self.draft.as_mut() {
            draft.persist_now(vfs);
        }
    }

    pub(super) fn forget(&self) {
        self.icons.borrow_mut().clear();
    }

    pub(super) fn sync(&mut self, plan: Option<Plan>, vfs: &Vfs) {
        let Some(current) = self.draft.as_ref() else {
            return;
        };

        if current.dirty || current.writing {
            return;
        }

        let Some(plan) = plan else {
            self.draft = None;

            return;
        };

        if current.plan.target_mod.is_none() != plan.target_mod.is_none() {
            self.draft = None;

            return;
        }

        if !current.plan.matches(&plan) || preview::stamp(&current.read_from) != Some(current.stamp) {
            self.draft = Draft::load(plan, vfs);
        }
    }

    fn haul(&mut self, at: iced::Point) {
        let Some(draft) = self.draft.as_mut() else {
            self.drag = Drag::Idle;

            return;
        };

        let (seat, origin, anchor) = match self.drag {
            Drag::Idle => return,
            Drag::Pressed { row } => {
                self.drag = Drag::Moving { seat: row, origin: row, anchor: at.y };

                return;
            }
            Drag::Moving { seat, origin, anchor } => (seat, origin, anchor),
        };

        let top = draft.chrome() as i64;
        let bottom = draft.len().saturating_sub(1) as i64;
        let pitch = match draft.values() == EditorMode::Raw {
            true => RAW_PITCH,
            false => ROW_PITCH,
        };

        let aimed = origin as i64 + ((at.y - anchor) / pitch).round() as i64;

        if aimed < top - DRAG_SLACK || aimed > bottom + DRAG_SLACK {
            self.drag = Drag::Idle;

            return;
        }

        let landed = aimed.clamp(top, bottom).max(0) as usize;
        let mut held = seat;

        while held < landed {
            draft.shift(held, held + 1);
            held += 1;
        }

        while held > landed {
            draft.shift(held, held - 1);
            held -= 1;
        }

        self.drag = Drag::Moving { seat: held, origin, anchor };
    }

    pub(super) fn update(&mut self, message: Message, vfs: &Vfs) -> iced::Task<Message> {
        if let Some(draft) = self.draft.as_mut() {
            let typing = matches!(&message, Message::Changed(row, column, _) if draft.buffering(*row, *column));

            if !typing {
                draft.resolve_buffer();
            }
        }

        match message {
            Message::Popup(msg) => {
                if self.frame.update(msg, GROUND_POPUP) {
                    self.flush_now(vfs);
                    self.draft = None;
                    self.offset = 0.0;
                    self.typing = None;
                    self.drag = Drag::Idle;
                    self.confirm.expire();
                }
            }
            Message::Changed(row, column, typed) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.edit(row, column, &typed);
                }
            }
            Message::Picked(row, column, raw) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.set(row, column, raw);
                }
            }
            Message::Focused(row) => self.typing = row,
            Message::Picking(row) => {
                self.picking = row;
                self.hunt.clear();

                if row.is_some() {
                    self.pick_frame = crate::widget::popup::cascaded(1);
                }
            }
            Message::Hunted(typed) => self.hunt = typed,
            Message::Pick(msg) => {
                if self.pick_frame.update(msg, PICK_POPUP) {
                    self.picking = None;
                    self.hunt.clear();
                }
            }
            Message::Confirmed(id) => {
                let aimed = self.picking.take();
                self.hunt.clear();

                if let (Some(row), Some(draft)) = (aimed, self.draft.as_mut()) {
                    draft.set(row, ENEMY_ID, i32::from(u16::try_from(id).unwrap_or_default()) - ENEMY_OFFSET);
                }
            }
            Message::Added => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.add();
                }
            }
            Message::Dropped(row) => {
                if !self.confirm.take(&Intent::Drop(row)) {
                    return self.confirm.set(Intent::Drop(row), Message::SyncExpired);
                }

                self.typing = None;

                if let Some(draft) = self.draft.as_mut() {
                    draft.drop(row);
                }
            }
            Message::Grabbed(row) => {
                self.typing = None;
                self.drag = Drag::Pressed { row };
            }
            Message::Hauled(at) => self.haul(at),
            Message::Released => self.drag = Drag::Idle,
            Message::Scrolled(offset) => self.offset = offset,
            Message::SyncExpired => self.confirm.expire(),
            Message::Sync => {
                if !self.confirm.take(&Intent::Sync) {
                    return self.confirm.set(Intent::Sync, Message::SyncExpired);
                }

                if let Some(draft) = self.draft.as_mut() {
                    draft.sync();
                }
            }
            Message::Persisted(..) => {}
        }

        if self.draft.as_ref().is_some_and(|draft| draft.dirty) {
            self.flush_now(vfs);
        }

        iced::Task::none()
    }

    pub(super) fn view<'a>(
        &'a self,
        window: iced::Size,
        enemies: &'a std::collections::HashMap<u32, kore::domains::enemy::scanner::EnemyEntry>,
    ) -> Option<iced::Element<'a, Message>> {
        let draft = self.draft.as_ref()?;

        Some(self.frame.view(
            &draft.plan.label,
            GROUND_POPUP,
            window,
            Message::Popup,
            move || self.body(draft, enemies),
            None,
        ))
    }

    pub(super) fn picker_view<'a>(
        &'a self,
        window: iced::Size,
        enemies: &'a std::collections::HashMap<u32, kore::domains::enemy::scanner::EnemyEntry>,
    ) -> Option<iced::Element<'a, Message>> {
        self.picking?;

        Some(self.pick_frame.view(
            PICK_TITLE,
            PICK_POPUP,
            window,
            Message::Pick,
            move || self.picker(enemies),
            None,
        ))
    }

    fn picker<'a>(
        &'a self,
        enemies: &'a std::collections::HashMap<u32, kore::domains::enemy::scanner::EnemyEntry>,
    ) -> iced::Element<'a, Message> {
        use iced::alignment::Horizontal;
        use iced::widget::{button, column, container, text, text_input};
        use iced::{Length, Theme};

        let found = resolve(&self.hunt, enemies);

        let field = text_input(PICK_HINT, &self.hunt)
            .size(PICK_LABEL)
            .padding(crate::widget::picker::COMBO_PADDING)
            .width(Length::Fixed(PICK_SEAT))
            .on_input(Message::Hunted)
            .on_submit_maybe(found.as_ref().map(|(id, _)| Message::Confirmed(*id)))
            .style(crate::app::theme::rounded_input);

        let faded = |held: String| {
            text(held).size(PICK_LABEL).style(|theme: &Theme| text::Style {
                color: Some(crate::app::theme::weak_text_color(theme)),
            })
        };

        let landing: iced::Element<'a, Message> = match &found {
            Some((id, name)) => iced::widget::row![
                faded(name.clone()),
                joint(),
                faded(format!("{id:03}-E")),
            ]
            .spacing(PICK_JOINT_GAP)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
            None => faded(PICK_MISSING.to_owned()).into(),
        };

        let confirm = button(crate::app::theme::centered_text(PICK_CONFIRM).size(PICK_LABEL).width(Length::Fill))
            .width(Length::Fixed(PICK_SEAT))
            .padding([3, 6])
            .on_press_maybe(found.as_ref().map(|(id, _)| Message::Confirmed(*id)))
            .style(crate::app::theme::primary_button);

        let body = column![field, landing, confirm].spacing(PICK_STEP).align_x(Horizontal::Center);

        container(body).padding(BODY_PADDING).width(Length::Fill).center_x(Length::Fill).into()
    }

    fn body<'a>(
        &'a self,
        draft: &'a Draft,
        enemies: &'a std::collections::HashMap<u32, kore::domains::enemy::scanner::EnemyEntry>,
    ) -> iced::Element<'a, Message> {
        use iced::widget::{container, scrollable, Column};
        use iced::Length;

        let fixed = draft.values() == EditorMode::Raw;

        let shown = match fixed {
            true => raw_columns(draft),
            false => Shape::of(draft).columns(),
        };

        let mut grid = Column::new().spacing(ROW_GAP);
        grid = grid.push(headings(&shown, fixed));

        for row in draft.chrome()..draft.len() {
            grid = grid.push(self.spawn_row(draft, enemies, &shown, row, fixed));
        }

        grid = grid.push(adding(draft.crowded()));

        let spread = match fixed {
            true => Length::Fixed(
                shown.len() as f32 * (RAW_CELL_WIDTH + COLUMN_GAP)
                    + GRIP_WIDTH
                    + COLUMN_GAP
                    + DROP_WIDTH
                    + CELL_PADDING[1] * 2.0
                    + BODY_PADDING * 2.0,
            ),
            false => Length::Fill,
        };

        let reading = match fixed {
            true => scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::new(),
                horizontal: scrollable::Scrollbar::new(),
            },
            false => scrollable::Direction::Vertical(scrollable::Scrollbar::new()),
        };

        let area = scrollable(container(grid).padding(BODY_PADDING).width(spread))
            .direction(reading)
            .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
            .width(Length::Fill)
            .height(Length::Fill);

        let mut stack = Column::new().height(Length::Fill);

        if draft.chrome() > 0 {
            stack = stack.push(chrome(draft));
            stack = stack.push(iced::widget::rule::horizontal(CHROME_RULE));
        }

        let seated = stack.push(crate::widget::smooth_scroll(area)).push(self.footer());

        if self.drag.row().is_none() {
            return seated.into();
        }

        let sheet = iced::widget::mouse_area(
            iced::widget::Space::new().width(Length::Fill).height(Length::Fill),
        )
        .interaction(iced::mouse::Interaction::Grabbing)
        .on_move(Message::Hauled)
        .on_release(Message::Released)
        .on_exit(Message::Released);

        iced::widget::stack![seated, sheet].into()
    }

    fn footer<'a>(&'a self) -> iced::Element<'a, Message> {
        use iced::widget::{button, container, text};
        use iced::Length;

        let armed = self.confirm.armed_for(&Intent::Sync);
        let label = if armed { crate::common::feedback::CONFIRM_LABEL } else { "Sync With \"game\"" };

        let sync = button(crate::app::theme::centered_text(label).size(CELL_SIZE).wrapping(text::Wrapping::None))
            .width(Length::Fixed(SYNC_WIDTH))
            .padding([CELL_INSET + 3.0, 10.0])
            .style(crate::app::theme::danger_button)
            .on_press(Message::Sync);

        container(sync)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(iced::Padding::ZERO.top(COLUMN_GAP).bottom(BODY_PADDING))
            .into()
    }

    fn spawn_row<'a>(
        &'a self,
        draft: &'a Draft,
        enemies: &'a std::collections::HashMap<u32, kore::domains::enemy::scanner::EnemyEntry>,
        shown: &[(String, u16, Cell)],
        row: usize,
        fixed: bool,
    ) -> iced::Element<'a, Message> {
        use iced::alignment::Vertical;
        use iced::widget::{button, container, Row};
        use iced::{Length, Theme};

        let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

        line = line.push(grip(row));

        for (_, span, cell) in shown {
            let held: iced::Element<'a, Message> = match cell {
                Cell::Enemy => self.enemy_cell(draft, enemies, row),
                Cell::One(column) => field(draft, row, *column),
                Cell::Boss(column) => treatment(draft, row, *column),
                Cell::Range(least, most) => ranged(draft, row, *least, *most),
            };

            line = line.push(container(held).width(reach(*span, fixed)));
        }

        let armed = self.confirm.armed_for(&Intent::Drop(row));
        let mark = if armed { CONFIRM_MARK } else { CLOSE_MARK };

        line = line.push(
            button(crate::app::theme::centered_text(mark).size(CELL_SIZE))
                .width(Length::Fixed(DROP_WIDTH))
                .padding(0)
                .on_press(Message::Dropped(row))
                .style(crate::app::theme::danger_button),
        );

        let carried = self.drag.row() == Some(row);

        container(line)
            .padding(CELL_PADDING)
            .width(Length::Fill)
            .style(move |theme: &Theme| match carried {
                true => carried_seat(theme),
                false => crate::app::theme::zebra_table_row(theme, row),
            })
            .into()
    }

    fn enemy_cell<'a>(
        &'a self,
        draft: &'a Draft,
        enemies: &'a std::collections::HashMap<u32, kore::domains::enemy::scanner::EnemyEntry>,
        row: usize,
    ) -> iced::Element<'a, Message> {
        use iced::widget::{button, container, image as iced_image};
        use iced::Length;

        let seated = |held: iced::Element<'a, Message>| {
            container(held).height(Length::Fixed(ICON_SIZE)).center_y(Length::Fixed(ICON_SIZE)).into()
        };

        if self.typing == Some(row) || draft.values() == EditorMode::Raw {
            return seated(field(draft, row, ENEMY_ID));
        }

        let shown = draft.reads(row, ENEMY_ID).saturating_add(ENEMY_OFFSET).max(0);

        let Some(handle) = u32::try_from(shown).ok().and_then(|id| self.icon(enemies, id)) else {
            return seated(field(draft, row, ENEMY_ID));
        };

        let icon = button(iced_image(handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE)))
            .padding(0)
            .style(|_theme: &iced::Theme, _status| iced::widget::button::Style::default())
            .on_press(Message::Picking(Some(row)));

        let named = u32::try_from(shown).ok().and_then(|id| enemies.get(&id)).map(|held| held.name.clone());

        let bubble = iced::widget::container(spoken(named, shown))
            .padding(TIP_PADDING)
            .style(iced::widget::container::bordered_box);

        let told = iced::widget::tooltip(icon, bubble, iced::widget::tooltip::Position::Top);

        container(told).width(Length::Fill).center_x(Length::Fill).into()
    }

    fn icon(
        &self,
        enemies: &std::collections::HashMap<u32, kore::domains::enemy::scanner::EnemyEntry>,
        id: u32,
    ) -> Option<iced::widget::image::Handle> {
        if let Some(cached) = self.icons.borrow().get(&id) {
            return Some(cached.clone());
        }

        let path = enemies.get(&id)?.icon_path.as_ref()?;
        let handle = crate::common::item_icon::load_scaled(path, ICON_SIZE as u32)?;
        self.icons.borrow_mut().insert(id, handle.clone());

        Some(handle)
    }
}

fn chrome<'a>(draft: &'a Draft) -> iced::Element<'a, Message> {
    use iced::widget::{container, Column};
    use iced::{Length, Padding};

    let mut stack = Column::new().spacing(CHROME_GAP);

    for row in 0..draft.chrome() {
        stack = stack.push(settings(draft, row));
    }

    container(stack)
        .padding(
            Padding::ZERO.left(BODY_PADDING).right(BODY_PADDING).top(BODY_PADDING).bottom(BODY_PADDING),
        )
        .width(Length::Fill)
        .into()
}

fn settings<'a>(draft: &'a Draft, row: usize) -> iced::Element<'a, Message> {
    use iced::widget::Column;
    use iced::Length;

    let kind = draft.kind(row);
    let width = draft.width(row).max(kind.width());
    let mut table = Column::new().width(Length::Fill);

    for (band, first) in (0..width).step_by(SETTING_BAND).enumerate() {
        let seats: Vec<Option<usize>> =
            (first..first + SETTING_BAND).map(|column| (column < width).then_some(column)).collect();

        table = table.push(band_labels(kind, &seats));
        table = table.push(band_fields(draft, row, &seats, band));
    }

    table.into()
}

fn band_labels<'a>(kind: Kind, seats: &[Option<usize>]) -> iced::Element<'a, Message> {
    use iced::alignment::Vertical;
    use iced::widget::{container, Row, Space};
    use iced::Length;

    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    for seat in seats {
        line = match seat {
            Some(column) => line.push(
                crate::app::theme::table_cell_text(label(kind, *column), Length::FillPortion(1))
                    .size(CELL_SIZE),
            ),
            None => line.push(Space::new().width(Length::FillPortion(1))),
        };
    }

    container(line)
        .padding(CELL_PADDING)
        .width(Length::Fill)
        .style(crate::app::theme::zebra_table_header)
        .into()
}

fn band_fields<'a>(
    draft: &'a Draft,
    row: usize,
    seats: &[Option<usize>],
    band: usize,
) -> iced::Element<'a, Message> {
    use iced::alignment::Vertical;
    use iced::widget::{container, Row, Space};
    use iced::{Length, Theme};

    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    for seat in seats {
        line = match seat {
            Some(column) => {
                let held = match rule(draft.kind(row), *column) {
                    Rule::Flag if draft.values() != EditorMode::Raw => flagged(draft, row, *column),
                    _ => field(draft, row, *column),
                };

                line.push(container(held).width(Length::FillPortion(1)))
            }
            None => line.push(Space::new().width(Length::FillPortion(1))),
        };
    }

    container(line)
        .padding(CELL_PADDING)
        .width(Length::Fill)
        .style(move |theme: &Theme| crate::app::theme::zebra_table_row(theme, band))
        .into()
}

fn adding<'a>(crowded: bool) -> iced::Element<'a, Message> {
    use iced::widget::{button, container, tooltip};
    use iced::Length;

    if !crowded {
        return button(crate::app::theme::centered_text(ADD_LABEL).size(CELL_SIZE).width(Length::Fill))
            .width(Length::Fill)
            .padding(CELL_PADDING)
            .on_press(Message::Added)
            .style(crate::app::theme::primary_button)
            .into();
    }

    let barred = button(crate::app::theme::centered_text(CROWDED_LABEL).size(CELL_SIZE).width(Length::Fill))
        .width(Length::Fill)
        .padding(CELL_PADDING)
        .style(crate::app::theme::danger_status);

    let bubble = container(crate::app::theme::centered_text(CROWDED_TIP).size(CELL_SIZE))
        .padding(TIP_PADDING)
        .style(container::bordered_box);

    tooltip(barred, bubble, tooltip::Position::Top).into()
}

fn grip<'a>(row: usize) -> iced::Element<'a, Message> {
    use iced::widget::{container, mouse_area, text};
    use iced::Length;

    let glyph = mouse_area(text(GRIP_MARK).size(CELL_SIZE))
        .interaction(iced::mouse::Interaction::Grab)
        .on_press(Message::Grabbed(row));

    container(glyph).width(Length::Fixed(GRIP_WIDTH)).center_x(Length::Fixed(GRIP_WIDTH)).into()
}

fn carried_seat(theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Color { a: CARRIED_TINT, ..theme.palette().primary }.into()),
        ..iced::widget::container::Style::default()
    }
}

fn joint<'a>() -> iced::widget::Text<'a> {
    iced::widget::text(PICK_JOINT).size(PICK_LABEL).font(iced::Font {
        weight: iced::font::Weight::Bold,
        ..iced::Font::DEFAULT
    })
}

fn spoken<'a>(named: Option<String>, id: i32) -> iced::Element<'a, Message> {
    use iced::alignment::Vertical;
    use iced::widget::{text, Row};

    let marked = format!("{id:03}-E");

    let Some(name) = named.filter(|held| !held.trim().is_empty()) else {
        return text(marked).size(PICK_LABEL).into();
    };

    Row::new()
        .spacing(PICK_JOINT_GAP)
        .align_y(Vertical::Center)
        .push(text(name).size(PICK_LABEL))
        .push(joint())
        .push(text(marked).size(PICK_LABEL))
        .into()
}

fn reach(span: u16, fixed: bool) -> iced::Length {
    match fixed {
        true => iced::Length::Fixed(RAW_CELL_WIDTH),
        false => iced::Length::FillPortion(span),
    }
}

fn headings<'a>(shown: &[(String, u16, Cell)], fixed: bool) -> iced::Element<'a, Message> {
    use iced::alignment::Vertical;
    use iced::widget::{container, Row};
    use iced::Length;

    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    line = line.push(crate::app::theme::table_cell_text("", Length::Fixed(GRIP_WIDTH)).size(CELL_SIZE));

    for (label, span, _) in shown {
        line = line.push(crate::app::theme::table_cell_text(label.clone(), reach(*span, fixed)).size(CELL_SIZE));
    }

    line = line.push(crate::app::theme::table_cell_text("X", Length::Fixed(DROP_WIDTH)).size(CELL_SIZE));

    container(line)
        .padding(CELL_PADDING)
        .width(Length::Fill)
        .style(crate::app::theme::zebra_table_header)
        .into()
}

fn treatment<'a>(draft: &'a Draft, row: usize, column: usize) -> iced::Element<'a, Message> {
    chosen(draft, row, column, &TREATMENTS)
}

fn flagged<'a>(draft: &'a Draft, row: usize, column: usize) -> iced::Element<'a, Message> {
    chosen(draft, row, column, &FLAGS)
}

fn chosen<'a>(
    draft: &'a Draft,
    row: usize,
    column: usize,
    offered: &'static [Treatment],
) -> iced::Element<'a, Message> {
    use iced::widget::pick_list;
    use iced::Length;

    let held = draft.reads(row, column);

    let Some(current) = offered.iter().find(|shown| shown.raw == held).cloned() else {
        return field(draft, row, column);
    };

    pick_list(offered.to_vec(), Some(current), move |pick: Treatment| {
        Message::Picked(row, column, pick.raw)
    })
    .width(Length::Fill)
    .padding(CELL_INSET)
    .text_size(CELL_SIZE)
    .style(crate::app::theme::combo_box)
    .menu_style(crate::app::theme::combo_box_menu)
    .into()
}

fn ranged<'a>(draft: &'a Draft, row: usize, least: usize, most: usize) -> iced::Element<'a, Message> {
    use iced::alignment::Vertical;
    use iced::widget::{container, text, Row};
    use iced::Length;

    Row::new()
        .spacing(RANGE_GAP)
        .align_y(Vertical::Center)
        .push(container(field(draft, row, least)).width(Length::FillPortion(1)))
        .push(text(RANGE_MARK).size(CELL_SIZE))
        .push(container(field(draft, row, most)).width(Length::FillPortion(1)))
        .into()
}

fn field<'a>(draft: &'a Draft, row: usize, column: usize) -> iced::Element<'a, Message> {
    use iced::alignment::Horizontal;
    use iced::widget::text_input;
    use iced::Length;

    text_input(&hint(draft.kind(row), column, draft.values()), draft.input(row, column))
        .on_input(move |typed| Message::Changed(row, column, typed))
        .on_submit(Message::Focused(None))
        .size(CELL_SIZE)
        .padding(CELL_INSET)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .style(crate::app::theme::rounded_input)
        .into()
}

pub(super) fn plan(label: String, game: &Path, target_mod: Option<String>, values: EditorMode) -> Plan {
    Plan { label, game: game.to_path_buf(), target_mod, values }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use nyanko::combat::Separator;
    use nyanko::common;

    use nyanko::chapter::stage::Battleground;

    use kore::domains::settings::EditorMode;

    use super::{body, cells, scan, Kind, Rule};

    // nyanko drops one row from `entries` on sight: the placeholder that is enemy 21 at
    // frame 27000 (raw id 23, raw frame 13500 before its Double scale). The editor keeps
    // it, because it is a real line somebody may want to change, so the corpus check has
    // to add it back rather than pretend the two sets are equal.
    fn hidden(line: &str, delimiter: char) -> bool {
        let parts = cells(body(line), delimiter);
        let read = |at: usize| parts.get(at).and_then(|cell| cell.trim().parse::<i32>().ok());

        read(0) == Some(23) && read(2) == Some(13500)
    }

    fn corpus() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.join(".claude/refs/game/stages");

        root.is_dir().then_some(root)
    }

    fn walk(root: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                walk(&path, found);
                continue;
            }

            let named = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();

            if named.starts_with("stage") && named.ends_with(".csv") {
                found.push(path);
            }
        }
    }

    // The editor writes raw lines, so it has to find the enemy rows itself rather than
    // going through the parser. Agreeing with nyanko on every shipped stage is what keeps
    // that mirror honest: header rows, config rows, comments, blanks and the 0 terminator
    // all have to land the same way.
    // Every other editor blanks a cell that already holds its column's declared default
    // and offers that default as the ghost value, so an empty field means "leave it".
    // The picker takes a name or an id, and the _e / -E suffix the export field wants is
    // optional here because the column already knows it is looking at an enemy.
    // A new row is written in file units, so the seeds go through the same scale the
    // editor shows them at: a spawn of 2 frames is a stored 1, because that column is Double.
    // The buffer prefix lets a half-typed value sit in the field without being committed
    // and normalised under the cursor, the way it does in the figures editor.
    // The header and config rows are seated ahead of the spawns, so a file that opens
    // straight into its config still leaves the spawn rows starting at the same place.
    #[test]
    fn the_chrome_rows_are_seated_before_the_spawns() {
        let with_head = super::Sheet { head: Some(0), config: 1, spawns: vec![2, 3] };
        let (seats, kinds, chrome) = super::seating(&with_head);

        assert_eq!(seats, [0, 1, 2, 3]);
        assert_eq!(chrome, 2, "a header and a config sit ahead of the spawns");
        assert_eq!(kinds[0], Kind::Head);
        assert_eq!(kinds[1], Kind::Setup);
        assert_eq!(kinds[2], Kind::Spawn);

        let bare = super::Sheet { head: None, config: 0, spawns: vec![1] };
        let (seats, kinds, chrome) = super::seating(&bare);

        assert_eq!(seats, [0, 1]);
        assert_eq!(chrome, 1, "a file with no header row still seats its config");
        assert_eq!(kinds[0], Kind::Setup);
    }

    // Each row kind reads its own published table, so the same index means different
    // things: column 1 is the spawn count, the config's base hitpoints and the header's
    // no-continues flag.
    // Measured over the 6,321 shipped stages: the header's second column runs 3,842 zeros
    // to 2,117 ones and the config's ninth 6,237 to 62, so both are booleans. The header's
    // base id genuinely reaches -1 (449 rows), so it floors there rather than at zero.
    #[test]
    fn the_chrome_rules_follow_their_measured_domains() {
        assert_eq!(super::rule(Kind::Head, 1), Rule::Flag);
        assert_eq!(super::rule(Kind::Setup, 8), Rule::Flag);
        assert_eq!(super::rule(Kind::Head, 0), Rule::Floor(-1));
        assert_eq!(super::rule(Kind::Setup, 0), Rule::Floor(0));
    }

    // The column names which enemy in the battleground becomes the animated base, so it
    // reads in the same space a spawn row's leading cell does and translates the same way.
    // Its zero means "none" rather than enemy 000, and a default that resolves below zero
    // offers no ghost value, so the 5,882 stages declaring none show an empty field.
    #[test]
    fn the_animated_base_id_translates_like_a_spawn_row() {
        use kore::domains::settings::EditorMode;

        assert_eq!(super::rule(Kind::Setup, 6), Rule::Offset(super::ENEMY_OFFSET));
        assert_eq!(super::shown_cell(Kind::Setup, 46, 6, EditorMode::Resolved), "44");
        assert_eq!(super::shown_cell(Kind::Setup, 46, 6, EditorMode::Raw), "46");
        assert_eq!(super::hint(Kind::Setup, 6, EditorMode::Resolved), "");
    }

    #[test]
    fn each_row_kind_names_its_own_columns() {
        assert_eq!(super::label(Kind::Spawn, 1), "Count");
        assert_eq!(super::label(Kind::Setup, 1), "Base HP");
        assert_eq!(super::label(Kind::Head, 1), "No Continues");
    }

    #[test]
    fn the_buffer_prefix_is_typable_and_plain_text_still_is() {
        for good in ["", "-", "5", "!", "!5", "!-", "!-5"] {
            assert!(super::typable(good, true), "{good:?} should be typable");
        }

        for bad in ["+5", "5a", "1.5", "!!5", "! 5"] {
            assert!(!super::typable(bad, true), "{bad:?} should be rejected");
        }
    }

    // start_frame, respawn_min and respawn_max carry nyanko's Double scale, so the file
    // stores half of what the editor shows and an odd number of frames cannot be stored
    // at all. Typing 5 lands on 4; Raw is where an exact cell gets written.
    #[test]
    fn an_odd_frame_count_snaps_because_the_file_stores_halves() {
        use kore::domains::settings::EditorMode;

        let stored = super::to_raw(Kind::Spawn, super::RESPAWN_MAX, 5, EditorMode::Resolved);

        assert_eq!(stored, 2, "five frames is two and a half stored, and the cell holds an integer");
        assert_eq!(super::shown_cell(Kind::Spawn, stored, super::RESPAWN_MAX, EditorMode::Resolved), "4");
        assert_eq!(super::shown_cell(Kind::Spawn, stored, super::RESPAWN_MAX, EditorMode::Raw), "2");
    }

    #[test]
    fn a_new_row_is_seeded_at_the_values_the_editor_shows() {
        use kore::domains::settings::EditorMode;

        for (column, shown) in super::SEEDS {
            let stored = super::seed(column, shown);

            assert_eq!(
                super::shown_cell(Kind::Spawn, stored, column, EditorMode::Resolved),
                shown.to_string(),
                "column {column} should read back as {shown}",
            );
        }
    }

    #[test]
    fn a_new_row_never_starts_on_the_terminator() {
        let stored = super::seed(super::ENEMY_ID, super::FIRST_ENEMY);

        assert_ne!(stored, 0, "a leading zero ends the file, so enemy 000 has to store 2");
        assert_eq!(stored, 2);
    }

    #[test]
    fn the_picker_takes_a_bare_id_a_suffixed_id_or_a_name() {
        use kore::domains::enemy::scanner::EnemyEntry;

        let mut roster = std::collections::HashMap::new();
        roster.insert(
            44,
            EnemyEntry {
                id: 44,
                name: "Gory Black".to_owned(),
                description: Vec::new(),
                stats: nyanko::combat::Entity::default(),
                icon_path: None,
                atk_anim_frames: 0,
            },
        );

        for typed in ["44", "44_e", "44-E", "Gory Black", " gory black "] {
            assert_eq!(
                super::resolve(typed, &roster).map(|(id, _)| id),
                Some(44),
                "{typed} should resolve",
            );
        }

        for typed in ["", "45", "Gory"] {
            assert!(super::resolve(typed, &roster).is_none(), "{typed} should not resolve");
        }
    }

    #[test]
    fn a_cell_holding_its_default_reads_back_empty() {
        use kore::domains::settings::EditorMode;

        let magnification = super::MAGNIFICATION;
        let held = super::fallback(Kind::Spawn, magnification);

        assert_eq!(held, 100, "nyanko declares 100 for the magnification column");
        assert_eq!(super::shown_cell(Kind::Spawn, held, magnification, EditorMode::Resolved), "");
        assert_eq!(super::shown_cell(Kind::Spawn, 150, magnification, EditorMode::Resolved), "150");
        assert_eq!(super::hint(Kind::Spawn, magnification, EditorMode::Resolved), "100");
    }

    // The file stores the enemy's page id plus two, and translating that is the whole
    // point of the resolved mode.
    #[test]
    fn the_enemy_column_reads_two_below_what_the_file_stores() {
        use kore::domains::settings::EditorMode;

        assert_eq!(super::shown_cell(Kind::Spawn, 2, super::ENEMY_ID, EditorMode::Resolved), "0");
        assert_eq!(super::shown_cell(Kind::Spawn, 2, super::ENEMY_ID, EditorMode::Raw), "2");
    }

    #[test]
    fn a_magnification_splits_on_any_of_the_three_marks() {
        for typed in ["100/200", "100\\200", "100|200"] {
            assert_eq!(super::magnifications(typed), (Some("100"), Some("200")), "{typed}");
        }

        assert_eq!(super::magnifications("100"), (Some("100"), Some("100")), "one number sets both");
    }

    #[test]
    fn the_line_scan_agrees_with_nyanko_on_every_shipped_stage() {
        let Some(root) = corpus() else {
            return;
        };

        let mut files = Vec::new();
        walk(&root, &mut files);

        assert!(files.len() > 5000, "expected the shipped stage corpus, found {}", files.len());

        let mut checked = 0;
        let mut skipped = 0;

        for path in &files {
            let Ok(bytes) = fs::read(path) else { continue };
            let scrubbed = common::scrub(&bytes);
            let delimiter = Separator::detect(&scrubbed).unwrap_or(Separator::Comma).char();
            let lines: Vec<String> = scrubbed.lines().map(str::to_owned).collect();

            let Ok(parsed) = Battleground::parse(&bytes, None) else {
                skipped += 1;
                continue;
            };

            let Some(sheet) = scan(&lines, delimiter) else {
                skipped += 1;
                continue;
            };

            let masked = sheet
                .spawns
                .iter()
                .filter(|index| lines.get(**index).is_some_and(|line| hidden(line, delimiter)))
                .count();

            assert_eq!(
                sheet.spawns.len() - masked,
                parsed.entries.len(),
                "{}: scanned {} spawn rows ({masked} hidden), nyanko parsed {}",
                path.display(),
                sheet.spawns.len(),
                parsed.entries.len(),
            );

            for index in &sheet.spawns {
                let Some(line) = lines.get(*index) else { continue };
                let row = super::split(line, delimiter);

                assert_eq!(
                    row.rebuild(0, delimiter),
                    *line,
                    "{}: line {index} does not survive an untouched rebuild",
                    path.display(),
                );
            }

            checked += 1;
        }

        assert!(checked > 5000, "only {checked} files agreed, {skipped} skipped");
    }

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bcc-ground-test-{label}-{}", std::process::id()));

        fs::create_dir_all(&dir).expect("failed to create the temp fixture dir");

        dir
    }

    fn seeded(label: &str, body: &str) -> (PathBuf, super::State, kore::Vfs) {
        let path = scratch_dir(label).join("stage00.csv");
        fs::write(&path, body).expect("failed to seed the temp fixture file");

        let seed = super::plan("test".to_owned(), &path, None, EditorMode::Resolved);
        let vfs = kore::Vfs::with_priority(&[]);
        let state = super::State { draft: super::Draft::load(seed, &vfs), ..super::State::default() };

        assert!(state.draft.is_some(), "the draft should load from the temp fixture");

        (path, state, vfs)
    }

    // A drag swaps whole file lines, so a reordered row keeps its own cells and its own
    // trailing comment, and the config row above the spawns never moves.
    #[test]
    fn dragging_a_spawn_row_swaps_its_line_and_leaves_the_chrome_alone() {
        let seeded_body = "0,0\n300,1000,30,90,0,100,9,9,0,100\n5,1,2,2,2,100,9,9,0,100 //first\n7,1,2,2,2,100,9,9,0,200\n0\n";
        let (path, mut state, vfs) = seeded("reorder", seeded_body);

        let first = state.draft.as_ref().map_or(0, |draft| draft.chrome());

        let _ = state.update(super::Message::Grabbed(first), &vfs);
        let _ = state.update(super::Message::Hauled(iced::Point::new(0.0, 0.0)), &vfs);
        let _ = state.update(super::Message::Hauled(iced::Point::new(0.0, super::ROW_PITCH)), &vfs);
        let _ = state.update(super::Message::Released, &vfs);

        let written = fs::read_to_string(&path).expect("the editor should have written the file");
        let lines: Vec<&str> = written.lines().collect();

        assert_eq!(lines[0], "0,0", "the header row stays put");
        assert_eq!(lines[1], "300,1000,30,90,0,100,9,9,0,100", "the config row stays put");
        assert_eq!(lines[2], "7,1,2,2,2,100,9,9,0,200", "the second spawn rose above the first");
        assert_eq!(lines[3], "5,1,2,2,2,100,9,9,0,100 //first", "the dragged row kept its comment");
        assert_eq!(lines[4], "0", "the terminator stays last");
    }

    // Eleven distinct ids is where the engine faults, and duplicates do not count towards
    // it, so a battleground of ten ids stays open however many rows it spreads them over.
    #[test]
    fn the_add_button_bars_an_eleventh_enemy_id_but_not_a_repeat() {
        let mut body = String::from("0,0\n300,1000,30,90,0,100,9,9,0,100\n");

        for id in 0..10 {
            body.push_str(&format!("{},1,2,2,2,100,9,9,0,100\n", id + 2));
        }

        body.push_str("0\n");

        let (_, mut state, vfs) = seeded("ceiling", &body);

        let crowded = |state: &super::State| state.draft.as_ref().is_some_and(super::Draft::crowded);

        assert!(!crowded(&state), "ten distinct ids is still under the ceiling");

        let _ = state.update(super::Message::Added, &vfs);

        assert!(!crowded(&state), "a seeded row repeats an id already on the field");

        if let Some(draft) = state.draft.as_mut() {
            let row = draft.len() - 1;
            draft.set(row, super::ENEMY_ID, 99);
        }

        assert!(crowded(&state), "the eleventh distinct id closes the button");

        let before = state.draft.as_ref().map_or(0, super::Draft::len);
        let _ = state.update(super::Message::Added, &vfs);

        assert_eq!(
            state.draft.as_ref().map_or(0, super::Draft::len),
            before,
            "a barred press adds nothing",
        );
    }

    // The seat is a function of how far the cursor has travelled from the grab, not a
    // running tally, so wandering up and coming back lands the row where the cursor is.
    // Past the ends of the list the hold lets go rather than following from a distance.
    #[test]
    fn the_hold_tracks_the_cursor_and_breaks_past_the_ends() {
        let seeded_body = "0,0\n300,1000,30,90,0,100,9,9,0,100\n5,1,2,2,2,100,9,9,0,100\n7,1,2,2,2,100,9,9,0,100\n9,1,2,2,2,100,9,9,0,100\n0\n";
        let (path, mut state, vfs) = seeded("tracking", seeded_body);

        let first = state.draft.as_ref().map_or(0, |draft| draft.chrome());
        let held = |state: &super::State| state.drag.row();

        let haul = |state: &mut super::State, y: f32| {
            let _ = state.update(super::Message::Hauled(iced::Point::new(0.0, y)), &vfs);
        };

        let _ = state.update(super::Message::Grabbed(first + 2), &vfs);
        haul(&mut state, 0.0);
        haul(&mut state, -super::ROW_PITCH * 2.0);

        assert_eq!(held(&state), Some(first), "two rows of travel lifts it to the top");

        haul(&mut state, -super::ROW_PITCH * 2.0 + 1.0);
        haul(&mut state, 0.0);

        assert_eq!(held(&state), Some(first + 2), "coming back returns it under the cursor");

        haul(&mut state, -super::ROW_PITCH * 4.0);

        assert_eq!(held(&state), None, "straying past the top lets the row go");

        let written = fs::read_to_string(&path).expect("the editor should have written the file");
        let lines: Vec<&str> = written.lines().collect();

        assert_eq!(lines[2], "5,1,2,2,2,100,9,9,0,100", "the round trip left the order as it started");
        assert_eq!(lines[4], "9,1,2,2,2,100,9,9,0,100", "the dragged row is back where it began");
    }
}
