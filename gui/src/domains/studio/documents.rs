use super::*;
use iced::widget::{button, column, container, pick_list, row, text, text_input};

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);

fn next_token() -> u64 {
    NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
}

pub(super) fn span_of(keys: &[Keyframe], at: usize) -> Option<(i32, i32)> {
    let here = keys.get(at)?.frame;

    let reaching = keys.get(at + 1).map_or(here, |next| next.frame);

    Some((here, reaching.max(here)))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Settled {
    Saved,
    Failed,
}

pub(super) struct Backing {
    pub(super) read_from: PathBuf,
    pub(super) stamp: Stamp,
    pub(super) seen: Stamp,
    pub(super) dirty: bool,
    pub(super) writing: bool,
    pub(super) failed: bool,
    pub(super) token: u64,
}

impl Backing {
    pub(super) fn open(source: &Path) -> Option<(Backing, Vec<u8>)> {
        let read_from = source.to_path_buf();

        let bytes = fs::read(&read_from)
            .inspect_err(|err| warn!(path = %read_from.display(), "Studio could not read the file: {}", err))
            .ok()?;

        let Some(stamp) = preview::stamp(&read_from) else {
            warn!(path = %read_from.display(), "Studio could not stamp the file");

            return None;
        };

        let backing = Backing {
            read_from,
            stamp,
            seen: stamp,
            dirty: false,
            writing: false,
            failed: false,
            token: next_token(),
        };

        Some((backing, bytes))
    }

    pub(super) fn busy(&self) -> bool {
        self.dirty || self.writing
    }

    pub(super) fn drifted(&self) -> bool {
        !self.dirty && !self.writing && self.seen != self.stamp
    }

    pub(super) fn sighted(&mut self, path: &Path, stamp: Option<Stamp>) {
        if self.read_from != path || self.dirty || self.writing {
            return;
        }

        self.seen = stamp.unwrap_or(self.stamp);
    }

    fn prepare(&mut self) -> (PathBuf, Stamp, u64) {
        self.dirty = false;
        self.writing = true;

        (self.read_from.clone(), self.stamp, self.token)
    }

    pub(super) fn settle(&mut self, path: PathBuf, stamp: Option<Stamp>) -> Settled {
        self.writing = false;

        let Some(stamp) = stamp else {
            self.failed = true;

            return Settled::Failed;
        };

        self.read_from = path;
        self.stamp = stamp;
        self.seen = stamp;
        self.failed = false;

        Settled::Saved
    }
}

pub(super) struct Draft {
    pub(super) backing: Backing,
    pub(super) doc: Maanim,
    pub(super) track: Option<usize>,
    pub(super) inputs: Vec<[String; 4]>,
    pub(super) cursors: Vec<[widget::Id; 3]>,
    pub(super) neutral: i32,
    pub(super) hint: String,
    pub(super) looping: String,
    pub(super) buffer: Option<(usize, Field)>,
    pub(super) looped: bool,
}

impl Draft {
    pub(super) fn load(anim: &Path) -> Option<Draft> {
        let (backing, bytes) = Backing::open(anim)?;

        let doc = Maanim::parse(&bytes)
            .inspect_err(|err| warn!(path = %backing.read_from.display(), "Animation editor could not parse the file: {}", err))
            .ok()?;

        let mut draft = Draft {
            backing,
            doc,
            track: None,
            inputs: Vec::new(),
            cursors: Vec::new(),
            neutral: 0,
            hint: CELL_HINT.to_string(),
            looping: String::new(),
            buffer: None,
            looped: false,
        };

        draft.restate();

        Some(draft)
    }

    pub(super) fn retrack(&mut self, at: usize) {
        if at >= self.doc.tracks().len() {
            return;
        }

        self.track = Some(at);
        self.restate();
    }

    pub(super) fn retrack_clamped(&mut self) {
        self.track = self.track.filter(|at| *at < self.doc.tracks().len());
        self.restate();
    }

    pub(super) fn curve(&self) -> Option<&nyanko::graphics::rig::AnimModification> {
        self.doc.track(self.track?)
    }

    pub(super) fn restate(&mut self) {
        let neutral = self.neutral;
        let looping = self.curve().map_or_else(String::new, |track| shown(track.loop_count, LOOP_DEFAULT));
        let track = self.curve();

        self.inputs = track
            .map(|track| {
                track
                    .keyframes
                    .iter()
                    .map(|key| {
                        let cells = [key.frame, key.value, key.ease, key.ease_power];

                        std::array::from_fn(|at| shown(cells[at], Field::ALL[at].default(neutral)))
                    })
                    .collect()
            })
            .unwrap_or_default();

        self.cursors = self
            .inputs
            .iter()
            .map(|_| std::array::from_fn(|_| widget::Id::unique()))
            .collect();
        self.looping = looping;
    }

    pub(super) fn set_looping(&mut self, value: &str) {
        if !typable(value) || self.track.is_none() {
            return;
        }

        self.looping = value.to_owned();

        if value.starts_with(BUFFER_MARK) {
            self.looped = true;

            return;
        }

        self.looped = false;

        let Some(parsed) = settled(value, LOOP_DEFAULT) else {
            return;
        };

        let Some(track) = self.track.and_then(|at| self.doc.edit(at)) else {
            return;
        };

        if track.loop_count == parsed {
            return;
        }

        track.loop_count = parsed;
        self.backing.dirty = true;
    }

    pub(super) fn set_ease(&mut self, at: usize, ease: i32) {
        let Some(track) = self.track else {
            return;
        };

        let Some(key) = self.doc.edit(track).and_then(|track| track.keyframes.get_mut(at)) else {
            return;
        };

        if key.ease == ease {
            return;
        }

        key.ease = ease;
        self.restate();
        self.backing.dirty = true;
    }

    pub(super) fn add_key(&mut self) {
        let Some(track) = self.track else {
            return;
        };

        if self.doc.add_key(track).is_none() {
            return;
        }

        self.restate();
        self.backing.dirty = true;
    }

    pub(super) fn drop_key(&mut self, at: usize) {
        let Some(track) = self.track else {
            return;
        };

        if !self.doc.remove_key(track, at) {
            return;
        }

        self.restate();
        self.backing.dirty = true;
    }

    pub(super) fn cursor(&self, at: usize, column: usize) -> widget::Id {
        self.cursors
            .get(at)
            .and_then(|row| row.get(column))
            .cloned()
            .unwrap_or_else(widget::Id::unique)
    }

    pub(super) fn key_at(&self, at: usize) -> Option<i32> {
        Some(self.curve()?.keyframes.get(at)?.frame)
    }

    pub(super) fn key_span(&self, at: usize) -> Option<(i32, i32)> {
        span_of(&self.curve()?.keyframes, at)
    }

    pub(super) fn part(&self) -> Option<i32> {
        self.curve().map(|track| track.part)
    }

    pub(super) fn buffering(&self, at: usize, field: Field) -> bool {
        self.buffer == Some((at, field))
    }

    pub(super) fn resolve_looping(&mut self) {
        if !std::mem::take(&mut self.looped) {
            return;
        }

        let Some(typed) = self.looping.strip_prefix(BUFFER_MARK).map(str::to_owned) else {
            return;
        };

        self.set_looping(&typed);
    }

    pub(super) fn resolve_buffer(&mut self) {
        let Some((at, field)) = self.buffer.take() else {
            return;
        };

        let Some(typed) = self
            .inputs
            .get(at)
            .and_then(|row| row.get(field.slot()))
            .and_then(|slot| slot.strip_prefix(BUFFER_MARK))
        else {
            return;
        };

        let typed = typed.to_owned();

        self.edit(at, field, &typed);
    }

    pub(super) fn edit(&mut self, at: usize, field: Field, value: &str) -> Option<usize> {
        if !typable(value) {
            return None;
        }

        let held = value.starts_with(BUFFER_MARK);

        let slot = self.inputs.get_mut(at).and_then(|row| row.get_mut(field.slot()))?;

        *slot = value.to_owned();

        if held {
            self.buffer = Some((at, field));

            return None;
        }

        if self.buffering(at, field) {
            self.buffer = None;
        }

        let parsed = settled(value, field.default(self.neutral))?;

        let track = self.track?;

        let key = self.doc.edit(track).and_then(|track| track.keyframes.get_mut(at))?;

        let cell = match field {
            Field::Frame => &mut key.frame,
            Field::Value => &mut key.value,
            Field::Ease => &mut key.ease,
            Field::Power => &mut key.ease_power,
        };

        if *cell == parsed {
            return None;
        }

        *cell = parsed;
        self.backing.dirty = true;

        if field != Field::Frame {
            return None;
        }

        self.reorder(track, at)
    }

    fn reorder(&mut self, track: usize, moved: usize) -> Option<usize> {
        let keys = &self.doc.track(track)?.keyframes;

        let mut order: Vec<usize> = (0..keys.len()).collect();
        order.sort_by_key(|at| keys[*at].frame);

        if order.iter().enumerate().all(|(to, from)| to == *from) {
            return None;
        }

        self.doc.sort_keys(track);
        self.inputs = order.iter().filter_map(|at| self.inputs.get(*at).cloned()).collect();
        self.cursors = order.iter().filter_map(|at| self.cursors.get(*at).cloned()).collect();

        order.iter().position(|from| *from == moved)
    }

    pub(super) fn persist_if_dirty(&mut self) -> Task<Message> {
        if !self.backing.dirty || self.backing.writing {
            return Task::none();
        }

        let (path, stamp, token) = self.backing.prepare();

        let doc = self.doc.clone();
        let reported = path.clone();

        Task::perform(
            smol::unblock(move || write_now(&path, &doc.write(), stamp)),
            move |stamp| Message::Persisted(token, reported.clone(), stamp),
        )
    }

    pub(super) fn persist_now(&mut self) {
        if !self.backing.busy() {
            return;
        }

        let (path, stamp, _) = self.backing.prepare();

        let written = write_now(&path, &self.doc.write(), stamp);

        self.backing.settle(path, written);
    }

    pub(super) fn looping(&self) -> Element<'_, Message> {
        let count = self.curve().map_or(0, |track| track.loop_count);

        let body = row![
            text("Loop").size(LABEL_SIZE),
            text_input(LOOP_HINT, &self.looping)
                .on_input(Message::LoopChanged)
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .align_x(Horizontal::Center)
                .width(Length::Fixed(LOOP_WIDTH))
                .style(theme::rounded_input),
            text(loop_label(count)).size(LABEL_SIZE),
        ]
        .spacing(ROW_GAP)
        .align_y(Vertical::Center);

        container(body).padding([LOOP_CARD_PAD, LOOP_CARD_PAD * 2.0]).style(theme::card_container_primary).into()
    }

    pub(super) fn key_row(
        &self,
        at: usize,
        span: Option<Span>,
        stripe: usize,
        armed: bool,
        width: f32,
    ) -> Element<'_, Message> {
        let cells = self.inputs.get(at).map_or(&[][..], |row| row.as_slice());
        let ease = self.curve().and_then(|track| track.keyframes.get(at)).map_or(0, |key| key.ease);

        let mut line = row![theme::centered_text(at.to_string())
            .size(LABEL_SIZE)
            .width(Length::Fixed(INDEX_WIDTH))]
        .spacing(ROW_GAP)
        .width(Length::Fill)
        .height(Length::Fixed(KEY_ROW_HEIGHT - KEY_ROW_PAD * 2.0))
        .align_y(Vertical::Center);

        for (column, field) in Field::TYPED.into_iter().enumerate() {
            let value = cells.get(field.slot()).map_or("", String::as_str);
            let live = field != Field::Power || ease_takes_power(ease);

            let hint = if field == Field::Value { self.hint.as_str() } else { CELL_HINT };
            let mut cell = text_input(hint, value)
                .id(self.cursor(at, column))
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .align_x(Horizontal::Center)
                .width(Length::Fill)
                .style(theme::rounded_input);

            if live {
                cell = cell.on_input(move |typed| Message::Changed(at, field, typed));
            }

            line = line.push(cell);
        }

        let curve = pick_list(EASES, Some(ease_label(ease)), move |label| {
            Message::EaseChanged(at, ease_value(label).unwrap_or(0))
        })
        .width(Length::Fixed(EASE_WIDTH))
        .padding([1, 4])
        .text_size(LABEL_SIZE)
        .style(theme::combo_box)
        .menu_style(theme::combo_box_menu);

        let act = |label, message| {
            button(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
                .width(Length::Fixed(STEP_WIDTH))
                .padding([0, 2])
                .on_press(message)
                .style(theme::neutral_button)
        };

        let actions =
            column![act("View", Message::Seek(at)), act("Bound", Message::Bound(at))].spacing(2.0);

        let mark = if armed { CONFIRM_MARK } else { CLOSE_LABEL };
        let drop = button(theme::centered_text(mark).size(CELL_SIZE))
            .width(Length::Fixed(DROP_WIDTH))
            .padding(0)
            .on_press(Message::DropKey(at))
            .style(theme::danger_button);

        let body = line.push(curve).push(actions).push(drop);

        let inset =
            Padding { top: KEY_ROW_PAD, right: KEY_ROW_INSET, bottom: KEY_ROW_PAD, left: KEY_ROW_INSET };

        let seated = container(body)
            .width(Length::Fixed(width))
            .padding(inset)
            .style(move |theme: &Theme| match span {
                Some(span) => container::Style {
                    background: Some(Color { a: span.tint(), ..theme.palette().primary }.into()),
                    ..container::Style::default()
                },
                None => theme::zebra_table_row(theme, stripe),
            });

        seated.into()
    }
}

