use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, row, scrollable, stack, text, tooltip, Space};
use iced::{font, Border, Color, Element, Font, Length, Padding, Theme};

use kore::common::context::GlobalContext;
use kore::common::io;
use kore::domains::cat::combo::{self, CatCombo, ComboMember};
use kore::domains::cat::scanner::CatEntry;
use kore::Vfs;

use crate::app::theme;
use crate::common::item_icon;
use crate::editor;
use crate::widget::{fit_column, roster_list, smooth_scroll};

use super::Message;

const HEADING_SIZE: f32 = 20.0;
const SECTION_SPACING: f32 = 16.0;
const SECTION_GAP: f32 = 11.0;
const HEADING_BODY_GAP: f32 = 8.0;
const EVOLVE_ITEM_SPACING: f32 = 6.0;
const MATERIAL_CANVAS: u32 = 128;
const MATERIAL_SIZE: f32 = 64.0;
const MATERIAL_SPACING: f32 = 5.0;
const AMOUNT_TEXT_SIZE: f32 = 13.0;
const XP_ICON_ID: i32 = 6;
const XP_ICON_HEIGHT: f32 = 32.0;
const XP_TEXT_SIZE: f32 = 18.0;
const XP_SPACING: f32 = 5.0;
const EVOLVE_TEXT_LINES: usize = 3;
const DESCRIPTION_LINES: usize = 3;
const COMBO_UNI_SIZE: f32 = 68.0;
const COMBO_NAME_SIZE: f32 = 18.0;
const COMBO_DESCRIPTION_SIZE: f32 = 14.0;
const COMBO_CHARAGROUP_SIZE: f32 = 11.0;
const COMBO_BLOCK_GAP: f32 = 4.0;
const COMBO_CARD_GAP: f32 = 8.0;
const COMBO_CARD_PADDING: Padding = Padding { top: 4.0, right: 12.0, bottom: 4.0, left: 12.0 };
const COMBO_BODY_GAP: f32 = 16.0;
const COMBO_SLOT_SPACING: f32 = 4.0;
const COMBO_SLOT_LABEL_SIZE: f32 = 10.0;
const COMBO_TEXT_FLOOR: f32 = 8.0;
const SCROLLBAR_SPACING: f32 = 6.0;

const MIN_WINDOW_WIDTH: f32 = 800.0;
const SIDEBAR_WIDTH: f32 = roster_list::LIST_WIDTH + 16.0;
const MAIN_CONTENT_PADDING: f32 = 16.0 + 16.0;
const SCROLLBAR_GUTTER: f32 = 10.0 + SCROLLBAR_SPACING;
const DESCRIPTION_AREA_WIDTH: f32 = MIN_WINDOW_WIDTH - SIDEBAR_WIDTH - MAIN_CONTENT_PADDING - SCROLLBAR_GUTTER;

type ComboKey = (u32, usize);

type ComboMemo = RefCell<Option<(ComboKey, Vec<CatCombo>)>>;

#[derive(Clone)]
struct TrimmedIcon {
    handle: Handle,
    width: u32,
    height: u32,
}

#[derive(Default)]
pub(super) struct State {
    boxed_icons: RefCell<HashMap<i32, Option<Handle>>>,
    trimmed_icons: RefCell<HashMap<i32, Option<TrimmedIcon>>>,
    combos: ComboMemo,
    slot_icons: RefCell<HashMap<PathBuf, Option<Handle>>>,
}

impl State {
    pub(super) fn forget(&self, id: i32) {
        self.boxed_icons.borrow_mut().remove(&id);
        self.trimmed_icons.borrow_mut().remove(&id);
    }

    pub(super) fn clear_icons(&self) {
        self.boxed_icons.borrow_mut().clear();
        self.trimmed_icons.borrow_mut().clear();
    }

    pub(super) fn clear_combos(&self) {
        self.combos.borrow_mut().take();
        self.slot_icons.borrow_mut().clear();
    }

