use super::*;
use kore::systems::animation::authoring::Cycle;
use iced::border::Radius;
use iced::widget::canvas::{self, Geometry, Path, Stroke};
use iced::widget::{canvas as canvas_widget, column, container, row, text};
use iced::{Pixels, Rectangle, Renderer};

pub(super) const BOARD_HEIGHT: f32 = 208.0;

const LANE_HEIGHT: f32 = 30.0;
const LANE_INSET: f32 = 5.0;
const LABEL_INSET: f32 = 5.0;
const CARD_PAD: f32 = 6.0;
const CARD_BLEED: f32 = 3.0;
const TICK_ROOM: f32 = 62.0;
const TICK_HEIGHT: f32 = 4.0;
const TICK_TINT: f32 = 0.45;
const GUTTER_MIN: f32 = 34.0;
const GUTTER_MAX: f32 = 96.0;
const CAPTION_SIZE: f32 = 10.0;
const GLYPH_SHARE: f32 = 0.62;
const CAPTION_ROOM: f32 = 7.0;
const OUTLINE: f32 = 1.0;
const SEPARATOR: f32 = 1.5;
const DARKEN_EDGE: f32 = 0.55;
const DARKEN_SEAM: f32 = 0.42;
const GHOST_TINT: f32 = 0.38;
const SHADOW_TINT: f32 = 0.45;
const HELD_HEIGHT: f32 = 3.0;
const PLAYHEAD_WIDTH: f32 = 1.5;
const MARK_WIDTH: f32 = 1.0;
const MARGIN: f64 = 0.06;
const LEEWAY_SHARE: f64 = 0.18;
const NARROWEST: f64 = 4.0;
const WIDEST_SHARE: f64 = 4.0;
const LINE_PIXELS: f32 = 40.0;
const ZOOM_RATE: f64 = 1.18;
const CLICK_SLOP: f32 = 3.0;
const MIN_PASS: f32 = 3.0;
const GRAIN_FLOOR: f32 = 1.0;
const GRAB: f32 = 6.0;
const FLOOR: i64 = i32::MIN as i64;
const CEILING: i64 = i32::MAX as i64;
const START_INK: Color = Color::from_rgb(0.24, 0.72, 0.36);
const LOOP_INK: Color = Color::from_rgb(0.46, 0.64, 0.16);
const FOLDED_INK: Color = Color::from_rgb(0.92, 0.78, 0.20);
const DIGIT: &str = "0";
const NO_PART_NOTICE: &str = "Select a part to see its channels";
const NO_CHANNEL_NOTICE: &str = "This part has no channels";

const CHANNEL_INKS: [Color; 15] = [
    Color::from_rgb(0.34, 0.80, 0.86),
    Color::from_rgb(0.17, 0.24, 0.81),
    Color::from_rgb(0.69, 0.34, 0.86),
    Color::from_rgb(0.17, 0.31, 0.81),
    Color::from_rgb(0.63, 0.34, 0.86),
    Color::from_rgb(0.17, 0.38, 0.81),
    Color::from_rgb(0.57, 0.34, 0.86),
    Color::from_rgb(0.17, 0.45, 0.81),
    Color::from_rgb(0.51, 0.34, 0.86),
    Color::from_rgb(0.17, 0.52, 0.81),
    Color::from_rgb(0.45, 0.34, 0.86),
    Color::from_rgb(0.17, 0.59, 0.81),
    Color::from_rgb(0.40, 0.34, 0.86),
    Color::from_rgb(0.17, 0.67, 0.81),
    Color::from_rgb(0.34, 0.34, 0.86),
];

fn ink(kind: i32) -> Color {
    let at = usize::try_from(kind).unwrap_or(0) % CHANNEL_INKS.len();

    CHANNEL_INKS[at]
}

fn darker(color: Color, share: f32) -> Color {
    Color { r: color.r * share, g: color.g * share, b: color.b * share, a: color.a }
}

