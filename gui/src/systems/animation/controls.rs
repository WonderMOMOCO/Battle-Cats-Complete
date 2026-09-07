use iced::widget::{button, column, container, mouse_area, opaque, pick_list, row, rule, scrollable, text, text_input, Button, Container, TextInput};
use iced::border::Radius;
use iced::{alignment, border, font, Background, Border, Color, Element, Font, Length, Padding, Theme, Vector};

use crate::app::state::AnimState;
use crate::app::theme;
use crate::common::fonts;
use crate::widget::{slide, smooth_scroll, Slide};

use super::{canvas, data};

pub(crate) const HOLD_DELAY_SECS: f32 = 0.2;
const TICK_SECS: f32 = 1.0 / 60.0;

const TILE_HEIGHT: f32 = 28.0;
const GAP: f32 = 4.0;
const ROW_GAP: f32 = 3.0;
const ICON_W: f32 = 60.0;
const NAV_W: f32 = 30.0;
const FRAME_INPUT_W: f32 = 80.0;
const RANGE_W: f32 = 60.0;
const TILDE_W: f32 = 20.0;
const COL2_W: f32 = NAV_W * 2.0 + FRAME_INPUT_W + GAP * 2.0;
const COL3_W: f32 = 100.0;
const OFFSET_PADDING: [f32; 2] = [4.0, 6.0];
const DIVIDER_W: f32 = 10.0;
const PANEL_WIDTH: f32 = ICON_W + COL2_W + COL3_W + DIVIDER_W * 2.0 + GAP * 4.0;

const ANIM_BUTTON_W: f32 = 70.0;
const ANIM_BUTTON_H: f32 = 25.0;
const GRID_GAP: f32 = 5.0;
const GRID_PAD: f32 = 2.0;
const GRID_ROWS: usize = 2;
const SCROLLBAR_W: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 2.0;

const TOGGLE_HEIGHT: f32 = 18.0;
const TOGGLE_TEXT_SIZE: f32 = 14.0;
const PLAY_TEXT_SIZE: f32 = 16.0;
const TILE_TEXT_SIZE: f32 = 14.0;
const ANIM_TEXT_STEPS: [f32; 6] = [13.0, 12.0, 11.0, 10.0, 9.0, 8.0];
const ANIM_TEXT_FLOOR: f32 = 8.0;
const ANIM_LABEL_INSET: f32 = 2.0;
const GLYPH_ADVANCE: f32 = 0.52;
const LINE_HEIGHT: f32 = 1.1;

const PANEL_PAD: f32 = 8.0;
const PANEL_ALPHA: f32 = 160.0 / 255.0;
const PANEL_SHADE: f32 = 0.15;
const DISABLED_ALPHA: f32 = 0.5;
const ALARM_TINT: f32 = 0.3;
const ALARM_EDGE: f32 = 1.0;

const PLAY_GLYPH: &str = "\u{25B6}";
const PAUSE_GLYPH: &str = "\u{25AE}\u{25AE}";

#[derive(Default)]
pub struct State {
    range_start_input: String,
    range_end_input: String,
    hold_dir: i8,
    hold_timer: f32,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePlay,
    ToggleExpanded,
    HoldStart(i8),
    HoldEnd,
    HoldCancel,
    FrameInputChanged(String),
    OffsetSelected(String),
    RangeStartChanged(String),
    RangeEndChanged(String),
    SelectAnimation(usize),
    ResetPan,
    OpenExport,
}

fn loop_max(data: &data::State) -> Option<f32> {
    data.loop_bound().map(|v| v.max(0) as f32)
}

fn display_max(data: &data::State) -> String {
    match data.loop_bound() {
        Some(value) if value > 999_999 => "???".to_string(),
        Some(value) => value.to_string(),
        None => "???".to_string(),
    }
}

fn frame_text(canvas: &canvas::State) -> String {
    (canvas.current_frame.trunc() as i32).to_string()
}

