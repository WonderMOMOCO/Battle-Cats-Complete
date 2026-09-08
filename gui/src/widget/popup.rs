use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{self, Interaction};
use iced::widget::{button, column, container, mouse_area, opaque, stack, text, Space};
use iced::{Alignment, Border, Color, Element, Event, Font, Length, Padding, Point, Rectangle, Size, Theme, Vector};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::theme;
use crate::common::glyphs::Ruler;

const HEADER_HEIGHT: f32 = 28.0;

pub(crate) const CHROME_HEIGHT: f32 = HEADER_HEIGHT + FRAME_BORDER_WIDTH * 2.0;
const HEADER_MARGIN_X: f32 = 50.0;
const HEADER_MARGIN_Y: f32 = 30.0;
const DEFAULT_BODY_ALPHA: f32 = 1.0;

pub const GLASS: f32 = 0.95;
pub(crate) const FRAME_BORDER: f32 = 3.0;
const FRAME_BORDER_WIDTH: f32 = FRAME_BORDER;
const MINIMUM_WINDOW: Size = Size::new(800.0, 600.0);
const GRIP_OUTSET: f32 = 5.0;
const GRIP_INSET: f32 = FRAME_BORDER_WIDTH;
const GRIP_BAND: f32 = GRIP_OUTSET + GRIP_INSET;
const GRIP_CORNER: f32 = 14.0;
const HIGHLIGHT_CORNER: f32 = 26.0;
const TITLE_SIZE: f32 = 14.0;
const TITLE_RESERVE: f32 = 34.0;
const ELLIPSIS: char = '…';

fn trimmed(title: &str, room: f32) -> String {
    let mut ruler = Ruler::new(Font::DEFAULT, TITLE_SIZE);

    if ruler.width(title) <= room {
        return title.to_owned();
    }

    let glyphs: Vec<char> = title.chars().collect();
    let mut low = 0;
    let mut high = glyphs.len();

    while low < high {
        let keep = (low + high).div_ceil(2);

        match ruler.width(&elided(&glyphs, keep)) <= room {
            true => low = keep,
            false => high = keep - 1,
        }
    }

    elided(&glyphs, low)
}

fn elided(glyphs: &[char], keep: usize) -> String {
    let head = keep.div_ceil(2);
    let tail = keep - head;

    glyphs[..head]
        .iter()
        .chain(std::iter::once(&ELLIPSIS))
        .chain(&glyphs[glyphs.len() - tail..])
        .collect()
}

#[derive(Default)]
struct Fit {
    title: String,
    room: f32,
    shown: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    CatFilter,
    EnemyFilter,
    StageFilter,
    ModImport,
    ModExport,
    CatAnimationExport,
    EnemyAnimationExport,
    UtilityAnimationExport,
    UtilityAnimationSettings,
    Keys,
    Pem,
    Exceptions,
    Changelog,
    Notice,
    Updater,
    InitErrors,
    CatAttributes,
    EnemyAttributes,
    UnitBuy,
    LevelCurve,
    Talents,
    TalentCosts,
    Combos,
    Explanation,
    EnemyName,
    EnemyDescription,
    ComboName,
    TalentText,
    MapName,
    StageName,
    MapStage,
    MapDrops,
    DropChara,
    ItemBuy,
    ItemName,
    Battleground,
    EnemyPick,
    Animator,
    StudioManage,
    StudioOnion,
    StudioShipout,
}

pub(crate) const KIND_COUNT: usize = 41;

const KINDS: [Kind; KIND_COUNT] = [
    Kind::CatFilter,
    Kind::EnemyFilter,
    Kind::StageFilter,
    Kind::ModImport,
    Kind::ModExport,
    Kind::CatAnimationExport,
    Kind::EnemyAnimationExport,
    Kind::UtilityAnimationExport,
    Kind::Keys,
    Kind::Pem,
    Kind::Exceptions,
    Kind::Changelog,
    Kind::Notice,
    Kind::Updater,
    Kind::InitErrors,
    Kind::CatAttributes,
    Kind::EnemyAttributes,
    Kind::UnitBuy,
    Kind::LevelCurve,
    Kind::Talents,
    Kind::TalentCosts,
    Kind::Combos,
    Kind::Explanation,
    Kind::EnemyName,
    Kind::EnemyDescription,
    Kind::ComboName,
    Kind::TalentText,
    Kind::MapName,
    Kind::StageName,
    Kind::MapStage,
    Kind::MapDrops,
    Kind::DropChara,
    Kind::ItemBuy,
    Kind::ItemName,
    Kind::Battleground,
    Kind::EnemyPick,
    Kind::Animator,
    Kind::UtilityAnimationSettings,
    Kind::StudioManage,
    Kind::StudioOnion,
    Kind::StudioShipout,
];

