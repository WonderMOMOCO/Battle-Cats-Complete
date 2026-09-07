use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use iced::alignment::Vertical;
use iced::widget::{container, operation, responsive, row, scrollable, space, text, Column};
use iced::{widget, Element, Font, Length, Padding, Size, Task};
use rustc_hash::{FxHashMap, FxHashSet};

use kore::Vfs;

use crate::app::theme;
use crate::common::{fonts, glyphs};
use crate::common::row_window::{self, RowWindow};
use crate::editor;
use crate::widget::{branches, list_row, open_mark, smooth_scroll, Guide, Tracer};

use super::{both_ways, Mode, EMPTY_TEXT_SIZE, SCROLLBAR_ALLOWANCE, TEXT_SIZE};

const CHAR_WIDTH: f32 = TEXT_SIZE * 0.65;
const ROW_HEIGHT: f32 = 24.0;
const ROW_SPACING: f32 = 0.0;
const ROW_PADDING: f32 = 6.0;
const INDENT: f32 = 12.0;

const MARKER_SIZE: f32 = ROW_HEIGHT * fonts::TRIANGLE_ROW;
const MARKER_LINE_HEIGHT: f32 = ROW_HEIGHT / MARKER_SIZE;
const MARKER_WIDTH: f32 = 16.0;

const SCROLL_TAIL: f32 = 14.0;

const FOLDER_OPEN: &str = "\u{25be}";
const FOLDER_SHUT: &str = "\u{25b8}";

#[derive(Debug, Clone)]
pub enum Message {
    Activate(usize),
    Scrolled(f32),
}

struct Row {
    name: Box<str>,
    depth: u16,
    folder: bool,
    expanded: bool,
    guide: Guide,
}

type PackGroup = (Box<str>, Vec<Box<str>>);

pub(super) struct State {
    expanded: FxHashSet<PathBuf>,
    flat_keys: Vec<Box<str>>,
    pack_groups: Vec<PackGroup>,
    pack_roots: Vec<Box<str>>,
    pack_dirty: bool,
    pack_count: usize,
    rows: Vec<Row>,
    selected_row: Option<usize>,
    has_folders: bool,
    widest: f32,
    scroll_offset: f32,
    scroll_id: widget::Id,
}

impl Default for State {
    fn default() -> Self {
        Self {
            expanded: FxHashSet::from_iter([PathBuf::new()]),
            flat_keys: Vec::new(),
            pack_groups: Vec::new(),
            pack_roots: Vec::new(),
            pack_dirty: true,
            pack_count: 0,
            rows: Vec::new(),
            selected_row: None,
            has_folders: false,
            widest: 0.0,
            scroll_offset: 0.0,
            scroll_id: widget::Id::unique(),
        }
    }
}

impl State {
    pub(super) fn update(&mut self, message: Message) -> Option<usize> {
        match message {
            Message::Activate(index) => Some(index),
            Message::Scrolled(offset) => {
                self.scroll_offset = offset;
                None
            }
        }
    }

    pub(super) fn reset(&mut self) {
        self.expanded.clear();
        self.expanded.insert(PathBuf::new());
        self.invalidate_packs();
        self.rewind();
    }

    pub(super) fn invalidate_packs(&mut self) {
        self.pack_dirty = true;
    }

    pub(super) fn prime_packs(&mut self, vfs: &Vfs, mount: Option<&str>, packs: &FxHashMap<String, String>) {
        if let Some(mount) = mount {
            self.group_packs(vfs, mount, packs);
        }
    }

    pub(super) fn rewind(&mut self) {
        self.scroll_offset = 0.0;
    }

    pub(super) fn clear(&mut self) {
        self.rows.clear();
        self.selected_row = None;
        self.widest = 0.0;
        self.has_folders = false;
    }

    pub(super) fn reveal(&mut self, path: &Path) {
        let mut current = path.parent();

        while let Some(dir) = current {
            self.expanded.insert(dir.to_path_buf());
            current = dir.parent();
        }
    }

    pub(super) fn open(&mut self, path: PathBuf) {
        self.expanded.insert(path);
    }

