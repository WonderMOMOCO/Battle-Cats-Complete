use iced::alignment::{Horizontal, Vertical};
use iced::border::Radius;
use iced::theme::Palette;
use iced::widget::overlay::menu;
use iced::widget::text::Text;
use iced::widget::{button, container, markdown, pick_list, progress_bar, scrollable, text, text_editor, text_input, toggler, Button};
use iced::{font, Background, Border, Color, Length, Padding, Theme};

use crate::widget::popup::GLASS;

pub const HEADER_SEPARATOR: &str = " :: ";

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;

const DISABLED_BUTTON_SHADE: f32 = 0.6;

pub const WEAK_TEXT_ALPHA: f32 = 0.4;


pub fn weak_text_color(theme: &Theme) -> Color {
    Color { a: WEAK_TEXT_ALPHA, ..theme.palette().text }
}

fn shade_color(color: Color, factor: f32) -> Color {
    Color { r: color.r * factor, g: color.g * factor, b: color.b * factor, a: color.a }
}

pub(crate) fn darken_color(color: Color, factor: f32) -> Color {
    let darken = |c: f32| c * (1.0 - factor);
    Color { r: darken(color.r), g: darken(color.g), b: darken(color.b), a: color.a }
}

fn lighten_color(color: Color, factor: f32) -> Color {
    let lighten = |c: f32| c + (1.0 - c) * factor;
    Color { r: lighten(color.r), g: lighten(color.g), b: lighten(color.b), a: color.a }
}

#[derive(PartialEq, Clone, Copy, Debug, Default, serde::Deserialize, serde::Serialize)]
pub enum AppTheme {
    Light,
    #[default]
    Dark,
}

impl AppTheme {
    pub fn palette(self) -> Palette {
        match self {
            Self::Light => Palette {
                background: Color::from_rgb8(245, 245, 245),
                text: Color::from_rgb8(25, 25, 25),
                primary: Color::from_rgb8(31, 106, 165),
                success: Color::from_rgb8(30, 130, 80),
                warning: Color::from_rgb8(180, 130, 20),
                danger: Color::from_rgb8(190, 40, 40),
            },
            Self::Dark => Palette {
                background: Color::from_rgb8(33, 33, 33),
                text: Color::from_rgb8(230, 230, 230),
                primary: Color::from_rgb8(31, 106, 165),
                success: Color::from_rgb8(46, 160, 90),
                warning: Color::from_rgb8(210, 180, 60),
                danger: Color::from_rgb8(210, 60, 60),
            },
        }
    }

    pub fn to_iced_theme(self) -> Theme {
        let name = match self {
            Self::Light => "BCC Light",
            Self::Dark => "BCC Dark",
        };

        Theme::custom(name, self.palette())
    }
}

fn disabled_button(theme: &Theme) -> button::Style {
    let pair = theme.extended_palette().background.strong;

    button::Style {
        background: Some(Background::Color(pair.color)),
        text_color: pair.text,
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        ..button::Style::default()
    }
}

fn feedback_color_button(
    theme: &Theme,
    color: Color,
    status: button::Status,
    text_override: Option<Color>,
) -> button::Style {
    if status == button::Status::Disabled {
        return disabled_button(theme);
    }

    let background = if status == button::Status::Hovered { Color { a: 0.8, ..color } } else { color };
    let style = solid_button(background);

    match text_override {
        Some(text_color) => button::Style { text_color, ..style },
        None => style,
    }
}

fn status_color_button(color: Color, text_override: Option<Color>) -> button::Style {
    let style = solid_button(color);

    match text_override {
        Some(text_color) => button::Style { text_color, ..style },
        None => style,
    }
}

pub fn warning_status(theme: &Theme, _status: button::Status) -> button::Style {
    status_color_button(theme.palette().warning, Some(theme.palette().text))
}

pub fn success_status(theme: &Theme, _status: button::Status) -> button::Style {
    status_color_button(theme.palette().success, Some(theme.palette().text))
}