fn faded(color: Color, alpha: f32) -> Color {
    Color { a: color.a * alpha, ..color }
}

fn blended(over: Color, under: Color, share: f32) -> Color {
    let mix = |over: f32, under: f32| under + (over - under) * share;

    Color { r: mix(over.r, under.r), g: mix(over.g, under.g), b: mix(over.b, under.b), a: 1.0 }
}

fn card_span(label: &str) -> (f32, f32) {
    let wide = worded(label) + CARD_PAD * 2.0;

    (wide, LABEL_INSET + wide)
}

fn worded(label: &str) -> f32 {
    glyphs::columns(label) * CAPTION_SIZE * GLYPH_SHARE
}

fn fits(label: &str, room: f32) -> bool {
    worded(label) + CAPTION_ROOM <= room
}

fn gutter_of(lanes: &[Lane]) -> f32 {
    let widest = lanes.iter().map(|lane| worded(lane.label)).fold(0.0, f32::max);

    (widest + CARD_PAD * 2.0 + LABEL_INSET * 2.0).clamp(GUTTER_MIN, GUTTER_MAX)
}

fn stepped(span: f64, body: f32) -> f64 {
    let want = (f64::from(body / TICK_ROOM)).max(1.0);
    let raw = (span / want).max(1.0);
    let power = 10f64.powf(raw.log10().floor());

    let lifted = match raw / power {
        base if base <= 1.0 => 1.0,
        base if base <= 2.5 => 2.5,
        base if base <= 5.0 => 5.0,
        _ => 10.0,
    };

    (lifted * power).max(5.0)
}

pub(super) struct Lane {
    label: &'static str,
    beat: Beat,
    keys: Vec<i32>,
    track: usize,
    ink: Color,
    shadowed: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Window {
    from: f64,
    span: f64,
    lift: f32,
}

#[derive(Default)]
pub(super) struct State {
    framed: Option<(usize, Window)>,
}

impl State {
    pub(super) fn seat(&mut self, part: usize, window: Window) {
        self.framed = Some((part, window));
    }

    fn window(&self, part: usize, cadence: Cadence, lanes: &[Lane]) -> Window {
        match self.framed {
            Some((held, window)) if held == part => window,
            _ => resting(cadence, lanes),
        }
    }

    pub(super) fn view<'a>(
        &self,
        part: Option<usize>,
        lanes: Vec<Lane>,
        cadence: Cadence,
        playhead: i32,
        picked: Option<usize>,
    ) -> Element<'a, Message> {
        let Some(part) = part else {
            return framed(plain(), centred(NO_PART_NOTICE));
        };

        if lanes.is_empty() {
            return framed(plain(), centred(NO_CHANNEL_NOTICE));
        }

        let window = self.window(part, cadence, &lanes);
        let ruler = canvas_widget(Ruler { gutter: gutter_of(&lanes), window })
            .width(Length::Fill)
            .height(Length::Fill);

        let board = canvas_widget(Board {
            lanes,
            cadence,
            window,
            part,
            playhead: i64::from(playhead),
            picked,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        framed(ruler.into(), smooth_scroll(board).into())
    }
}

pub(super) fn lanes(doc: &Maanim, part: usize) -> Vec<Lane> {
    let Ok(wanted) = i32::try_from(part) else {
        return Vec::new();
    };

    let tracks = doc.tracks();

    let mut lanes: Vec<Lane> = tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| track.part == wanted)
        .filter_map(|(at, track)| {
            Some(Lane {
                label: kind_label(track.kind),
                beat: Beat::of(track)?,
                keys: track.keyframes.iter().map(|key| key.frame).collect(),
                track: at,
                ink: ink(track.kind),
                shadowed: false,
            })
        })
        .collect();

    let mut seen: u32 = 0;
    let mut stray: Vec<i32> = Vec::new();