pub struct Sheet {
    pub(super) backing: Backing,
    pub(super) doc: Imgcut,
    art: RgbaImage,
    pub(super) source: picture::Source,
    pub(super) outlines: Vec<picture::Outline>,
    pub(super) inputs: Vec<[String; 5]>,
    pub(super) picked: Option<usize>,
    pub(super) hidden: Option<usize>,
    buffer: Option<(usize, usize)>,
}

impl std::fmt::Debug for Sheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sheet").field("cuts", &self.doc.count()).finish()
    }
}

impl Sheet {
    pub(super) fn assemble(backing: Backing, bytes: Vec<u8>, art: Vec<u8>) -> Option<Sheet> {
        let doc = Imgcut::parse(&bytes)
            .inspect_err(|err| warn!(path = %backing.read_from.display(), "Animation editor could not parse the cut list: {}", err))
            .ok()?;

        let decoded = image::load_from_memory(&art)
            .inspect_err(|err| warn!("Animation editor could not decode the atlas: {}", err))
            .ok()?
            .to_rgba8();

        let (width, height) = (decoded.width(), decoded.height());

        let mut sheet = Sheet {
            backing,
            doc,
            art: decoded,
            source: picture::Source::new(art, width, height),
            outlines: Vec::new(),
            inputs: Vec::new(),
            picked: None,
            hidden: None,
            buffer: None,
        };

        sheet.restate();

        Some(sheet)
    }