impl Kind {
    fn key(self) -> &'static str {
        match self {
            Self::CatFilter => "cat_filter",
            Self::EnemyFilter => "enemy_filter",
            Self::StageFilter => "stage_filter",
            Self::UtilityAnimationExport => "utility_animation_export",
            Self::UtilityAnimationSettings => "utility_animation_settings",
            Self::ModImport => "mod_import",
            Self::ModExport => "mod_export",
            Self::CatAnimationExport => "cat_animation_export",
            Self::EnemyAnimationExport => "enemy_animation_export",
            Self::Keys => "keys",
            Self::Pem => "pem",
            Self::Exceptions => "exceptions",
            Self::Changelog => "changelog",
            Self::Notice => "notice",
            Self::Updater => "updater",
            Self::InitErrors => "init_errors",
            Self::CatAttributes => "cat_attributes",
            Self::EnemyAttributes => "enemy_attributes",
            Self::UnitBuy => "unit_buy",
            Self::Talents => "talents",
            Self::TalentCosts => "talent_costs",
            Self::Combos => "combos",
            Self::LevelCurve => "level_curve",
            Self::Explanation => "explanation",
            Self::EnemyName => "enemy_name",
            Self::EnemyDescription => "enemy_description",
            Self::ComboName => "combo_name",
            Self::TalentText => "talent_text",
            Self::MapName => "map_name",
            Self::StageName => "stage_name",
            Self::MapStage => "map_stage",
            Self::MapDrops => "map_drops",
            Self::DropChara => "drop_chara",
            Self::ItemBuy => "item_buy",
            Self::ItemName => "item_name",
            Self::Battleground => "battleground",
            Self::EnemyPick => "enemy_pick",
            Self::Animator => "animator",
            Self::StudioManage => "studio_manage",
            Self::StudioOnion => "studio_onion",
            Self::StudioShipout => "studio_shipout",
        }
    }

    fn slot(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy)]
pub struct Spec {
    kind: Kind,
    minimum: Size,
}

impl Spec {
    pub const fn new(kind: Kind, minimum: Size) -> Self {
        Self { kind, minimum }
    }