pub fn danger_status(theme: &Theme, _status: button::Status) -> button::Style {
    status_color_button(theme.palette().danger, None)
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    feedback_color_button(theme, theme.palette().primary, status, None)
}

pub fn danger_button(theme: &Theme, status: button::Status) -> button::Style {
    feedback_color_button(theme, theme.palette().danger, status, None)
}

pub fn success_button(theme: &Theme, status: button::Status) -> button::Style {
    feedback_color_button(theme, theme.palette().success, status, Some(theme.palette().text))
}

pub fn warning_button(theme: &Theme, status: button::Status) -> button::Style {
    feedback_color_button(theme, theme.palette().warning, status, Some(theme.palette().text))
}

pub type ButtonStyleFn = fn(&Theme, button::Status) -> button::Style;

pub fn feedback_button_style(feedback: Option<bool>) -> ButtonStyleFn {
    match feedback {
        Some(true) => success_button,
        Some(false) => danger_button,
        None => primary_button,
    }
}

pub const ACTION_BUTTON_WIDTH: f32 = 190.0;
pub const POPUP_ACTION_BUTTON_WIDTH: f32 = 110.0;
pub const MANAGE_BUTTON_WIDTH: f32 = 170.0;
pub const STATUS_BUTTON_WIDTH: f32 = 210.0;

pub fn sized_button<'a, Message: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    width: f32,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
) -> Button<'a, Message> {
    button(centered_text(label).size(13))
        .width(Length::Fixed(width))
        .padding([8, 10])
        .style(style)
}

pub fn toggle_button(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let palette = theme.extended_palette();

    let pair = if is_active {
        if status == button::Status::Hovered { palette.primary.strong } else { palette.primary.base }
    } else if status == button::Status::Hovered {
        palette.background.strongest
    } else {
        palette.background.strong
    };

    button::Style {
        background: Some(Background::Color(pair.color)),
        text_color: if is_active { Color::WHITE } else { pair.text },
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        ..button::Style::default()
    }
}

pub fn header_toggle_button(theme: &Theme, status: button::Status, is_selected: bool, is_available: bool) -> button::Style {
    let palette = theme.palette();
    let border_color = lighten_color(palette.background, 0.4);

    if !is_available {
        let ext = theme.extended_palette();
        let background = shade_color(ext.background.weak.color, DISABLED_BUTTON_SHADE);

        return button::Style {
            background: Some(Background::Color(background)),
            text_color: Color { a: 0.4, ..ext.background.weak.text },
            border: Border { color: border_color, width: 1.0, radius: Radius::from(RADIUS_SM) },
            ..button::Style::default()
        };
    }

    let base = toggle_button(theme, status, is_selected);

    let (border_color, border_width) = if is_selected { (Color::WHITE, 2.0) } else { (border_color, 1.0) };

    button::Style {
        border: Border { color: border_color, width: border_width, ..base.border },
        ..base
    }
}

pub const OVERLAY_BUTTON_SIZE: f32 = 30.0;

pub fn overlay_button(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let base = toggle_button(theme, status, is_active);

    button::Style {
        border: Border { color: theme.extended_palette().background.strong.color, width: 1.0, ..base.border },
        ..base
    }
}

pub fn neutral_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let pair = match status {
        button::Status::Hovered => palette.background.strongest,
        button::Status::Pressed => palette.background.weak,
        _ => palette.background.strong,
    };

    button::Style {
        background: Some(Background::Color(pair.color)),
        text_color: pair.text,
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        ..button::Style::default()
    }
}

pub fn inert_button(theme: &Theme, _status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: weak_text_color(theme),
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        ..button::Style::default()
    }
}

pub fn solid_button(background: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        ..button::Style::default()
    }
}

const SIDEBAR_RADIUS: f32 = 10.0;
const OPAQUE: f32 = 1.0;