    pub(super) fn span(&self) -> (i32, i32) {
        (self.art.width() as i32, self.art.height() as i32)
    }

    fn outside(&self, at: usize, cell: usize) -> bool {
        let (width, height) = self.span();
        let Some(cut) = self.doc.cut(at) else {
            return false;
        };

        match cell {
            0 => cut.x < 0 || cut.x > width,
            1 => cut.y < 0 || cut.y > height,
            2 => cut.width < 0 || cut.x.saturating_add(cut.width) > width,
            3 => cut.height < 0 || cut.y.saturating_add(cut.height) > height,
            _ => false,
        }
    }

    pub(super) fn opaque(&self, at: usize) -> Option<[i32; 4]> {
        let cut = self.doc.cut(at)?;
        let (span_x, span_y) = self.span();

        let left = cut.x.clamp(0, span_x);
        let top = cut.y.clamp(0, span_y);
        let right = cut.x.saturating_add(cut.width).clamp(0, span_x);
        let bottom = cut.y.saturating_add(cut.height).clamp(0, span_y);

        let raw = self.art.as_raw();
        let stride = span_x as usize * 4;
        let mut seen: Option<[i32; 4]> = None;

        for y in top..bottom {
            let row = y as usize * stride;

            for x in left..right {
                if raw.get(row + x as usize * 4 + 3).copied().unwrap_or(0) < ALPHA_FLOOR {
                    continue;
                }

                seen = Some(match seen {
                    None => [x, y, x, y],
                    Some([near_x, near_y, far_x, far_y]) => {
                        [near_x.min(x), near_y.min(y), far_x.max(x), far_y.max(y)]
                    }
                });
            }
        }

        let [near_x, near_y, far_x, far_y] = seen?;
        let region = [near_x, near_y, far_x - near_x + 1, far_y - near_y + 1];

        (region != [cut.x, cut.y, cut.width, cut.height]).then_some(region)
    }