fn frame_bounds(canvas: &canvas::State, data: &data::State) -> (f32, Option<f32>) {
    if data.is_model() {
        return (0.0, Some(0.0));
    }

    let min = canvas.loop_start.unwrap_or(0.0).max(0.0);
    let max = canvas.loop_end.or_else(|| loop_max(data)).map(|max| max.max(min));

    (min, max)
}

pub(super) fn clamp_frame(canvas: &mut canvas::State, data: &data::State) {
    let (min, max) = frame_bounds(canvas, data);
    canvas.current_frame = max.map_or(canvas.current_frame.max(min), |max| canvas.current_frame.clamp(min, max));
}

fn effective_max(canvas: &canvas::State, data: &data::State) -> String {
    if data.is_model() {
        return "0".to_string();
    }

    canvas.loop_end.map_or_else(|| display_max(data), |end| (end.trunc() as i32).to_string())
}

fn bold() -> font::Font {
    font::Font { weight: font::Weight::Bold, ..font::Font::DEFAULT }
}

fn panel_style(t: &Theme) -> container::Style {
    let palette = t.palette();
    let shade = |c: f32| c * PANEL_SHADE;

    container::Style {
        background: Some(Background::Color(Color {
            r: shade(palette.background.r),
            g: shade(palette.background.g),
            b: shade(palette.background.b),
            a: PANEL_ALPHA,
        })),
        border: Border {
            color: t.extended_palette().background.strong.color,
            width: 1.0,
            radius: Radius {
                top_left: theme::RADIUS_LG,
                top_right: theme::RADIUS_LG,
                bottom_left: 0.0,
                bottom_right: 0.0,
            },
        },
        ..container::Style::default()
    }
}

fn tile_style(t: &Theme) -> container::Style {
    let palette = t.extended_palette();

    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: theme::RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

fn step_style(t: &Theme, enabled: bool) -> container::Style {
    let pair = t.extended_palette().background.strong;
    let alpha = if enabled { 1.0 } else { DISABLED_ALPHA };

    container::Style {
        background: Some(Background::Color(Color { a: alpha, ..pair.color })),
        text_color: Some(Color { a: alpha, ..pair.text }),
        border: border::rounded(theme::RADIUS_SM),
        ..container::Style::default()
    }
}

fn input_style(t: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        ..text_input::default(t, status)
    }
}

fn control_style(t: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let base = theme::toggle_button(t, status, is_active);

    if status != button::Status::Disabled {
        return base;
    }

    button::Style {
        background: base.background.map(|background| match background {
            Background::Color(color) => Background::Color(Color { a: color.a * DISABLED_ALPHA, ..color }),
            other => other,
        }),
        text_color: Color { a: DISABLED_ALPHA, ..base.text_color },
        ..base
    }
}

fn clip_style(t: &Theme, status: button::Status, is_active: bool, alarmed: bool) -> button::Style {
    let base = control_style(t, status, is_active);

    if !alarmed {
        return base;
    }

    let palette = t.palette();
    let tinted = Color { a: ALARM_TINT, ..palette.danger };

    button::Style {
        background: Some(Background::Color(tinted)),
        border: Border { color: palette.danger, width: ALARM_EDGE, ..base.border },
        ..base
    }
}

fn tile<'a>(width: f32, content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content)
        .width(Length::Fixed(width))
        .height(Length::Fixed(TILE_HEIGHT))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(tile_style)
}

fn label_tile<'a>(width: f32, label: impl text::IntoFragment<'a>) -> Container<'a, Message> {
    tile(width, text(label).size(TILE_TEXT_SIZE))
}

fn tile_input<'a>(
    placeholder: &str,
    value: &str,
    enabled: bool,
    on_input: impl Fn(String) -> Message + 'a,
) -> TextInput<'a, Message> {
    let input = text_input(placeholder, value)
        .width(Length::Fill)
        .padding(0)
        .size(TILE_TEXT_SIZE)
        .align_x(alignment::Horizontal::Center)
        .style(input_style);

    if enabled { input.on_input(on_input) } else { input }
}