fn sidebar_style(theme: &Theme, radius: Radius, alpha: f32) -> container::Style {
    let palette = theme.palette();
    let background = Color { a: alpha, ..shade_color(palette.background, 0.35) };
    let border_color = shade_color(palette.background, 0.6);

    container::Style {
        background: Some(Background::Color(background)),
        border: Border { color: border_color, width: 1.0, radius },
        ..container::Style::default()
    }
}

pub fn sidebar_container(theme: &Theme) -> container::Style {
    let radius = Radius { top_left: SIDEBAR_RADIUS, bottom_left: SIDEBAR_RADIUS, top_right: 0.0, bottom_right: 0.0 };

    sidebar_style(theme, radius, GLASS)
}

pub fn left_sidebar_container(theme: &Theme) -> container::Style {
    let radius = Radius { top_left: 0.0, bottom_left: 0.0, top_right: SIDEBAR_RADIUS, bottom_right: SIDEBAR_RADIUS };

    sidebar_style(theme, radius, OPAQUE)
}

pub fn list_panel_container(theme: &Theme) -> container::Style {
    let style = container::bordered_box(theme);

    container::Style {
        border: Border {
            radius: Radius { top_left: 0.0, bottom_left: 0.0, top_right: RADIUS_MD, bottom_right: RADIUS_MD },
            ..style.border
        },
        ..style
    }
}


pub const NAV_TOGGLE_SIZE: f32 = 37.0;
pub const NAV_TOGGLE_TOP: f32 = 2.5;
pub const NAV_TOGGLE_RIGHT: f32 = 10.0;

const DARK_PANEL_SHADE: f32 = 0.62;

pub fn dark_panel_container(theme: &Theme) -> container::Style {
    let background = shade_color(theme.palette().background, DARK_PANEL_SHADE);

    container::Style { background: Some(Background::Color(background)), ..list_panel_container(theme) }
}

const WORKSPACE_SHADE: f32 = 0.78;
const WORKSPACE_OUTLINE: f32 = 0.28;
const WORKSPACE_OUTLINE_WIDTH: f32 = 3.0;

pub fn workspace_container(theme: &Theme) -> container::Style {
    let background = shade_color(theme.palette().background, WORKSPACE_SHADE);

    container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            color: lighten_color(background, WORKSPACE_OUTLINE),
            width: WORKSPACE_OUTLINE_WIDTH,
            radius: Radius::from(RADIUS_MD),
        },
        ..container::Style::default()
    }
}

const WORKSPACE_HEADER_SHADE: f32 = 0.5;

pub fn workspace_header_container(theme: &Theme) -> container::Style {
    let background = shade_color(theme.palette().background, WORKSPACE_HEADER_SHADE);

    container::Style { background: Some(Background::Color(background)), ..workspace_container(theme) }
}

const CARD_SHADE: f32 = 0.15;
const CARD_ALPHA: f32 = 0.92;

pub fn card_container(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    container::Style {
        background: Some(Background::Color(Color { a: CARD_ALPHA, ..shade_color(palette.background, CARD_SHADE) })),
        border: Border { radius: Radius::from(RADIUS_MD), ..Border::default() },
        ..container::Style::default()
    }
}

const CARD_MUTED_ALPHA: f32 = 0.4;

pub fn card_container_muted(theme: &Theme) -> container::Style {
    let base = card_container(theme);
    let faded = base
        .background
        .map(|background| match background {
            Background::Color(color) => Background::Color(Color { a: color.a * CARD_MUTED_ALPHA, ..color }),
            other => other,
        });

    container::Style { background: faded, ..base }
}

const CARD_TINT_SHADE: f32 = 0.55;
const CARD_TINT_ALPHA: f32 = 0.5;

fn card_container_tinted(theme: &Theme, tint: Color) -> container::Style {
    let base = darken_color(tint, CARD_TINT_SHADE);

    container::Style {
        background: Some(Background::Color(Color { a: CARD_TINT_ALPHA, ..base })),
        ..card_container(theme)
    }
}

