use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, pick_list, scrollable, text, text_input, Column, Container, Id, Row};
use iced::{Element, Length, Padding, Theme};

use crate::app::theme;
use crate::common::feedback::CONFIRM_LABEL;
use crate::widget::{hover_hint, smooth_scroll};

use super::resolved::{Choice, Face, Toggle};
use super::schema::Subject;
use super::{Draft, Message, GLYPH_RATIO, LABEL_HEIGHTS};

pub(super) const CARD_WIDTH: f32 = 86.0;
pub(super) const CARD_GAP: f32 = 8.0;
pub(super) const BODY_PADDING: f32 = 12.0;

const LABEL_SIZE: f32 = 12.0;
const INPUT_SIZE: f32 = 13.0;
const CARD_PADDING: f32 = 6.0;
const SYNC_WIDTH: f32 = 172.0;
const FIELD_PADDING: f32 = 4.0;
const SEARCH_FRACTION: f32 = 2.0 / 3.0;

const CARET_RESERVE: f32 = 18.0;
const WIDE_SPAN: usize = 2;
const DISABLED_LABEL: &str = "Disabled";
const FIELD_INSET: f32 = 2.0;

const OPAQUE_HINT: &str =
    "This value is not resolved and represents a raw data value\nEdit this attribute at your own risk";

pub(super) fn grid_id(subject: Subject) -> Id {
    Id::new(match subject {
        Subject::Cat => "figures-grid-cat",
        Subject::Enemy => "figures-grid-enemy",
        Subject::Buy => "figures-grid-buy",
        Subject::Curve => "figures-grid-curve",
        Subject::Talents => "figures-grid-talents",
        Subject::Costs => "figures-grid-costs",
        Subject::Combo => "figures-grid-combo",
        Subject::MapStage => "figures-grid-map-stage",
        Subject::MapDrops => "figures-grid-map-drops",
        Subject::DropChara => "figures-grid-drop-chara",
        Subject::ItemBuy => "figures-grid-item-buy",
    })
}

pub(super) fn usable(width: f32) -> f32 {
    (width - BODY_PADDING * 2.0).max(CARD_WIDTH)
}

pub(super) fn grid<'a>(
    draft: &'a Draft,
    width: f32,
    shown: &[usize],
    dim_from: Option<usize>,
) -> Element<'a, Message> {
    let area = scrollable(centred(lay(draft, width, shown, dim_from)))
        .id(grid_id(draft.schema().subject()))
        .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
        .width(Length::Fill)
        .height(Length::Fill);

    smooth_scroll(area).into()
}

// The same cards without the scroll area, for rows that stay pinned above it. `header`
// supplies the surrounding padding, so this one only centres.
pub(super) fn band<'a>(draft: &'a Draft, width: f32, shown: &[usize]) -> Element<'a, Message> {
    container(lay(draft, width, shown, None)).width(Length::Fill).center_x(Length::Fill).into()
}

fn centred<'a>(rows: Column<'a, Message>) -> Container<'a, Message> {
    container(rows)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(Padding::ZERO.left(BODY_PADDING).right(BODY_PADDING).bottom(BODY_PADDING))
}

fn lay<'a>(draft: &'a Draft, width: f32, shown: &[usize], dim_from: Option<usize>) -> Column<'a, Message> {
    let per_row = (((usable(width) + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).floor() as usize).max(1);

    let mut rows = Column::new().spacing(CARD_GAP);
    let mut line = Row::new().spacing(CARD_GAP);
    let mut used = 0;

    for index in shown {
        let span = span(draft.face(*index)).min(per_row);

        if used > 0 && used + span > per_row {
            rows = rows.push(line);
            line = Row::new().spacing(CARD_GAP);
            used = 0;
        }

        line = line.push(card(draft, *index, dim_from.is_some_and(|first| *index >= first)));
        used += span;
    }

    rows.push(line)
}

fn span(face: Face) -> usize {
    let Face::Choice(options, _) = face else {
        return 1;
    };

    let inner = CARD_WIDTH - CARD_PADDING * 2.0 - CARET_RESERVE;
    let widest = options.iter().map(|choice| choice.label().chars().count()).max().unwrap_or(0);

    if widest as f32 * INPUT_SIZE * GLYPH_RATIO <= inner { 1 } else { WIDE_SPAN }
}

fn card(draft: &Draft, index: usize, dimmed: bool) -> Element<'_, Message> {
    let schema = draft.schema();
    let face = draft.face(index);
    let opaque = face == Face::Danger;
    let slots = span(face);

    let field = match face {
        Face::Toggle(current) => toggle_field(index, current),
        Face::Choice(options, current) => choice_field(index, options, current),
        Face::Disabled(reason) => disabled_field(reason),
        Face::Number | Face::Danger => number_field(draft, index),
    };

    let caption = text(draft.label(index))
        .size(LABEL_SIZE)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .style(move |theme: &Theme| text::Style {
            color: dimmed.then(|| theme::weak_text_color(theme)),
        });

    let hint = if opaque { Some(OPAQUE_HINT) } else { draft.note(index) };

    let caption = match hint {
        Some(text) => hover_hint(caption, text),
        None => caption.into(),
    };

    let label = container(caption)
        .height(Length::Fixed(LABEL_HEIGHTS[schema.subject().slot()]))
        .align_y(Vertical::Center);

    let style = if opaque {
        theme::card_container_danger
    } else if dimmed {
        theme::card_container_muted
    } else {
        theme::card_container
    };

    let width = CARD_WIDTH * slots as f32 + CARD_GAP * (slots - 1) as f32;

    container(column![label, field].spacing(2))
        .width(Length::Fixed(width))
        .padding(CARD_PADDING)
        .style(style)
        .into()
}

