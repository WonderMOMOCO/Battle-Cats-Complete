use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, row, scrollable, slider, text, text_input, Space};
use iced::alignment::{Horizontal, Vertical};
use iced::{font, Alignment, Color, Element, Length, Padding, Theme};
use nyanko::combat::{get_talent, Entity};
use nyanko::cat::unit::{LevelCurve, Talent, TalentCost, TalentGroup};
use nyanko::files::img022;

use kore::systems::combat::registry::{get_display_def, AbilityIcon};
use kore::domains::cat::game::talents as talent_logic;
use kore::Vfs;

use crate::app::theme;
use crate::common::ability_icon;
use crate::common::skill_name;
use crate::common::fonts::{self, MISC_SYMBOLS_LINE_HEIGHT};
use crate::common::{CustomAssets, SpriteSheet};
use crate::widget::{fallback_icon, smooth_scroll};

const HEADER_SCALE: f32 = 0.75;

const HEADER_ICON_SIZE: f32 = 40.0 * HEADER_SCALE;
const HEADER_NAME_HEIGHT: f32 = 35.0 * HEADER_SCALE;
const HEADER_NAME_TEXT_SIZE: f32 = 18.0 * HEADER_SCALE;
const HEADER_ARROW_SIZE: f32 = 22.0 * HEADER_SCALE;
const HEADER_ARROW_RADIUS: f32 = 5.0 * HEADER_SCALE;
const HEADER_OUTLINE_WIDTH: f32 = 1.5;
const HEADER_OUTLINE_SHADE: f32 = 0.8;
const HEADER_ARROW_TOP_PAD: f32 = 4.0 * HEADER_SCALE;

const DARK_BOX_BG: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 };

const ARROW_EXPANDED: &str = "\u{25B2}";
const ARROW_COLLAPSED: &str = "\u{25BC}";
const HEADER_FALLBACK_SIZE: f32 = 24.0 * HEADER_SCALE;
const HEADER_PADDING: f32 = 2.0 * HEADER_SCALE;
const HEADER_GAP: f32 = 6.0 * HEADER_SCALE;

const SCROLLBAR_GAP: f32 = 4.0;
const NP_ICON_SIZE: f32 = 20.0;
const HEADER_NP_ICON_SIZE: f32 = 24.0;
const HEADER_NP_TEXT_SIZE: f32 = 20.0;
const TALENT_BTN_WIDTH: f32 = 100.0;
const TALENT_BTN_HEIGHT: f32 = 23.0;
const SECTION_SPACING: f32 = 2.0;

fn typable_digits(value: &str) -> bool {
    value.chars().all(|glyph| glyph.is_ascii_digit())
}

fn bold_text<'a>(content: impl ToString, size: f32) -> iced::widget::Text<'a> {
    text(content.to_string())
        .size(size)
        .font(font::Font { weight: font::Weight::Bold, ..Default::default() })
        .color(Color::WHITE)
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleGroup(u32, u8),
    LevelChanged(u8, u8),
    LevelInputChanged(u8, String),
    ToggleNormal,
    ToggleUltra,
}

pub struct ToggleAvailability {
    pub has_normal_enabled: bool,
    pub has_ultra_enabled: bool,
    pub has_ultra_talents: bool,
}

pub fn toggle_availability(talent_data: &Talent, talent_levels: Option<&HashMap<u8, u8>>) -> ToggleAvailability {
    let mut has_normal_enabled = false;
    let mut has_ultra_enabled = false;
    let mut has_ultra_talents = false;

    for (index, group) in talent_data.groups.iter().enumerate() {
        let level = talent_levels.and_then(|levels| levels.get(&(index as u8))).copied().unwrap_or(0);
        if group.limit == 1 {
            has_ultra_talents = true;
            if level > 0 { has_ultra_enabled = true; }
        } else if level > 0 {
            has_normal_enabled = true;
        }
    }

    ToggleAvailability { has_normal_enabled, has_ultra_enabled, has_ultra_talents }
}

