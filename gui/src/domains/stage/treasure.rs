use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use iced::alignment::Horizontal;
use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, row, space, text, tooltip, Column};
use iced::{Alignment, Element, Length, Theme};
use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::stage::RewardStructure;

use kore::domains::stage::treasure;
use kore::domains::stage::Stage;
use kore::{ItemStore, Vfs};

use crate::app::theme;
use crate::common::item_icon;
use crate::editor;
use crate::widget::section;

const TREASURE_TABLE_WIDTH: f32 = 345.0;
const MAX_ICON_SIZE: f32 = 32.0;
const HEADER_TEXT_SIZE: f32 = 13.0;
const CELL_PADDING: [u16; 2] = [4, 8];
const COLUMN_SPACING: f32 = 8.0;

fn format_drop_chance(raw_chance: u32, drop_rule: i32) -> String {
    if drop_rule == -3 || drop_rule == -4 {
        return "100%".to_string();
    }
    format!("{}%", raw_chance)
}

fn format_treasure_rule(drop_rule: i32) -> &'static str {
    match drop_rule {
        1 => "Once, Then Unlimited",
        0 => "Unlimited",
        -1 => "Unlimited (%)",
        -3 => "Guaranteed (Once)",
        -4 => "Guaranteed (Unlimited)",
        _ => "Unknown Rule",
    }
}

#[derive(Default)]
pub struct State {
    icon_cache: RefCell<HashMap<u32, Handle>>,
}

impl State {
    pub fn forget(&self, id: u32) {
        self.icon_cache.borrow_mut().remove(&id);
    }

    pub fn clear_icons(&self) {
        self.icon_cache.borrow_mut().clear();
    }

    fn icon(&self, id: u32, path: &Path) -> Option<Handle> {
        if let Some(cached) = self.icon_cache.borrow().get(&id) {
            return Some(cached.clone());
        }

        let handle = item_icon::load_scaled(path, MAX_ICON_SIZE as u32)?;
        self.icon_cache.borrow_mut().insert(id, handle.clone());
        Some(handle)
    }

    fn item_row<'a>(
        &'a self,
        index: usize,
        item_id: u32,
        left_label: String,
        amount_label: String,
        image_path: Option<&Path>,
        drop_name: String,
    ) -> Element<'a, super::Message> {
        let icon_element: Element<'a, super::Message> = match image_path.and_then(|path| self.icon(item_id, path)) {
            Some(handle) => tooltip(
                iced_image(handle).width(Length::Fixed(MAX_ICON_SIZE)).height(Length::Fixed(MAX_ICON_SIZE)),
                container(text(drop_name.clone())).padding(6).style(container::bordered_box),
                tooltip::Position::Top,
            ).into(),
            None => theme::centered_text(drop_name).into(),
        };

        let held = container(
            row![
                theme::table_cell_text(left_label, Length::FillPortion(1)),
                container(icon_element).width(Length::FillPortion(1)).align_x(Horizontal::Center),
                theme::table_cell_text(amount_label, Length::FillPortion(1)),
            ]
                .spacing(COLUMN_SPACING)
                .align_y(Alignment::Center)
        )
            .style(move |theme: &Theme| theme::zebra_table_row(theme, index))
            .padding(CELL_PADDING)
            .width(Length::Fill);

        editor::target(held, editor::Target::TreasureDrop(item_id))
    }

    pub fn view<'a>(
        &'a self,
        stage: &'a Stage,
        items: &ItemStore,
        drop_charas: &'a HashMap<u32, u32>,
        unit_buys: &'a HashMap<u32, UnitBuy>,
        vfs: &'a Vfs,
    ) -> Element<'a, super::Message> {
        match &stage.rewards {
            RewardStructure::Treasure { drop_rule, drops } => {
                let valid_drops: Vec<_> = drops.iter().filter(|drop| drop.chance > 0).collect();
                if valid_drops.is_empty() {
                    return space().into();
                }

                let mut grid = column![header_row("Chance")];

                for (index, drop) in valid_drops.into_iter().enumerate() {
                    let resolved = treasure::resolve_drop(vfs, drop.item_id, drop.amount, items, drop_charas, unit_buys);
                    grid = grid.push(self.item_row(
                        index,
                        drop.item_id,
                        format_drop_chance(drop.chance, *drop_rule),
                        resolved.amount_display.clone(),
                        resolved.image_path.as_deref(),
                        resolved.name.clone(),
                    ));
                }

                table(format!("Treasure | {}", format_treasure_rule(*drop_rule)), grid)
            }
            RewardStructure::Timed(timed_scores) => {
                if timed_scores.is_empty() {
                    return space().into();
                }

                let mut grid = column![header_row("Score")];

                for (index, score) in timed_scores.iter().enumerate() {
                    let resolved = treasure::resolve_drop(vfs, score.item_id, score.amount, items, drop_charas, unit_buys);
                    grid = grid.push(self.item_row(
                        index,
                        score.item_id,
                        score.score.to_string(),
                        resolved.amount_display.clone(),
                        resolved.image_path.as_deref(),
                        resolved.name.clone(),
                    ));
                }

                table("Timed Score Rewards", grid)
            }
            RewardStructure::None => space().into(),
        }
    }
}

fn header_row<'a>(first_column: &'a str) -> Element<'a, super::Message> {
    container(
        row![
            theme::table_cell_text(first_column, Length::FillPortion(1)).size(HEADER_TEXT_SIZE),
            theme::table_cell_text("Item", Length::FillPortion(1)).size(HEADER_TEXT_SIZE),
            theme::table_cell_text("Amount", Length::FillPortion(1)).size(HEADER_TEXT_SIZE),
        ]
            .spacing(COLUMN_SPACING)
    )
        .style(theme::zebra_table_header)
        .padding(CELL_PADDING)
        .width(Length::Fill)
        .into()
}

fn table<'a>(title: impl ToString, grid: Column<'a, super::Message>) -> Element<'a, super::Message> {
    let panel = container(section(title, Length::Fill, grid.width(Length::Fill)))
        .width(Length::Fixed(TREASURE_TABLE_WIDTH));

    editor::target(panel, editor::Target::StageDrops)
}