pub(super) fn number(draft: &Draft, index: usize) -> Element<'_, Message> {
    number_field(draft, index)
}

pub(super) fn options<'a, T>(
    values: Vec<T>,
    current: T,
    picked: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
{
    pick_list(values, Some(current), picked)
        .width(Length::Fill)
        .padding(FIELD_INSET)
        .text_size(INPUT_SIZE)
        .style(theme::combo_box)
        .menu_style(theme::combo_box_menu)
        .into()
}

fn number_field(draft: &Draft, index: usize) -> Element<'_, Message> {
    let schema = draft.schema();
    let values = draft.values();
    let scaled = schema.to_display(index, schema.fallback(index), values);
    let hint = draft.rule(index).to_display(scaled, values).to_string();

    text_input(&hint, draft.input(index))
        .on_input(move |entry| Message::Changed(index, entry))
        .size(INPUT_SIZE)
        .padding(FIELD_INSET)
        .align_x(Horizontal::Center)
        .style(theme::rounded_input)
        .into()
}

fn toggle_field<'a>(index: usize, current: Toggle) -> Element<'a, Message> {
    let style: theme::ButtonStyleFn = match current {
        Toggle::Yes => theme::success_button,
        Toggle::No => theme::neutral_button,
    };

    let label = theme::centered_text(current.to_string())
        .size(INPUT_SIZE)
        .width(Length::Fill)
        .wrapping(text::Wrapping::None);

    button(label)
        .width(Length::Fill)
        .padding(FIELD_INSET)
        .style(style)
        .on_press(Message::Picked(index, current.flip().raw()))
        .into()
}

fn disabled_field<'a>(reason: &'a str) -> Element<'a, Message> {
    let label = theme::centered_text(DISABLED_LABEL)
        .size(INPUT_SIZE)
        .width(Length::Fill)
        .wrapping(text::Wrapping::None);

    let inert = button(label).width(Length::Fill).padding(FIELD_INSET).style(theme::inert_button);

    hover_hint(inert, reason)
}

fn choice_field<'a>(
    index: usize,
    options: &'static [Choice],
    current: Choice,
) -> Element<'a, Message> {
    pick_list(options, Some(current), move |choice: Choice| Message::Picked(index, choice.raw()))
        .width(Length::Fill)
        .padding(FIELD_INSET)
        .text_size(INPUT_SIZE)
        .style(theme::combo_box)
        .menu_style(theme::combo_box_menu)
        .into()
}

pub(super) fn search<'a>(query: &str, width: f32, placeholder: &'a str) -> Element<'a, Message> {
    header(search_field(query, width, placeholder, Message::SearchChanged))
}

pub(super) fn hunt<'a>(query: &str, width: f32, placeholder: &'a str) -> Element<'a, Message> {
    header(search_field(query, width, placeholder, Message::HuntChanged))
}

fn search_field<'a>(
    query: &str,
    width: f32,
    placeholder: &'a str,
    typed: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let field = text_input(placeholder, query)
        .on_input(typed)
        .size(INPUT_SIZE)
        .padding(FIELD_PADDING)
        .width(Length::Fixed((usable(width) * SEARCH_FRACTION).max(CARD_WIDTH)))
        .style(theme::rounded_input);

    container(field).width(Length::Fill).center_x(Length::Fill).into()
}

pub(super) fn header<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content.into())
        .padding(Padding::ZERO.top(BODY_PADDING).left(BODY_PADDING).right(BODY_PADDING).bottom(CARD_GAP))
        .into()
}

pub(super) fn comment<'a>(value: &str) -> Element<'a, Message> {
    text_input("Comment...", value)
        .on_input(Message::CommentChanged)
        .size(INPUT_SIZE)
        .padding(FIELD_PADDING)
        .width(Length::Fill)
        .style(theme::rounded_input)
        .into()
}

pub(super) fn footer<'a>(rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut stack = Column::with_capacity(rows.len()).spacing(CARD_GAP).align_x(Horizontal::Center);

    for row in rows {
        stack = stack.push(row);
    }

    container(stack)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(Padding::ZERO.top(CARD_GAP).left(BODY_PADDING).right(BODY_PADDING).bottom(BODY_PADDING))
        .into()
}

pub(super) fn sync<'a>(armed: bool) -> Element<'a, Message> {
    let label = if armed { CONFIRM_LABEL } else { "Sync With \"game\"" };

    let content = theme::centered_text(label).size(INPUT_SIZE).wrapping(text::Wrapping::None);

    button(content)
        .width(Length::Fixed(SYNC_WIDTH))
        .padding([FIELD_PADDING + 1.0, 10.0])
        .style(theme::danger_button)
        .on_press(Message::Sync)
        .into()
}

pub(super) fn shell<'a>(
    top: Option<Element<'a, Message>>,
    body: Element<'a, Message>,
    bottom: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut stack = Column::new().height(Length::Fill);

    if let Some(top) = top {
        stack = stack.push(top);
    }

    stack.push(body).push(bottom).into()
}
