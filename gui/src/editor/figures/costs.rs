use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, scrollable, text, Column, Row};
use iced::{Element, Length, Theme};

use kore::domains::settings::EditorMode;

use crate::app::theme;
use crate::widget::smooth_scroll;

use super::resolved::Rule;
use super::schema::{COST_HEAD, COST_LEVELS};
use super::{cards, Address, Draft, Frame, Marks, Message};

const NOTICE: &str =
    "Cost curves are shared by every unit; dimmed ids are ones this unit's talents never reference";

const NOTICE_SIZE: f32 = 11.0;
const LABEL_SIZE: f32 = 12.0;

const KEY_WIDTH: f32 = 34.0;
const KEY_GAP: f32 = 4.0;
const KEY_INSET: f32 = 3.0;

const COLUMN_GAP: f32 = 6.0;
const ROW_GAP: f32 = 2.0;
const ROW_PADDING: f32 = 4.0;
const HEAD_GAP: f32 = 6.0;

const LEVEL_WIDTH: f32 = 68.0;
const FIELD_WIDTH: f32 = 84.0;
const TOTAL_WIDTH: f32 = 68.0;

const HEADINGS: [(&str, f32); 3] =
    [("Level", LEVEL_WIDTH), ("NP Cost", FIELD_WIDTH), ("Total NP", TOTAL_WIDTH)];

const TABLE_WIDTH: f32 = LEVEL_WIDTH + FIELD_WIDTH + TOTAL_WIDTH + COLUMN_GAP * 2.0;

pub(super) fn view<'a>(draft: &'a Draft, frame: Frame<'a>) -> Element<'a, Message> {
    let Frame { width, armed, used, .. } = frame;

    let body = if draft.values() == EditorMode::Resolved {
        curve(draft)
    } else {
        let shown: Vec<usize> = (0..draft.len()).collect();

        cards::grid(draft, width, &shown, unstored(draft))
    };

    cards::shell(Some(head(draft, width, used)), body, cards::footer(vec![cards::sync(armed)]))
}

fn head<'a>(draft: &'a Draft, width: f32, used: Marks) -> Element<'a, Message> {
    let caption = text(NOTICE)
        .size(NOTICE_SIZE)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .style(text::secondary);

    cards::header(column![caption, curves(draft, width, used)].spacing(HEAD_GAP))
}

fn curves<'a>(draft: &'a Draft, width: f32, used: Marks) -> Element<'a, Message> {
    let per_row = (((cards::usable(width) + KEY_GAP) / (KEY_WIDTH + KEY_GAP)).floor() as usize).max(1);
    let current = draft.keyed();

    let mut rows = Column::new().spacing(KEY_GAP).align_x(Horizontal::Center);
    let mut line = Row::new().spacing(KEY_GAP).align_y(Vertical::Center);
    let mut placed = 0;

    for id in draft.keys().iter().copied() {
        if placed == per_row {
            rows = rows.push(line);
            line = Row::new().spacing(KEY_GAP).align_y(Vertical::Center);
            placed = 0;
        }

        line = line.push(curve_key(id, current == Some(id), references(used, id)));
        placed += 1;
    }

    centred(rows.push(line).into())
}

fn curve_key<'a>(id: u32, current: bool, referenced: bool) -> Element<'a, Message> {
    let label = theme::centered_text(id.to_string())
        .size(LABEL_SIZE)
        .width(Length::Fill)
        .wrapping(text::Wrapping::None);

    button(label)
        .width(Length::Fixed(KEY_WIDTH))
        .padding(KEY_INSET)
        .style(move |theme: &Theme, status| {
            theme::header_toggle_button(theme, status, current, referenced)
        })
        .on_press(Message::Aimed(Address::Keyed(id)))
        .into()
}

fn references(used: Marks, id: u32) -> bool {
    used.iter().any(|cost| u32::from(*cost) == id)
}

fn curve<'a>(draft: &'a Draft) -> Element<'a, Message> {
    let stored = draft.stored();
    let mut rows = Column::new().spacing(ROW_GAP);
    let mut total: i32 = 0;

    for level in 0..COST_LEVELS {
        let index = COST_HEAD + level;
        total = total.saturating_add(draft.reads_at(index).unwrap_or_default());

        rows = rows.push(striped(step(draft, level, index, total, index >= stored), level));
    }

    let area = scrollable(column![headings(), rows]).width(Length::Fill).height(Length::Fill);

    smooth_scroll(area).into()
}

fn headings<'a>() -> Element<'a, Message> {
    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    for (label, width) in HEADINGS {
        line = line.push(
            container(text(label).size(LABEL_SIZE).align_x(Horizontal::Center).width(Length::Fill))
                .width(Length::Fixed(width)),
        );
    }

    let framed = container(line.width(Length::Fixed(TABLE_WIDTH)))
        .padding(ROW_PADDING)
        .style(theme::zebra_table_header);

    centred(framed.into())
}

fn step<'a>(
    draft: &'a Draft,
    level: usize,
    index: usize,
    total: i32,
    dimmed: bool,
) -> Element<'a, Message> {
    let cells = vec![
        faded(format!("Level {}", level + 1), dimmed),
        cards::number(draft, index),
        faded(total.to_string(), dimmed),
    ];

    let mut line = Row::new().spacing(COLUMN_GAP).align_y(Vertical::Center);

    for (cell, (_, width)) in cells.into_iter().zip(HEADINGS) {
        line = line.push(container(cell).width(Length::Fixed(width)));
    }

    line.width(Length::Fixed(TABLE_WIDTH)).into()
}

fn faded<'a>(label: String, dimmed: bool) -> Element<'a, Message> {
    text(label)
        .size(LABEL_SIZE)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .style(move |theme: &Theme| text::Style {
            color: dimmed.then(|| theme::weak_text_color(theme)),
        })
        .into()
}

fn striped<'a>(content: Element<'a, Message>, index: usize) -> Element<'a, Message> {
    let framed = container(content)
        .padding(ROW_PADDING)
        .style(move |theme: &Theme| theme::zebra_table_row(theme, index));

    centred(framed.into())
}

fn centred<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content).width(Length::Fill).center_x(Length::Fill).into()
}

fn unstored(draft: &Draft) -> Option<usize> {
    (draft.stored() < draft.len()).then(|| draft.stored())
}

pub(super) fn rule(index: usize) -> Rule {
    match index.checked_sub(COST_HEAD) {
        None => Rule::Plain,
        Some(level) if level < COST_LEVELS => Rule::Floor(0),
        Some(_) => Rule::Opaque,
    }
}

#[cfg(test)]
mod tests {
    use super::{rule, Rule, COST_HEAD, COST_LEVELS};

    // nyanko parses each cost with u16::from_str and drops what fails, so a negative
    // cost would not be stored as -1: it would vanish and shift every later level up one.
    #[test]
    fn every_level_refuses_a_cost_the_engine_would_drop() {
        for index in COST_HEAD..COST_HEAD + COST_LEVELS {
            assert_eq!(rule(index), Rule::Floor(0), "level column {index} must stay non-negative");
        }
    }

    #[test]
    fn a_column_past_the_tenth_level_is_flagged_rather_than_shown_as_a_cost() {
        assert_eq!(rule(COST_HEAD + COST_LEVELS), Rule::Opaque);
    }
}