    pub(super) fn toggle(&mut self, path: PathBuf) {
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
        }
    }

    pub(super) fn center(&mut self) -> Task<Message> {
        let Some(index) = self.selected_row else {
            return Task::none();
        };

        self.scroll_offset = index as f32 * (ROW_HEIGHT + ROW_SPACING);

        operation::scroll_to(
            self.scroll_id.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: self.scroll_offset },
        )
    }

    pub(super) fn snap_to_top(&self) -> Task<Message> {
        operation::scroll_to(self.scroll_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y: 0.0 })
    }

    pub(super) fn refresh_keys(&mut self, vfs: &Vfs, mount: Option<&str>, mode: Mode, packs: &FxHashMap<String, String>) {
        self.flat_keys.clear();

        let Some(mount) = mount else {
            return;
        };

        match mode {
            Mode::Tree => {}
            Mode::Flat => {
                self.flat_keys = vfs.keys(mount);
                self.flat_keys.sort_unstable();
            }
            Mode::Pack => self.group_packs(vfs, mount, packs),
        }
    }

    fn group_packs(&mut self, vfs: &Vfs, mount: &str, packs: &FxHashMap<String, String>) {
        let count = vfs.count(mount);

        if !self.pack_dirty && self.pack_count == count {
            return;
        }

        let mut grouped: BTreeMap<&str, Vec<Box<str>>> = BTreeMap::new();
        let mut roots: Vec<Box<str>> = Vec::new();

        for name in vfs.keys(mount) {
            match packs.get(name.as_ref()) {
                Some(pack) if !pack.is_empty() => match grouped.get_mut(pack.as_str()) {
                    Some(bucket) => bucket.push(name),
                    None => {
                        grouped.insert(pack.as_str(), vec![name]);
                    }
                },
                _ => roots.push(name),
            }
        }

        roots.sort_unstable();
        self.pack_roots = roots;

        self.pack_groups = grouped
            .into_iter()
            .map(|(pack, mut files)| {
                files.sort_unstable();
                (pack.into(), files)
            })
            .collect();

        self.pack_count = count;
        self.pack_dirty = false;
    }

    pub(super) fn entry(&self, vfs: &Vfs, mount: &str, mode: Mode, index: usize) -> Option<(bool, PathBuf)> {
        let target = self.rows.get(index)?;

        match mode {
            Mode::Tree => self.path_of(index).map(|path| (target.folder, path)),
            Mode::Pack if target.folder => Some((true, PathBuf::from(target.name.as_ref()))),
            Mode::Flat | Mode::Pack => vfs.stored(mount, target.name.as_ref()).map(|path| (false, path)),
        }
    }

    fn path_of(&self, index: usize) -> Option<PathBuf> {
        let target = self.rows.get(index)?;
        let mut parts: Vec<&str> = vec![target.name.as_ref()];
        let mut wanted = target.depth;

        for row in self.rows[..index].iter().rev() {
            if wanted == 0 {
                break;
            }

            if row.depth == wanted - 1 {
                wanted -= 1;
                parts.push(row.name.as_ref());
            }
        }

        let mut path = PathBuf::new();

        for part in parts.iter().rev() {
            path.push(part);
        }

        Some(path)
    }

    pub(super) fn rebuild(&mut self, vfs: &Vfs, mount: &str, mode: Mode, query: &str, selected: Option<&Path>) -> bool {
        self.clear();

        let anchor = selected.and_then(|path| Some((path.parent()?, path.file_name()?.to_str()?)));

        let mut flatten = Flatten {
            vfs,
            mount,
            expanded: &self.expanded,
            anchor,
            query: query.trim(),
            flat_keys: &self.flat_keys,
            pack_groups: &self.pack_groups,
            pack_roots: &self.pack_roots,
            rows: Vec::new(),
            selected_row: None,
            folders: false,
            widest: 0.0,
        };

        match mode {
            Mode::Tree => flatten.walk(Path::new(""), 0),
            Mode::Flat => flatten.flat(),
            Mode::Pack => flatten.packs(),
        }

        self.has_folders = flatten.folders;
        self.widest = flatten.widest + if flatten.folders { MARKER_WIDTH } else { 0.0 };
        self.rows = flatten.rows;
        self.selected_row = flatten.selected_row;

        let mut tracer = Tracer::default();

        for index in (0..self.rows.len()).rev() {
            let depth = self.rows[index].depth;
            let above = index.checked_sub(1).map(|prev| self.rows[prev].depth);

            self.rows[index].guide = tracer.back(depth, above);
        }

        !self.rows.is_empty()
    }

    pub(super) fn view(&self, empty: Option<&'static str>) -> Element<'_, Message> {
        let body: Element<'_, Message> = empty.map_or_else(
            || responsive(move |size: Size| self.view_rows(size)).into(),
            |label| {
                container(theme::centered_text(label).size(EMPTY_TEXT_SIZE).style(text::danger))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            },
        );

        container(container(body).padding(theme::CONSOLE_BORDER_WIDTH))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::mock_console_container)
            .into()
    }

    fn view_rows(&self, size: Size) -> Element<'_, Message> {
        let tail = if self.widest > size.width { SCROLL_TAIL } else { 0.0 };

        let RowWindow { range, pad_before, pad_after } =
            row_window::compute_with(self.rows.len(), size.height - tail, self.scroll_offset, ROW_HEIGHT, ROW_SPACING);

        let width = self.widest.max(size.width - SCROLLBAR_ALLOWANCE);
        let font = glyphs::mono();
        let mut list = Column::with_capacity(range.len() + 3).spacing(ROW_SPACING);

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for index in range {
            let Some(row) = self.rows.get(index) else {
                continue;
            };

            list = list.push(self.view_row(index, row, width, font));
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        if tail > 0.0 {
            list = list.push(space().height(Length::Fixed(tail)));
        }

        smooth_scroll(
            scrollable(list)
                .id(self.scroll_id.clone())
                .direction(both_ways())
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        )
            .into()
    }

    fn view_row<'a>(&self, index: usize, row: &'a Row, width: f32, font: Font) -> Element<'a, Message> {
        let name = text(row.name.as_ref()).font(font).size(TEXT_SIZE).wrapping(text::Wrapping::None);

        let label = if self.has_folders {
            let stem = branches(row.guide, row.depth, INDENT, ROW_HEIGHT, MARKER_SIZE);

            match (row.folder, row.expanded) {
                (true, true) => row![stem.opened(), marker(true), name],
                (true, false) => row![stem, marker(false), name],
                (false, _) => row![stem.reach(MARKER_WIDTH), name],
            }
        } else {
            row![name]
        };

        let content = container(label.align_y(Vertical::Center))
            .height(Length::Fixed(ROW_HEIGHT))
            .align_y(Vertical::Center)
            .padding(Padding::default().left(ROW_PADDING).right(ROW_PADDING));

        editor::target(
            list_row(content, self.selected_row == Some(index), false, Length::Fixed(width), Message::Activate(index)),
            editor::Target::FileRow(index),
        )
    }
}