    pub(crate) const fn kind(self) -> Kind {
        self.kind
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

const EDGES: [Edge; 8] = [
    Edge::Top,
    Edge::Bottom,
    Edge::Left,
    Edge::Right,
    Edge::TopLeft,
    Edge::TopRight,
    Edge::BottomLeft,
    Edge::BottomRight,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Start,
    End,
}

impl Edge {
    fn horizontal(self) -> Option<Side> {
        match self {
            Self::Left | Self::TopLeft | Self::BottomLeft => Some(Side::Start),
            Self::Right | Self::TopRight | Self::BottomRight => Some(Side::End),
            Self::Top | Self::Bottom => None,
        }
    }

    fn vertical(self) -> Option<Side> {
        match self {
            Self::Top | Self::TopLeft | Self::TopRight => Some(Side::Start),
            Self::Bottom | Self::BottomLeft | Self::BottomRight => Some(Side::End),
            Self::Left | Self::Right => None,
        }
    }

    fn interaction(self) -> Interaction {
        match self {
            Self::Left | Self::Right => Interaction::ResizingHorizontally,
            Self::Top | Self::Bottom => Interaction::ResizingVertically,
            Self::TopLeft | Self::BottomRight => Interaction::ResizingDiagonallyDown,
            Self::TopRight | Self::BottomLeft => Interaction::ResizingDiagonallyUp,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct StoredSize {
    pub width: f32,
    pub height: f32,
}

thread_local! {
    static SIZES: Cell<[Option<Size>; KIND_COUNT]> = const { Cell::new([None; KIND_COUNT]) };
    static RAISES: Cell<u64> = const { Cell::new(0) };
    static ORDER: Cell<[u64; KIND_COUNT]> = const { Cell::new([0; KIND_COUNT]) };
    static SHARED: Cell<[bool; KIND_COUNT]> = const { Cell::new([false; KIND_COUNT]) };
}

fn next_raise() -> u64 {
    RAISES.with(|counter| {
        let next = counter.get().saturating_add(1);
        counter.set(next);

        next
    })
}

pub(crate) fn raise(kind: Kind) -> u64 {
    let next = next_raise();

    ORDER.with(|order| {
        let mut slots = order.get();
        slots[kind.slot()] = next;
        order.set(slots);
    });

    next
}

pub(crate) fn order(kind: Kind) -> u64 {
    ORDER.with(|order| order.get()[kind.slot()])
}

fn stored_size(kind: Kind) -> Option<Size> {
    SIZES.with(|sizes| sizes.get()[kind.slot()])
}

fn store_size(kind: Kind, size: Option<Size>) {
    SIZES.with(|sizes| {
        let mut slots = sizes.get();
        slots[kind.slot()] = size;
        sizes.set(slots);
    });
}

pub(crate) fn snapshot() -> BTreeMap<String, StoredSize> {
    KINDS
        .into_iter()
        .filter_map(|kind| {
            let size = stored_size(kind)?;

            Some((kind.key().to_owned(), StoredSize { width: size.width, height: size.height }))
        })
        .collect()
}

pub(crate) fn restore(stored: &BTreeMap<String, StoredSize>) {
    for kind in KINDS {
        let Some(entry) = stored.get(kind.key()).filter(|entry| entry.width.is_finite() && entry.height.is_finite()) else {
            continue;
        };

        store_size(kind, Some(Size::new(entry.width, entry.height)));
    }
}

#[derive(Default)]
pub struct State {
    position: Option<Point>,
    nudge: f32,
    drag: Drag,
    hovered: Option<Edge>,
    raised: u64,
    fit: RefCell<Fit>,
}

impl State {
    fn fitted(&self, title: &str, room: f32) -> String {
        let mut fit = self.fit.borrow_mut();

        if fit.title == title && fit.room == room {
            return fit.shown.clone();
        }

        let shown = trimmed(title, room);

        *fit = Fit { title: title.to_owned(), room, shown: shown.clone() };

        shown
    }
}

const CASCADE_STEP: f32 = 26.0;

pub fn cascaded(steps: usize) -> State {
    State { nudge: steps as f32 * CASCADE_STEP, raised: next_raise(), ..State::default() }
}

#[derive(Default, Clone, Copy)]
enum Drag {
    #[default]
    Idle,
    Pressed,
    Moving {
        last: Point,
    },
    Grabbed {
        edge: Edge,
    },
    Resizing {
        edge: Edge,
        origin: Point,
        position: Point,
        size: Size,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Raise,
    HeaderPressed,
    EdgePressed(Edge),
    EdgeEntered(Edge),
    EdgeExited(Edge),
    Dragged(Point, Size),
    Released,
    Close,
}

impl State {
    pub(crate) fn raised(&self) -> u64 {
        self.raised
    }

    pub fn update(&mut self, message: Message, spec: Spec) -> bool {
        match message {
            Message::Raise => self.raised = raise(spec.kind),
            Message::HeaderPressed => self.drag = Drag::Pressed,
            Message::EdgePressed(edge) => {
                self.raised = raise(spec.kind);
                self.drag = Drag::Grabbed { edge };
            }
            Message::EdgeEntered(edge) => self.hovered = Some(edge),
            Message::EdgeExited(edge) => {
                if self.hovered == Some(edge) {
                    self.hovered = None;
                }
            }
            Message::Dragged(cursor, window) => match self.drag {
                Drag::Idle => {}
                Drag::Pressed => self.drag = Drag::Moving { last: cursor },
                Drag::Moving { last } => {
                    let (position, size) = self.resolved(spec, window);
                    let next = Point::new(position.x + cursor.x - last.x, position.y + cursor.y - last.y);
                    self.position = Some(clamp(next, size, window));
                    self.drag = Drag::Moving { last: cursor };
                }
                Drag::Grabbed { edge } => {
                    let (position, size) = self.resolved(spec, window);
                    self.drag = Drag::Resizing { edge, origin: cursor, position, size };
                }
                Drag::Resizing { edge, origin, position, size } => {
                    self.position = Some(resize(spec, edge, cursor - origin, position, size, window));
                }
            },
            Message::Released => self.drag = Drag::Idle,
            Message::Close => {
                self.drag = Drag::Idle;
                return true;
            }
        }

        false
    }

    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        title: &'a str,
        spec: Spec,
        window: Size,
        to_message: fn(Message) -> M,
        content: impl Fn() -> Element<'a, M> + 'a,
        body_alpha: Option<f32>,
    ) -> Element<'a, M> {
        let body_alpha = body_alpha.unwrap_or(DEFAULT_BODY_ALPHA);

        let bounds = if window.width < 1.0 || window.height < 1.0 { MINIMUM_WINDOW } else { window };
        let (position, size) = self.resolved(spec, bounds);

        let close_button = button(text("✕").size(18))
            .style(button::text)
            .padding(2.0)
            .on_press(to_message(Message::Close));

        let room = (size.width - TITLE_RESERVE * 2.0).max(0.0);

        let title_layer = container(text(self.fitted(title, room)).size(TITLE_SIZE).wrapping(text::Wrapping::None))
            .width(Length::Fixed(room))
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        let title_layer = container(title_layer)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        let button_layer = container(close_button)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::Center);

        let header = mouse_area(
            container(stack![title_layer, button_layer])
                .width(Length::Fill)
                .height(Length::Fixed(HEADER_HEIGHT))
                .padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 10.0 })
                .style(header_style),
        )
        .interaction(Interaction::Grab)
        .on_press(to_message(Message::HeaderPressed));

        let body = container(content())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |theme: &Theme| body_style(theme, body_alpha));

        let framed = container(column![header, body])
            .width(Length::Fixed(size.width))
            .height(Length::Fixed(size.height))
            .padding(FRAME_BORDER_WIDTH)
            .style(frame_style);

        let frame = opaque(framed);

        let mut layers = vec![focused(frame, position, to_message(Message::Raise))];

        let lit = match self.drag {
            Drag::Grabbed { edge } | Drag::Resizing { edge, .. } => Some(edge),
            Drag::Idle => self.hovered,
            Drag::Pressed | Drag::Moving { .. } => None,
        };

        if let Some(edge) = lit {
            layers.extend(highlight(edge, position, size));
        }

        layers.extend(EDGES.map(|edge| grip(edge, position, size, to_message)));

        let mut drag_layer = mouse_area(Space::new().width(Length::Fill).height(Length::Fill));

        if !matches!(self.drag, Drag::Idle) {
            let interaction = match self.drag {
                Drag::Grabbed { edge } | Drag::Resizing { edge, .. } => edge.interaction(),
                _ => Interaction::Grabbing,
            };

            drag_layer = drag_layer
                .interaction(interaction)
                .on_move(move |cursor| to_message(Message::Dragged(cursor, bounds)))
                .on_release(to_message(Message::Released))
                .on_exit(to_message(Message::Released));
        }

        layers.push(drag_layer.into());

        stack(layers).into()
    }