pub struct ViewCtx<'a, 'b> {
    pub cat_id: u32,
    pub talent_data: &'a Talent,
    pub talent_levels: Option<&'a HashMap<u8, u8>>,
    pub level_inputs: &'a HashMap<u8, String>,
    pub talent_costs: &'a HashMap<u8, TalentCost>,
    pub descriptions: &'a [String],
    pub current_stats: Option<&'b Entity>,
    pub curve: Option<&'a LevelCurve>,
    pub unit_level: i32,
    pub sheets: &'a [SpriteSheet],
    pub img022_sheets: &'a [SpriteSheet],
    pub assets: &'a CustomAssets,
    pub vfs: &'a Vfs,
}

pub struct State {
    icons: ability_icon::Cache,
    np_icons: ability_icon::Cache,
    skill_name_cache: skill_name::Cache,
    expanded: HashMap<(u32, u8), bool>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            icons: ability_icon::Cache::default(),
            np_icons: ability_icon::Cache::default(),
            skill_name_cache: RefCell::new(HashMap::new()),
            expanded: HashMap::new(),
        }
    }
}

impl State {
    pub(super) fn clear_icons(&self) {
        self.icons.clear();
        self.np_icons.clear();
        self.skill_name_cache.borrow_mut().clear();
    }

    pub fn update(&mut self, message: Message) {
        if let Message::ToggleGroup(cat_id, index) = message {
            let key = (cat_id, index);
            let current = self.expanded.get(&key).copied().unwrap_or(false);
            self.expanded.insert(key, !current);
        }
    }

    pub fn set_level(&self, index: u8, level: u8, levels: &mut HashMap<u8, u8>, inputs: &mut HashMap<u8, String>) {
        levels.insert(index, level);
        inputs.remove(&index);
    }

    pub fn set_level_input(
        &self,
        index: u8,
        input: String,
        levels: Option<&mut HashMap<u8, u8>>,
        inputs: &mut HashMap<u8, String>,
        talent_data: Option<&Talent>,
    ) {
        if !typable_digits(&input) {
            return;
        }

        let max_level = talent_data
            .and_then(|data| data.groups.get(index as usize))
            .map_or(u8::MAX, |group| group.max_level.max(1));

        let trimmed = input.trim();
        let parsed = trimmed.parse::<u8>().ok();

        if let Some(levels) = levels {
            if let Some(parsed) = parsed {
                levels.insert(index, parsed.min(max_level));
            } else if trimmed.is_empty() {
                levels.insert(index, 0);
            }
        }

        let clamped_input = match parsed {
            Some(parsed) if parsed > max_level => max_level.to_string(),
            _ => input,
        };

        inputs.insert(index, clamped_input);
    }

    pub fn toggle(&self, is_ultra: bool, talent_data: &Talent, levels: &mut HashMap<u8, u8>, inputs: &mut HashMap<u8, String>) {
        let availability = toggle_availability(talent_data, Some(levels));
        let has_enabled = if is_ultra { availability.has_ultra_enabled } else { availability.has_normal_enabled };

        for (index, group) in talent_data.groups.iter().enumerate() {
            let target_group = if is_ultra { group.limit == 1 } else { group.limit != 1 };
            if target_group {
                let new_level = if has_enabled { 0 } else { group.max_level.max(1) };
                levels.insert(index as u8, new_level);
                inputs.remove(&(index as u8));
            }
        }
    }