fn control_button<'a>(label: &'a str, font: Font, size: f32, width: f32, on_press: Option<Message>) -> Button<'a, Message> {
    button(text(label).font(font).size(size).width(Length::Fill).height(Length::Fill).center())
        .width(Length::Fixed(width))
        .height(Length::Fixed(TILE_HEIGHT))
        .padding(0)
        .on_press_maybe(on_press)
        .style(|t: &Theme, status| control_style(t, status, false))
}

fn step_control<'a>(label: &'a str, direction: i8, enabled: bool) -> Element<'a, Message> {
    let face = container(text(label).font(fonts::MISC_SYMBOLS).size(TILE_TEXT_SIZE))
        .width(Length::Fixed(NAV_W))
        .height(Length::Fixed(TILE_HEIGHT))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |t: &Theme| step_style(t, enabled));

    if !enabled {
        return face.into();
    }

    mouse_area(face)
        .on_press(Message::HoldStart(direction))
        .on_release(Message::HoldEnd)
        .on_exit(Message::HoldCancel)
        .into()
}

fn divider<'a>() -> Container<'a, Message> {
    container(rule::vertical(1))
        .width(Length::Fixed(DIVIDER_W))
        .height(Length::Fixed(TILE_HEIGHT * 2.0 + GAP))
        .align_x(alignment::Horizontal::Center)
}

fn vacant_tile<'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Length::Fixed(ANIM_BUTTON_W))
        .height(Length::Fixed(ANIM_BUTTON_H))
        .style(|t: &Theme| {
            let base = theme::toggle_button(t, button::Status::Active, false);

            container::Style {
                background: base.background,
                text_color: Some(base.text_color),
                border: base.border,
                ..container::Style::default()
            }
        })
        .into()
}

fn fitted_size(label: &str) -> f32 {
    let room_w = ANIM_BUTTON_W - ANIM_LABEL_INSET * 2.0;
    let room_h = ANIM_BUTTON_H - ANIM_LABEL_INSET * 2.0;
    let glyphs = label.chars().count();

    ANIM_TEXT_STEPS
        .into_iter()
        .find(|size| {
            let per_line = (room_w / (size * GLYPH_ADVANCE)).floor().max(1.0);
            let lines = (room_h / (size * LINE_HEIGHT)).floor().max(1.0);
            glyphs as f32 <= per_line * lines
        })
        .unwrap_or(ANIM_TEXT_FLOOR)
}

fn clip_tile(data: &data::State, index: usize, alarmed: bool) -> Element<'_, Message> {
    let Some(clip) = data.clip(index) else {
        return vacant_tile();
    };

    let is_active = data.selected() == Some(index);
    let label = clip.label();
    let size = fitted_size(&label);

    let face = container(
        text(label)
            .size(size)
            .line_height(text::LineHeight::Relative(LINE_HEIGHT))
            .wrapping(text::Wrapping::WordOrGlyph)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .clip(true);

    button(face)
        .width(Length::Fixed(ANIM_BUTTON_W))
        .height(Length::Fixed(ANIM_BUTTON_H))
        .padding(ANIM_LABEL_INSET)
        .on_press(Message::SelectAnimation(index))
        .style(move |t: &Theme, status| clip_style(t, status, is_active, alarmed))
        .into()
}

fn anim_grid<'a>(data: &'a data::State, alarms: &[usize]) -> Element<'a, Message> {
    let slots = data.slots();
    let mut grid = column![].spacing(GRID_GAP);

    for chunk in slots.chunks(data::COLUMNS) {
        let mut buttons = row![].spacing(GRID_GAP);

        for slot in chunk {
            buttons = buttons.push(
                slot.map_or_else(vacant_tile, |index| clip_tile(data, index, alarms.contains(&index))),
            );
        }

        grid = grid.push(buttons);
    }

    if slots.len() <= data::COLUMNS * GRID_ROWS {
        return grid.into();
    }

    let visible = ANIM_BUTTON_H * GRID_ROWS as f32 + GRID_GAP * (GRID_ROWS - 1) as f32;
    let bar = scrollable::Scrollbar::new().width(SCROLLBAR_W).scroller_width(SCROLLBAR_W).margin(SCROLLBAR_MARGIN);

    smooth_scroll(
        scrollable(container(grid).width(Length::Fill).align_x(alignment::Horizontal::Center))
            .direction(scrollable::Direction::Vertical(bar))
            .height(Length::Fixed(visible)),
    )
    .into()
}