    pub(crate) fn body_width(&self, spec: Spec, window: Size) -> f32 {
        let bounds = if window.width < 1.0 || window.height < 1.0 { MINIMUM_WINDOW } else { window };
        let (_, size) = self.resolved(spec, bounds);

        size.width - FRAME_BORDER_WIDTH * 2.0
    }

    pub(crate) fn body_height(&self, spec: Spec, window: Size) -> f32 {
        let bounds = if window.width < 1.0 || window.height < 1.0 { MINIMUM_WINDOW } else { window };
        let (_, size) = self.resolved(spec, bounds);

        size.height - FRAME_BORDER_WIDTH * 2.0 - HEADER_HEIGHT
    }

    fn resolved(&self, spec: Spec, window: Size) -> (Point, Size) {
        let stored = stored_size(spec.kind).unwrap_or(spec.minimum);
        let size = Size::new(stored.width.max(spec.minimum.width), stored.height.max(spec.minimum.height));

        let centered = Point::new(
            ((window.width - size.width) / 2.0).max(0.0) + self.nudge,
            ((window.height - size.height) / 2.0).max(0.0) + self.nudge,
        );

        (clamp(self.position.unwrap_or(centered), size, window), size)
    }
}

fn resize(spec: Spec, edge: Edge, delta: Vector, position: Point, size: Size, window: Size) -> Point {
    let minimum = spec.minimum;
    let right = position.x + size.width;
    let bottom = position.y + size.height;

    let x = match edge.horizontal() {
        Some(Side::Start) => (position.x + delta.x).min(right - minimum.width),
        Some(Side::End) | None => position.x,
    };

    let y = match edge.vertical() {
        Some(Side::Start) => (position.y + delta.y).min(bottom - minimum.height).max(0.0),
        Some(Side::End) | None => position.y,
    };

    let width = match edge.horizontal() {
        Some(Side::Start) => right - x,
        Some(Side::End) => (size.width + delta.x).max(minimum.width),
        None => size.width,
    };

    let height = match edge.vertical() {
        Some(Side::Start) => bottom - y,
        Some(Side::End) => (size.height + delta.y).max(minimum.height),
        None => size.height,
    };

    let resized = Size::new(width, height);
    store_size(spec.kind, (resized != minimum).then_some(resized));

    clamp(Point::new(x, y), resized, window)
}