    for lane in lanes.iter_mut().rev() {
        let Some(kind) = tracks.get(lane.track).map(|track| track.kind) else {
            continue;
        };

        lane.shadowed = match u32::try_from(kind).ok().filter(|held| *held < u32::BITS) {
            Some(held) => {
                let known = seen & (1 << held) != 0;
                seen |= 1 << held;
                known
            }
            None => {
                let known = stray.contains(&kind);

                if !known {
                    stray.push(kind);
                }

                known
            }
        };
    }

    lanes
}

fn authored(lanes: &[Lane]) -> (i64, i64) {
    let low = lanes.iter().map(|lane| i64::from(lane.beat.first)).min().unwrap_or(0);
    let high = lanes.iter().map(|lane| i64::from(lane.beat.last)).max().unwrap_or(0);

    (low, high.max(low))
}

fn resting(cadence: Cadence, lanes: &[Lane]) -> Window {
    let (low, high) = authored(lanes);
    let from = cadence.lead.min(low) as f64;
    let span = ((high as f64) - from).max(NARROWEST);
    let pad = span * MARGIN;

    Window { from: from - pad, span: span + pad * 2.0, lift: 0.0 }
}

pub(super) fn vacant<'a>(notice: &'a str) -> Element<'a, Message> {
    framed(plain(), centred(notice))
}

fn plain<'a>() -> Element<'a, Message> {
    row![text("Timeline").size(LABEL_SIZE)].align_y(Vertical::Center).into()
}

fn framed<'a>(head: Element<'a, Message>, body: Element<'a, Message>) -> Element<'a, Message> {
    let header = container(head)
        .width(Length::Fill)
        .height(Length::Fixed(KEY_HEAD_HEIGHT))
        .align_y(Vertical::Center)
        .padding(Padding { top: 0.0, right: KEY_ROW_INSET, bottom: 0.0, left: KEY_ROW_INSET })
        .style(theme::zebra_table_header);

    let card = console_card(column![header, body].width(Length::Fill).height(Length::Fill));

    editor::deflect(card, true)
}

struct Ruler {
    gutter: f32,
    window: Window,
}

impl canvas::Program<Message> for Ruler {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let ink = theme.extended_palette().background.strong.text;
        let body = (bounds.width - self.gutter).max(1.0);
        let middle = bounds.height / 2.0;

        frame.fill_text(canvas::Text {
            content: "Timeline".to_owned(),
            position: Point::new(LABEL_INSET, middle),
            color: faded(ink, TICK_TINT + 0.35),
            size: Pixels(CAPTION_SIZE),
            align_y: Vertical::Center,
            ..canvas::Text::default()
        });

        let step = stepped(self.window.span, body);
        let mut mark = (self.window.from / step).ceil() * step;

        while mark < self.window.from + self.window.span {
            let at = self.gutter + (((mark - self.window.from) / self.window.span) as f32) * body;
            let label = format!("{}", mark as i64);

            if at >= self.gutter && at <= bounds.width - worded(&label) / 2.0 {
                frame.fill(
                    &Path::rectangle(
                        Point::new(at - MARK_WIDTH / 2.0, bounds.height - TICK_HEIGHT),
                        Size::new(MARK_WIDTH, TICK_HEIGHT),
                    ),
                    faded(ink, TICK_TINT),
                );

                frame.fill_text(canvas::Text {
                    content: label,
                    position: Point::new(at, middle - 1.0),
                    color: faded(ink, TICK_TINT + 0.35),
                    size: Pixels(CAPTION_SIZE),
                    align_x: iced::advanced::text::Alignment::Center,
                    align_y: Vertical::Center,
                    ..canvas::Text::default()
                });
            }

            mark += step;
        }

        vec![frame.into_geometry()]
    }
}

struct Board {
    lanes: Vec<Lane>,
    cadence: Cadence,
    window: Window,
    part: usize,
    playhead: i64,
    picked: Option<usize>,
}