    pub(super) fn find(&self, at: usize) -> Option<Point> {
        let cut = self.doc.cut(at)?;

        Some(Point::new(
            cut.x as f32 + cut.width as f32 / 2.0,
            cut.y as f32 + cut.height as f32 / 2.0,
        ))
    }

    pub(super) fn restate(&mut self) {
        let (picked, hidden) = (self.picked, self.hidden);

        self.outlines = (0..self.doc.count())
            .filter(|at| hidden != Some(*at))
            .filter_map(|at| Some((at, self.doc.cut(at)?)))
            .map(|(at, cut)| picture::Outline {
                x: cut.x as f32,
                y: cut.y as f32,
                width: cut.width as f32,
                height: cut.height as f32,
                bold: picked == Some(at),
            })
            .collect();

        self.inputs = (0..self.doc.count())
            .map(|at| {
                std::array::from_fn(|cell| match cell {
                    CUT_NAME_FIELD => self.doc.name(at).unwrap_or_default().to_owned(),
                    cell => self.doc.field(at, cell).unwrap_or(0).to_string(),
                })
            })
            .collect();
    }

    pub(super) fn buffering(&self, at: usize, cell: usize) -> bool {
        self.buffer == Some((at, cell))
    }

    pub(super) fn resolve_buffer(&mut self) {
        let Some((at, cell)) = self.buffer.take() else {
            return;
        };

        let Some(typed) = self
            .inputs
            .get(at)
            .and_then(|row| row.get(cell))
            .and_then(|slot| slot.strip_prefix(BUFFER_MARK))
            .map(str::to_owned)
        else {
            return;
        };

        self.edit(at, cell, &typed);
    }