impl State {
    pub fn update(&mut self, message: Message, canvas: &mut canvas::State, data: &mut data::State, anim_state: &mut AnimState) {
        match message {
            Message::TogglePlay => canvas.is_playing = !canvas.is_playing,
            Message::ToggleExpanded => anim_state.controls_expanded = !anim_state.controls_expanded,
            Message::HoldStart(dir) => {
                self.hold_dir = dir;
                self.hold_timer = 0.0;
            }
            Message::HoldEnd => {
                if self.hold_dir != 0 && self.hold_timer <= HOLD_DELAY_SECS {
                    self.step(canvas, data, self.hold_dir as f32);
                }
                self.hold_dir = 0;
                self.hold_timer = 0.0;
            }
            Message::HoldCancel => {
                self.hold_dir = 0;
                self.hold_timer = 0.0;
            }
            Message::FrameInputChanged(value) => {
                let trimmed = value.trim();
                let target = if trimmed.is_empty() { Some(0.0) } else { trimmed.parse::<f32>().ok() };

                if let Some(target) = target {
                    canvas.current_frame = target;
                    clamp_frame(canvas, data);
                }
            }
            Message::OffsetSelected(label) => {
                data.select_offset(&label);
                anim_state.placement = data.selected_offset();
            }
            Message::RangeStartChanged(value) => {
                if value.is_empty() {
                    canvas.loop_start = None;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    let start = parsed.max(0.0);
                    canvas.loop_start = Some(start);
                    if canvas.current_frame < start {
                        canvas.current_frame = start;
                    }
                }
                self.range_start_input = value;
            }
            Message::RangeEndChanged(value) => {
                if value.is_empty() {
                    canvas.loop_end = None;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    let end = parsed.max(0.0);
                    canvas.loop_end = Some(end);
                    if canvas.current_frame > end {
                        canvas.current_frame = end;
                    }
                }
                self.range_end_input = value;
            }
            Message::SelectAnimation(index) => {
                data.select(index);
                canvas.current_frame = 0.0;
            }
            Message::ResetPan => canvas.pan = Vector::new(0.0, 0.0),
            Message::OpenExport => {}
        }
    }

    fn step(&mut self, canvas: &mut canvas::State, data: &data::State, direction: f32) {
        let (min, max) = frame_bounds(canvas, data);
        let stepped = canvas.current_frame + direction;

        canvas.current_frame = if stepped < min {
            max.unwrap_or(min)
        } else if max.is_some_and(|max| stepped > max) {
            min
        } else {
            stepped
        };
    }

    pub fn holding(&self) -> bool {
        self.hold_dir != 0
    }

    pub fn set_range(&mut self, start: f32, end: f32) {
        self.range_start_input = (start.trunc() as i32).to_string();
        self.range_end_input = (end.trunc() as i32).to_string();
    }

    pub fn tick(&mut self, canvas: &mut canvas::State, data: &data::State) {
        if self.hold_dir == 0 {
            self.hold_timer = 0.0;
            return;
        }

        self.hold_timer += TICK_SECS;
        if self.hold_timer <= HOLD_DELAY_SECS {
            return;
        }

        let delta = self.hold_dir as f32 * TICK_SECS * 30.0;
        self.step(canvas, data, delta);
    }