impl Lane {
    fn penned(&self, key: usize) -> (i64, i64) {
        let below = key.checked_sub(1).and_then(|at| self.keys.get(at));
        let above = self.keys.get(key + 1);

        (
            below.map_or(FLOOR, |frame| i64::from(*frame) + 1),
            above.map_or(CEILING, |frame| i64::from(*frame) - 1),
        )
    }
}

impl Board {
    fn gutter(&self) -> f32 {
        gutter_of(&self.lanes)
    }

    fn body(&self, width: f32) -> f32 {
        (width - self.gutter()).max(1.0)
    }

    fn at(&self, width: f32, frame: f64) -> f32 {
        self.gutter() + (((frame - self.window.from) / self.window.span) as f32) * self.body(width)
    }

    fn frame_at(&self, width: f32, x: f32) -> f64 {
        self.window.from + f64::from((x - self.gutter()) / self.body(width)) * self.window.span
    }

    fn tallest(&self, height: f32) -> f32 {
        (self.lanes.len() as f32 * LANE_HEIGHT - height).max(0.0)
    }

    fn lane_at(&self, at: Point) -> Option<usize> {
        let seated = at.y + self.window.lift;

        (seated >= 0.0).then(|| (seated / LANE_HEIGHT) as usize).filter(|at| *at < self.lanes.len())
    }

    fn boundary(&self, bounds: Rectangle, at: Point) -> Option<(usize, usize)> {
        if at.x < self.gutter() {
            return None;
        }

        let index = self.lane_at(at)?;
        let lane = self.lanes.get(index)?;
        let frame = self.frame_at(bounds.width, at.x);

        let (key, _) = lane
            .keys
            .iter()
            .enumerate()
            .map(|(key, held)| (key, (f64::from(*held) - frame).abs()))
            .min_by(|left, right| left.1.total_cmp(&right.1))?;

        let seated = self.at(bounds.width, f64::from(*lane.keys.get(key)?));

        ((seated - at.x).abs() <= GRAB).then_some((index, key))
    }

    fn reframed(&self, window: Window, height: f32) -> Message {
        let span = window.span.clamp(NARROWEST, self.widest());
        let (low, _) = authored(&self.lanes);
        let leeway = span * LEEWAY_SHARE;
        let least = (low.min(self.cadence.lead) as f64) - leeway;
        let most = ((self.cadence.extent as f64) + leeway - span).max(least);

        Message::Framed(
            self.part,
            Window {
                from: window.from.clamp(least, most),
                span,
                lift: window.lift.clamp(0.0, self.tallest(height)),
            },
        )
    }

    fn widest(&self) -> f64 {
        let (low, high) = authored(&self.lanes);
        let reach = (self.cadence.extent - low.min(self.cadence.lead)).max(high - low).max(1);

        (reach as f64) * WIDEST_SHARE
    }
}

impl canvas::Program<Message> for Board {
    type State = Grip;