fn marker<'a>(expanded: bool) -> Element<'a, Message> {
    let glyph = text(if expanded { FOLDER_OPEN } else { FOLDER_SHUT })
        .font(fonts::MISC_SYMBOLS)
        .size(MARKER_SIZE)
        .line_height(MARKER_LINE_HEIGHT)
        .width(Length::Fixed(MARKER_WIDTH));

    match expanded {
        true => glyph.style(open_mark).into(),
        false => glyph.into(),
    }
}

struct Flatten<'a> {
    vfs: &'a Vfs,
    mount: &'a str,
    expanded: &'a FxHashSet<PathBuf>,
    anchor: Option<(&'a Path, &'a str)>,
    query: &'a str,
    flat_keys: &'a [Box<str>],
    pack_groups: &'a [PackGroup],
    pack_roots: &'a [Box<str>],
    rows: Vec<Row>,
    selected_row: Option<usize>,
    folders: bool,
    widest: f32,
}

pub(super) fn matches(name: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let (name, query) = (name.as_bytes(), query.as_bytes());

    name.len() >= query.len() && name.windows(query.len()).any(|window| window.eq_ignore_ascii_case(query))
}

impl Flatten<'_> {
    fn push(&mut self, row: Row) {
        let span = ROW_PADDING * 2.0 + INDENT * f32::from(row.depth) + CHAR_WIDTH * row.name.chars().count() as f32;

        self.widest = self.widest.max(span);
        self.rows.push(row);
    }

    fn flat(&mut self) {
        let marked = self.anchor.map(|(_, name)| name);

        for name in self.flat_keys {
            if !matches(name, self.query) {
                continue;
            }

            if marked == Some(name.as_ref()) {
                self.selected_row = Some(self.rows.len());
            }

            self.push(Row { name: name.clone(), depth: 0, folder: false, expanded: false, guide: Guide::default() });
        }
    }

    fn packs(&mut self) {
        let marked = self.anchor.map(|(_, name)| name);
        let query = self.query;

        for (pack, files) in self.pack_groups {
            if !files.iter().any(|file| matches(file, query)) {
                continue;
            }

            let open = self.expanded.contains(Path::new(pack.as_ref()));

            self.folders = true;
            self.push(Row { name: pack.clone(), depth: 0, folder: true, expanded: open, guide: Guide::default() });

            if !open {
                continue;
            }

            for file in files {
                if !matches(file, query) {
                    continue;
                }

                if marked == Some(file.as_ref()) {
                    self.selected_row = Some(self.rows.len());
                }

                self.push(Row { name: file.clone(), depth: 1, folder: false, expanded: false, guide: Guide::default() });
            }
        }

        for file in self.pack_roots {
            if !matches(file, query) {
                continue;
            }

            if marked == Some(file.as_ref()) {
                self.selected_row = Some(self.rows.len());
            }

            self.push(Row { name: file.clone(), depth: 0, folder: false, expanded: false, guide: Guide::default() });
        }
    }

    fn walk(&mut self, dir: &Path, depth: u16) {
        let Some(listing) = self.vfs.browse(self.mount, dir) else {
            return;
        };

        let marked = self.anchor.filter(|(parent, _)| *parent == dir).map(|(_, name)| name);
        let query = self.query;

        for folder in listing.folders {
            let path = dir.join(folder.as_ref());

            if !self.vfs.any(self.mount, &path, |name| matches(name, query)) {
                continue;
            }

            let open = self.expanded.contains(&path);

            self.folders = true;
            self.push(Row { name: folder, depth, folder: true, expanded: open, guide: Guide::default() });

            if open {
                self.walk(&path, depth + 1);
            }
        }

        for file in listing.files {
            if !matches(&file, self.query) {
                continue;
            }

            if marked == Some(file.as_ref()) {
                self.selected_row = Some(self.rows.len());
            }

            self.push(Row { name: file, depth, folder: false, expanded: false, guide: Guide::default() });
        }
    }
}