pub fn card_container_danger(theme: &Theme) -> container::Style {
    card_container_tinted(theme, theme.palette().danger)
}

const CARD_PRIMARY_EDGE: f32 = 0.35;
const CARD_PRIMARY_BORDER: f32 = 2.0;

pub fn card_container_primary(theme: &Theme) -> container::Style {
    let primary = theme.palette().primary;

    container::Style {
        background: Some(Background::Color(primary)),
        border: Border {
            color: darken_color(primary, CARD_PRIMARY_EDGE),
            width: CARD_PRIMARY_BORDER,
            radius: Radius::from(RADIUS_MD),
        },
        ..container::Style::default()
    }
}

pub fn card_container_outlined(theme: &Theme) -> container::Style {
    let border_color = lighten_color(theme.palette().background, 0.4);

    container::Style {
        border: Border { color: border_color, width: 3.0, radius: Radius::from(RADIUS_MD) },
        ..card_container(theme)
    }
}

pub fn combo_box(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = pick_list::default(theme, status);

    pick_list::Style { border: Border { radius: Radius::from(RADIUS_SM), ..base.border }, ..base }
}

const STALE_TEXT_ALPHA: f32 = 0.4;

pub fn combo_box_stale(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = combo_box(theme, status);

    pick_list::Style { text_color: Color { a: STALE_TEXT_ALPHA, ..base.text_color }, ..base }
}

pub fn combo_box_idle(theme: &Theme) -> container::Style {
    let base = pick_list::default(theme, pick_list::Status::Active);

    container::Style {
        background: base.background.into(),
        border: Border { radius: Radius::from(RADIUS_SM), ..base.border },
        ..container::Style::default()
    }
}

pub fn combo_box_menu(theme: &Theme) -> menu::Style {
    let palette = theme.palette();
    let base = menu::default(theme);

    menu::Style {
        selected_text_color: palette.text,
        selected_background: Background::Color(palette.primary),
        border: Border { radius: Radius::from(RADIUS_SM), ..base.border },
        ..base
    }
}

const ENABLED_OUTLINE_WIDTH: f32 = 2.0;

pub fn enabled_outline(theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: theme.palette().success,
            width: ENABLED_OUTLINE_WIDTH,
            radius: Radius::from(RADIUS_SM),
        },
        ..container::Style::default()
    }
}

pub fn status_badge(theme: &Theme, ok: bool) -> container::Style {
    let palette = theme.palette();
    let background = if ok { palette.success } else { palette.danger };

    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(Color::WHITE),
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        ..container::Style::default()
    }
}

const PROGRESS_TRACK_SHADE: f32 = 0.55;

pub fn fading_rail(theme: &Theme, reveal: f32) -> scrollable::Style {
    let palette = theme.extended_palette();
    let veiled = |color: Color| Color { a: color.a * reveal, ..color };
    let base = scrollable::default(theme, scrollable::Status::Active {
        is_horizontal_scrollbar_disabled: false,
        is_vertical_scrollbar_disabled: false,
    });

    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(veiled(palette.background.strongest.color)),
            border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        },
    };

    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        ..base
    }
}

pub fn progress_track(theme: &Theme) -> progress_bar::Style {
    let palette = theme.palette();

    progress_bar::Style {
        background: Background::Color(shade_color(palette.background, PROGRESS_TRACK_SHADE)),
        bar: Background::Color(palette.primary),
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
    }
}

pub fn ios_toggle(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let base = toggler::default(theme, status);

    toggler::Style { border_radius: Some(Radius::from(RADIUS_SM)), ..base }
}

const TABLE_HEADER_SHADE: f32 = 0.55;
const TABLE_ROW_SHADE: f32 = 0.8;

