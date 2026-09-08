mod animation;
mod clone;
mod imgcut;

use iced::widget::{column, container, row, scrollable};
use iced::{Element, Length, Size, Subscription, Task};

use kore::domains::settings::Settings;

use crate::app::state::AppState;
use crate::app::theme;
use crate::widget::{list_row, smooth_scroll};

const SIDEBAR_WIDTH: f32 = 110.0;
const SIDEBAR_PADDING: f32 = 8.0;
const ROW_GAP: f32 = 4.0;
const LABEL_SIZE: f32 = 14.0;

const TOOLS: [(&str, Tool); 3] = [("Imgcut", Tool::Imgcut), ("Animation", Tool::Animation), ("Clone", Tool::Clone)];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Imgcut,
    Animation,
    Clone,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToolSelected(Tool),
    Imgcut(imgcut::Message),
    Animation(animation::Message),
    Clone(clone::Message),
}

#[derive(Default)]
pub struct State {
    tool: Tool,
    imgcut: imgcut::State,
    animation: animation::State,
    clone: clone::State,
}

impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        if self.tool != Tool::Animation {
            return Subscription::none();
        }

        iced::time::every(self.animation.pace()).map(|_| Message::Animation(animation::Message::Tick))
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState) -> Task<Message> {
        match message {
            Message::ToolSelected(tool) => {
                self.tool = tool;

                if tool == Tool::Animation {
                    let tick = Task::done(Message::Animation(animation::Message::Tick));

                    return Task::batch([tick, self.export_scroll_task()]);
                }

                Task::none()
            }
            Message::Imgcut(msg) => self.imgcut.update(msg).map(Message::Imgcut),
            Message::Animation(msg) => self.animation.update(msg, settings, app_state).map(Message::Animation),
            Message::Clone(msg) => self.clone.update(msg).map(Message::Clone),
        }
    }

    pub fn export_popup_visible(&self) -> bool {
        self.tool == Tool::Animation && self.animation.export_popup_visible()
    }

    pub(crate) fn export_scroll_task<M: 'static>(&self) -> Task<M> {
        self.animation.export_scroll_task()
    }

    pub fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        if self.tool != Tool::Animation {
            return None;
        }

        self.animation.export_popup_view(window).map(|view| view.map(Message::Animation))
    }

    pub fn settings_popup_visible(&self) -> bool {
        self.tool == Tool::Animation && self.animation.settings_popup_visible()
    }

    pub fn settings_popup_view<'a>(
        &'a self,
        settings: &'a Settings,
        window: Size,
    ) -> Option<Element<'a, Message>> {
        if self.tool != Tool::Animation {
            return None;
        }

        self.animation.settings_popup_view(settings, window).map(|view| view.map(Message::Animation))
    }

    pub(crate) fn animation_expanded(&self) -> bool {
        self.tool == Tool::Animation && self.animation.is_expanded()
    }

    pub fn expanded_view<'a>(
        &'a self,
        settings: &'a Settings,
        app_state: &'a AppState,
    ) -> Option<Element<'a, Message>> {
        if self.tool != Tool::Animation {
            return None;
        }

        self.animation.expanded_view(settings, app_state).map(|view| view.map(Message::Animation))
    }

    pub fn view<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState) -> Element<'a, Message> {
        let body = match self.tool {
            Tool::Imgcut => self.imgcut.view().map(Message::Imgcut),
            Tool::Animation => self.animation.view(settings, app_state).map(Message::Animation),
            Tool::Clone => self.clone.view().map(Message::Clone),
        };

        row![
            self.view_sidebar(),
            container(body).width(Length::Fill).height(Length::Fill),
        ]
        .height(Length::Fill)
        .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let mut tools = column![].spacing(ROW_GAP);

        for (label, tool) in TOOLS {
            let content = container(theme::button_label(label).size(LABEL_SIZE)).padding([8, 12]).width(Length::Fill);

            tools = tools.push(list_row(content, self.tool == tool, true, Length::Fill, Message::ToolSelected(tool)));
        }

        container(smooth_scroll(scrollable(tools).width(Length::Fill).height(Length::Fill)))
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill)
            .padding(SIDEBAR_PADDING)
            .style(theme::list_panel_container)
            .into()
    }
}
