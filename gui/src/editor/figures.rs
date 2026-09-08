mod cards;
mod combat;
mod combos;
mod costs;
mod mapdata;
mod itembuy;
mod mapdrops;
pub(super) mod resolved;
mod schema;
mod talents;
mod unitbuy;
mod unitlevel;

pub(crate) use itembuy::ITEM_ID_COLUMN;
pub(crate) use mapdata::MAP_HEADER_LINES;
pub(crate) use mapdrops::{MATERIAL_FIRST as MAP_DROPS_MATERIALS, WIDTH as MAP_DROPS_WIDTH};
pub(crate) use schema::{Subject, COUNT, FORMS, SUBJECTS};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use iced::widget::{operation, scrollable};
use iced::{Element, Size, Task};
use nyanko::combat::Separator;
use nyanko::common;
use tracing::warn;

use kore::common::preview::{self, Stamp};
use kore::domains::cat::scanner::CatEntry;
use kore::domains::{mods, settings::EditorMode};
use kore::{Vault, Vfs};

use crate::common::feedback::Slot;
use crate::widget::popup;

use cards::CARD_WIDTH;
use resolved::{Face, Rule};

const POPUP_SIZE: Size = Size::new(364.0, 376.0);

const CAT_POPUP: popup::Spec = popup::Spec::new(popup::Kind::CatAttributes, POPUP_SIZE);
const ENEMY_POPUP: popup::Spec = popup::Spec::new(popup::Kind::EnemyAttributes, POPUP_SIZE);
const BUY_POPUP: popup::Spec = popup::Spec::new(popup::Kind::UnitBuy, POPUP_SIZE);
const CURVE_POPUP: popup::Spec = popup::Spec::new(popup::Kind::LevelCurve, POPUP_SIZE);
const TALENT_SIZE: Size = Size::new(760.0, 540.0);
const TALENT_POPUP: popup::Spec = popup::Spec::new(popup::Kind::Talents, TALENT_SIZE);
const COSTS_SIZE: Size = Size::new(364.0, 520.0);
const COSTS_POPUP: popup::Spec = popup::Spec::new(popup::Kind::TalentCosts, COSTS_SIZE);
pub(super) const COMBO_SIZE: Size = Size::new(420.0, 296.0);
const COMBO_POPUP: popup::Spec = popup::Spec::new(popup::Kind::Combos, COMBO_SIZE);
const MAP_STAGE_SIZE: Size = Size::new(364.0, 520.0);
const MAP_DROPS_SIZE: Size = Size::new(364.0, 520.0);
const MAP_DROPS_POPUP: popup::Spec = popup::Spec::new(popup::Kind::MapDrops, MAP_DROPS_SIZE);
const DROP_CHARA_SIZE: Size = Size::new(364.0, 300.0);
const DROP_CHARA_POPUP: popup::Spec = popup::Spec::new(popup::Kind::DropChara, DROP_CHARA_SIZE);
const ITEM_BUY_SIZE: Size = Size::new(364.0, 460.0);
const ITEM_BUY_POPUP: popup::Spec = popup::Spec::new(popup::Kind::ItemBuy, ITEM_BUY_SIZE);
const MAP_STAGE_POPUP: popup::Spec = popup::Spec::new(popup::Kind::MapStage, MAP_STAGE_SIZE);

pub(super) fn kind(subject: Subject) -> popup::Kind {
    spec(subject).kind()
}

fn spec(subject: Subject) -> popup::Spec {
    match subject {
        Subject::Cat => CAT_POPUP,
        Subject::Enemy => ENEMY_POPUP,
        Subject::Buy => BUY_POPUP,
        Subject::Curve => CURVE_POPUP,
        Subject::Talents => TALENT_POPUP,
        Subject::Costs => COSTS_POPUP,
        Subject::Combo => COMBO_POPUP,
        Subject::MapStage => MAP_STAGE_POPUP,
        Subject::MapDrops => MAP_DROPS_POPUP,
        Subject::DropChara => DROP_CHARA_POPUP,
        Subject::ItemBuy => ITEM_BUY_POPUP,
    }
}

const LABEL_SIZE: f32 = 12.0;
const CARD_PADDING: f32 = 6.0;
const COMMENT: &str = "//";
const BUFFER_MARK: char = '!';
const LINE_RATIO: f32 = 1.3;
const GLYPH_RATIO: f32 = 0.55;

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);

fn next_token() -> u64 {
    NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
}

static LABEL_HEIGHTS: LazyLock<[f32; COUNT]> =
    LazyLock::new(|| SUBJECTS.map(|subject| label_height(schema::of(subject))));

fn label_height(schema: &schema::Schema) -> f32 {
    let inner = CARD_WIDTH - CARD_PADDING * 2.0;
    let per_line = ((inner / (LABEL_SIZE * GLYPH_RATIO)).floor() as usize).max(1);

    let lines = (0..schema.known())
        .map(|index| wrapped_lines(&schema.label(index), per_line))
        .max()
        .unwrap_or(1);

    lines as f32 * LABEL_SIZE * LINE_RATIO
}

struct Row {
    cells: Vec<i32>,
    written: Vec<String>,
    stored: usize,
    comment: String,
}

fn split_row(line: &str, delimiter: char, schema: &schema::Schema) -> Row {
    split_span(line, delimiter, schema, 0, schema.known())
}

fn split_span(line: &str, delimiter: char, schema: &schema::Schema, first: usize, len: usize) -> Row {
    let (numeric, comment) = match line.find(COMMENT) {
        Some(at) => {
            let (head, tail) = line.split_at(at);

            (head.trim_end(), tail[COMMENT.len()..].trim().to_owned())
        }
        None => (line, String::new()),
    };

    let mut fields: Vec<&str> = numeric.split(delimiter).collect();

    while fields.last().is_some_and(|field| field.trim().is_empty()) {
        fields.pop();
    }

    let stored = fields.len();
    let mut written: Vec<String> = fields.iter().map(|field| (*field).to_owned()).collect();
    let mut cells: Vec<i32> = fields
        .iter()
        .enumerate()
        .map(|(at, field)| parse_scaled(field, schema.decimals(first + at)).unwrap_or(0))
        .collect();

    while cells.len() < len {
        let fallback = schema.fallback(first + cells.len());
        written.push(fallback.to_string());
        cells.push(fallback);
    }

    Row { cells, written, stored, comment }
}

// A cell that lives on a line other than the addressed one. MapStageData's two map-wide
// rows are the only users: they lead the draft's flat cell space and are drawn above the
// scroll, so the popup is one draft and one menu entry rather than three of each.
struct Slab {
    line: usize,
    len: usize,
    written: Vec<String>,
    stored: usize,
    touched: usize,
    comment: String,
}

fn chrome_rows(
    schema: &schema::Schema,
    lines: &[String],
    delimiter: char,
) -> (Vec<Slab>, Vec<i32>) {
    let mut slabs = Vec::new();
    let mut cells = Vec::new();

    for (line, len) in schema.chrome().iter().copied() {
        let raw = lines.get(line).map(String::as_str).unwrap_or_default();
        let row = split_span(raw, delimiter, schema, cells.len(), len);

        cells.extend(row.cells);
        slabs.push(Slab {
            line,
            len,
            written: row.written,
            stored: row.stored,
            touched: 0,
            comment: row.comment,
        });
    }

    (slabs, cells)
}

fn vacant(plan: &Plan, lines: &[String], delimiter: char) -> String {
    let mut fields: Vec<String> =
        (0..plan.schema.known()).map(|index| plan.schema.fallback(index).to_string()).collect();

    if let Some(id) = plan.address.key()
        && let Some(first) = fields.first_mut()
    {
        *first = id.to_string();
    }

    if plan.schema.subject() == Subject::Combo {
        combos::seed(&mut fields, plan.schema, plan.anchor);
        combos::seed_key(&mut fields, plan.schema, lines, delimiter);
    }

    fields.join(&delimiter.to_string())
}

fn shown(schema: &schema::Schema, index: usize, raw: i32, values: EditorMode, rule: Rule) -> String {
    if raw == schema.fallback(index) {
        return String::new();
    }

    let held = rule.to_display(schema.to_display(index, raw, values), values);

    format_scaled(held, schema.decimals(index))
}

fn typable(value: &str, signed: bool, places: u32) -> bool {
    typable_digits(value.strip_prefix(BUFFER_MARK).unwrap_or(value), signed, places)
}

fn typable_digits(value: &str, signed: bool, places: u32) -> bool {
    let body = match value.strip_prefix('-') {
        Some(_) if !signed => return false,
        Some(rest) => rest,
        None => value,
    };

    let mut points = 0;

    for glyph in body.chars() {
        if glyph == '.' && places > 0 {
            points += 1;

            if points > 1 {
                return false;
            }

            continue;
        }

        if !glyph.is_ascii_digit() {
            return false;
        }
    }

    true
}