    pub(super) fn edit(&mut self, at: usize, cell: usize, value: &str) {
        let named = cell == CUT_NAME_FIELD;

        if named && !authoring::nameable(value) {
            return;
        }

        if !named && !typable(value) {
            return;
        }

        let Some(slot) = self.inputs.get_mut(at).and_then(|row| row.get_mut(cell)) else {
            return;
        };

        *slot = value.to_owned();

        if value.starts_with(BUFFER_MARK) {
            self.buffer = Some((at, cell));

            return;
        }

        if self.buffering(at, cell) {
            self.buffer = None;
        }

        let changed = match named {
            true => self.doc.set_name(at, value),
            false => settled(value, 0).is_some_and(|parsed| self.doc.set_field(at, cell, parsed.max(0))),
        };

        if changed {
            self.backing.dirty = true;
            self.restate();
        }
    }

    pub(super) fn place(&mut self, at: usize, region: [i32; 4]) {
        if self.doc.place(at, region) {
            self.backing.dirty = true;
            self.restate();
        }
    }

    pub(super) fn over(&self, at: Point) -> Option<usize> {
        (0..self.doc.count()).rev().find(|held| {
            self.doc.cut(*held).is_some_and(|cut| {
                let (x, y) = (at.x.floor() as i32, at.y.floor() as i32);

                x >= cut.x && y >= cut.y && x < cut.x + cut.width && y < cut.y + cut.height
            })
        })
    }

    pub(super) fn persist_if_dirty(&mut self) -> Task<Message> {
        if !self.backing.dirty || self.backing.writing {
            return Task::none();
        }

        let (path, stamp, token) = self.backing.prepare();

        let doc = self.doc.clone();
        let reported = path.clone();

        Task::perform(
            smol::unblock(move || write_now(&path, &doc.write(), stamp)),
            move |stamp| Message::Persisted(token, reported.clone(), stamp),
        )
    }

    pub(super) fn persist_now(&mut self) -> Settled {
        if !self.backing.busy() {
            return Settled::Saved;
        }

        let (path, stamp, _) = self.backing.prepare();

        let written = write_now(&path, &self.doc.write(), stamp);

        self.backing.settle(path, written)
    }

    pub(super) fn cut_row(&self, at: usize, framing: bool, armed: bool, picked: bool, width: f32) -> Element<'_, Message> {
        let cells = self.inputs.get(at).map_or(&[][..], |row| row.as_slice());

        let mut line = row![theme::centered_text(at.to_string())
            .size(LABEL_SIZE)
            .width(Length::Fixed(INDEX_WIDTH))]
        .spacing(ROW_GAP)
        .width(Length::Fill)
        .height(Length::Fixed(KEY_ROW_HEIGHT - KEY_ROW_PAD * 2.0))
        .align_y(Vertical::Center);

        for cell in 0..CUT_FIELDS.len() {
            let named = cell == CUT_NAME_FIELD;
            let adrift = !named && self.outside(at, cell);
            let value = match framing && !named {
                true => "",
                false => cells.get(cell).map_or("", String::as_str),
            };

            let entry = text_input(if named { "" } else { CELL_HINT }, value)
                .on_input(move |typed| Message::Cut(at, cell, typed))
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .width(match named {
                    true => Length::Fill,
                    false => Length::Fixed(CUT_CELL_WIDTH),
                })
                .style(move |theme: &Theme, status| adrift_input(theme, status, adrift));

            let entry = match named {
                true => entry,
                false => entry.align_x(Horizontal::Center),
            };

            let cell: Element<'_, Message> = match adrift {
                true => tip(entry, OUT_OF_BOUNDS),
                false => entry.into(),
            };