    pub fn header_view<'a>(
        &'a self,
        talent_data: &'a Talent,
        talent_levels: Option<&'a HashMap<u8, u8>>,
        talent_costs: &'a HashMap<u8, TalentCost>,
        img022_sheets: &'a [SpriteSheet],
    ) -> Element<'a, Message> {
        let total_np = talent_levels.map_or(0, |levels| talent_logic::get_total_np_cost(talent_data, levels, talent_costs));
        let availability = toggle_availability(talent_data, talent_levels);

        let np_row = row![self.np_icon(img022_sheets, HEADER_NP_ICON_SIZE), bold_text(total_np, HEADER_NP_TEXT_SIZE)]
            .spacing(6)
            .align_y(Alignment::Center);

        let normal_label = if availability.has_normal_enabled { "No Talents" } else { "All Talents" };
        let normal_btn = button(theme::centered_text(normal_label).size(12))
            .width(Length::Fixed(TALENT_BTN_WIDTH))
            .height(Length::Fixed(TALENT_BTN_HEIGHT))
            .on_press(Message::ToggleNormal);

        let ultra_label = if availability.has_ultra_enabled { "No Ultra" } else { "All Ultra" };
        let ultra_btn = button(theme::centered_text(ultra_label).size(12))
            .width(Length::Fixed(TALENT_BTN_WIDTH))
            .height(Length::Fixed(TALENT_BTN_HEIGHT))
            .on_press_maybe(availability.has_ultra_talents.then_some(Message::ToggleUltra));

        column![np_row, normal_btn, ultra_btn].spacing(5).into()
    }

    pub fn view<'a>(&'a self, ctx: ViewCtx<'a, '_>) -> Element<'a, Message> {
        let mut col = column![].spacing(8).width(Length::Fill);

        for (index, group) in ctx.talent_data.groups.iter().enumerate() {
            col = col.push(self.group_view(&ctx, index as u8, group));
        }

        smooth_scroll(scrollable(col).spacing(SCROLLBAR_GAP).height(Length::Fill).width(Length::Fill)).into()
    }

    fn group_view<'a>(&'a self, ctx: &ViewCtx<'a, '_>, index: u8, group: &'a TalentGroup) -> Element<'a, Message> {
        let expanded = self.expanded.get(&(ctx.cat_id, index)).copied().unwrap_or(false);
        let bg_color = if group.limit == 1 { Color::from_rgb8(120, 20, 20) } else { Color::from_rgb8(180, 140, 20) };

        let mut inner = column![self.group_header(ctx, index, group, expanded)].spacing(0);

        if expanded {
            inner = inner.push(self.group_body(ctx, index, group));
        }

        container(inner)
            .padding(6)
            .width(Length::Fill)
            .style(move |_theme: &Theme| container::Style {
                background: Some(bg_color.into()),
                border: iced::border::rounded(5),
                ..Default::default()
            })
            .into()
    }

    fn group_header<'a>(&'a self, ctx: &ViewCtx<'a, '_>, index: u8, group: &'a TalentGroup, expanded: bool) -> Element<'a, Message> {
        let icon = self.talent_icon(group, ctx.sheets, ctx.assets);

        let name_el: Element<Message> = match self.skill_name_handle(group, ctx.vfs) {
            Some(handle) => iced_image(handle).height(Length::Fixed(HEADER_NAME_HEIGHT)).into(),
            None => {
                let fallback_text = get_talent(group.ability_id)
                    .map_or_else(|| format!("Unknown Skill (ID: {})", group.ability_id), |def| get_display_def(def.identity).name.to_string());
                bold_text(fallback_text, HEADER_NAME_TEXT_SIZE).into()
            }
        };

        let content = row![
            row![icon, name_el].spacing(HEADER_GAP).align_y(Alignment::Center).width(Length::Fill),
            header_arrow(expanded),
        ]
            .spacing(HEADER_GAP)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        button(content)
            .on_press(Message::ToggleGroup(ctx.cat_id, index))
            .style(button::text)
            .padding(HEADER_PADDING)
            .width(Length::Fill)
            .into()
    }

    fn group_body<'a>(&'a self, ctx: &ViewCtx<'a, '_>, index: u8, group: &'a TalentGroup) -> Element<'a, Message> {
        let description_text = ctx.descriptions
            .get(group.text_id as usize)
            .cloned()
            .unwrap_or_else(|| "No skill description found".to_string());

        let description_box = dark_box(text(description_text).size(13).color(Color::WHITE).width(Length::Fill));

        let current_level = ctx.talent_levels.and_then(|levels| levels.get(&index)).copied().unwrap_or(0);
        let np_cost = talent_logic::get_talent_np_cost(group.cost_id, current_level, ctx.talent_costs);

        let np_row = row![self.np_icon(ctx.img022_sheets, NP_ICON_SIZE), bold_text(np_cost, 18.0)]
            .spacing(4)
            .align_y(Alignment::Center);
        let np_box = dark_box(np_row);

        let max_level = group.max_level.max(1);
        let input_text = ctx.level_inputs.get(&index).cloned().unwrap_or_else(|| current_level.to_string());

        let level_row = row![
            bold_text("Level:", 14.0),
            slider(0..=max_level, current_level, move |value| Message::LevelChanged(index, value)).width(Length::Fixed(160.0)),
            text_input("0", &input_text)
                .on_input(move |value| Message::LevelInputChanged(index, value))
                .width(Length::Fixed(50.0)),
        ]
            .spacing(8)
            .align_y(Alignment::Center);

        let mut level_col = column![level_row].spacing(4);

        if let Some(stats) = ctx.current_stats
            && let Some(display_text) = talent_logic::calculate_talent_display(group, stats, current_level, ctx.curve, ctx.unit_level) {
            level_col = level_col.push(text(display_text).size(15).color(Color::WHITE).font(font::Font { weight: font::Weight::Bold, ..Default::default() }));
        }

        let level_box = dark_box(level_col);

        column![description_box, Space::new().height(Length::Fixed(SECTION_SPACING)), np_box, Space::new().height(Length::Fixed(SECTION_SPACING)), level_box]
            .padding([6, 0])
            .width(Length::Fill)
            .into()
    }

    fn talent_icon<'a>(&'a self, group: &TalentGroup, sheets: &'a [SpriteSheet], assets: &'a CustomAssets) -> Element<'a, Message> {
        let Some(def) = get_talent(group.ability_id) else {
            return text("?").size(HEADER_FALLBACK_SIZE).into();
        };
        let display_def = get_display_def(def.identity);

        match display_def.icon {
            AbilityIcon::Custom(custom) => {
                if let Some(handle) = assets.get_icon_texture(custom) {
                    return iced_image(handle).width(Length::Fixed(HEADER_ICON_SIZE)).height(Length::Fixed(HEADER_ICON_SIZE)).into();
                }
            }
            AbilityIcon::Standard(icon_id) => {
                if let Some(handle) = self.icons.handle(icon_id, sheets) {
                    return iced_image(handle).width(Length::Fixed(HEADER_ICON_SIZE)).height(Length::Fixed(HEADER_ICON_SIZE)).into();
                }
            }
            AbilityIcon::None => {}
        }

        fallback_icon(display_def.fallback)
    }

    fn np_icon<'a>(&'a self, img022_sheets: &'a [SpriteSheet], size: f32) -> Element<'a, Message> {
        self.np_icons.handle(img022::ICON_NP_COST, img022_sheets)
            .map_or_else(|| bold_text("NP Cost", 18.0).into(), |handle| iced_image(handle).height(Length::Fixed(size)).into())
    }


    fn skill_name_handle(&self, group: &TalentGroup, vfs: &Vfs) -> Option<Handle> {
        skill_name::load(&self.skill_name_cache, group, vfs, false)
    }
}

fn header_arrow<'a>(expanded: bool) -> Element<'a, Message> {
    let glyph = if expanded { ARROW_EXPANDED } else { ARROW_COLLAPSED };

    container(
        text(glyph)
            .font(fonts::MISC_SYMBOLS)
            .size(HEADER_ARROW_SIZE)
            .line_height(MISC_SYMBOLS_LINE_HEIGHT)
            .color(Color::WHITE),
    )
        .width(Length::Fixed(HEADER_ICON_SIZE))
        .height(Length::Fixed(HEADER_ICON_SIZE))
        .padding(Padding { top: HEADER_ARROW_TOP_PAD, right: 0.0, bottom: 0.0, left: 0.0 })
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(|theme: &Theme| container::Style {
            background: Some(DARK_BOX_BG.into()),
            border: iced::border::rounded(HEADER_ARROW_RADIUS)
                .color(theme::darken_color(theme.palette().background, HEADER_OUTLINE_SHADE))
                .width(HEADER_OUTLINE_WIDTH),
            ..Default::default()
        })
        .into()
}

fn dark_box<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(4)
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(DARK_BOX_BG.into()),
            border: iced::border::rounded(4),
            ..Default::default()
        })
        .into()
}

