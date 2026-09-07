use super::*;
use iced::widget::{column, container, Column, Space};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Seam {
    Fresh,
    Spoken,
    Crowned,
    Moved,
}

impl Seam {
    pub(super) fn tinted(self) -> bool {
        self != Self::Moved
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shelf {
    FreshData,
    FreshArt,
    MovedData,
    MovedArt,
}

pub(super) struct Named {
    name: String,
    width: f32,
}

pub(super) struct Drawn {
    name: String,
    path: PathBuf,
    width: f32,
}

#[derive(Default)]
pub(super) struct Ready {
    pub(super) fresh: Vec<u32>,
    pub(super) spoken: Vec<usize>,
    pub(super) changed: Vec<(usize, forms::Diff)>,
    pub(super) unlocked: Vec<(usize, forms::Diff)>,
    pub(super) talents: Vec<usize>,
    pub(super) raised: Vec<usize>,
    pub(super) art: HashMap<(u32, usize), PathBuf>,
    pub(super) foe_art: HashMap<u32, PathBuf>,
    pub(super) foes_new: Vec<u32>,
    pub(super) foes_changed: Vec<(usize, forms::Diff)>,
    pub(super) foes_spoken: Vec<usize>,
}

#[derive(Default)]
pub(super) struct Terrain {
    pub(super) fresh: Grouped,
    pub(super) spoken: Grouped,
    pub(super) tongues: HashMap<GlobalMapId, Vec<String>>,
    pub(super) banners: HashMap<GlobalMapId, PathBuf>,
    pub(super) plates: HashMap<GlobalStageId, PathBuf>,
    pub(super) opened: Grouped,
    pub(super) added: Grouped,
    pub(super) moved: Grouped,
    pub(super) crowned: Grouped,
    pub(super) moves: HashMap<GlobalMapId, (u8, u8)>,
}

#[derive(Default)]
pub(super) struct Shelves {
    pub(super) fresh_data: Vec<Named>,
    pub(super) fresh_art: Vec<Drawn>,
    pub(super) moved_data: Vec<Named>,
    pub(super) moved_art: Vec<Drawn>,
}

impl Shelves {
    pub(super) fn is_empty(&self) -> bool {
        self.fresh_data.is_empty()
            && self.fresh_art.is_empty()
            && self.moved_data.is_empty()
            && self.moved_art.is_empty()
    }
}

pub(super) fn shelve(ore: &Diff, vfs: &Vfs) -> Shelves {
    let mut shelves = Shelves::default();
    let mut ruler = Ruler::new(font::Font { weight: font::Weight::Bold, ..font::Font::DEFAULT }, FILE_NAME_SIZE);

    let resolved: Vec<Option<PathBuf>> = ore
        .touched
        .par_iter()
        .map(|held| pictured(&held.file).then(|| vfs.pristine(&held.file)).flatten())
        .collect();

    for (held, art) in ore.touched.iter().zip(resolved) {
        let width = ruler.width(&held.file);

        match (held.status, art) {
            (Status::Baseline, Some(path)) => {
                shelves.fresh_art.push(Drawn { name: held.file.clone(), path, width });
            }
            (Status::Baseline, None) => shelves.fresh_data.push(Named { name: held.file.clone(), width }),
            (Status::Changed, Some(path)) => {
                shelves.moved_art.push(Drawn { name: held.file.clone(), path, width });
            }
            (Status::Changed, None) => shelves.moved_data.push(Named { name: held.file.clone(), width }),
        }
    }

    shelves.fresh_data.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    shelves.moved_data.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    shelves.fresh_art.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    shelves.moved_art.sort_unstable_by(|left, right| left.name.cmp(&right.name));

    shelves
}

fn cell_width(available: f32, columns: usize) -> f32 {
    let usable = (available - PAGE_PADDING * 2.0 - SCROLLBAR_RESERVE).max(0.0);

    ((usable - CARD_SPACING * (columns - 1) as f32) / columns as f32).max(1.0)
}

fn caption(widest: f32, room: f32) -> f32 {
    let usable = (room - FILE_NAME_SIZE).max(1.0);

    (widest / usable).ceil().max(1.0) * NAME_LINE
}

#[derive(Clone, Copy)]
pub(super) enum Slab {
    Section(&'static str, usize),
    Group(&'static str),
    Data(Shelf, usize, f32),
    Art(Shelf, usize, f32),
}

impl Slab {
    fn height(self) -> f32 {
        match self {
            Self::Section(..) => SECTION_SLAB,
            Self::Group(_) => GROUP_SLAB,
            Self::Data(_, _, cell) | Self::Art(_, _, cell) => cell + CARD_SPACING,
        }
    }
}

pub(super) struct Pane {
    range: Range<usize>,
    before: f32,
    after: f32,
}

fn pane(slabs: &[Slab], offset: f32, viewport: f32) -> Pane {
    let seen = (offset - PAGE_PADDING).max(0.0);
    let top = (seen - VIRTUAL_BUFFER).max(0.0);
    let bottom = seen + viewport + VIRTUAL_BUFFER;

    let mut start = 0;
    let mut before = 0.0;
    let mut cursor = 0.0;
    let mut found = false;

    for (index, slab) in slabs.iter().enumerate() {
        if found && cursor >= bottom {
            return Pane {
                range: start..index,
                before,
                after: slabs[index..].iter().map(|slab| slab.height()).sum(),
            };
        }

        let height = slab.height();

        if !found && cursor + height > top {
            found = true;
            start = index;
            before = cursor;
        }

        cursor += height;
    }

    Pane { range: start..slabs.len(), before, after: 0.0 }
}

impl State {
    fn art_of(&self, shelf: Shelf) -> &[Drawn] {
        match shelf {
            Shelf::FreshArt => &self.files.fresh_art,
            Shelf::MovedArt => &self.files.moved_art,
            Shelf::FreshData | Shelf::MovedData => &[],
        }
    }

    fn data_of(&self, shelf: Shelf) -> &[Named] {
        match shelf {
            Shelf::FreshData => &self.files.fresh_data,
            Shelf::MovedData => &self.files.moved_data,
            Shelf::FreshArt | Shelf::MovedArt => &[],
        }
    }

    fn slabs(&self, width: f32) -> Vec<Slab> {
        let data_columns = cards_per_row(width, NAME_MIN_WIDTH);
        let art_columns = cards_per_row(width, ART_MIN_WIDTH);

        let data_room = cell_width(width, data_columns) - BOX_PADDING * 2.0;
        let art_room = cell_width(width, art_columns) - CARD_PADDING * 2.0;

        let mut slabs = Vec::new();

        for (title, data, art) in [
            ("New", Shelf::FreshData, Shelf::FreshArt),
            ("Changed", Shelf::MovedData, Shelf::MovedArt),
        ] {
            let named = self.data_of(data);
            let drawn = self.art_of(art);

            if named.is_empty() && drawn.is_empty() {
                continue;
            }

            slabs.push(Slab::Section(title, named.len() + drawn.len()));

            if self.folded(Tab::Files, title, named.len() + drawn.len()) {
                continue;
            }

            if !named.is_empty() {
                slabs.push(Slab::Group("Data"));
                slabs.extend((0..named.len().div_ceil(data_columns)).map(|row| {
                    let widest = widest(band(named, row, data_columns).iter().map(|held| held.width));

                    Slab::Data(data, row, caption(widest, data_room) + BOX_PADDING * 2.0)
                }));
            }

            if !drawn.is_empty() {
                slabs.push(Slab::Group("Images"));
                slabs.extend((0..drawn.len().div_ceil(art_columns)).map(|row| {
                    let widest = widest(band(drawn, row, art_columns).iter().map(|held| held.width));
                    let cell = CARD_PADDING * 2.0 + ART_TILE_SIZE + CARD_SPACING + caption(widest, art_room);

                    Slab::Art(art, row, cell)
                }));
            }
        }

        slabs
    }

    fn visible_art(&self, window: Size) -> Vec<PathBuf> {
        let width = window.width - SIDEBAR_WIDTH;
        let columns = cards_per_row(width, ART_MIN_WIDTH);
        let slabs = self.slabs(width);
        let pane = pane(&slabs, self.offsets.get(&self.tab).copied().unwrap_or_default(), window.height);

        slabs[pane.range]
            .iter()
            .filter_map(|slab| match slab {
                Slab::Art(shelf, row, _) => Some(band(self.art_of(*shelf), *row, columns)),
                _ => None,
            })
            .flatten()
            .map(|held| held.path.to_path_buf())
            .collect()
    }

    pub(super) fn ensure_tiles(&mut self, window: Size) -> Task<Message> {
        if self.tab != Tab::Files || self.decoding {
            return Task::none();
        }

        let visible = self.visible_art(window);

        if self.tiles.len() > TILE_BUDGET {
            let keep: HashSet<PathBuf> = visible.iter().cloned().collect();

            self.tiles.retain(|path, _| keep.contains(path));
        }

        let wanted: Vec<PathBuf> =
            visible.into_iter().filter(|path| !self.tiles.contains_key(path)).collect();

        if wanted.is_empty() {
            return Task::none();
        }

        for path in &wanted {
            self.tiles.insert(path.to_path_buf(), None);
        }

        self.decoding = true;

        let generation = self.sheet_generation;
        let (tx, rx) = unbounded();

        thread::spawn(move || {
            let loaded = wanted
                .into_iter()
                .map(|path| {
                    let handle = item_icon::load_boxed(&path, ART_CANVAS);

                    (path, handle)
                })
                .collect();

            let _ = tx.unbounded_send(Message::TilesLoaded(generation, loaded));
        });

        Task::stream(rx)
    }

    pub(super) fn view_files(&self, window: Size) -> Element<'_, Message> {
        let width = window.width - SIDEBAR_WIDTH;
        let slabs = self.slabs(width);
        let pane = pane(&slabs, self.offsets.get(&self.tab).copied().unwrap_or_default(), window.height);

        let data_columns = cards_per_row(width, NAME_MIN_WIDTH);
        let art_columns = cards_per_row(width, ART_MIN_WIDTH);

        let mut body = Column::with_capacity(pane.range.len() + 2).width(Length::Fill);

        body = body.push(Space::new().height(pane.before));

        for slab in &slabs[pane.range] {
            body = body.push(self.view_slab(*slab, data_columns, art_columns));
        }

        body.push(Space::new().height(pane.after)).into()
    }

    fn view_slab(&self, slab: Slab, data_columns: usize, art_columns: usize) -> Element<'_, Message> {
        let content: Element<'_, Message> = match slab {
            Slab::Section(title, count) => column![
                Space::new().height(SECTION_SPACING),
                self.view_fold(Tab::Files, title, count, Horizontal::Left, || Space::new().into()),
            ]
            .width(Length::Fill)
            .into(),
            Slab::Group(title) => strong(title, GROUP_TITLE_SIZE).into(),
            Slab::Data(shelf, row, cell) => {
                let cells = band(self.data_of(shelf), row, data_columns)
                    .iter()
                    .map(|held| name_cell(&held.name, cell))
                    .collect();

                banded(cells, data_columns)
            }
            Slab::Art(shelf, row, cell) => {
                let cells = band(self.art_of(shelf), row, art_columns)
                    .iter()
                    .map(|held| self.view_art(&held.name, &held.path, cell))
                    .collect();

                banded(cells, art_columns)
            }
        };

        container(content).height(Length::Fixed(slab.height())).width(Length::Fill).into()
    }

    fn view_art<'a>(&self, name: &str, path: &Path, cell: f32) -> Element<'a, Message> {
        let body: Element<'a, Message> = match self.tiles.get(path).cloned().flatten() {
            Some(handle) => iced_image(handle)
                .width(Length::Fixed(ART_TILE_SIZE))
                .height(Length::Fixed(ART_TILE_SIZE))
                .content_fit(ContentFit::Contain)
                .into(),
            None => Space::new().width(Length::Fixed(ART_TILE_SIZE)).height(Length::Fixed(ART_TILE_SIZE)).into(),
        };

        let card = column![
            container(body)
                .width(Length::Fill)
                .height(Length::Fixed(ART_TILE_SIZE))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            file_name(name).align_x(Horizontal::Center).width(Length::Fill),
        ]
        .spacing(CARD_SPACING)
        .align_x(Horizontal::Center)
        .width(Length::Fill);

        open_file(
            container(card)
                .padding(CARD_PADDING)
                .width(Length::Fill)
                .height(Length::Fixed(cell))
                .style(theme::card_container),
            name,
        )
    }
}
