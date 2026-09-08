use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, rule, scrollable, space, text, Column, Container};
use iced::{Element, Length, Padding, Theme};
use nyanko::chapter::Category;

use kore::domains::stage::filter::{CompiledStageFilter, StageFilterState, StageLookupContext};
use kore::domains::stage::{navigate, GlobalMapId, GlobalStageId, StageDataState};
use kore::Vault;

use crate::app::theme;
use crate::editor;
use crate::widget::{list_row, smooth_scroll};

use super::category::CategoryExt;

const BTN_SPACING_Y: f32 = 4.0;
const BTN_TEXT_SIZE: f32 = 12.0;
const BTN_PADDING: Padding = Padding { top: 5.0, right: 8.0, bottom: 5.0, left: 8.0 };

const RULE_THICKNESS: f32 = 1.0;
const COLUMN_GAP: f32 = 7.0;
const COLUMN_SEPARATOR_WIDTH: f32 = RULE_THICKNESS + COLUMN_GAP * 2.0;
const CATEGORY_COLUMN_WIDTH: f32 = 180.0;
const COLUMN_WIDTH: f32 = 200.0;

const SCROLLBAR_WIDTH: f32 = 10.0;
const SCROLLBAR_GAP: f32 = 2.0;
const SCROLLBAR_RESERVE: f32 = SCROLLBAR_WIDTH + SCROLLBAR_GAP;

const FILTER_RULE_GAP: f32 = 8.0;

#[derive(Debug, Clone)]
pub enum Message {
    ToggleFilter,
    SelectCategory(Category),
    SelectMap(GlobalMapId),
    SelectStage(GlobalStageId),
}

#[derive(Default)]
pub struct State {
    compiled_filter: Option<CompiledStageFilter>,
    matching_stages: HashSet<GlobalStageId>,
    matching_maps: HashSet<GlobalMapId>,
    matching_categories: HashSet<Category>,
    sorted_categories: Option<Vec<Category>>,
}

impl State {
    pub fn update(&mut self, message: Message, data: &mut StageDataState) {
        match message {
            Message::ToggleFilter => {}
            Message::SelectCategory(category) => {
                data.selected_category = Some(category);
                data.selected_map = None;
                data.selected_stage = None;
            }
            Message::SelectMap(map_id) => {
                data.selected_map = Some(map_id);
                data.selected_stage = None;
            }
            Message::SelectStage(stage_id) => {
                data.selected_stage = Some(stage_id);
            }
        }
    }

    pub fn refresh(&mut self, filter_state: &StageFilterState, data: &StageDataState, vault: &Vault) {
        if self.sorted_categories.is_none() {
            let mut categories = navigate::get_categories(&data.registry);
            categories.sort_by_key(|category| category.sort_order());
            self.sorted_categories = Some(categories);
        }

        let mut hasher = DefaultHasher::new();
        filter_state.hash(&mut hasher);
        let current_hash = hasher.finish();

        if self.compiled_filter.as_ref().is_none_or(|cf| cf.source_hash != current_hash) {
            let mut compiled = filter_state.compile();
            compiled.source_hash = current_hash;

            self.matching_stages.clear();
            self.matching_maps.clear();
            self.matching_categories.clear();

            if compiled.is_active() {
                let ctx = StageLookupContext::from_data(data, vault);

                for (stage_key, stage) in &data.registry.stages {
                    let map_key = GlobalMapId { category: stage_key.category.clone(), map: stage_key.map };
                    let Some(map) = data.registry.maps.get(&map_key) else { continue };

                    if compiled.matches(stage_key.category.display_name(), map, stage, &ctx) {
                        self.matching_stages.insert(stage_key.clone());
                        self.matching_maps.insert(map_key);
                        self.matching_categories.insert(stage_key.category.clone());
                    }
                }
            }

            self.compiled_filter = Some(compiled);
        }
    }

    pub fn invalidate(&mut self) {
        self.compiled_filter = None;
        self.sorted_categories = None;
    }

    pub fn invalidate_filter(&mut self) {
        self.compiled_filter = None;
    }

