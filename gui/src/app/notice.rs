use iced::widget::{button, column, markdown, scrollable, text, Space};
use iced::{Alignment, Element, Length, Size, Theme};
use sha2::{Digest, Sha256};

use crate::widget::{popup, smooth_scroll};

use super::Message;

const POPUP: popup::Spec = popup::Spec::new(popup::Kind::Notice, Size::new(500.0, 400.0));
const HASH_BYTES: usize = 8;
const SCROLLBAR_GAP: f32 = 8.0;

// If "NOTICE_CONTENT" is empty, no notice appears
pub(super) const NOTICE_TITLE: &str = "NOTICE";
pub(super) const NOTICE_CONTENT: &str = r#"
"#;

pub(super) fn parse_content() -> Vec<markdown::Item> {
    crate::common::markdown::parse(NOTICE_CONTENT)
}

fn digest(title: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update([0u8]);
    hasher.update(content.as_bytes());

    hasher
        .finalize()
        .iter()
        .take(HASH_BYTES)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn hash() -> String {
    digest(NOTICE_TITLE, NOTICE_CONTENT)
}

pub(super) fn should_show(acknowledged: &[String]) -> bool {
    !NOTICE_CONTENT.trim().is_empty() && !acknowledged.contains(&hash())
}

pub(super) fn update(state: &mut popup::State, message: popup::Message) -> bool {
    state.update(message, POPUP)
}

pub(super) fn view<'a>(state: &'a popup::State, window: Size, theme: Theme, items: &'a [markdown::Item]) -> Element<'a, Message> {
    state.view(NOTICE_TITLE, POPUP, window, Message::Notice, move || {
        column![
            smooth_scroll(
                scrollable(
                    markdown::view(items, markdown::Settings::with_text_size(14.0, super::theme::markdown_style(&theme)))
                        .map(Message::OpenUrl)
                )
                    .height(Length::Fill)
                    .spacing(SCROLLBAR_GAP)
            ),
            Space::new().height(20.0),
            button(text("Acknowledge").size(16.0))
                .style(button::primary)
                .on_press(Message::AcknowledgeNotice),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20.0)
        .into()
    }, None)
}