fn grip_bounds(edge: Edge, position: Point, size: Size) -> Rectangle {
    let (x, y, width, height) = (position.x, position.y, size.width, size.height);
    let far_x = x + width + GRIP_OUTSET - GRIP_CORNER;
    let far_y = y + height + GRIP_OUTSET - GRIP_CORNER;
    let corner = Size::new(GRIP_CORNER, GRIP_CORNER);

    match edge {
        Edge::Top => Rectangle::new(Point::new(x, y - GRIP_OUTSET), Size::new(width, GRIP_BAND)),
        Edge::Bottom => Rectangle::new(Point::new(x, y + height - GRIP_INSET), Size::new(width, GRIP_BAND)),
        Edge::Left => Rectangle::new(Point::new(x - GRIP_OUTSET, y), Size::new(GRIP_BAND, height)),
        Edge::Right => Rectangle::new(Point::new(x + width - GRIP_INSET, y), Size::new(GRIP_BAND, height)),
        Edge::TopLeft => Rectangle::new(Point::new(x - GRIP_OUTSET, y - GRIP_OUTSET), corner),
        Edge::TopRight => Rectangle::new(Point::new(far_x, y - GRIP_OUTSET), corner),
        Edge::BottomLeft => Rectangle::new(Point::new(x - GRIP_OUTSET, far_y), corner),
        Edge::BottomRight => Rectangle::new(Point::new(far_x, far_y), corner),
    }
}

fn grip<'a, M: Clone + 'a>(edge: Edge, position: Point, size: Size, to_message: fn(Message) -> M) -> Element<'a, M> {
    let bounds = grip_bounds(edge, position, size);

    let area = mouse_area(Space::new().width(bounds.width).height(bounds.height))
        .interaction(edge.interaction())
        .on_press(to_message(Message::EdgePressed(edge)))
        .on_enter(to_message(Message::EdgeEntered(edge)))
        .on_exit(to_message(Message::EdgeExited(edge)));

    anchored(area, bounds.position())
}

fn highlight<'a, M: Clone + 'a>(edge: Edge, position: Point, size: Size) -> Vec<Element<'a, M>> {
    let (x, y, width, height) = (position.x, position.y, size.width, size.height);
    let along_x = HIGHLIGHT_CORNER.min(width);
    let along_y = HIGHLIGHT_CORNER.min(height);

    let top = |length: f32| Rectangle::new(Point::new(x, y), Size::new(length, FRAME_BORDER_WIDTH));
    let bottom = |length: f32| Rectangle::new(Point::new(x, y + height - FRAME_BORDER_WIDTH), Size::new(length, FRAME_BORDER_WIDTH));
    let left = |length: f32| Rectangle::new(Point::new(x, y), Size::new(FRAME_BORDER_WIDTH, length));
    let right = |length: f32| Rectangle::new(Point::new(x + width - FRAME_BORDER_WIDTH, y), Size::new(FRAME_BORDER_WIDTH, length));
    let shift_x = |bounds: Rectangle| Rectangle { x: bounds.x + width - bounds.width, ..bounds };
    let shift_y = |bounds: Rectangle| Rectangle { y: bounds.y + height - bounds.height, ..bounds };

    let bars = match edge {
        Edge::Top => vec![top(width)],
        Edge::Bottom => vec![bottom(width)],
        Edge::Left => vec![left(height)],
        Edge::Right => vec![right(height)],
        Edge::TopLeft => vec![top(along_x), left(along_y)],
        Edge::TopRight => vec![shift_x(top(along_x)), right(along_y)],
        Edge::BottomLeft => vec![bottom(along_x), shift_y(left(along_y))],
        Edge::BottomRight => vec![shift_x(bottom(along_x)), shift_y(right(along_y))],
    };

    bars.into_iter().map(|bounds| anchored(bar(bounds.size()), bounds.position())).collect()
}