// A column whose cells are written with a decimal point is held as a whole number of its
// smallest place -- DropItem's crown multipliers are hundredths -- so the whole editor keeps
// its i32 cell model and only the text boundary knows. `places` is 0 for every other column,
// where both of these are the plain i32 conversions.
fn decimal_unit(places: u32) -> i32 {
    10_i32.saturating_pow(places)
}

fn parse_scaled(text: &str, places: u32) -> Option<i32> {
    let text = text.trim();

    if places == 0 {
        return text.parse::<i32>().ok();
    }

    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let negative = whole.starts_with('-');

    let held: String = fraction.chars().take(places as usize).collect();
    let part: i32 = format!("{:0<width$}", held, width = places as usize).parse().ok()?;

    let whole: i32 = match whole {
        "" | "-" => 0,
        held => held.parse().ok()?,
    };

    let magnitude = whole.checked_abs()?.checked_mul(decimal_unit(places))?.checked_add(part)?;

    Some(if negative { -magnitude } else { magnitude })
}

fn format_scaled(value: i32, places: u32) -> String {
    if places == 0 {
        return value.to_string();
    }

    let unit = decimal_unit(places);
    let whole = value / unit;
    let part = (value % unit).abs();

    if part == 0 {
        return whole.to_string();
    }

    let digits = format!("{:0width$}", part, width = places as usize);
    let digits = digits.trim_end_matches('0');
    let sign = if value < 0 && whole == 0 { "-" } else { "" };

    format!("{sign}{whole}.{digits}")
}

fn wrapped_lines(label: &str, per_line: usize) -> usize {
    let mut lines = 1;
    let mut used = 0;

    for word in label.split_whitespace() {
        let length = word.chars().count();

        if used == 0 {
            used = length;
            continue;
        }

        if used + 1 + length <= per_line {
            used += 1 + length;
            continue;
        }

        lines += 1;
        used = length;
    }

    lines
}

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Changed(usize, String),
    Picked(usize, i32),
    Scrolled(f32),
    Picker(Option<usize>),
    CommentChanged(String),
    SearchChanged(String),
    HuntChanged(String),
    Aimed(Address),
    Slotted(usize, i32, i32),
    Removed,
    Sync,
    ConfirmExpired,
    Persisted(u64, PathBuf, Option<Stamp>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Sync,
    Remove,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Address {
    Line(usize),
    Keyed(u32),
    // A row addressed by an id that is not in the leading cell. Gatyaitembuy names its item
    // in column 3, and finding the row by that beats counting past a header the file need
    // not carry.
    Column { at: usize, id: u32 },
    Appended,
}

impl Address {
    fn locate(self, lines: &[String], delimiter: char) -> Option<usize> {
        match self {
            Address::Line(row) => Some(row),
            Address::Keyed(id) => lines.iter().position(|line| leading(line, delimiter) == Some(id)),
            Address::Column { at, id } => {
                lines.iter().position(|line| cell_of(line, delimiter, at) == Some(id))
            }
            Address::Appended => None,
        }
    }

    fn insertion(self, lines: &[String], delimiter: char) -> Option<usize> {
        let id = match self {
            Address::Appended => return Some(lines.len()),
            Address::Keyed(id) => id,
            Address::Line(_) | Address::Column { .. } => return None,
        };

        let at = lines
            .iter()
            .position(|line| leading(line, delimiter).is_some_and(|other| other > id))
            .unwrap_or(lines.len());

        Some(at)
    }

    fn nearest(self, lines: &[String], delimiter: char) -> Option<usize> {
        self.key()?;

        lines.iter().position(|line| leading(line, delimiter).is_some())
    }

    fn key(self) -> Option<u32> {
        match self {
            Address::Keyed(id) => Some(id),
            Address::Line(_) | Address::Column { .. } | Address::Appended => None,
        }
    }
}

fn leading(line: &str, delimiter: char) -> Option<u32> {
    cell_of(line, delimiter, 0)
}

fn cell_of(line: &str, delimiter: char, at: usize) -> Option<u32> {
    line.split(delimiter).nth(at)?.trim().parse().ok()
}

#[derive(Clone)]
pub(crate) struct Plan {
    address: Address,
    labels: Option<Arc<[String]>>,
    label: String,
    game: PathBuf,
    target_mod: Option<String>,
    schema: &'static schema::Schema,
    values: EditorMode,
    anchor: Option<(i32, i32)>,
}

impl Plan {
    pub(super) fn subject(&self) -> schema::Subject {
        self.schema.subject()
    }

    fn matches(&self, other: &Plan) -> bool {
        self.address == other.address
            && self.labels == other.labels
            && self.game == other.game
            && self.target_mod == other.target_mod
            && self.label == other.label
            && self.values == other.values
            && self.anchor == other.anchor
    }

    fn rowed(self, address: Address) -> Plan {
        Plan { address, ..self }
    }

    pub(super) fn anchored(self, anchor: Option<(i32, i32)>) -> Plan {
        Plan { anchor, ..self }
    }

    // Column names the static table cannot know, because they come from the game's own data
    // -- DropItem's material slots are named by the items `MAT_IDS` points them at. Resolved
    // once where the vault is in hand rather than per draw.
    pub(super) fn named(self, labels: Vec<String>) -> Plan {
        Plan { labels: Some(labels.into()), ..self }
    }

    fn source(&self, vfs: &Vfs) -> PathBuf {
        self.target_mod
            .as_deref()
            .and_then(|name| mods::find(vfs, name, &self.game))
            .unwrap_or_else(|| self.game.clone())
    }
}

pub(crate) type Marks = [u8; schema::TALENT_SLOTS];

#[derive(Clone, Copy)]
pub(super) struct Frame<'a> {
    width: f32,
    height: f32,
    offset: f32,
    query: &'a str,
    armed: bool,
    removing: bool,
    cap: Option<i32>,
    used: Marks,
    cats: &'a [CatEntry],
    names: &'a talents::Names,
    catalogue: &'a combos::Catalogue,
    vault: &'a Vault,
    picker: Option<usize>,
    hunt: &'a str,
}

#[derive(Default)]
pub(super) struct State {
    draft: Option<Draft>,
    frame: popup::State,
    query: String,
    offset: f32,
    confirm: Slot<Intent>,
    pinned: Option<Address>,
    names: talents::Names,
    catalogue: combos::Catalogue,
    picker: Option<usize>,
    hunt: String,
}

struct Draft {
    plan: Plan,
    chrome: Vec<Slab>,
    absent: bool,
    row: usize,
    read_from: PathBuf,
    stamp: Stamp,
    delimiter: char,
    keys: Vec<u32>,
    keyed: Option<u32>,
    lines: Vec<String>,
    cells: Vec<i32>,
    written: Vec<String>,
    stored: usize,
    touched: usize,
    rules: Vec<Rule>,
    comment: String,
    inputs: Vec<String>,
    failed: bool,
    buffer: Option<usize>,
    dirty: bool,
    writing: bool,
    token: u64,
}

impl State {
    pub(super) fn begin(&mut self, plan: Plan, nudge: usize, vfs: &Vfs) {
        self.frame = popup::cascaded(nudge);
        self.offset = 0.0;
        self.picker = None;
        self.reload(plan, vfs);
    }

    pub(super) fn restore_scroll<M: Send + 'static>(&self) -> Option<Task<M>> {
        let draft = self.draft.as_ref()?;
        let target = scrollable::AbsoluteOffset { x: 0.0, y: self.offset };