            line = line.push(cell);
        }

        let act = |label: &'static str, message, lit: bool| {
            button(theme::centered_text(label).size(LABEL_SIZE).width(Length::Fill))
                .width(Length::Fixed(CUT_STEP_WIDTH))
                .padding([0, 2])
                .on_press(message)
                .style(move |theme: &Theme, status| match lit {
                    true => theme::toggle_button(theme, status, true),
                    false => theme::neutral_button(theme, status),
                })
        };

        let actions = column![
            row![act("Set", Message::Frame(at), framing), act("Trim", Message::Trim(at), false)]
                .spacing(2.0),
            row![act("Find", Message::Find(at), false), act("Select", Message::Slice(at), false)]
                .spacing(2.0),
        ]
        .spacing(2.0)
        .width(Length::Fixed(CUT_STEP_WIDTH * 2.0 + 2.0));

        let mark = if armed { CONFIRM_MARK } else { CLOSE_LABEL };
        let drop = button(theme::centered_text(mark).size(CELL_SIZE))
            .width(Length::Fixed(DROP_WIDTH))
            .padding(0)
            .on_press(Message::DropCut(at))
            .style(theme::danger_button);

        let inset =
            Padding { top: KEY_ROW_PAD, right: KEY_ROW_INSET, bottom: KEY_ROW_PAD, left: KEY_ROW_INSET };

        container(line.push(actions).push(drop))
            .width(Length::Fixed(width))
            .padding(inset)
            .style(move |theme: &Theme| match picked {
                true => container::Style {
                    background: Some(Color { a: ACTIVE_TINT, ..theme.palette().primary }.into()),
                    ..container::Style::default()
                },
                false => theme::zebra_table_row(theme, at),
            })
            .into()
    }
}

pub(super) struct Pose {
    pub(super) backing: Backing,
    pub(super) doc: Mamodel,
    pub(super) part: Option<usize>,
    pub(super) row: Option<usize>,
    pub(super) inputs: Vec<String>,
    pub(super) cursors: Vec<widget::Id>,
    pub(super) hints: Vec<String>,
    pub(super) cuts: usize,
    pub(super) align: [String; 2],
    pub(super) buffer: Option<Slotted>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Slotted {
    Cell(usize),
    Axis(usize),
}

impl Pose {
    pub(super) fn load(model: &Path) -> Option<Pose> {
        let (backing, bytes) = Backing::open(model)?;

        let doc = Mamodel::parse(&bytes)
            .inspect_err(|err| warn!(path = %backing.read_from.display(), "Animation editor could not parse the model: {}", err))
            .ok()?;

        let mut pose = Pose {
            backing,
            doc,
            part: None,
            row: None,
            inputs: Vec::new(),
            cursors: Vec::new(),
            hints: Vec::new(),
            cuts: 0,
            align: [String::new(), String::new()],
            buffer: None,
        };

        pose.reseat();
        pose.restate();

        Some(pose)
    }

    pub(super) fn reseat(&mut self) {
        let cells = authoring::defaults(self.doc.model());

        self.hints = FIELDS
            .iter()
            .enumerate()
            .map(|(at, _)| cells.get(at).map_or_else(String::new, i32::to_string))
            .collect();
    }

    pub(super) fn aim(&mut self, row: Option<usize>) {
        let row = row.filter(|row| *row < self.doc.offsets());

        if self.row == row {
            return;
        }

        self.row = row;
        self.restate();
    }

    fn hint(&self, at: usize) -> &str {
        self.hints.get(at).map_or("", String::as_str)
    }

    fn fallback(&self, at: usize) -> i32 {
        authoring::defaults(self.doc.model()).get(at).copied().unwrap_or(0)
    }

    pub(super) fn pick(&mut self, at: usize) {
        if at >= self.doc.count() {
            return;
        }

        self.part = Some(at);
        self.restate();
    }

    pub(super) fn restate(&mut self) {
        let (x, y) = self.row.and_then(|row| self.doc.offset(row)).unwrap_or_default();
        self.align = [x.to_string(), y.to_string()];

        let Some(part) = self.part else {
            self.inputs.clear();
            self.cursors.clear();

            return;
        };

        self.inputs = (0..FIELDS.len())
            .map(|at| match at {
                NAME_FIELD => self.doc.name(part).unwrap_or_default().to_owned(),
                at => self.doc.field(part, at).unwrap_or(0).to_string(),
            })
            .collect();

        self.cursors = self.inputs.iter().map(|_| widget::Id::unique()).collect();
    }

    pub(super) fn buffering(&self, held: Slotted) -> bool {
        self.buffer == Some(held)
    }

    pub(super) fn resolve_buffer(&mut self) {
        let Some(held) = self.buffer.take() else {
            return;
        };

        let typed = match held {
            Slotted::Cell(at) => self.inputs.get(at),
            Slotted::Axis(axis) => self.align.get(axis),
        };

        let Some(typed) = typed.and_then(|slot| slot.strip_prefix(BUFFER_MARK)).map(str::to_owned) else {
            return;
        };

        match held {
            Slotted::Cell(at) => self.edit(at, &typed),
            Slotted::Axis(axis) => self.shift(axis, &typed),
        }
    }