fn bar<'a, M: Clone + 'a>(size: Size) -> Element<'a, M> {
    container(Space::new())
        .width(Length::Fixed(size.width))
        .height(Length::Fixed(size.height))
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().primary.into()),
            ..container::Style::default()
        })
        .into()
}

struct Slots(Vec<(usize, usize)>);

struct Layered<'a, M> {
    content: Element<'a, M>,
    slots: Vec<(usize, usize)>,
}

fn report_shared(kind: Kind) {
    let reported = SHARED.with(|shared| {
        let mut slots = shared.get();
        let seen = slots[kind.slot()];
        slots[kind.slot()] = true;
        shared.set(slots);

        seen
    });

    if !reported {
        warn!("popup {kind:?} is open twice; the pair shares its stored size and its z-order stamp");
    }
}

pub(crate) fn layered<'a, M: 'a>(popups: Vec<(Kind, Element<'a, M>)>) -> Element<'a, M> {
    let mut slots: Vec<(usize, usize)> = Vec::with_capacity(popups.len());
    let mut layers = Vec::with_capacity(popups.len());

    for (kind, view) in popups {
        let slot = kind.slot();
        let repeat = slots.iter().filter(|(seen, _)| *seen == slot).count();

        if repeat > 0 {
            report_shared(kind);
        }

        slots.push((slot, repeat));
        layers.push(view);
    }

    Element::new(Layered { content: stack(layers).into(), slots })
}

impl<'a, M> Widget<M, Theme, iced::Renderer> for Layered<'a, M> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<Slots>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Slots(self.slots.clone()))
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        let previous = std::mem::take(&mut tree.state.downcast_mut::<Slots>().0);

        if let Some(inner) = tree.children.first_mut() {
            if inner.children.len() == previous.len() && previous != self.slots {
                let mut held: Vec<Option<widget::Tree>> = inner.children.drain(..).map(Some).collect();

                inner.children = self
                    .slots
                    .iter()
                    .map(|slot| {
                        previous
                            .iter()
                            .position(|held_slot| held_slot == slot)
                            .and_then(|index| held[index].take())
                            .unwrap_or_else(widget::Tree::empty)
                    })
                    .collect();
            }

            inner.diff(&self.content);
        }

        tree.state.downcast_mut::<Slots>().0.clone_from(&self.slots);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let Some(inner) = tree.children.first_mut() else { return layout::Node::new(Size::ZERO) };

        self.content.as_widget_mut().layout(inner, renderer, limits)
    }

    fn operate(&mut self, tree: &mut widget::Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn widget::Operation) {
        let Some(inner) = tree.children.first_mut() else { return };

        self.content.as_widget_mut().operate(inner, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let Some(inner) = tree.children.first_mut() else { return };

        self.content
            .as_widget_mut()
            .update(inner, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        tree.children.first().map_or(mouse::Interaction::None, |inner| {
            self.content.as_widget().mouse_interaction(inner, layout, cursor, viewport, renderer)
        })
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(inner) = tree.children.first() else { return };

        self.content.as_widget().draw(inner, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, iced::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree.children.first_mut()?, layout, renderer, viewport, translation)
    }
}

struct Anchored<'a, M> {
    content: Element<'a, M>,
    position: Point,
    on_press: Option<M>,
}