        Some(operation::scroll_to(cards::grid_id(draft.plan.subject()), target))
    }

    fn reload(&mut self, plan: Plan, vfs: &Vfs) {
        self.draft = Draft::load(plan, vfs);
        self.pinned = self.draft.as_ref().and_then(Draft::pin);
    }

    pub(super) fn relocalize(&self) {
        self.names.forget();
        self.catalogue.forget();
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

    pub(super) fn flush(&mut self, vfs: &Vfs) -> Task<Message> {
        let Some(draft) = self.draft.as_mut() else {
            return Task::none();
        };

        draft.resolve_buffer();
        draft.persist_if_dirty(vfs)
    }

    pub(super) fn flush_now(&mut self, vfs: &Vfs) {
        let Some(draft) = self.draft.as_mut() else {
            return;
        };

        draft.resolve_buffer();
        draft.persist_now(vfs);
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

        let plan = match self.pinned {
            Some(address) if plan.anchor == current.plan.anchor => plan.rowed(address),
            _ => plan,
        };

        if current.plan.target_mod.is_none() != plan.target_mod.is_none() {
            self.draft = None;

            return;
        }

        if !current.plan.matches(&plan) {
            self.reload(plan, vfs);

            return;
        }

        if preview::stamp(&current.read_from) != Some(current.stamp) {
            self.reload(plan, vfs);
        }
    }

    pub(super) fn update(&mut self, message: Message, vfs: &Vfs) -> Task<Message> {
        if let Some(draft) = self.draft.as_mut() {
            let still_typing = matches!(&message, Message::Changed(index, _) if draft.buffering(*index));

            if !still_typing {
                draft.resolve_buffer();
            }
        }

        match message {
            Message::Popup(msg) => {
                let Some(spec) = self.draft.as_ref().map(|draft| spec(draft.plan.subject())) else {
                    return Task::none();
                };

                if self.frame.update(msg, spec) {
                    let task = self.draft.as_mut().map_or(Task::none(), |draft| draft.persist_if_dirty(vfs));

                    self.draft = None;
                    self.offset = 0.0;
                    self.picker = None;
                    self.confirm.expire();

                    return task;
                }
            }
            Message::Changed(index, value) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.edit(index, &value);
                }
            }
            Message::Picked(index, raw) => {
                self.picker = None;
                self.hunt.clear();

                if let Some(draft) = self.draft.as_mut() {
                    draft.pick(index, raw);
                }
            }
            Message::Picker(index) => {
                self.picker = index;
                self.hunt.clear();
                self.offset = 0.0;
            }
            Message::Aimed(address) => {
                let Some(draft) = self.draft.as_mut().filter(|draft| draft.aimed() != address) else {
                    return Task::none();
                };

                draft.persist_now(vfs);

                if draft.aim(address) {
                    self.offset = 0.0;
                    self.picker = None;
                }
            }
            Message::Slotted(index, unit, form) => {
                self.picker = None;
                self.hunt.clear();

                if let Some(draft) = self.draft.as_mut() {
                    draft.slotted(index, unit, form);
                }
            }
            Message::Scrolled(offset) => self.offset = offset,
            Message::SearchChanged(query) => self.query = query,
            Message::HuntChanged(query) => {
                self.hunt = query;
                self.offset = 0.0;
            }
            Message::CommentChanged(value) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.set_comment(value);
                }
            }
            Message::ConfirmExpired => self.confirm.expire(),
            Message::Sync => {
                if !self.confirm.take(&Intent::Sync) {
                    return self.confirm.set(Intent::Sync, Message::ConfirmExpired);
                }

                if let Some(draft) = self.draft.as_mut() {
                    draft.sync();
                }
            }
            Message::Removed => {
                if !self.confirm.take(&Intent::Remove) {
                    return self.confirm.set(Intent::Remove, Message::ConfirmExpired);
                }

                let Some(draft) = self.draft.as_mut() else {
                    return Task::none();
                };

                if !draft.remove() {
                    return Task::none();
                }

                draft.persist_now(vfs);
                draft.aim(Address::Appended);

                self.offset = 0.0;
                self.picker = None;
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

        self.pinned = self.draft.as_ref().and_then(Draft::pin);

        self.draft.as_mut().map_or(Task::none(), |draft| draft.persist_if_dirty(vfs))
    }

    pub(super) fn view<'a>(
        &'a self,
        window: Size,
        cap: Option<i32>,
        used: Marks,
        cats: &'a [CatEntry],
        vault: &'a Vault,
    ) -> Option<Element<'a, Message>> {
        let draft = self.draft.as_ref()?;
        let spec = spec(draft.plan.subject());
        let width = self.frame.body_width(spec, window);
        let height = self.frame.body_height(spec, window);
        let armed = self.confirm.armed_for(&Intent::Sync);
        let removing = self.confirm.armed_for(&Intent::Remove);
        let query = self.query.as_str();
        let frame = Frame {
            width,
            height,
            offset: self.offset,
            query,
            armed,
            removing,
            cap,
            used,
            cats,
            names: &self.names,
            catalogue: &self.catalogue,
            vault,
            picker: self.picker,
            hunt: &self.hunt,
        };

        Some(self.frame.view(
            &draft.plan.label,
            spec,
            window,
            Message::Popup,
            move || draft.body(frame),
            None,
        ))
    }
}

fn write_now(path: &Path, body: &[u8], stamp: Stamp) -> Option<Stamp> {
    preview::save(path, body, stamp)
        .inspect_err(|err| warn!(path = %path.display(), "Stats editor could not write the file: {}", err))
        .ok()
}

impl Draft {
    fn load(plan: Plan, vfs: &Vfs) -> Option<Draft> {
        let read_from = plan.source(vfs);

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Stats editor could not read the file: {}", err))
            .ok()?;

        let Some(stamp) = preview::stamp(&read_from) else {
            warn!(path = %read_from.display(), "Stats editor could not stamp the file");

            return None;
        };

        let body = common::scrub(&bytes);
        let delimiter = Separator::detect(&body).unwrap_or(Separator::Comma).char();
        let lines: Vec<String> = body.lines().map(str::to_owned).collect();

        let switches = plan.schema.switches();
        let keys: Vec<u32> = match switches {
            true => lines.iter().filter_map(|line| leading(line, delimiter)).collect(),
            false => Vec::new(),
        };

        let found = plan
            .address
            .locate(&lines, delimiter)
            .or_else(|| switches.then(|| plan.address.nearest(&lines, delimiter)).flatten());
        let absent = found.is_none();

        let Some(row) = found.or_else(|| plan.address.insertion(&lines, delimiter)) else {
            warn!(path = %read_from.display(), rows = lines.len(), "Stats editor found no row for this subject");

            return None;
        };

        let blank = vacant(&plan, &lines, delimiter);
        let raw = match lines.get(row).filter(|_| !absent) {
            Some(line) => line.as_str(),
            None if plan.schema.creates() => blank.as_str(),
            None => {
                warn!(
                    path = %read_from.display(),
                    row,
                    rows = lines.len(),
                    "Stats editor found no row for this form"
                );

                return None;
            }
        };

        let keyed = switches.then(|| lines.get(row).and_then(|line| leading(line, delimiter))).flatten();

        let (chrome, lead) = chrome_rows(plan.schema, &lines, delimiter);
        let span = plan.schema.known() - lead.len();
        let Row { mut cells, written, stored, comment } =
            split_span(raw, delimiter, plan.schema, lead.len(), span);

        let mut held = lead;
        held.append(&mut cells);
        let cells = held;

        let rules: Vec<Rule> = (0..cells.len())
            .map(|index| resolved::rule(plan.subject(), index, plan.schema.field(index), &cells))
            .collect();
        let inputs = (0..cells.len())
            .map(|index| shown(plan.schema, index, cells[index], plan.values, rules[index]))
            .collect();

        Some(Draft {
            plan,
            chrome,
            absent,
            row,
            read_from,
            stamp,
            delimiter,
            keys,
            keyed,
            lines,
            cells,
            written,
            stored,
            touched: 0,
            rules,
            comment,
            inputs,
            failed: false,
            buffer: None,
            dirty: false,
            writing: false,
            token: next_token(),
        })
    }

    fn edit(&mut self, index: usize, value: &str) {
        if !self.write(index, value) {
            return;
        }

        if let Some(twin) = self.mirror(index) {
            self.write(twin, value);
        }

        self.stage();
    }

    fn resolve_buffer(&mut self) {
        let Some(index) = self.buffer.take() else {
            return;
        };

        let Some(typed) = self.inputs.get(index).and_then(|slot| slot.strip_prefix(BUFFER_MARK))
        else {
            return;
        };

        let typed = typed.to_owned();

        self.edit(index, &typed);
    }

    fn buffering(&self, index: usize) -> bool {
        self.buffer == Some(index)
    }

    fn mirror(&self, index: usize) -> Option<usize> {
        if self.plan.values != EditorMode::Resolved
            || self.plan.subject() != Subject::Talents
        {
            return None;
        }

        talents::mirror(index, &self.cells)
    }