pub fn zebra_table_header(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let background = shade_color(palette.background, TABLE_HEADER_SHADE);

    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(Color::WHITE),
        border: Border {
            radius: Radius { top_left: RADIUS_SM, top_right: RADIUS_SM, bottom_left: 0.0, bottom_right: 0.0 },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn zebra_table_footer(theme: &Theme) -> container::Style {
    let base = zebra_table_header(theme);

    container::Style {
        border: Border {
            radius: Radius { top_left: 0.0, top_right: 0.0, bottom_left: RADIUS_SM, bottom_right: RADIUS_SM },
            ..base.border
        },
        ..base
    }
}

pub fn pager_step(theme: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Disabled => weak_text_color(theme),
        button::Status::Hovered | button::Status::Pressed => theme.palette().primary,
        button::Status::Active => Color::WHITE,
    };

    button::Style { text_color, ..button::Style::default() }
}

pub fn zebra_table_row(theme: &Theme, index: usize) -> container::Style {
    let palette = theme.palette();
    let background = if index.is_multiple_of(2) { shade_color(palette.background, TABLE_ROW_SHADE) } else { palette.background };

    container::Style { background: Some(Background::Color(background)), ..container::Style::default() }
}

pub fn table_cell_text<'a>(content: impl text::IntoFragment<'a>, width: Length) -> Text<'a> {
    text(content).width(width).align_x(Horizontal::Center).align_y(Vertical::Center)
}

pub fn button_label<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).width(Length::Fill).align_x(Horizontal::Center)
}

pub fn bold_text<'a>(content: impl ToString) -> Text<'a> {
    text(content.to_string()).font(font::Font { weight: font::Weight::Bold, ..font::Font::DEFAULT })
}

pub fn centered_text<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).align_x(Horizontal::Center).align_y(Vertical::Center)
}

pub const CONSOLE_BORDER_WIDTH: f32 = 4.0;
const CONSOLE_BORDER_SHADE: f32 = 0.25;
const CONSOLE_BACKGROUND_SHADE: f32 = 0.4;

pub fn mock_console_container(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let border_color = shade_color(palette.background, CONSOLE_BORDER_SHADE);
    let background = shade_color(palette.background, CONSOLE_BACKGROUND_SHADE);

    container::Style {
        background: Some(Background::Color(background)),
        border: Border { color: border_color, width: CONSOLE_BORDER_WIDTH, radius: Radius::from(RADIUS_SM) },
        ..container::Style::default()
    }
}

const NOTICE_ALPHA: f32 = 220.0 / 255.0;
const NOTICE_SHADE: f32 = 0.0;

pub fn plain_editor(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let style = text_editor::default(theme, status);

    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        ..style
    }
}

pub fn notice_banner(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(Color {
            r: NOTICE_SHADE,
            g: NOTICE_SHADE,
            b: NOTICE_SHADE,
            a: NOTICE_ALPHA,
        })),
        border: Border {
            color: palette.danger,
            width: 1.0,
            radius: Radius { top_left: 0.0, top_right: 0.0, bottom_left: RADIUS_LG, bottom_right: RADIUS_LG },
        },
        ..container::Style::default()
    }
}

pub fn rounded_editor(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let style = text_editor::default(theme, status);
    let background = match style.background {
        Background::Color(color) => Background::Color(shade_color(color, 0.5)),
        other => other,
    };

    text_editor::Style {
        background,
        border: Border { radius: Radius::from(RADIUS_SM), ..style.border },
        ..style
    }
}

pub fn rounded_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let style = text_input::default(theme, status);
    let background = match style.background {
        Background::Color(color) => Background::Color(shade_color(color, 0.5)),
        other => other,
    };

    text_input::Style {
        background,
        border: Border { radius: Radius::from(RADIUS_SM), ..style.border },
        ..style
    }
}

const INLINE_CODE_PADDING: Padding = Padding { top: 1.2, right: 3.0, bottom: 1.2, left: 3.0 };

pub fn markdown_style(theme: &Theme) -> markdown::Style {
    markdown::Style { inline_code_padding: INLINE_CODE_PADDING, ..markdown::Style::from(theme) }
}