fn anchored<'a, M: Clone + 'a>(content: impl Into<Element<'a, M>>, position: Point) -> Element<'a, M> {
    Element::new(Anchored {
        content: content.into(),
        position,
        on_press: None,
    })
}

fn focused<'a, M: Clone + 'a>(content: impl Into<Element<'a, M>>, position: Point, message: M) -> Element<'a, M> {
    Element::new(Anchored {
        content: content.into(),
        position,
        on_press: Some(message),
    })
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for Anchored<'a, M> {
    fn tag(&self) -> widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let node = self
            .content
            .as_widget_mut()
            .layout(tree, renderer, &layout::Limits::new(Size::ZERO, Size::INFINITE))
            .move_to(self.position);

        layout::Node::with_children(limits.max(), vec![node])
    }

    fn operate(&mut self, tree: &mut widget::Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn widget::Operation) {
        if let Some(child) = layout.children().next() {
            self.content.as_widget_mut().operate(tree, child, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else { return };

        if let Some(message) = self.on_press.as_ref()
            && matches!(event, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)))
            && cursor.is_over(child.bounds())
        {
            shell.publish(message.clone());
        }

        self.content
            .as_widget_mut()
            .update(tree, event, child, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        layout.children().next().map_or(mouse::Interaction::None, |child| {
            self.content.as_widget().mouse_interaction(tree, child, cursor, viewport, renderer)
        })
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if let (Some(clipped_viewport), Some(child)) = (bounds.intersection(viewport), layout.children().next()) {
            self.content
                .as_widget()
                .draw(tree, renderer, theme, style, child, cursor, &clipped_viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, iced::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout.children().next()?, renderer, viewport, translation)
    }
}

fn clamp(position: Point, size: Size, window: Size) -> Point {
    let min_x = HEADER_MARGIN_X - size.width;
    let max_x = (window.width - HEADER_MARGIN_X).max(min_x);
    let max_y = (window.height - HEADER_MARGIN_Y).max(0.0);

    Point::new(position.x.clamp(min_x, max_x), position.y.clamp(0.0, max_y))
}

fn header_color(theme: &Theme) -> Color {
    let palette = theme.palette();
    let shade = |c: f32| c * 0.7;

    Color {
        r: shade(palette.background.r),
        g: shade(palette.background.g),
        b: shade(palette.background.b),
        a: palette.background.a,
    }
}

fn inner_radius() -> f32 {
    (theme::RADIUS_MD - FRAME_BORDER_WIDTH).max(0.0)
}

fn header_style(theme: &Theme) -> container::Style {
    let radius = inner_radius();

    container::Style {
        background: Some(header_color(theme).into()),
        border: Border {
            radius: Radius { top_left: radius, top_right: radius, bottom_right: 0.0, bottom_left: 0.0 },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn frame_style(theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: header_color(theme),
            width: FRAME_BORDER_WIDTH,
            radius: theme::RADIUS_MD.into(),
        },
        ..container::Style::default()
    }
}

fn body_style(theme: &Theme, alpha: f32) -> container::Style {
    let palette = theme.extended_palette();
    let background = Color { a: alpha, ..palette.background.base.color };
    let radius = inner_radius();

    container::Style {
        background: Some(background.into()),
        border: Border {
            radius: Radius { top_left: 0.0, top_right: 0.0, bottom_left: radius, bottom_right: radius },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // slot() is the discriminant and indexes SIZES/ORDER, so every variant must be listed
    // in KINDS exactly once. The order it is listed in does not matter.
    #[test]
    fn every_kind_owns_its_slot() {
        let mut seen = [false; KIND_COUNT];

        for kind in KINDS {
            let slot = kind.slot();

            assert!(slot < KIND_COUNT, "{kind:?} sits past KIND_COUNT and would index out of bounds");
            assert!(!seen[slot], "{kind:?} is listed twice in KINDS");

            seen[slot] = true;
        }

        assert!(seen.into_iter().all(|listed| listed), "a kind is missing from KINDS and would not persist its size");
    }

    #[test]
    fn every_kind_owns_its_storage_key() {
        for (index, kind) in KINDS.into_iter().enumerate() {
            assert!(
                !KINDS[index + 1..].iter().any(|other| other.key() == kind.key()),
                "{kind:?} shares its stored key with another kind"
            );
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct Marker(usize);

    struct Leaf(Marker);

    impl<M> Widget<M, Theme, iced::Renderer> for Leaf {
        fn tag(&self) -> widget::tree::Tag {
            widget::tree::Tag::of::<Marker>()
        }

        fn state(&self) -> widget::tree::State {
            widget::tree::State::new(self.0)
        }

        fn size(&self) -> Size<Length> {
            Size::new(Length::Shrink, Length::Shrink)
        }

        fn layout(&mut self, _tree: &mut widget::Tree, _renderer: &iced::Renderer, _limits: &layout::Limits) -> layout::Node {
            layout::Node::new(Size::ZERO)
        }

        fn draw(
            &self,
            _tree: &widget::Tree,
            _renderer: &mut iced::Renderer,
            _theme: &Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    // Every popup is a fresh Element each frame, so the id in the widget differs from the id
    // in the tree: seeing the old id proves the state moved instead of being rebuilt.
    fn leaf<'a>(id: usize) -> Element<'a, ()> {
        Element::new(Leaf(Marker(id)))
    }

    fn markers(tree: &widget::Tree) -> Vec<Marker> {
        tree.children[0].children.iter().map(|child| *child.state.downcast_ref::<Marker>()).collect()
    }

    #[test]
    fn a_raised_popup_carries_its_state_to_its_new_layer() {
        let opened = layered(vec![(Kind::CatFilter, leaf(1)), (Kind::ModExport, leaf(2))]);
        let mut tree = widget::Tree::new(&opened);

        assert_eq!(markers(&tree), vec![Marker(1), Marker(2)]);

        let raised = layered(vec![(Kind::ModExport, leaf(20)), (Kind::CatFilter, leaf(10))]);
        raised.as_widget().diff(&mut tree);

        assert_eq!(markers(&tree), vec![Marker(2), Marker(1)], "the layers reordered but the state did not follow");
    }

    #[test]
    fn closing_a_popup_leaves_the_survivor_on_its_own_state() {
        let opened = layered(vec![(Kind::CatFilter, leaf(1)), (Kind::ModExport, leaf(2))]);
        let mut tree = widget::Tree::new(&opened);

        let closed = layered(vec![(Kind::ModExport, leaf(20))]);
        closed.as_widget().diff(&mut tree);

        assert_eq!(markers(&tree), vec![Marker(2)], "the survivor inherited the closed popup's state");
    }

    // Two popup states can be built with the same kind (studio's idle and session viewers both
    // hold an Animator export popup). Only one renders today; if both ever did, neither may
    // steal the other's state.
    #[test]
    fn popups_sharing_a_kind_keep_their_own_state() {
        let opened = layered(vec![
            (Kind::Animator, leaf(1)),
            (Kind::Animator, leaf(2)),
            (Kind::CatFilter, leaf(3)),
        ]);

        let mut tree = widget::Tree::new(&opened);

        let raised = layered(vec![
            (Kind::CatFilter, leaf(30)),
            (Kind::Animator, leaf(10)),
            (Kind::Animator, leaf(20)),
        ]);

        raised.as_widget().diff(&mut tree);

        assert_eq!(markers(&tree), vec![Marker(3), Marker(1), Marker(2)], "a twin lost its state to its own kind");
    }

    #[test]
    fn an_opened_popup_starts_on_fresh_state() {
        let opened = layered(vec![(Kind::CatFilter, leaf(1))]);
        let mut tree = widget::Tree::new(&opened);

        let joined = layered(vec![(Kind::CatFilter, leaf(10)), (Kind::ModExport, leaf(2))]);
        joined.as_widget().diff(&mut tree);

        assert_eq!(markers(&tree), vec![Marker(1), Marker(2)]);
    }

    // The budget was a character count against a flat 0.55 glyph ratio, so a full-width
    // name was charged about half what it measures and the title ran out past the header.
    #[test]
    fn a_title_is_trimmed_to_the_room_it_actually_measures() {
        let mut ruler = Ruler::new(Font::DEFAULT, TITLE_SIZE);

        // A narrow popup: NARROW_WIDTH 252 less TITLE_RESERVE either side.
        let room = 184.0;

        for title in [
            "\u{4f1d}\u{8aac}\u{306e}\u{306f}\u{3058}\u{307e}\u{308a} :: Map_Name_ja.csv",
            "The Legend Begins :: Map_Name_en.csv",
            "\u{77ed}\u{3044}",
            "",
        ] {
            let shown = super::trimmed(title, room);

            assert!(
                ruler.width(&shown) <= room,
                "{title} trimmed to {shown}, {} wide in {room}",
                ruler.width(&shown),
            );
        }
    }

    #[test]
    fn a_full_width_title_keeps_fewer_glyphs_than_a_latin_one() {
        let room = 184.0;

        let latin = super::trimmed("The Legend Begins :: Map_Name_en.csv", room);
        let kana = super::trimmed(
            "\u{3067}\u{3093}\u{305b}\u{3064}\u{306e}\u{306f}\u{3058}\u{307e}\u{308a} :: Map_Name_ja.csv",
            room,
        );

        assert!(
            kana.chars().count() < latin.chars().count(),
            "kana kept {} glyphs, latin {}",
            kana.chars().count(),
            latin.chars().count(),
        );
    }

    #[test]
    fn a_title_that_fits_is_left_alone() {
        assert_eq!(super::trimmed("Stage", 184.0), "Stage");
    }
}