    fn write(&mut self, index: usize, value: &str) -> bool {
        let rule = self.rule(index);
        let values = self.plan.values;

        let places = self.plan.schema.decimals(index);

        if !typable(value, rule.signed(values), places) {
            return false;
        }

        let buffering = value.starts_with(BUFFER_MARK);

        let Some(slot) = self.inputs.get_mut(index) else {
            return false;
        };

        if buffering {
            *slot = value.to_owned();
            self.buffer = Some(index);

            return false;
        }

        if self.buffer == Some(index) {
            self.buffer = None;
        }

        let raw = if value.is_empty() {
            *slot = String::new();
            self.plan.schema.fallback(index)
        } else {
            let Some(display) = parse_scaled(value, places) else {
                *slot = value.to_owned();

                return false;
            };

            let unshifted = rule.to_raw(rule.clamp(display, values), values);
            let raw = self.plan.schema.to_raw(index, unshifted, values);
            *slot = shown(self.plan.schema, index, raw, values, rule);

            raw
        };

        let Some(cell) = self.cells.get_mut(index) else {
            return false;
        };

        if *cell == raw {
            return false;
        }

        *cell = raw;
        self.record(index, raw);

        true
    }

    fn record(&mut self, index: usize, raw: i32) {
        let mut first = 0;

        for slab in &mut self.chrome {
            if index < first + slab.len {
                let at = index - first;

                if let Some(slot) = slab.written.get_mut(at) {
                    *slot = format_scaled(raw, self.plan.schema.decimals(index));
                }

                slab.touched = slab.touched.max(at + 1);

                return;
            }

            first += slab.len;
        }

        let at = index - first;

        if let Some(slot) = self.written.get_mut(at) {
            *slot = format_scaled(raw, self.plan.schema.decimals(index));
        }

        self.touched = self.touched.max(at + 1);
    }

    fn lead(&self) -> usize {
        self.chrome.iter().map(|slab| slab.len).sum()
    }

    fn restage_chrome(&mut self) {
        for slab in &self.chrome {
            let width = slab.stored.max(slab.touched);
            let mut line = slab.written[..width].join(&self.delimiter.to_string());

            if !slab.comment.is_empty() {
                line.push(self.delimiter);
                line.push(' ');
                line.push_str(COMMENT);
                line.push(' ');
                line.push_str(&slab.comment);
            }

            if let Some(slot) = self.lines.get_mut(slab.line) {
                *slot = line;
            }
        }
    }

    fn pick(&mut self, index: usize, raw: i32) {
        if self.cells.get(index) == Some(&raw) {
            return;
        }

        let Some(cell) = self.cells.get_mut(index) else {
            return;
        };

        *cell = raw;
        self.record(index, raw);

        let display = shown(self.plan.schema, index, raw, self.plan.values, self.rule(index));

        if let Some(slot) = self.inputs.get_mut(index) {
            *slot = display;
        }

        self.stage();
    }

    fn set_comment(&mut self, value: String) {
        if self.comment == value {
            return;
        }

        self.comment = value;
        self.stage();
    }

    fn sync(&mut self) {
        let Ok(bytes) = fs::read(&self.plan.game) else {
            warn!(path = %self.plan.game.display(), "Stats editor could not read the vanilla file");
            self.failed = true;

            return;
        };

        let body = common::scrub(&bytes);
        let delimiter = Separator::detect(&body).unwrap_or(Separator::Comma).char();
        let vanilla: Vec<String> = body.lines().map(str::to_owned).collect();

        let blank = vacant(&self.plan, &vanilla, delimiter);
        let located = self.address().locate(&vanilla, delimiter).and_then(|row| vanilla.get(row));

        let raw = match located {
            Some(line) => line.as_str(),
            None if self.plan.schema.creates() => blank.as_str(),
            None => {
                self.failed = true;

                return;
            }
        };

        // Sync is row-scoped, and for a subject with chrome the "row" is every line the draft
        // owns -- restoring the stage row while leaving edited map settings behind would put
        // the popup in a state the file never had.
        let (chrome, lead) = chrome_rows(self.plan.schema, &vanilla, delimiter);
        let span = self.plan.schema.known() - lead.len();
        let Row { mut cells, written, stored, comment } =
            split_span(raw, delimiter, self.plan.schema, lead.len(), span);

        let mut held = lead;
        held.append(&mut cells);

        self.chrome = chrome;
        self.cells = held;
        self.written = written;
        self.stored = stored;
        self.touched = 0;
        self.comment = comment;

        self.inputs = (0..self.cells.len())
            .map(|index| shown(self.plan.schema, index, self.cells[index], self.plan.values, self.rule(index)))
            .collect();
        self.buffer = None;
        self.stage();
    }

    fn stage(&mut self) {
        self.restage_chrome();

        let width = self.stored.max(self.touched);
        let mut line = self.written[..width].join(&self.delimiter.to_string());

        if !self.comment.is_empty() {
            line.push(self.delimiter);
            line.push(' ');
            line.push_str(COMMENT);
            line.push(' ');
            line.push_str(&self.comment);
        }

        self.restate();

        let empty = self.plan.schema.vacant(&self.cells);

        if self.absent {
            if empty {
                return;
            }

            self.row = self.plan.address.insertion(&self.lines, self.delimiter).unwrap_or(self.row);
            self.lines.insert(self.row, line);
            self.absent = false;
            self.plan = self.plan.clone().rowed(Address::Line(self.row));
        } else if empty && self.plan.schema.creates() {
            self.lines.remove(self.row);
            self.absent = true;
        } else {
            let Some(slot) = self.lines.get_mut(self.row) else {
                self.failed = true;

                return;
            };

            *slot = line;
        }

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

    fn restate(&mut self) {
        if !self.plan.schema.creates() {
            return;
        }

        self.rules = (0..self.cells.len())
            .map(|index| {
                resolved::rule(self.plan.subject(), index, self.plan.schema.field(index), &self.cells)
            })
            .collect();
    }

    fn destination(&self, vfs: &Vfs) -> Option<(PathBuf, Stamp)> {
        let Some(name) = self.plan.target_mod.as_deref() else {
            return Some((self.plan.game.clone(), self.stamp));
        };

        if self.read_from != self.plan.game {
            return Some((self.read_from.clone(), self.stamp));
        }

        let path = mods::ensure(vfs, name, &self.plan.game)
            .inspect_err(|err| warn!(source = %self.plan.game.display(), "Stats editor could not stage the file: {}", err))
            .ok()?;

        let stamp = preview::stamp(&path)?;

        Some((path, stamp))
    }

    fn schema(&self) -> &'static schema::Schema {
        self.plan.schema
    }

    fn values(&self) -> EditorMode {
        self.plan.values
    }

    fn rule(&self, index: usize) -> Rule {
        self.rules.get(index).copied().unwrap_or(Rule::Opaque)
    }