    pub fn view<'a>(&'a self, data: &'a StageDataState, filter_state: &StageFilterState, busy: bool) -> Element<'a, Message> {
        let Some(sorted_categories) = self.sorted_categories.as_ref() else {
            return space().into();
        };

        if busy && sorted_categories.is_empty() {
            return space().into();
        }

        let Some(compiled_filter) = self.compiled_filter.as_ref() else {
            return space().into();
        };

        let filter_active = compiled_filter.is_active();
        let is_filter_set = filter_state.is_active();
        let filter_btn = button(button_face("Filter"))
            .width(Length::Fill)
            .padding(0)
            .on_press(Message::ToggleFilter)
            .style(move |theme: &Theme, status| theme::toggle_button(theme, status, is_filter_set));

        let mut cat_col = button_column();
        let mut cat_count = 0;
        for category in sorted_categories {
            if filter_active && !self.matching_categories.contains(category) {
                continue;
            }

            let is_selected = data.selected_category.as_ref() == Some(category);
            cat_col = cat_col.push(sidebar_button(
                category.display_name(),
                is_selected,
                CATEGORY_COLUMN_WIDTH,
                Message::SelectCategory(category.clone()),
            ));
            cat_count += 1;
        }

        let mut columns = row![column_body(CATEGORY_COLUMN_WIDTH, cat_col, cat_count, "No Categories Found!")]
            .spacing(COLUMN_GAP)
            .height(Length::Fill);

        if let Some(category) = &data.selected_category {
            let mut map_col = button_column();
            let mut map_count = 0;

            for map in navigate::get_maps(&data.registry, category) {
                let map_key = GlobalMapId { category: category.clone(), map: map.map_id };
                if filter_active && !self.matching_maps.contains(&map_key) {
                    continue;
                }

                let is_selected = data.selected_map.as_ref() == Some(&map_key);
                let row = sidebar_button(&map.name, is_selected, COLUMN_WIDTH, Message::SelectMap(map_key));

                map_col = map_col.push(editor::target(row, editor::Target::MapRow(map.map_id)));
                map_count += 1;
            }

            columns = columns.push(rule::vertical(RULE_THICKNESS));
            columns = columns.push(column_body(COLUMN_WIDTH, map_col, map_count, "No Maps Found!"));
        }

        if let Some(map_id) = &data.selected_map
            && data.registry.maps.contains_key(map_id)
        {
            let mut stage_col = button_column();
            let mut stage_count = 0;

            for stage in navigate::get_stages(&data.registry, map_id) {
                let stage_key = GlobalStageId {
                    category: map_id.category.clone(),
                    map: map_id.map,
                    stage: stage.stage_id,
                };

                if filter_active && !self.matching_stages.contains(&stage_key) {
                    continue;
                }

                let is_selected = data.selected_stage.as_ref() == Some(&stage_key);
                let id = stage.stage_id;
                let row = sidebar_button(&stage.name, is_selected, COLUMN_WIDTH, Message::SelectStage(stage_key));

                stage_col = stage_col.push(editor::target(row, editor::Target::StageRow(id)));
                stage_count += 1;
            }

            columns = columns.push(rule::vertical(RULE_THICKNESS));
            columns = columns.push(column_body(COLUMN_WIDTH, stage_col, stage_count, "No Stages Found!"));
        }

        column![
            filter_btn,
            rule::horizontal(RULE_THICKNESS),
            smooth_scroll(
                scrollable(columns)
                    .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()))
                    .height(Length::Fill)
            ),
        ]
            .spacing(FILTER_RULE_GAP)
            .width(Length::Fixed(sidebar_width(data)))
            .height(Length::Fill)
            .into()
    }
}

pub fn sidebar_width(data: &StageDataState) -> f32 {
    let mut width = CATEGORY_COLUMN_WIDTH;
    if data.selected_category.is_some() {
        width += COLUMN_SEPARATOR_WIDTH + COLUMN_WIDTH;
    }
    if data.selected_map.as_ref().is_some_and(|map_id| data.registry.maps.contains_key(map_id)) {
        width += COLUMN_SEPARATOR_WIDTH + COLUMN_WIDTH;
    }
    width
}

fn button_column<'a>() -> Column<'a, Message> {
    column![]
        .spacing(BTN_SPACING_Y)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .padding(Padding { top: BTN_SPACING_Y, bottom: BTN_SPACING_Y, ..Padding::ZERO })
}

fn column_body<'a>(width: f32, content: Column<'a, Message>, count: usize, empty: &'a str) -> Element<'a, Message> {
    if count > 0 {
        return column_scroller(width, content);
    }

    container(theme::centered_text(empty).size(BTN_TEXT_SIZE).style(text::danger))
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
}

fn column_scroller<'a>(width: f32, content: Column<'a, Message>) -> Element<'a, Message> {
    smooth_scroll(
        scrollable(content)
            .spacing(SCROLLBAR_GAP)
            .width(Length::Fixed(width))
            .height(Length::Fill)
    ).into()
}

fn button_face(label: &str) -> Container<'_, Message> {
    container(theme::button_label(label).size(BTN_TEXT_SIZE))
        .width(Length::Fill)
        .align_y(Vertical::Center)
        .padding(BTN_PADDING)
}

fn sidebar_button(label: &str, is_selected: bool, width: f32, msg: Message) -> Element<'_, Message> {
    list_row(button_face(label), is_selected, true, Length::Fixed(width - SCROLLBAR_RESERVE), msg)
}
