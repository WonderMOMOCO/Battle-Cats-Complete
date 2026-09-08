use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use iced::alignment::Horizontal;
use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, row, space, text, tooltip, Column};
use iced::{Color, Element, Length, Theme};

use kore::domains::stage::materials;
use kore::domains::stage::{Map, Stage};
use kore::{ItemStore, Vfs};

use crate::app::theme;
use crate::common::item_icon;
use crate::editor;
use crate::widget::section;

const MAT_TABLE_WIDTH: f32 = 345.0;
const MAX_ICON_SIZE: f32 = 32.0;
const COL_SPACING: f32 = 8.0;
const CELL_PADDING_Y: u16 = 4;
const CELL_PADDING_X: u16 = 6;
const CELL_PADDING: [u16; 2] = [CELL_PADDING_Y, CELL_PADDING_X];
const CHUNK_SPACING: f32 = 8.0;
const LABEL_TEXT_SIZE: f32 = 12.0;
const FALLBACK_TEXT_SIZE: f32 = 11.0;
const EMPTY_CHANCE_ALPHA: f32 = 0.35;

pub fn has_drops(_stage: &Stage, map: &Map) -> bool {
    map.drop_items.as_ref().is_some_and(|drops| drops.material_drops.iter().any(|&count| count > 0))
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

    pub fn view<'a>(
        &'a self,
        stage: &'a Stage,
        map: &'a Map,
        selected_crown: u8,
        items: &ItemStore,
        vfs: &'a Vfs,
    ) -> Element<'a, super::Message> {
        if !has_drops(stage, map) {
            return space().into();
        }

        let Some(drops) = &map.drop_items else {
            return space().into();
        };

        let stage_idx = stage.stage_id as usize;
        let base_amount = drops.stage_drops.get(stage_idx).copied().unwrap_or(0);
        let multiplier = drops.crown_multipliers.get(selected_crown as usize).copied().unwrap_or(1.0);
        let final_amount = (base_amount as f32 * multiplier).round() as u32;

        let base_mats = &drops.material_drops[0..8];
        let z_mats = &drops.material_drops[8..16];

        let mut grid_col = column![].spacing(CHUNK_SPACING);

        if base_mats.iter().any(|&count| count > 0) {
            grid_col = self.push_chunks(grid_col, base_mats, 0, items, vfs);
        }
        if z_mats.iter().any(|&count| count > 0) {
            grid_col = self.push_chunks(grid_col, z_mats, 8, items, vfs);
        }

        let panel = container(section(format!("Materials | Amount: {} ({}×{:.2})", final_amount, base_amount, multiplier), Length::Fill, grid_col))
            .width(Length::Fixed(MAT_TABLE_WIDTH));

        editor::target(panel, editor::Target::MapDrops)
    }

    fn push_chunks<'a>(
        &'a self,
        mut col: Column<'a, super::Message>,
        chances: &'a [u32],
        offset: usize,
        items: &ItemStore,
        vfs: &'a Vfs,
    ) -> Column<'a, super::Message> {
        let column_width = (MAT_TABLE_WIDTH - (CELL_PADDING_X as f32 * 2.0) - (COL_SPACING * 3.0)) / 4.0;

        for (chunk_idx, chunk) in chances.chunks(4).enumerate() {
            let chunk_offset = offset + (chunk_idx * 4);

            let resolved: Vec<_> = chunk.iter().enumerate()
                .map(|(i, &chance)| (materials::resolve(vfs, chunk_offset + i, chance, items), chance))
                .collect();

            let mut name_row = row![].spacing(COL_SPACING);
            let mut icon_row = row![].spacing(COL_SPACING);
            let mut pct_row = row![].spacing(COL_SPACING);

            for (idx, (drop, chance)) in resolved.iter().enumerate() {
                let material_id = 10000 + (chunk_offset + idx) as u32;

                name_row = name_row.push(theme::table_cell_text(drop.name.clone(), Length::Fixed(column_width)).size(LABEL_TEXT_SIZE));

                let icon_element: Element<'a, super::Message> = drop
                    .image_path
                    .as_ref()
                    .and_then(|path| self.icon(material_id, path))
                    .map_or_else(
                        || text(drop.name.clone()).size(FALLBACK_TEXT_SIZE).into(),
                        |handle| tooltip(
                            iced_image(handle).width(Length::Fixed(MAX_ICON_SIZE)).height(Length::Fixed(MAX_ICON_SIZE)),
                            container(text(drop.name.clone())).padding(6).style(container::bordered_box),
                            tooltip::Position::Top,
                        ).into(),
                    );

                icon_row = icon_row.push(cell(icon_element, column_width));

                let is_empty = *chance == 0;
                let chance_text = theme::centered_text(format!("{}%", chance)).style(move |theme: &Theme| text::Style {
                    color: Some(chance_color(theme, is_empty)),
                });

                pct_row = pct_row.push(cell(chance_text, column_width));
            }

            col = col.push(column![
                container(name_row).style(theme::zebra_table_header).padding(CELL_PADDING).width(Length::Fill),
                container(icon_row).style(|theme: &Theme| theme::zebra_table_row(theme, 0)).padding(CELL_PADDING).width(Length::Fill),
                container(pct_row).style(|theme: &Theme| theme::zebra_table_row(theme, 1)).padding(CELL_PADDING).width(Length::Fill),
            ].width(Length::Fill));
        }

        col
    }
}

fn chance_color(theme: &Theme, is_empty: bool) -> Color {
    let color = theme.palette().text;

    if is_empty { Color { a: EMPTY_CHANCE_ALPHA, ..color } } else { color }
}

fn cell<'a>(content: impl Into<Element<'a, super::Message>>, width: f32) -> Element<'a, super::Message> {
    container(content)
        .width(Length::Fixed(width))
        .align_x(Horizontal::Center)
        .into()
}