    fn draw(
        &self,
        _grip: &Grip,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let palette = theme.extended_palette();
        let width = bounds.width;
        let gutter = self.gutter();
        let near = |at: f32| at.clamp(-width, width * 2.0);

        let low = self.window.from.floor() as i64 - 1;
        let high = (self.window.from + self.window.span).ceil() as i64 + 1;
        let reach = high;

        for (index, lane) in self.lanes.iter().enumerate() {
            let top = index as f32 * LANE_HEIGHT - self.window.lift;

            if top + LANE_HEIGHT < 0.0 || top > bounds.height {
                continue;
            }

            let stripe = match (index % 2, self.picked == Some(lane.track)) {
                (_, true) => palette.primary.weak.color,
                (0, _) => palette.background.base.color,
                _ => palette.background.weak.color,
            };

            frame.fill(&Path::rectangle(Point::new(0.0, top), Size::new(width, LANE_HEIGHT)), stripe);

            let dim = match lane.shadowed {
                true => SHADOW_TINT,
                false => 1.0,
            };

            let body = top + LANE_INSET;
            let tall = (LANE_HEIGHT - LANE_INSET * 2.0).max(1.0);
            let outline = |frame: &mut canvas::Frame, block: &Path, alpha: f32| {
                frame.fill(block, faded(lane.ink, alpha));
                frame.stroke(
                    block,
                    Stroke::default()
                        .with_color(faded(darker(lane.ink, DARKEN_EDGE), alpha))
                        .with_width(OUTLINE),
                );
            };

            let (_, card_right) = card_span(lane.label);
            let clip = card_right - CARD_BLEED;

            if let Some(settles) = lane.beat.settles().filter(|settles| *settles < reach) {
                let start = near(self.at(width, settles as f64)).max(clip);
                let end = near(self.at(width, reach as f64)).max(clip);

                frame.fill(
                    &Path::rectangle(
                        Point::new(start, top + (LANE_HEIGHT - HELD_HEIGHT) / 2.0),
                        Size::new((end - start).max(0.0), HELD_HEIGHT),
                    ),
                    faded(lane.ink, GHOST_TINT * dim),
                );
            }

            if lane.keys.len() < 2 {
                let tick = near(self.at(width, f64::from(lane.beat.first)));

                if tick >= card_right {
                    outline(
                        &mut frame,
                        &Path::rounded_rectangle(
                            Point::new(tick - SEPARATOR, body),
                            Size::new(SEPARATOR * 2.0, tall),
                            Radius::from(SEPARATOR),
                        ),
                        dim,
                    );
                }

                continue;
            }

            let stride = (f64::from(lane.beat.span) / self.window.span) as f32 * self.body(width);
            let grain = stride / (lane.keys.len().saturating_sub(1)).max(1) as f32;

            if stride < MIN_PASS {
                let stop = lane.beat.settles().unwrap_or(reach).min(reach);
                let start = near(self.at(width, f64::from(lane.beat.first))).max(clip);
                let end = near(self.at(width, stop as f64)).max(clip);

                frame.fill(
                    &Path::rectangle(Point::new(start, body), Size::new((end - start).max(0.0), tall)),
                    faded(lane.ink, GHOST_TINT * dim),
                );

                continue;
            }

            for pass in lane.beat.repeats(low, reach) {
                let shift = lane.beat.shifted(pass) as f64;
                let alpha = match pass {
                    0 => dim,
                    _ => GHOST_TINT * dim,
                };

                let opens = self.at(width, f64::from(lane.beat.first) + shift);
                let closes = self.at(width, f64::from(lane.beat.last) + shift);

                if closes < card_right || opens > width {
                    continue;
                }

                let met = opens < card_right;
                let (opens, closes) = (near(opens).max(clip), near(closes));
                let round = Radius::from(theme::RADIUS_SM);
                let block = Path::rounded_rectangle(
                    Point::new(opens, body),
                    Size::new((closes - opens).max(1.0), tall),
                    match met {
                        true => Radius { top_left: 0.0, bottom_left: 0.0, ..round },
                        false => round,
                    },
                );

                outline(&mut frame, &block, alpha);

                if grain < GRAIN_FLOOR {
                    continue;
                }

                for (step, pair) in lane.keys.windows(2).enumerate() {
                    let start = self.at(width, f64::from(pair[0]) + shift);
                    let end = self.at(width, f64::from(pair[1]) + shift);

                    if end < card_right || start > width {
                        continue;
                    }

                    if step > 0 && start > card_right {
                        frame.fill(
                            &Path::rectangle(
                                Point::new(start - SEPARATOR / 2.0, body),
                                Size::new(SEPARATOR, tall),
                            ),
                            faded(darker(lane.ink, DARKEN_SEAM), alpha),
                        );
                    }

                    let room = end - start;

                    if !fits(DIGIT, room) {
                        continue;
                    }

                    let caption = pair[0].to_string();
                    let middle = (start + end) / 2.0;
                    let reaches = middle - worded(&caption) / 2.0 - CAPTION_ROOM;

                    if fits(&caption, room) && reaches > card_right && middle < width {
                        frame.fill_text(canvas::Text {
                            content: caption,
                            position: Point::new(middle, top + LANE_HEIGHT / 2.0),
                            color: faded(Color::WHITE, alpha),
                            size: Pixels(CAPTION_SIZE),
                            align_x: iced::advanced::text::Alignment::Center,
                            align_y: Vertical::Center,
                            ..canvas::Text::default()
                        });
                    }
                }
            }
        }

        for (index, lane) in self.lanes.iter().enumerate() {
            let top = index as f32 * LANE_HEIGHT - self.window.lift;

            if top + LANE_HEIGHT < 0.0 || top > bounds.height {
                continue;
            }

            let (card_width, card_right) = card_span(lane.label);

            if card_right + LABEL_INSET > gutter {
                continue;
            }

            let stripe = match (index % 2, self.picked == Some(lane.track)) {
                (_, true) => palette.primary.weak.color,
                (0, _) => palette.background.base.color,
                _ => palette.background.weak.color,
            };

            let dim = match lane.shadowed {
                true => SHADOW_TINT,
                false => 1.0,
            };

            let card = Path::rounded_rectangle(
                Point::new(LABEL_INSET, top + LANE_INSET),
                Size::new(card_width, (LANE_HEIGHT - LANE_INSET * 2.0).max(1.0)),
                Radius::from(theme::RADIUS_SM),
            );

            frame.fill(&card, blended(lane.ink, stripe, dim));
            frame.stroke(
                &card,
                Stroke::default()
                    .with_color(blended(darker(lane.ink, DARKEN_EDGE), stripe, dim))
                    .with_width(OUTLINE),
            );

            frame.fill_text(canvas::Text {
                content: lane.label.to_owned(),
                position: Point::new(LABEL_INSET + card_width / 2.0, top + LANE_HEIGHT / 2.0),
                color: faded(Color::WHITE, dim),
                size: Pixels(CAPTION_SIZE),
                align_x: iced::advanced::text::Alignment::Center,
                align_y: Vertical::Center,
                ..canvas::Text::default()
            });
        }

        let rule = |frame: &mut canvas::Frame, at: f32, color: Color, thick: f32| {
            let at = near(at);

            if at < gutter {
                return;
            }

            frame.fill(
                &Path::rectangle(Point::new(at - thick / 2.0, 0.0), Size::new(thick, bounds.height)),
                color,
            );
        };

        if let Cycle::Every(_) = self.cadence.cycle {
            rule(&mut frame, self.at(width, self.cadence.settled as f64), LOOP_INK, MARK_WIDTH);
        }

        rule(&mut frame, self.at(width, 0.0), START_INK, MARK_WIDTH);

        let folded = self.cadence.fold(self.playhead);

        let ink = match folded == self.playhead {
            true => palette.danger.base.color,
            false => FOLDED_INK,
        };

        rule(&mut frame, self.at(width, folded as f64), ink, PLAYHEAD_WIDTH);

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        grip: &mut Grip,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let iced::Event::Mouse(mouse) = event else {
            return None;
        };

        match mouse {
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let at = cursor.position_in(bounds)?;

                *grip = Grip { held: Some(at), from: Some(self.window), ..Grip::default() };

                Some(canvas::Action::request_redraw().and_capture())
            }
            mouse::Event::ButtonPressed(mouse::Button::Right) => {
                let at = cursor.position_in(bounds)?;

                *grip = Grip {
                    held: Some(at),
                    aimed: self.lane_at(at),
                    sliding: self.boundary(bounds, at),
                    ..Grip::default()
                };

                let lane = grip.sliding.map(|(lane, _)| lane).or(grip.aimed)?;

                Some(canvas::Action::publish(Message::Pick(self.lanes.get(lane)?.track)).and_capture())
            }
            mouse::Event::CursorMoved { position } => {
                let at = Point::new(position.x - bounds.x, position.y - bounds.y);

                if let Some(held) = grip.held {
                    grip.walked = grip.walked.max((at.x - held.x).abs()).max((at.y - held.y).abs());
                }

                if let Some((lane, key)) = grip.sliding {
                    let held = self.lanes.get(lane)?;
                    let (low, high) = held.penned(key);

                    if low > high {
                        return Some(canvas::Action::capture());
                    }

                    if !std::mem::replace(&mut grip.pinned, true) {
                        return Some(
                            canvas::Action::publish(Message::Framed(self.part, self.window))
                                .and_capture(),
                        );
                    }

                    let reach = at.x.clamp(self.gutter(), bounds.width);
                    let landed = self.frame_at(bounds.width, reach).round() as i64;
                    let landed = landed.clamp(low, high);

                    if held.keys.get(key).is_some_and(|frame| i64::from(*frame) == landed) {
                        return Some(canvas::Action::capture());
                    }

                    return Some(
                        canvas::Action::publish(Message::Changed(key, Field::Frame, landed.to_string()))
                            .and_capture(),
                    );
                }

                let (Some(held), Some(from)) = (grip.held, grip.from) else {
                    return None;
                };

                let shifted = f64::from(-(at.x - held.x) / self.body(bounds.width)) * from.span;
                let raised = from.lift - (at.y - held.y);

                Some(
                    canvas::Action::publish(self.reframed(
                        Window { from: from.from + shifted, lift: raised, ..from },
                        bounds.height,
                    ))
                    .and_capture(),
                )
            }
            mouse::Event::ButtonReleased(button) => {
                let aimed = grip.sliding.take().is_some() || grip.aimed.take().is_some();
                let held = grip.held.take();

                grip.from = None;

                let walked = std::mem::take(&mut grip.walked);
                let at = held.filter(|_| !aimed && walked < CLICK_SLOP)?;

                if *button != mouse::Button::Left || at.x < self.gutter() {
                    return Some(canvas::Action::capture());
                }

                let landed = self.frame_at(bounds.width, at.x);

                Some(canvas::Action::publish(Message::Scrub(landed.round() as i32)).and_capture())
            }
            mouse::Event::CursorLeft => {
                *grip = Grip::default();

                None
            }
            mouse::Event::WheelScrolled { delta } => {
                let at = cursor.position_in(bounds).filter(|at| at.x >= self.gutter())?;
                let step = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / LINE_PIXELS,
                };

                let share = f64::from((at.x - self.gutter()) / self.body(bounds.width));
                let anchor = self.window.from + share * self.window.span;
                let span = (self.window.span / ZOOM_RATE.powf(f64::from(step))).clamp(NARROWEST, self.widest());

                Some(
                    canvas::Action::publish(self.reframed(
                        Window { from: anchor - share * span, span, lift: self.window.lift },
                        bounds.height,
                    ))
                    .and_capture(),
                )
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        grip: &Grip,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if grip.sliding.is_some() {
            return mouse::Interaction::ResizingHorizontally;
        }

        let Some(at) = cursor.position_in(bounds).filter(|at| at.x >= self.gutter()) else {
            return mouse::Interaction::default();
        };

        match (grip.held.is_some(), self.boundary(bounds, at).is_some()) {
            (true, _) => mouse::Interaction::Grabbing,
            (_, true) => mouse::Interaction::ResizingHorizontally,
            _ => mouse::Interaction::Grab,
        }
    }
}

#[derive(Default)]
pub(super) struct Grip {
    held: Option<Point>,
    from: Option<Window>,
    walked: f32,
    sliding: Option<(usize, usize)>,
    aimed: Option<usize>,
    pinned: bool,
}