    fn note(&self, index: usize) -> Option<&'static str> {
        resolved::note(self.plan.subject(), self.plan.schema.field(index))
    }

    fn face(&self, index: usize) -> Face {
        let rule = self.rule(index);
        let raw = self.cells.get(index).copied().unwrap_or_default();

        if let Some(gate) = rule.gate()
            && self.plan.values == EditorMode::Resolved
            && self.reads(gate.field) == Some(gate.blocked)
        {
            return Face::Disabled(gate.reason);
        }

        rule.face(raw, self.plan.values)
    }

    fn reads(&self, field: &str) -> Option<i32> {
        self.plan.schema.index_of(field).and_then(|index| self.cells.get(index).copied())
    }

    fn address(&self) -> Address {
        self.keyed.map_or(self.plan.address, Address::Keyed)
    }

    fn aimed(&self) -> Address {
        if self.absent {
            return self.plan.address;
        }

        self.keyed.map_or(Address::Line(self.row), Address::Keyed)
    }

    fn aim(&mut self, address: Address) -> bool {
        if let Some(row) = address.locate(&self.lines, self.delimiter) {
            return self.retarget(row);
        }

        if !self.plan.schema.creates() {
            return false;
        }

        let Some(row) = address.insertion(&self.lines, self.delimiter) else {
            return false;
        };

        let blank = vacant(&self.plan, &self.lines, self.delimiter);
        self.plan = self.plan.clone().rowed(address);
        self.adopt(&blank, row, true);

        true
    }

    fn keys(&self) -> &[u32] {
        &self.keys
    }

    fn keyed(&self) -> Option<u32> {
        self.keyed
    }

    fn pin(&self) -> Option<Address> {
        if let Some(id) = self.keyed {
            return Some(Address::Keyed(id));
        }

        (self.plan.schema.appends() && !self.absent).then_some(Address::Line(self.row))
    }

    fn row(&self) -> usize {
        self.row
    }

    fn at(&self, field: &str) -> Option<usize> {
        self.plan.schema.index_of(field)
    }

    fn slotted(&mut self, index: usize, unit: i32, form: i32) {
        let wanted: Vec<(usize, i32)> = [(index, unit), (index + 1, form)]
            .into_iter()
            .filter(|(slot, raw)| self.cells.get(*slot) != Some(raw))
            .collect();

        if wanted.is_empty() {
            return;
        }

        self.overwrite(&wanted);

        let packing = combos::packed(self.plan.schema, &self.cells);
        self.overwrite(&packing);

        self.stage();
    }

    fn overwrite(&mut self, cells: &[(usize, i32)]) {
        for (slot, raw) in cells.iter().copied() {
            let Some(cell) = self.cells.get_mut(slot) else {
                continue;
            };

            *cell = raw;
            self.record(slot, raw);

            let display = shown(self.plan.schema, slot, raw, self.plan.values, self.rule(slot));

            if let Some(field) = self.inputs.get_mut(slot) {
                *field = display;
            }
        }
    }

    fn anchor(&self) -> Option<(i32, i32)> {
        self.plan.anchor
    }

    fn absent(&self) -> bool {
        self.absent
    }

    fn retarget(&mut self, row: usize) -> bool {
        let Some(line) = self.lines.get(row).cloned() else {
            return false;
        };

        self.plan = self.plan.clone().rowed(Address::Line(row));
        self.adopt(&line, row, false);

        true
    }

    fn adopt(&mut self, line: &str, row: usize, absent: bool) {
        let Row { cells, written, stored, comment } = split_row(line, self.delimiter, self.plan.schema);

        self.row = row;
        self.absent = absent;
        self.cells = cells;
        self.written = written;
        self.stored = stored;
        self.touched = 0;
        self.comment = comment;
        self.keyed = self
            .plan
            .schema
            .switches()
            .then(|| self.lines.get(row).and_then(|line| leading(line, self.delimiter)))
            .flatten();

        self.rules = (0..self.cells.len())
            .map(|index| {
                resolved::rule(self.plan.subject(), index, self.plan.schema.field(index), &self.cells)
            })
            .collect();

        self.inputs = (0..self.cells.len())
            .map(|index| {
                shown(self.plan.schema, index, self.cells[index], self.plan.values, self.rules[index])
            })
            .collect();

        self.buffer = None;
    }

    fn remove(&mut self) -> bool {
        if self.absent || self.row >= self.lines.len() {
            return false;
        }

        if self.row + 1 == self.lines.len() {
            self.lines.remove(self.row);
            self.dirty = true;

            return true;
        }

        let Some(index) = self.plan.schema.index_of(combos::SERIES) else {
            return false;
        };

        self.overwrite(&[(index, combos::RETIRED)]);
        self.stage();

        true
    }


    fn stored(&self) -> usize {
        self.stored
    }

    // The reward block's names depend on the row's own contents, which the static table
    // cannot see, so the label is asked of the draft rather than of the schema.
    fn label(&self, index: usize) -> std::borrow::Cow<'static, str> {
        // A blank entry means the column has no name of its own in that table, not that it
        // should render nameless.
        if let Some(named) =
            self.plan.labels.as_ref().and_then(|held| held.get(index)).filter(|held| !held.is_empty())
        {
            return std::borrow::Cow::Owned(named.clone());
        }

        if self.plan.subject() == Subject::MapStage
            && let Some(named) = mapdata::label(index, &self.cells)
        {
            return named;
        }

        self.plan.schema.label(index)
    }

    fn reads_at(&self, index: usize) -> Option<i32> {
        self.cells.get(index).copied()
    }

    fn len(&self) -> usize {
        self.cells.len()
    }

    fn input(&self, index: usize) -> &str {
        self.inputs.get(index).map_or("", String::as_str)
    }

    fn comment(&self) -> &str {
        &self.comment
    }

    fn body<'a>(&'a self, frame: Frame<'a>) -> Element<'a, Message> {
        let Frame { width, query, armed, cap, .. } = frame;

        match self.plan.subject() {
            Subject::Cat | Subject::Enemy => combat::view(self, width, query, armed),
            Subject::Buy => unitbuy::view(self, width, query, armed),
            Subject::Curve => unitlevel::view(self, width, armed, cap),
            Subject::Talents => talents::view(self, frame),
            Subject::Costs => costs::view(self, frame),
            Subject::Combo => combos::view(self, frame),
            Subject::MapStage => mapdata::view(self, width, armed),
            Subject::MapDrops => mapdrops::view(self, width, query, armed),
            Subject::DropChara => mapdrops::chara_view(self, width, armed),
            Subject::ItemBuy => itembuy::view(self, width, query, armed),
        }
    }
}

