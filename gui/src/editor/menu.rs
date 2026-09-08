use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::alignment;
use iced::widget::{button, container, mouse_area, row, text as text_widget, tooltip, Column, Space};
use iced::{Border, Element, Length, Padding, Pixels, Size, Theme};

use crate::app::theme;
use crate::common::feedback::{CONFIRM_LABEL, CONFIRM_SHORT_LABEL, FAILURE_LABEL};
use crate::common::fonts;
use crate::common::glyphs;

use super::{Item, Message, Trail};

pub(super) const SCALE: f32 = 0.85;

const TEXT_SIZE: f32 = 13.0 * SCALE;
const FRAME_PADDING: f32 = 4.0 * SCALE;
const ITEM_PADDING_X: f32 = 10.0 * SCALE;
const ITEM_PADDING_Y: f32 = 5.0 * SCALE;
const BORDER_WIDTH: f32 = 1.0;
const WRAPPING: text::Wrapping = text::Wrapping::None;
const SAFETY: f32 = 1.0;
const TOOLTIP_PADDING: f32 = 8.0 * SCALE;
const TOOLTIP_GAP: f32 = FRAME_PADDING + BORDER_WIDTH;
const FRAME_SHADE: f32 = 0.35;
const ARROW: &str = "▶";
const ARROW_GAP: f32 = 12.0 * SCALE;

type Paragraph = <iced::Renderer as text::Renderer>::Paragraph;

#[derive(Clone, Copy)]
pub(super) struct Marks<'a> {
    pub(super) armed: Option<&'a [usize]>,
    pub(super) failed: Option<&'a [usize]>,
}

#[derive(Clone, Copy)]
enum Mark {
    Idle,
    Armed,
    Failed,
}

impl Marks<'_> {
    fn at(self, trail: &[usize]) -> Mark {
        if self.armed.is_some_and(|slot| slot == trail) {
            return Mark::Armed;
        }

        if self.failed.is_some_and(|slot| slot == trail) {
            return Mark::Failed;
        }

        Mark::Idle
    }
}

impl Mark {
    fn label(self, idle: &str, terse: bool) -> &str {
        match self {
            Mark::Armed if terse => CONFIRM_SHORT_LABEL,
            Mark::Armed => CONFIRM_LABEL,
            Mark::Failed => FAILURE_LABEL,
            Mark::Idle => idle,
        }
    }

    fn alerting(self) -> bool {
        !matches!(self, Mark::Idle)
    }
}

pub(super) fn measure(renderer: &iced::Renderer, items: &[Item]) -> Size {
    let frame = FRAME_PADDING * 2.0 + BORDER_WIDTH * 2.0;

    let (width, height) = items.iter().fold((0.0_f32, 0.0_f32), |(width, height), item| {
        let bounds = label_bounds(renderer, &item.label);
        let confirm = if item.terse { CONFIRM_SHORT_LABEL } else { CONFIRM_LABEL };
        let armed = if item.confirm { label_bounds(renderer, confirm).width } else { 0.0 };
        let failed = if item.action.is_some() { label_bounds(renderer, FAILURE_LABEL).width } else { 0.0 };
        let arrow = if item.opens() { ARROW_GAP + TEXT_SIZE } else { 0.0 };

        (width.max(bounds.width + arrow).max(armed).max(failed), height + row_height(bounds.height))
    });

    Size::new(width + ITEM_PADDING_X * 2.0 + frame + SAFETY, height + frame)
}

fn row_height(label: f32) -> f32 {
    label + ITEM_PADDING_Y * 2.0
}

pub(super) fn row_offset(renderer: &iced::Renderer, items: &[Item], index: usize) -> f32 {
    let rows: f32 = items
        .iter()
        .take(index)
        .map(|item| row_height(label_bounds(renderer, &item.label).height))
        .sum();

    rows + FRAME_PADDING + BORDER_WIDTH
}

fn label_bounds(renderer: &iced::Renderer, label: &str) -> Size {
    Paragraph::with_text(text::Text {
        content: label,
        bounds: Size::INFINITE,
        size: Pixels(TEXT_SIZE),
        line_height: text::LineHeight::default(),
        font: renderer.default_font(),
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: glyphs::shaping(label),
        wrapping: WRAPPING,
    })
    .min_bounds()
}

pub(super) fn view<'a, M: Clone + 'a>(
    items: &'a [Item],
    marks: Marks<'_>,
    prefix: &[usize],
    to_message: fn(Message) -> M,
) -> Element<'a, M> {
    let mut column = Column::with_capacity(items.len()).width(Length::Fill);

    for (index, item) in items.iter().enumerate() {
        let mut trail = Trail::with_capacity(prefix.len() + 1);

        trail.extend_from_slice(prefix);
        trail.push(index);

        column = column.push(entry(item, trail, marks, to_message));
    }

    container(column).width(Length::Fill).padding(FRAME_PADDING).style(frame).into()
}

fn entry<'a, M: Clone + 'a>(
    item: &'a Item,
    trail: Trail,
    marks: Marks<'_>,
    to_message: fn(Message) -> M,
) -> Element<'a, M> {
    let mark = marks.at(&trail);

    let shown = mark.label(item.label.as_str(), item.terse);
    let label = text_widget(shown).size(TEXT_SIZE).shaping(glyphs::shaping(shown)).wrapping(WRAPPING);

    let content: Element<'a, M> = if item.opens() {
        row![
            label,
            Space::new().width(Length::Fill),
            text_widget(ARROW)
                .font(fonts::MISC_SYMBOLS)
                .size(TEXT_SIZE)
                .line_height(fonts::MISC_SYMBOLS_LINE_HEIGHT),
        ]
        .align_y(alignment::Vertical::Center)
        .into()
    } else {
        label.into()
    };

    let mut entry = button(content)
        .width(Length::Fill)
        .padding(Padding::new(ITEM_PADDING_Y).left(ITEM_PADDING_X).right(ITEM_PADDING_X))
        .style(if mark.alerting() { armed_style } else { entry_style });

    if item.live() {
        entry = entry.on_press(to_message(Message::Invoked(trail.clone())));
    }

    let wrapped: Element<'a, M> = match item.hint.as_deref() {
        Some(hint) => {
            let bubble = container(text_widget(hint).size(TEXT_SIZE).shaping(glyphs::shaping(hint)))
                .padding(TOOLTIP_PADDING)
                .style(container::bordered_box);

            tooltip(entry, bubble, tooltip::Position::Top).gap(TOOLTIP_GAP).into()
        }
        None => entry.into(),
    };

    mouse_area(wrapped).on_enter(to_message(Message::Hovered(trail))).into()
}

fn frame(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(theme::darken_color(palette.background.base.color, FRAME_SHADE).into()),
        border: Border {
            color: palette.background.strong.color,
            width: BORDER_WIDTH,
            radius: theme::RADIUS_SM.into(),
        },
        ..container::Style::default()
    }
}

fn armed_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.danger.strong.color,
        button::Status::Active | button::Status::Disabled => palette.danger.base.color,
    };

    button::Style {
        background: Some(background.into()),
        text_color: palette.danger.base.text,
        border: Border::default().rounded(theme::RADIUS_SM),
        ..button::Style::default()
    }
}

fn entry_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => {
            (Some(palette.primary.base.color.into()), palette.primary.base.text)
        }
        button::Status::Active => (None, palette.background.base.text),
        button::Status::Disabled => (None, theme::weak_text_color(theme)),
    };

    button::Style {
        background,
        text_color,
        border: Border::default().rounded(theme::RADIUS_SM),
        ..button::Style::default()
    }
}