    pub(super) fn edit(&mut self, at: usize, value: &str) {
        let Some(part) = self.part else {
            return;
        };

        let named = at == NAME_FIELD;

        if named && !nameable(value) {
            return;
        }

        if !named && !typable(value) {
            return;
        }

        let Some(slot) = self.inputs.get_mut(at) else {
            return;
        };

        *slot = value.to_owned();

        if !named && value.starts_with(BUFFER_MARK) {
            self.buffer = Some(Slotted::Cell(at));

            return;
        }

        if self.buffering(Slotted::Cell(at)) {
            self.buffer = None;
        }

        let changed = match named {
            true => self.doc.set_name(part, value),
            false => settled(value, self.fallback(at))
                .map(|parsed| bound(self.doc.model(), self.cuts, at, parsed))
                .is_some_and(|parsed| self.doc.set_field(part, at, parsed)),
        };

        self.backing.dirty |= changed;
    }

    pub(super) fn shift(&mut self, axis: usize, value: &str) {
        if !typable(value) {
            return;
        }

        let Some(slot) = self.align.get_mut(axis) else {
            return;
        };

        *slot = value.to_owned();

        if value.starts_with(BUFFER_MARK) {
            self.buffer = Some(Slotted::Axis(axis));

            return;
        }

        if self.buffering(Slotted::Axis(axis)) {
            self.buffer = None;
        }

        let Some(parsed) = settled(value, 0) else {
            return;
        };

        let Some(row) = self.row else {
            return;
        };

        self.backing.dirty |= self.doc.set_offset(row, axis, parsed);
    }

    pub(super) fn grow(&mut self, parent: Option<usize>) {
        let added = self.doc.add_part(parent);

        self.backing.dirty = true;
        self.pick(added);
    }

    pub(super) fn persist_if_dirty(&mut self) -> Task<Message> {
        if !self.backing.dirty || self.backing.writing {
            return Task::none();
        }

        let (path, stamp, token) = self.backing.prepare();

        let doc = self.doc.clone();
        let reported = path.clone();

        Task::perform(
            smol::unblock(move || write_now(&path, &doc.write(), stamp)),
            move |stamp| Message::Persisted(token, reported.clone(), stamp),
        )
    }

    pub(super) fn persist_now(&mut self) -> Settled {
        if !self.backing.busy() {
            return Settled::Saved;
        }

        let (path, stamp, _) = self.backing.prepare();

        let written = write_now(&path, &self.doc.write(), stamp);

        self.backing.settle(path, written)
    }

    pub(super) fn aligning(&self) -> Element<'_, Message> {
        let live = self.row.is_some_and(|row| self.doc.offset(row).is_some());

        let axis = |at: usize| {
            let cell = text_input(CELL_HINT, self.align.get(at).map_or("", String::as_str))
                .size(CELL_SIZE)
                .padding(CELL_PADDING)
                .align_x(Horizontal::Center)
                .width(Length::Fixed(ALIGN_WIDTH))
                .style(theme::rounded_input);

            match live {
                true => cell.on_input(move |typed| Message::OffsetChanged(at, typed)),
                false => cell,
            }
        };

        let body = row![text("Offset").size(LABEL_SIZE), axis(0), axis(1)]
            .spacing(ROW_GAP)
            .align_y(Vertical::Center);

        container(body)
            .padding([LOOP_CARD_PAD, LOOP_CARD_PAD * 2.0])
            .style(theme::card_container_primary)
            .into()
    }

    pub(super) fn inert(&self, at: usize) -> bool {
        matches!(at, PLACE_X_FIELD | PLACE_Y_FIELD)
            && self.part.is_some_and(|part| self.doc.parent(part).is_none())
    }

    pub(super) fn field_row(&self, at: usize, width: f32) -> Element<'_, Message> {
        let named = at == NAME_FIELD;
        let index = if named { String::new() } else { at.to_string() };
        let hint = if named { "" } else { self.hint(at) };
        let inert = self.inert(at);

        let held = text_input(hint, self.inputs.get(at).map_or("", String::as_str))
            .id(self.cursors.get(at).cloned().unwrap_or_else(widget::Id::unique))
            .size(CELL_SIZE)
            .padding(CELL_PADDING)
            .width(Length::Fill)
            .style(theme::rounded_input);