    pub fn view<'a>(&'a self, canvas: &'a canvas::State, data: &'a data::State, anim_state: &AnimState, alarms: &[usize]) -> Element<'a, Message> {
        let is_expanded = anim_state.controls_expanded;
        let expand_icon = if is_expanded { "▼" } else { "▲" };

        let toggle = button(
            text(expand_icon)
                .size(TOGGLE_TEXT_SIZE)
                .font(bold())
                .width(Length::Fill)
                .height(Length::Fill)
                .center(),
        )
        .width(Length::Fill)
        .height(Length::Fixed(TOGGLE_HEIGHT))
        .padding(0)
        .on_press(Message::ToggleExpanded)
        .style(button::text);

        let body = column![
            rule::horizontal(1),
            container(anim_grid(data, alarms))
                .width(Length::Fill)
                .padding(Padding { top: GRID_PAD, right: 0.0, bottom: GRID_PAD, left: 0.0 })
                .align_x(alignment::Horizontal::Center),
            rule::horizontal(1),
            container(self.transport_row(canvas, data))
                .padding(Padding { top: GRID_PAD, right: 0.0, bottom: 0.0, left: 0.0 }),
        ]
        .spacing(ROW_GAP)
        .width(Length::Fill);

        let drawer = container(body)
            .width(Length::Fill)
            .padding(Padding { top: ROW_GAP, ..Padding::ZERO });
        let panel = column![toggle, slide(drawer, is_expanded, Slide::Down)].width(Length::Fixed(PANEL_WIDTH));

        opaque(container(panel).padding(PANEL_PAD).style(panel_style))
    }

    fn transport_row<'a>(&'a self, canvas: &'a canvas::State, data: &'a data::State) -> Element<'a, Message> {
        let base_available = data.held_unit.is_some();
        let anim_loaded = base_available && data.current_clip().is_some();

        let play_icon = if canvas.is_playing { PAUSE_GLYPH } else { PLAY_GLYPH };

        let transport = column![
            control_button(play_icon, fonts::MISC_SYMBOLS, PLAY_TEXT_SIZE, ICON_W, anim_loaded.then_some(Message::TogglePlay)),
            control_button("Orient", Font::DEFAULT, TILE_TEXT_SIZE, ICON_W, base_available.then_some(Message::ResetPan)),
        ]
        .spacing(GAP);

        let frame_row: Element<'a, Message> = if canvas.is_playing {
            row![
                tile(RANGE_W, tile_input("0", &self.range_start_input, anim_loaded, Message::RangeStartChanged)),
                label_tile(TILDE_W, "~"),
                tile(RANGE_W, tile_input(&display_max(data), &self.range_end_input, anim_loaded, Message::RangeEndChanged)),
            ]
            .spacing(GAP)
            .into()
        } else {
            let frame_input = tile_input("0", &frame_text(canvas), anim_loaded, Message::FrameInputChanged);

            row![
                step_control("◀", -1, anim_loaded),
                tile(FRAME_INPUT_W, frame_input),
                step_control("▶", 1, anim_loaded),
            ]
            .spacing(GAP)
            .into()
        };

        let counter_row = row![
            label_tile(RANGE_W, frame_text(canvas)),
            label_tile(TILDE_W, "/"),
            label_tile(RANGE_W, effective_max(canvas, data)),
        ]
        .spacing(GAP);

        let playback = column![frame_row, counter_row].spacing(GAP);

        let choices = data.offset_choices();
        let offset_row = pick_list(choices, Some(data.offset_label()), |label| Message::OffsetSelected(label.to_string()))
            .width(Length::Fixed(COL3_W))
            .padding(OFFSET_PADDING)
            .text_size(TILE_TEXT_SIZE)
            .style(theme::combo_box)
            .menu_style(theme::combo_box_menu);

        let slot = control_button(
            "Export",
            Font::DEFAULT,
            TILE_TEXT_SIZE,
            COL3_W,
            base_available.then_some(Message::OpenExport),
        );

        let output = column![slot, offset_row]
        .spacing(GAP);

        row![transport, divider(), playback, divider(), output]
            .spacing(GAP)
            .align_y(alignment::Vertical::Center)
            .into()
    }
}