pub(super) fn plan(
    subject: Subject,
    address: Address,
    label: String,
    game: &Path,
    target_mod: Option<String>,
    values: EditorMode,
) -> Plan {
    Plan {
        address,
        labels: None,
        label,
        game: game.to_path_buf(),
        target_mod,
        schema: schema::of(subject),
        values,
        anchor: None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use kore::common::preview::{self, Stamp};
    use kore::domains::settings::EditorMode;
    use kore::Vfs;

    use super::resolved::{self, Rule};
    use super::schema::{self, Subject};
    use super::{plan, split_row, write_now, Address, Draft, Message, Row, State};

    const VANILLA: &str = "100,3,10,8,15,140,50,75,0,320,0,0,0,8,0,9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // \u{30cd}\u{30b3}";

    fn rebuild(row: &Row, touched: usize) -> String {
        let width = row.stored.max(touched);
        let mut line = row.written[..width].join(",");

        if !row.comment.is_empty() {
            line.push_str(", // ");
            line.push_str(&row.comment);
        }

        line
    }

    fn rows(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| format!("{id},0,0")).collect()
    }

    #[test]
    fn a_new_keyed_row_lands_in_id_order() {
        let lines = rows(&["ID", "677", "678", "705", "709"]);

        let at = super::Address::Keyed(680).insertion(&lines, ',');

        assert_eq!(at, Some(3), "680 belongs between 678 and 705, not appended");
    }

    #[test]
    fn a_new_keyed_row_past_the_end_appends() {
        let lines = rows(&["ID", "677", "678"]);

        assert_eq!(super::Address::Keyed(900).insertion(&lines, ','), Some(3));
    }

    #[test]
    fn a_header_never_captures_the_insertion_point() {
        let lines = rows(&["ID", "9"]);

        assert_eq!(
            super::Address::Keyed(1).insertion(&lines, ','),
            Some(1),
            "the header parses to no id, so it must never be treated as a larger one",
        );
    }

    #[test]
    fn a_talent_row_is_vacant_only_when_every_slot_is_empty() {
        let talents = schema::of(Subject::Talents);
        let mut cells = vec![0; talents.known()];

        assert!(talents.vacant(&cells), "no ability ids means the row carries nothing");

        cells[super::schema::TALENT_HEAD] = 1;
        assert!(!talents.vacant(&cells), "slot A holds an ability, so the row must survive");

        assert!(
            !schema::of(Subject::Cat).vacant(&[0; 8]),
            "only talents may delete their own row",
        );
    }

    const VANILLA_CURVE: &str = "1,25,5,5,5,5,10,10,10,10,10,";
    const VANILLA_ULTRA: &str = "3,50";

    #[test]
    fn a_single_level_cost_curve_is_written_back_byte_for_byte() {
        let row = split_row(VANILLA_ULTRA, ',', schema::of(Subject::Costs));

        assert_eq!(rebuild(&row, 0), VANILLA_ULTRA, "an ultra curve stores one level and must stay that wide");
    }

    #[test]
    fn a_cost_curve_grows_only_as_far_as_the_level_edited() {
        let row = split_row(VANILLA_ULTRA, ',', schema::of(Subject::Costs));

        assert_eq!(rebuild(&row, 4), "3,50,0,0", "levels past the one typed into stay out of the file");
    }

    #[test]
    fn a_ten_level_curve_keeps_every_cost_and_drops_only_its_trailing_empty() {
        let row = split_row(VANILLA_CURVE, ',', schema::of(Subject::Costs));

        assert_eq!(row.stored, 11, "the id plus ten levels are data; the trailing comma is not");
        assert_eq!(rebuild(&row, 0), VANILLA_CURVE.trim_end_matches(','));
    }

    #[test]
    fn a_missing_cost_curve_falls_back_to_the_first_one_the_file_has() {
        let lines = rows(&["lvID", "1", "2"]);

        assert_eq!(
            Address::Keyed(7).nearest(&lines, ','),
            Some(1),
            "the header parses to no id, so the fallback must land on the first real curve",
        );

        assert_eq!(
            Address::Line(0).nearest(&lines, ','),
            None,
            "only a keyed subject may be re-pointed at another row",
        );
    }

    const VANILLA_COMBO: &str = "3066,6,-1,781,1,689,0,-1,-1,-1,-1,-1,-1,7,1,-1";

    #[test]
    fn a_combo_row_is_written_back_byte_for_byte() {
        let row = split_row(VANILLA_COMBO, ',', schema::of(Subject::Combo));

        assert_eq!(rebuild(&row, 0), VANILLA_COMBO, "a no-op commit must not rewrite the combo");
    }

    #[test]
    fn a_combo_slot_names_the_columns_nyanko_publishes() {
        let schema = schema::of(Subject::Combo);

        for slot in 0..super::schema::COMBO_SLOTS {
            for field in [super::schema::combo_unit(slot), super::schema::combo_form(slot)] {
                assert!(
                    schema.index_of(&field).is_some(),
                    "combo: the slot view addresses {field}, which nyanko no longer publishes",
                );
            }
        }
    }

    #[test]
    fn only_talents_may_delete_their_own_row() {
        let combo = schema::of(Subject::Combo);

        assert!(
            !combo.vacant(&vec![-1; combo.known()]),
            "an emptied combo must be left in the file, not removed under it",
        );
    }

    #[test]
    fn typing_accepts_only_digits_and_a_leading_minus() {
        for good in ["", "-", "0", "42", "-7"] {
            assert!(super::typable(good, true, 0), "{good:?} should be typable");
        }

        for bad in ["+5", " 5", "5 ", "5a", "1.5", "--1", "1-"] {
            assert!(!super::typable(bad, true, 0), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn an_unsigned_field_refuses_the_minus_key() {
        assert!(!super::typable("-", false, 0), "a non-negative column must not accept a lone minus");
        assert!(!super::typable("-1", false, 0), "a non-negative column must not accept a negative");
        assert!(super::typable("1", false, 0), "digits stay typable");
    }

    #[test]
    fn resolved_leaves_a_value_it_cannot_represent_alone() {
        use super::resolved::{self, Face, Rule};
        use kore::domains::settings::EditorMode;

        let schema = schema::of(Subject::Cat);
        let Some(index) = schema.index_of("boss_wave_immune") else {
            panic!("nyanko no longer publishes boss_wave_immune");
        };

        let rule = resolved::rule(Subject::Cat, index, schema.field(index), &[]);
        assert_eq!(rule, Rule::Flag, "boss_wave_immune is a flag");

        assert_eq!(
            rule.face(-1, EditorMode::Resolved),
            Face::Danger,
            "-1 is not a flag state, so Resolved must fall back to the raw card",
        );

        assert_eq!(
            super::shown(schema, index, -1, EditorMode::Resolved, rule),
            "-1",
            "the unrepresentable value is shown literally, not blanked or coerced",
        );

        let mut cells = vec!["0"; schema.known()];
        cells[index] = "-1";
        let line = cells.join(",");

        let row = split_row(&line, ',', schema);
        assert_eq!(row.written[index], "-1", "the stored text is kept verbatim");
        assert_eq!(rebuild(&row, 0), line, "opening in Resolved must not rewrite the row");
    }

    // DropItem writes its crown multipliers with a decimal point and the file ships exactly
    // five of them, so the round trip has to reproduce each one character for character --
    // "1" and not "1.00", or an untouched row stops matching itself.
    #[test]
    fn a_decimal_cell_round_trips_every_multiplier_the_file_ships() {
        for (text, held) in [("0.5", 50), ("0.75", 75), ("1", 100), ("1.25", 125), ("1.5", 150)] {
            assert_eq!(super::parse_scaled(text, 2), Some(held), "{text} reads as hundredths");
            assert_eq!(super::format_scaled(held, 2), text, "{held} writes back as {text}");
        }
    }

    #[test]
    fn a_column_with_no_places_is_the_plain_integer_path() {
        assert_eq!(super::parse_scaled("-42", 0), Some(-42));
        assert_eq!(super::format_scaled(-42, 0), "-42");
        assert_eq!(super::parse_scaled("0.75", 0), None, "a point is not an integer");
    }

    #[test]
    fn a_negative_decimal_keeps_its_sign_either_side_of_the_point() {
        assert_eq!(super::parse_scaled("-0.75", 2), Some(-75));
        assert_eq!(super::format_scaled(-75, 2), "-0.75");
        assert_eq!(super::parse_scaled("-1.5", 2), Some(-150));
        assert_eq!(super::format_scaled(-150, 2), "-1.5");
    }

    #[test]
    fn a_decimal_column_accepts_one_point_and_no_more() {
        assert!(super::typable("0.7", false, 2));
        assert!(super::typable("0.", false, 2), "a half typed value has to survive the keystroke");
        assert!(!super::typable("0.7.5", false, 2));
        assert!(!super::typable("0.7", false, 0), "an integer column still refuses the point");
    }

    #[test]
    fn an_untouched_row_is_written_back_byte_for_byte() {
        let row = split_row(VANILLA, ',', schema::of(Subject::Cat));

        assert_eq!(rebuild(&row, 0), VANILLA, "a no-op commit must not rewrite the row");
    }

    #[test]
    fn a_short_row_is_not_padded_to_the_full_column_table() {
        let row = split_row(VANILLA, ',', schema::of(Subject::Cat));

        assert_eq!(row.stored, 52, "vanilla cat 001 stores 52 columns");
        assert_eq!(row.cells.len(), schema::of(Subject::Cat).known(), "cells pad for display");
        assert_eq!(rebuild(&row, 0).matches(',').count(), VANILLA.matches(',').count());
    }

    #[test]
    fn editing_one_column_leaves_every_other_column_verbatim() {
        let mut row = split_row(VANILLA, ',', schema::of(Subject::Cat));
        row.written[3] = "999".to_owned();

        let before: Vec<&str> = VANILLA.split(" // ").next().unwrap_or_default().split(',').collect();
        let after = rebuild(&row, 4);
        let after: Vec<&str> = after.split(" // ").next().unwrap_or_default().split(',').collect();

        assert_eq!(before.len(), after.len(), "editing must not change the column count");

        for (index, (was, now)) in before.iter().zip(&after).enumerate() {
            if index == 3 {
                assert_eq!(*now, "999");
                continue;
            }

            assert_eq!(was, now, "column {index} was rewritten by an unrelated edit");
        }
    }

    #[test]
    fn editing_past_the_stored_width_pads_only_that_far() {
        let mut row = split_row(VANILLA, ',', schema::of(Subject::Cat));
        row.written[90] = "1".to_owned();

        let line = rebuild(&row, 91);
        let head = line.strip_suffix(", // \u{30cd}\u{30b3}").unwrap_or(&line);

        assert_eq!(head.split(',').count(), 91, "padding must stop at the edited column");
    }

    #[test]
    fn typing_below_a_floor_without_buffering_snaps_immediately() {
        let schema = schema::of(Subject::Cat);
        let Some(index) = schema.index_of("cooldown") else {
            panic!("nyanko no longer publishes cooldown");
        };

        let mut draft = bare_draft(Subject::Cat, index, 100);

        assert!(draft.write(index, "1"), "the floor clamp still counts as a change");
        assert_eq!(
            draft.reads_at(index), Some(30),
            "1 frame clamps up to the 60-frame floor, then halves back into raw units",
        );
        assert_eq!(
            draft.input(index), "60",
            "without a buffer mark the field snaps to 60 on the very first digit",
        );
    }

    #[test]
    fn a_buffer_marked_cooldown_edit_defers_the_floor_and_the_scale() {
        let schema = schema::of(Subject::Cat);
        let Some(index) = schema.index_of("cooldown") else {
            panic!("nyanko no longer publishes cooldown");
        };

        let mut draft = bare_draft(Subject::Cat, index, 100);

        for partial in ["!1", "!15", "!150"] {
            assert!(!draft.write(index, partial), "a buffered keystroke never touches the cell");
            assert_eq!(draft.input(index), partial, "the field must echo exactly what was typed");
            assert_eq!(draft.reads_at(index), Some(100), "the cell stays untouched while buffering");
        }

        assert!(draft.write(index, "150"), "the debanged value commits a real change");
        assert_eq!(draft.reads_at(index), Some(75), "150 frames halves into 75 raw units");
        assert_eq!(draft.input(index), "150", "150 clears the floor, so the field shows it verbatim");
    }

    #[test]
    fn a_buffer_marked_money_value_stays_literal_until_flushed() {
        const COST_DOWN: i32 = 25;

        let schema = schema::of(Subject::Talents);
        let index = schema::TALENT_HEAD + 2;

        let mut cells = vec![0; schema.known()];
        cells[schema::TALENT_HEAD] = COST_DOWN;

        let rules: Vec<Rule> = (0..cells.len())
            .map(|position| resolved::rule(Subject::Talents, position, schema.field(position), &cells))
            .collect();

        assert_eq!(
            rules[index], Rule::Ratio(3, 2),
            "ability 25's first value pair should resolve as a 1.5x-money cost reduction",
        );

        let mut draft = draft_with(Subject::Talents, cells, rules);

        for partial in ["!1", "!15", "!150"] {
            assert!(!draft.write(index, partial), "a buffered keystroke never touches the cell");
            assert_eq!(draft.input(index), partial);
            assert_eq!(draft.reads_at(index), Some(0));
        }

        assert!(draft.write(index, "150"), "the debanged value commits a real change");
        assert_eq!(
            draft.reads_at(index), Some(100),
            "150 resolved money divides by the 1.5x ratio into 100 raw cost units",
        );
        assert_eq!(draft.input(index), "150");
    }

    // The map-wide cells and the stage's own row are one draft over three lines of the file,
    // so an edit above the scroll has to land on line 0 or 1 and leave the stage row alone.
    #[test]
    fn a_map_wide_edit_lands_on_its_own_line_and_not_the_stage_row() {
        let dir = scratch_dir("mapdata-chrome-lines");
        let path = dir.join("MapStageDataS_002.csv");
        let schema = schema::of(Subject::MapStage);

        let seeded = "19,194,-1,-1,-1,0\n3\n80,1520,3,99,33,100,2,1\n";
        std::fs::write(&path, seeded).expect("failed to seed the temp fixture file");

        let stage = 0;
        let seed = plan(
            Subject::MapStage,
            Address::Line(super::MAP_HEADER_LINES + stage),
            "test".to_owned(),
            &path,
            None,
            EditorMode::Raw,
        );

        let vfs = Vfs::with_priority(&[]);
        let mut state = State { draft: Draft::load(seed, &vfs), ..State::default() };
        assert!(state.draft.is_some(), "the draft should load from the temp fixture");

        let Some(map_number) = schema.index_of("map_number") else {
            panic!("nyanko no longer publishes map_number");
        };
        let Some(pattern) = schema.index_of(schema::MAP_PATTERN_FIELD) else {
            panic!("the joined table lost its map pattern column");
        };
        let Some(xp) = schema.index_of("xp") else {
            panic!("nyanko no longer publishes xp");
        };

        assert!(map_number < pattern && pattern < xp, "the map-wide cells must lead the stage row");

        let _ = state.update(Message::Changed(map_number, "42".to_owned()), &vfs);
        let _ = state.update(Message::Changed(pattern, "7".to_owned()), &vfs);
        let _ = state.update(Message::Changed(xp, "9999".to_owned()), &vfs);

        complete_write(&mut state, &vfs);

        let body = std::fs::read_to_string(&path).expect("the completed write should have landed");
        let lines: Vec<&str> = body.lines().collect();

        assert_eq!(lines[0], "42,194,-1,-1,-1,0", "the header edit belongs on line 0, padded no further");
        assert_eq!(lines[1], "7", "the pattern edit belongs on line 1");
        assert_eq!(lines[2], "80,9999,3,99,33,100,2,1", "the stage row keeps every column it had");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rapid_keystrokes_never_write_synchronously_and_coalesce_into_one_task() {
        let dir = scratch_dir("keystrokes-defer-disk");
        let path = dir.join("unitbuy.csv");
        let schema = schema::of(Subject::Buy);
        let Some(index) = schema.index_of("stage_unlock_requirement") else {
            panic!("nyanko no longer publishes stage_unlock_requirement");
        };

        let seed = plan(Subject::Buy, Address::Line(0), "test".to_owned(), &path, None, EditorMode::Resolved);
        std::fs::write(&path, format!("{}\n", super::vacant(&seed, &[], ',')))
            .expect("failed to seed the temp fixture file");
        let seeded = std::fs::read(&path).expect("seeded fixture should be readable");

        let vfs = Vfs::with_priority(&[]);
        let mut state = State { draft: Draft::load(seed, &vfs), ..State::default() };
        assert!(state.draft.is_some(), "the draft should load from the temp fixture");

        for typed in ["1", "12", "123"] {
            let _ = state.update(Message::Changed(index, typed.to_owned()), &vfs);
        }

        let untouched = std::fs::read(&path).expect("fixture should still be readable");
        assert_eq!(
            untouched, seeded,
            "no keystroke may block `update` on disk I/O. The write only happens once the async task the first keystroke started is driven to completion",
        );

        let draft = state.draft.as_ref().expect("the draft survives typing");
        assert!(draft.writing, "the first keystroke must have started the async write immediately, no delay");
        assert!(draft.dirty, "later keystrokes coalesce behind the in-flight write instead of starting a second one");
        assert_eq!(draft.input(index), "123");

        complete_write(&mut state, &vfs);

        let body = std::fs::read_to_string(&path).expect("the completed write should have landed on disk");
        assert!(body.contains("123"), "the completion must persist the latest coalesced value");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn state_flush_resolves_a_buffered_field_and_starts_the_write_without_any_message_at_all() {
        let dir = scratch_dir("flush-on-navigate");
        let path = dir.join("unitbuy.csv");
        let schema = schema::of(Subject::Buy);
        let Some(index) = schema.index_of("stage_unlock_requirement") else {
            panic!("nyanko no longer publishes stage_unlock_requirement");
        };

        let seed = plan(Subject::Buy, Address::Line(0), "test".to_owned(), &path, None, EditorMode::Resolved);
        std::fs::write(&path, format!("{}\n", super::vacant(&seed, &[], ',')))
            .expect("failed to seed the temp fixture file");

        let vfs = Vfs::with_priority(&[]);
        let mut state = State { draft: Draft::load(seed, &vfs), ..State::default() };
        assert!(state.draft.is_some(), "the draft should load from the temp fixture");

        let _ = state.update(Message::Changed(index, "!7".to_owned()), &vfs);
        assert_eq!(state.draft.as_ref().and_then(|draft| draft.buffer), Some(index));

        // No `Message` at all here. This is what page navigation calls directly, ahead of
        // switching `current_page`, since a page change never routes through `figures::Message`.
        let _ = state.flush(&vfs);

        let draft = state.draft.as_ref().expect("the draft survives a flush with no message");
        assert_eq!(draft.buffer, None, "flush() alone must resolve the buffered field");
        assert_eq!(draft.input(index), "7");
        assert!(draft.writing, "flush() must have started the write, not just staged it");
        assert!(!draft.failed);

        complete_write(&mut state, &vfs);

        let draft = state.draft.as_ref().expect("the draft survives the completed write");
        assert!(!draft.failed, "the flush should have committed cleanly to the temp file");
        assert!(!draft.writing && !draft.dirty, "nothing left pending once the write completes");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn state_update_resolves_a_buffered_field_once_a_different_message_arrives() {
        let dir = scratch_dir("flush-on-deselect");
        let path = dir.join("unitbuy.csv");
        let schema = schema::of(Subject::Buy);
        let Some(index) = schema.index_of("stage_unlock_requirement") else {
            panic!("nyanko no longer publishes stage_unlock_requirement");
        };

        let seed = plan(Subject::Buy, Address::Line(0), "test".to_owned(), &path, None, EditorMode::Resolved);
        std::fs::write(&path, format!("{}
", super::vacant(&seed, &[], ',')))
            .expect("failed to seed the temp fixture file");

        let vfs = Vfs::with_priority(&[]);
        let mut state = State { draft: Draft::load(seed, &vfs), ..State::default() };
        assert!(state.draft.is_some(), "the draft should load from the temp fixture");

        let _ = state.update(Message::Changed(index, "!12".to_owned()), &vfs);
        assert_eq!(state.draft.as_ref().and_then(|draft| draft.buffer), Some(index));

        let _ = state.update(Message::Changed(index, "!123".to_owned()), &vfs);
        assert_eq!(
            state.draft.as_ref().map(|draft| draft.input(index)), Some("!123"),
            "typing on into the same buffered field must not flush it",
        );

        let _ = state.update(Message::Scrolled(4.0), &vfs);

        let draft = state.draft.as_ref().expect("the draft survives an unrelated message");
        assert_eq!(draft.buffer, None, "any other message flushes the buffered field");
        assert_eq!(draft.input(index), "123", "the bang is gone and the value is written plain");
        assert!(draft.writing, "the resolved value's write starts immediately, no debounce");
        assert!(!draft.failed);

        std::fs::remove_dir_all(&dir).ok();
    }

    fn complete_write(state: &mut State, vfs: &Vfs) {
        let draft = state.draft.as_mut().expect("a write can only complete on a live draft");
        let (path, body, stamp, token) = draft.prepare_write(vfs).expect("destination must resolve in these fixtures");

        // Dropping the returned `Task` cancels the real write, but `blocking::unblock` may have
        // already run it on the pool. Either way the file ends up holding this body.
        let result = write_now(&path, &body, stamp)
            .or_else(|| preview::stamp(&path).and_then(|fresh| write_now(&path, &body, fresh)));

        let _ = state.update(Message::Persisted(token, path, result), vfs);
    }

    fn scratch_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("bcc-figures-test-{label}-{}", std::process::id()));

        std::fs::create_dir_all(&dir).expect("failed to create the temp fixture dir");

        dir
    }

    fn bare_draft(subject: Subject, index: usize, seed: i32) -> Draft {
        let schema = schema::of(subject);
        let mut cells = vec![0; schema.known()];
        cells[index] = seed;

        let rules: Vec<Rule> = (0..cells.len())
            .map(|position| resolved::rule(subject, position, schema.field(position), &cells))
            .collect();

        draft_with(subject, cells, rules)
    }

    fn draft_with(subject: Subject, cells: Vec<i32>, rules: Vec<Rule>) -> Draft {
        let width = cells.len();
        let plan = plan(subject, Address::Line(0), "test".to_owned(), Path::new("test.csv"), None, EditorMode::Resolved);

        Draft {
            plan,
            chrome: Vec::new(),
            absent: false,
            row: 0,
            read_from: std::path::PathBuf::new(),
            stamp: Stamp::default(),
            delimiter: ',',
            keys: Vec::new(),
            keyed: None,
            lines: Vec::new(),
            cells,
            writing: false,
            token: 0,
            written: vec![String::new(); width],
            stored: width,
            touched: 0,
            rules,
            comment: String::new(),
            inputs: vec![String::new(); width],
            failed: false,
            buffer: None,
            dirty: false,
        }
    }
}

#[cfg(test)]
mod append_tests {
    use std::fs;

    use kore::domains::settings::EditorMode;
    use kore::Vfs;

    use super::schema::{self, Subject};
    use super::{plan, Address, Draft};

    // The empty entry is the only way to make a combo now, so editing it has to write the
    // row, give it a live key and re-address the popup onto it without a further click.
    #[test]
    fn editing_the_empty_entry_materialises_a_combo_and_selects_it() {
        let root = std::env::temp_dir().join(format!("bcc-combo-new-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("scratch");
        let file = root.join("NyancomboData.csv");

        fs::write(&file, "3066,6,-1,781,1,-1,-1,-1,-1,-1,-1,-1,-1,7,1,-1\n").expect("seed");

        let vfs = Vfs::with_priority(&[String::new()]);
        let made = plan(Subject::Combo, Address::Appended, "t".to_owned(), &file, None, EditorMode::Resolved)
            .anchored(Some((44, 0)));

        let mut state = super::State::default();
        state.begin(made, 0, &vfs);

        let schema = schema::of(Subject::Combo);
        let anchored = schema.index_of("slot_1_unit_id").expect("published slot column");
        let second = schema.index_of("slot_2_unit_id").expect("published slot column");

        let draft = state.draft.as_ref().expect("the blank combo opens");
        assert!(draft.absent(), "nothing is written until something is edited");
        assert_eq!(draft.reads_at(anchored), Some(44), "the blank opens holding its own unit");
        assert_eq!(fs::read_to_string(&file).expect("read").lines().count(), 1, "and wrote nothing");

        let _ = state.update(super::Message::Slotted(second, 90, 2), &vfs);

        assert_eq!(state.draft.as_ref().map(super::Draft::row), Some(1), "the popup moved onto it");
        assert_eq!(state.draft.as_ref().map(super::Draft::absent), Some(false), "and it is real now");
        assert_eq!(state.pinned, Some(Address::Line(1)), "so a refresh cannot rebase it");

        state.flush_now(&vfs);

        let written = fs::read_to_string(&file).expect("read back");
        let lines: Vec<&str> = written.lines().collect();

        assert_eq!(lines.len(), 2, "the edit materialised the row");
        assert!(lines[1].starts_with("3067,6,"), "with the next key in its series: {:?}", lines[1]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn removing_a_combo_returns_the_popup_to_the_empty_entry() {
        let root = std::env::temp_dir().join(format!("bcc-combo-back-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("scratch");
        let file = root.join("NyancomboData.csv");

        fs::write(&file, "0,1,-1,44,0,-1,-1,-1,-1,-1,-1,-1,-1,5,0,-1\n1,1,-1,90,2,-1,-1,-1,-1,-1,-1,-1,-1,7,1,-1\n")
            .expect("seed");

        let vfs = Vfs::with_priority(&[String::new()]);
        let made = plan(Subject::Combo, Address::Line(0), "t".to_owned(), &file, None, EditorMode::Resolved)
            .anchored(Some((44, 0)));

        let mut state = super::State::default();
        state.begin(made, 0, &vfs);

        let _ = state.update(super::Message::Removed, &vfs);
        assert!(!state.draft.as_ref().is_some_and(super::Draft::absent), "the first press only arms it");

        let _ = state.update(super::Message::Removed, &vfs);

        let draft = state.draft.as_ref().expect("the popup stays open");
        assert!(draft.absent(), "and lands back on the empty entry rather than closing");
        assert_eq!(state.pinned, None, "an unwritten combo pins nothing");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn switching_unit_or_form_retargets_the_combo_popup_instead_of_holding_its_row() {
        let root = std::env::temp_dir().join(format!("bcc-combo-anchor-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("scratch");
        let file = root.join("NyancomboData.csv");

        fs::write(&file, "0,1,-1,44,0,-1,-1,-1,-1,-1,-1,-1,-1,5,0,-1\n1,1,-1,90,2,-1,-1,-1,-1,-1,-1,-1,-1,7,1,-1\n")
            .expect("seed");

        let vfs = Vfs::with_priority(&[String::new()]);
        let opened = plan(Subject::Combo, Address::Line(0), "t".to_owned(), &file, None, EditorMode::Resolved)
            .anchored(Some((44, 0)));

        let mut state = super::State::default();
        state.begin(opened, 0, &vfs);
        assert_eq!(state.pinned, Some(Address::Line(0)), "the popup pins the combo it opened on");

        let same = plan(Subject::Combo, Address::Line(1), "t".to_owned(), &file, None, EditorMode::Resolved)
            .anchored(Some((44, 0)));
        state.sync(Some(same), &vfs);
        assert_eq!(
            state.draft.as_ref().map(super::Draft::row),
            Some(0),
            "the same unit and form must not drag the popup off the combo being edited",
        );

        let moved = plan(Subject::Combo, Address::Line(1), "t".to_owned(), &file, None, EditorMode::Resolved)
            .anchored(Some((90, 2)));
        state.sync(Some(moved), &vfs);
        assert_eq!(
            state.draft.as_ref().map(super::Draft::row),
            Some(1),
            "a different unit or form re-targets the popup onto that unit's combo",
        );

        let _ = fs::remove_dir_all(&root);
    }

    // The shipped file keeps 126 superseded rows rather than deleting them, because the
    // localized name tables are addressed by line. Removing one must do the same.
    #[test]
    fn removing_a_combo_mid_file_retires_it_instead_of_shifting_every_name_below() {
        let root = std::env::temp_dir().join(format!("bcc-combo-remove-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("scratch");
        let file = root.join("NyancomboData.csv");

        let seeded = "0,1,-1,44,0,-1,-1,-1,-1,-1,-1,-1,-1,5,0,-1\n1,1,-1,90,2,-1,-1,-1,-1,-1,-1,-1,-1,7,1,-1\n";
        fs::write(&file, seeded).expect("seed");

        let vfs = Vfs::with_priority(&[String::new()]);
        let made = plan(Subject::Combo, Address::Line(0), "t".to_owned(), &file, None, EditorMode::Resolved);
        let mut draft = Draft::load(made, &vfs).expect("the seeded combo loads");

        assert!(draft.remove(), "a written combo can be removed");
        draft.persist_now(&vfs);

        let written = fs::read_to_string(&file).expect("read back");
        let lines: Vec<&str> = written.lines().collect();

        assert_eq!(lines.len(), 2, "the line stays so every name below keeps its address");
        assert!(lines[0].starts_with("0,-1,"), "and is retired by its series: {:?}", lines[0]);
        assert!(lines[1].starts_with("1,1,"), "the combo below is untouched: {:?}", lines[1]);

        let last = plan(Subject::Combo, Address::Line(1), "t".to_owned(), &file, None, EditorMode::Resolved);
        let mut tail = Draft::load(last, &vfs).expect("the last combo loads");

        assert!(tail.remove(), "the last row can go outright");
        tail.persist_now(&vfs);

        let after = fs::read_to_string(&file).expect("read back");
        assert_eq!(after.lines().count(), 1, "nothing below it, so nothing to keep aligned");

        let _ = fs::remove_dir_all(&root);
    }

}