        let cell: Element<'_, Message> = match inert {
            true => panel::tip(held, ROOT_FIELD_HINT),
            false => held.on_input(move |typed| Message::Field(at, typed)).into(),
        };

        let label = text(FIELDS.get(at).copied().unwrap_or_default()).size(LABEL_SIZE);
        let label = match inert {
            true => label.style(|theme: &Theme| text::Style { color: Some(theme::weak_text_color(theme)) }),
            false => label,
        };

        let body = row![
            theme::centered_text(index).size(LABEL_SIZE).width(Length::Fixed(INDEX_WIDTH)),
            label.width(Length::Fixed(FACT_LABEL)),
            cell,
        ]
        .spacing(ROW_GAP)
        .width(Length::Fill)
        .height(Length::Fixed(FIELD_ROW_HEIGHT - KEY_ROW_PAD * 2.0))
        .align_y(Vertical::Center);

        let inset =
            Padding { top: KEY_ROW_PAD, right: KEY_ROW_INSET, bottom: KEY_ROW_PAD, left: KEY_ROW_INSET };

        container(body)
            .width(Length::Fixed(width))
            .padding(inset)
            .style(move |theme: &Theme| theme::zebra_table_row(theme, at))
            .into()
    }
}

fn shown(value: i32, default: i32) -> String {
    if value == default { String::new() } else { value.to_string() }
}

pub(super) fn settled(value: &str, default: i32) -> Option<i32> {
    if value.is_empty() {
        return Some(default);
    }

    value.parse().ok()
}

fn typable(value: &str) -> bool {
    let mut chars = value.strip_prefix(BUFFER_MARK).unwrap_or(value).chars();

    match chars.next() {
        None => true,
        Some('-') => chars.all(|digit| digit.is_ascii_digit()),
        Some(first) if first.is_ascii_digit() => chars.all(|digit| digit.is_ascii_digit()),
        Some(_) => false,
    }
}

pub(super) fn named(path: &Path) -> Option<String> {
    path.file_name().and_then(|name| name.to_str()).map(str::to_owned)
}

pub(super) fn revalue_file(anim: &Path, moved: &[Option<usize>]) {
    let Some((mut backing, bytes)) = Backing::open(anim) else {
        return;
    };

    let Ok(mut doc) = Maanim::parse(&bytes) else {
        return;
    };

    if !doc.revalue(SPRITE_KIND, moved) {
        return;
    }

    backing.dirty = true;

    let (path, stamp, _) = backing.prepare();

    if write_now(&path, &doc.write(), stamp).is_none() {
        warn!(path = %path.display(), "Studio could not save a recut animation");
    }
}

pub(super) fn retarget_file(anim: &Path, moved: &[Option<usize>]) {
    let Some((mut backing, bytes)) = Backing::open(anim) else {
        return;
    };

    let parsed = Maanim::parse(&bytes)
        .inspect_err(|err| warn!(path = %anim.display(), "Studio could not reindex the file: {}", err));

    let Ok(mut doc) = parsed else {
        return;
    };

    if !doc.retarget(moved) {
        return;
    }

    backing.dirty = true;

    let (path, stamp, _) = backing.prepare();

    if write_now(&path, &doc.write(), stamp).is_none() {
        warn!(path = %path.display(), "Studio could not save a reindexed animation");
    }
}

pub(super) fn write_now(path: &Path, body: &[u8], stamp: Stamp) -> Option<Stamp> {
    preview::save(path, body, stamp)
        .inspect_err(|err| warn!(path = %path.display(), "Studio could not write the file: {}", err))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(frames: &[i32]) -> Vec<Keyframe> {
        frames.iter().map(|frame| Keyframe { frame: *frame, ..Keyframe::default() }).collect()
    }

    #[test]
    fn a_bound_reaches_the_next_key_rather_than_stopping_short_of_it() {
        // The strip marks a segment across both of its keys, so the bound has to hold both.
        // Stopping a frame short cut off the pose being animated towards, and collapsed to a
        // single frame outright wherever two keys sit next to each other.
        let spread = keys(&[0, 10, 25]);

        assert_eq!(span_of(&spread, 0), Some((0, 10)));
        assert_eq!(span_of(&spread, 1), Some((10, 25)));

        assert_eq!(span_of(&keys(&[3, 4]), 0), Some((3, 4)));
    }

    #[test]
    fn the_last_key_bounds_itself_rather_than_repeating_its_neighbour() {
        // The strip marks it alone, having nothing to reach towards, and the bound matches.
        let keys = keys(&[0, 10, 25]);

        assert_eq!(span_of(&keys, 2), Some((25, 25)));
        assert_ne!(span_of(&keys, 2), span_of(&keys, 1));
    }

    #[test]
    fn keys_sharing_a_frame_do_not_invert_the_bound() {
        let keys = keys(&[4, 4]);

        assert_eq!(span_of(&keys, 0), Some((4, 4)));
    }
}