    fn boxed_icon(&self, id: i32, vfs: &Vfs) -> Option<Handle> {
        if let Some(cached) = self.boxed_icons.borrow().get(&id) {
            return cached.clone();
        }

        let handle = io::gatya_item_icon(vfs, id).and_then(|path| item_icon::load_boxed(&path, MATERIAL_CANVAS));
        self.boxed_icons.borrow_mut().insert(id, handle.clone());
        handle
    }

    fn trimmed_icon(&self, id: i32, vfs: &Vfs) -> Option<TrimmedIcon> {
        if let Some(cached) = self.trimmed_icons.borrow().get(&id) {
            return cached.clone();
        }

        let loaded = io::gatya_item_icon(vfs, id)
            .and_then(|path| item_icon::load_cropped(&path))
            .map(|(handle, width, height)| TrimmedIcon { handle, width, height });
        self.trimmed_icons.borrow_mut().insert(id, loaded.clone());
        loaded
    }

    fn slot_icon(&self, path: &Path) -> Option<Handle> {
        if let Some(cached) = self.slot_icons.borrow().get(path) {
            return cached.clone();
        }

        let handle = item_icon::load_boxed(path, COMBO_UNI_SIZE as u32);
        self.slot_icons.borrow_mut().insert(path.to_path_buf(), handle.clone());
        handle
    }

    fn resolve_combos(&self, key: ComboKey, ctx: GlobalContext<'_>) {
        if self.combos.borrow().as_ref().is_some_and(|(cached, _)| *cached == key) {
            return;
        }

        let resolved = combo::combos(ctx, key.0, key.1);
        self.combos.borrow_mut().replace((key, resolved));
    }

    pub(super) fn view<'a>(&'a self, cat: &'a CatEntry, form: usize, ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let mut lines = cat.description.get(form)
            .and_then(|d| d.as_ref())
            .filter(|lines| lines.iter().any(|line| !line.trim().is_empty()))
            .cloned()
            .unwrap_or_else(|| vec!["No description available".to_string()]);

        lines.resize(lines.len().max(DESCRIPTION_LINES), String::new());

        let description = lines.join("\n");

        let description_section = editor::target(
            column![
                text("Description").size(HEADING_SIZE),
                text(description),
            ]
                .spacing(HEADING_BODY_GAP)
                .width(Length::Fill),
            editor::Target::CatExplanation,
        );

        let mut content = column![description_section]
            .spacing(SECTION_SPACING)
            .width(Length::Fill);

        if let Some(evolve) = self.view_evolve(cat, form, ctx) {
            content = content.push(evolve);
        }

        if let Some(combos) = self.view_combos(cat.id, form, ctx) {
            content = content.push(combos);
        }

        let scroller = scrollable(content)
            .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().spacing(SCROLLBAR_SPACING)));

        smooth_scroll(scroller).into()
    }

    fn view_combos(&self, cat_id: u32, form: usize, ctx: GlobalContext<'_>) -> Option<Element<'static, Message>> {
        self.resolve_combos((cat_id, form), ctx);

        let cached = self.combos.borrow();
        let (_, combos) = cached.as_ref()?;

        if combos.is_empty() {
            return None;
        }

        let mut cards = column![].spacing(COMBO_CARD_GAP).width(Length::Fixed(DESCRIPTION_AREA_WIDTH));

        for combo in combos {
            cards = cards.push(self.view_combo(combo, cat_id));
        }

        Some(
            column![
                Space::new().height(Length::Fixed(SECTION_GAP)),
                text("Cat Combo").size(HEADING_SIZE),
                Space::new().height(Length::Fixed(HEADING_BODY_GAP)),
                container(cards).width(Length::Fill),
            ]
                .width(Length::Fill)
                .into(),
        )
    }

    fn view_combo(&self, combo: &CatCombo, cat_id: u32) -> Element<'static, Message> {
        let mut lines = vec![(combo.name.clone(), COMBO_NAME_SIZE)];

        if !combo.effect.trim().is_empty() {
            lines.push((combo.effect.clone(), COMBO_DESCRIPTION_SIZE));
        }

        if let Some(restriction) = &combo.restriction {
            lines.push((restriction.clone(), COMBO_CHARAGROUP_SIZE));
        }

        let block = fit_column(lines, COMBO_BLOCK_GAP, COMBO_TEXT_FLOOR);

        let mut slots = row![].spacing(COMBO_SLOT_SPACING);

        for member in &combo.members {
            slots = slots.push(self.view_member(member, cat_id));
        }

        let card = container(
            row![
                container(block).width(Length::Fill).center_x(Length::Fill),
                slots,
            ]
                .spacing(COMBO_BODY_GAP)
                .align_y(Vertical::Center),
        )
            .padding(COMBO_CARD_PADDING)
            .width(Length::Fill)
            .style(theme::card_container_outlined);

        editor::target(card, editor::Target::CatCombo(combo.line))
    }

    fn view_member(&self, member: &ComboMember, cat_id: u32) -> Element<'static, Message> {
        let icon: Element<'static, Message> = member
            .icon
            .as_deref()
            .and_then(|path| self.slot_icon(path))
            .map_or_else(
                || container(Space::new())
                    .width(Length::Fixed(COMBO_UNI_SIZE))
                    .height(Length::Fixed(COMBO_UNI_SIZE))
                    .style(placeholder_style)
                    .into(),
                |handle| iced_image(handle)
                    .width(Length::Fixed(COMBO_UNI_SIZE))
                    .height(Length::Fixed(COMBO_UNI_SIZE))
                    .into(),
            );

        let label = member.label();

        let slot: Element<'static, Message> = match &label {
            Some(text_id) => stack![
                icon,
                container(
                    container(text(text_id.clone()).size(COMBO_SLOT_LABEL_SIZE))
                        .padding(Padding { top: 1.0, right: 3.0, bottom: 1.0, left: 3.0 })
                        .style(badge_style)
                )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            ]
                .width(Length::Fixed(COMBO_UNI_SIZE))
                .height(Length::Fixed(COMBO_UNI_SIZE))
                .into(),
            None => icon,
        };

        let jump = member.id.filter(|id| *id != cat_id).map(|id| Message::JumpToUnit(id, member.form));

        let slot: Element<'static, Message> = match jump {
            Some(message) => button(slot).padding(0).style(slot_button_style).on_press(message).into(),
            None => slot,
        };

        let Some(hint) = member.name.clone().or(label) else {
            return slot;
        };

        tooltip(
            slot,
            container(text(hint)).padding(6).style(container::bordered_box),
            tooltip::Position::Top,
        )
            .into()
    }

    fn view_evolve<'a>(&'a self, cat: &'a CatEntry, form: usize, ctx: GlobalContext<'a>) -> Option<Element<'a, Message>> {
        let vfs = &ctx.vault.vfs;
        let (materials, xp_cost) = match form {
            2 => (cat.unitbuy.true_form_materials().collect::<Vec<_>>(), cat.unitbuy.true_form_xp_cost),
            3 => (cat.unitbuy.ultra_form_materials().collect::<Vec<_>>(), cat.unitbuy.ultra_form_xp_cost),
            _ => return None,
        };

        let evolve_text = cat.evolve_text.texts.get(form)
            .and_then(|t| t.as_ref())
            .filter(|lines| !lines.is_empty())
            .map(|lines| {
                let mut padded = lines.clone();
                padded.resize(padded.len().max(EVOLVE_TEXT_LINES), String::new());
                padded.join("\n")
            });

        if materials.is_empty() && evolve_text.is_none() && xp_cost <= 0 {
            return None;
        }

        let mut section = column![
            Space::new().height(Length::Fixed(SECTION_GAP)),
            text("Evolve").size(HEADING_SIZE),
            Space::new().height(Length::Fixed(HEADING_BODY_GAP)),
        ]
            .width(Length::Fill);

        if let Some(evolve_text) = evolve_text {
            section = section.push(text(evolve_text));
            section = section.push(Space::new().height(Length::Fixed(EVOLVE_ITEM_SPACING)));
        }

        if !materials.is_empty() {
            let mut icon_row = row![].spacing(MATERIAL_SPACING);
            for material in &materials {
                icon_row = icon_row.push(self.view_material(material.item_id, material.quantity, ctx));
            }
            section = section.push(icon_row);
            section = section.push(Space::new().height(Length::Fixed(EVOLVE_ITEM_SPACING)));
        }

        if xp_cost > 0 {
            section = section.push(self.view_xp_cost(xp_cost, vfs));
        }

        Some(section.into())
    }

    fn view_material<'a>(&'a self, item_id: i32, amount: i32, ctx: GlobalContext<'a>) -> Element<'a, Message> {
        let vfs = &ctx.vault.vfs;
        let icon: Element<'a, Message> = self.boxed_icon(item_id, vfs).map_or_else(
            || container(text(format!("ID {}", item_id)).size(AMOUNT_TEXT_SIZE + 1.0))
                .width(Length::Fixed(MATERIAL_SIZE))
                .height(Length::Fixed(MATERIAL_SIZE))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(placeholder_style)
                .into(),
            |handle| iced_image(handle)
                .width(Length::Fixed(MATERIAL_SIZE))
                .height(Length::Fixed(MATERIAL_SIZE))
                .into(),
        );

        let badge = container(text(format!("×{}", amount)).size(AMOUNT_TEXT_SIZE))
            .padding(Padding { top: 1.0, right: 4.0, bottom: 1.0, left: 4.0 })
            .style(badge_style);

        let stacked = stack![
            icon,
            container(badge)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Right)
                .align_y(Vertical::Bottom),
        ]
            .width(Length::Fixed(MATERIAL_SIZE))
            .height(Length::Fixed(MATERIAL_SIZE));

        let Some(name) = u32::try_from(item_id).ok().and_then(|id| ctx.vault.vds.items.name(vfs, id)) else {
            return stacked.into();
        };

        tooltip(
            stacked,
            container(text(name)).padding(6).style(container::bordered_box),
            tooltip::Position::Top,
        )
            .into()
    }

    fn view_xp_cost<'a>(&'a self, xp_cost: i32, vfs: &Vfs) -> Element<'a, Message> {
        let icon: Element<'a, Message> = self.trimmed_icon(XP_ICON_ID, vfs).map_or_else(
            || container(text("XP").size(AMOUNT_TEXT_SIZE))
                .width(Length::Fixed(XP_ICON_HEIGHT))
                .height(Length::Fixed(XP_ICON_HEIGHT))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(placeholder_style)
                .into(),
            |icon| {
                let display_width = if icon.height > 0 {
                    XP_ICON_HEIGHT * (icon.width as f32 / icon.height as f32)
                } else {
                    XP_ICON_HEIGHT
                };

                iced_image(icon.handle)
                    .width(Length::Fixed(display_width))
                    .height(Length::Fixed(XP_ICON_HEIGHT))
                    .into()
            },
        );

        row![
            icon,
            text(xp_cost.to_string()).size(XP_TEXT_SIZE).font(Font { weight: font::Weight::Bold, ..Font::DEFAULT }),
        ]
            .spacing(XP_SPACING)
            .align_y(Vertical::Center)
            .into()
    }
}

fn slot_button_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style::default()
}

fn placeholder_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: Border { radius: 4.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}

fn badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color { a: 0.63, ..Color::BLACK }.into()),
        text_color: Some(Color::WHITE),
        border: Border { radius: 4.0.into(), ..Border::default() },
        ..container::Style::default()
    }
}
