use std::path::PathBuf;

use iced::alignment::Horizontal;
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length, Padding, Size, Task};

use kore::domains::settings::Settings;
use kore::domains::utilities::animation as builder;

use crate::app::state::{AnimState, AppState};
use crate::app::theme;
use crate::common::dialog;
use crate::common::feedback;
use crate::systems::animation as viewer;
use crate::widget::{picker, popup};

const PANEL_PADDING: f32 = 12.0;
const ROW_GAP: f32 = 8.0;

const SETTINGS_POPUP: popup::Spec =
    popup::Spec::new(popup::Kind::UtilityAnimationSettings, Size::new(150.0, 123.0));
const SETTINGS_PADDING: f32 = 3.0;
const SETTINGS_LIFT: f32 = 8.0;

#[derive(Debug, Clone)]
pub enum Message {
    PickPng,
    PickImgcut,
    PickMamodel,
    PngPicked(Option<PathBuf>),
    ImgcutPicked(Option<PathBuf>),
    MamodelPicked(Option<PathBuf>),
    AddAnims,
    AnimsPicked(Vec<PathBuf>),
    RemoveAnim,
    RemoveConfirmExpired,
    WipeAnims,
    WipeConfirmExpired,
    Viewer(viewer::Message),
    OpenSettings,
    SettingsPopup(popup::Message),
    ToggleOrigin(bool),
    ToggleParts(bool),
    ToggleWorld(bool),
    Tick,
}

pub struct State {
    png: Option<PathBuf>,
    imgcut: Option<PathBuf>,
    mamodel: Option<PathBuf>,
    anims: Vec<PathBuf>,
    confirm_remove: feedback::Slot<PathBuf>,
    confirm_wipe: feedback::Slot<()>,
    viewer: viewer::State,
    settings_open: bool,
    settings_popup: popup::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            png: None,
            imgcut: None,
            mamodel: None,
            anims: Vec::new(),
            confirm_remove: feedback::Slot::default(),
            confirm_wipe: feedback::Slot::default(),
            viewer: viewer::State::with_popup(popup::Kind::UtilityAnimationExport),
            settings_open: false,
            settings_popup: popup::State::default(),
        }
    }
}

impl State {
    pub(super) fn pace(&self) -> std::time::Duration {
        self.viewer.pace()
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState) -> Task<Message> {
        match message {
            Message::PickPng => Task::perform(dialog::file("PNG Image", &["png"]), Message::PngPicked),
            Message::PickImgcut => {
                Task::perform(dialog::file("Sprite Cut List", &["imgcut"]), Message::ImgcutPicked)
            }
            Message::PickMamodel => Task::perform(dialog::file("Model", &["mamodel"]), Message::MamodelPicked),
            Message::AddAnims => Task::perform(dialog::files("Animation", &["maanim"]), Message::AnimsPicked),
            Message::PngPicked(picked) => {
                self.replace(picked, |state| &mut state.png);
                Task::none()
            }
            Message::ImgcutPicked(picked) => {
                self.replace(picked, |state| &mut state.imgcut);
                Task::none()
            }
            Message::MamodelPicked(picked) => {
                self.replace(picked, |state| &mut state.mamodel);
                Task::none()
            }
            Message::AnimsPicked(picked) => {
                for path in picked {
                    if !self.anims.contains(&path) {
                        self.anims.push(path);
                    }
                }
                Task::none()
            }
            Message::RemoveAnim => {
                let Some(anim) = self.viewer.selected_anim().cloned() else {
                    return Task::none();
                };

                if self.confirm_remove.take(&anim) {
                    self.anims.retain(|path| *path != anim);
                    Task::none()
                } else {
                    self.confirm_remove.set(anim, Message::RemoveConfirmExpired)
                }
            }
            Message::RemoveConfirmExpired => {
                self.confirm_remove.expire();
                Task::none()
            }
            Message::WipeAnims => {
                if self.confirm_wipe.is_set() {
                    self.anims.clear();
                    self.confirm_wipe.clear();
                    Task::none()
                } else {
                    self.confirm_wipe.set((), Message::WipeConfirmExpired)
                }
            }
            Message::WipeConfirmExpired => {
                self.confirm_wipe.expire();
                Task::none()
            }
            Message::Tick => {
                self.sync(settings, &app_state.animation);
                self.viewer.tick();
                Task::none()
            }
            Message::OpenSettings => {
                self.settings_open = true;
                Task::none()
            }
            Message::SettingsPopup(msg) => {
                if self.settings_popup.update(msg, SETTINGS_POPUP) {
                    self.settings_open = false;
                }

                Task::none()
            }
            Message::ToggleOrigin(on) => {
                settings.animation.show_origin = on;
                Task::none()
            }
            Message::ToggleParts(on) => {
                settings.animation.show_rig = on;
                Task::none()
            }
            Message::ToggleWorld(on) => {
                settings.animation.show_world = on;
                Task::none()
            }
            Message::Viewer(msg) => {
                self.viewer.update(msg, settings, &mut app_state.animation).map(Message::Viewer)
            }
        }
    }

    fn replace(&mut self, picked: Option<PathBuf>, field: impl Fn(&mut Self) -> &mut Option<PathBuf>) {
        let Some(path) = picked else {
            return;
        };

        *field(self) = Some(path);
        self.viewer.reset_playhead();
    }

    fn sync(&mut self, settings: &Settings, anim_state: &AnimState) {
        let (Some(png), Some(cut), Some(model)) = (&self.png, &self.imgcut, &self.mamodel) else {
            self.viewer.sync("", Default::default, settings, anim_state);
            return;
        };

        let key = builder::key(png, cut, model, &self.anims, settings.utilities.frame_count);
        let anims = &self.anims;

        let frames = settings.utilities.frame_count;

        self.viewer.sync(&key, || builder::clips(png, cut, model, anims, frames), settings, anim_state);
    }

    pub(crate) fn is_expanded(&self) -> bool {
        self.viewer.is_expanded()
    }

    pub fn export_popup_visible(&self) -> bool {
        self.viewer.export_popup_open()
    }

    pub(crate) fn export_scroll_task<M: 'static>(&self) -> Task<M> {
        self.viewer.export_scroll_task()
    }

    pub fn settings_popup_visible(&self) -> bool {
        self.settings_open
    }

    pub fn settings_popup_view<'a>(
        &'a self,
        settings: &'a Settings,
        window: Size,
    ) -> Option<Element<'a, Message>> {
        self.settings_open.then(|| {
            self.settings_popup.view(
                "Settings",
                SETTINGS_POPUP,
                window,
                Message::SettingsPopup,
                move || settings_content(settings),
                None,
            )
        })
    }

    pub fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.viewer.export_popup_view(window).map(|view| view.map(Message::Viewer))
    }

    pub fn expanded_view<'a>(
        &'a self,
        settings: &'a Settings,
        app_state: &'a AppState,
    ) -> Option<Element<'a, Message>> {
        self.viewer.expanded_view(settings, &app_state.animation).map(|view| view.map(Message::Viewer))
    }

    pub fn view<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState) -> Element<'a, Message> {
        column![self.view_controls(), self.view_viewer(settings, app_state)]
            .spacing(ROW_GAP)
            .height(Length::Fill)
            .into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        let files = row![
            picker::slot("Add PNG", self.png.as_deref(), Message::PickPng),
            picker::slot("Add IMGCUT", self.imgcut.as_deref(), Message::PickImgcut),
            picker::slot("Add MAMODEL", self.mamodel.as_deref(), Message::PickMamodel),
        ]
        .spacing(ROW_GAP);

        let tracks = row![
            picker::action("Add MAANIM", Message::AddAnims).style(theme::primary_button),
            self.maanim_button(),
            self.settings_button(),
        ]
        .spacing(ROW_GAP)
        .align_y(Alignment::Center);

        let body = column![centered(files), centered(tracks)].spacing(ROW_GAP);

        container(body).padding(PANEL_PADDING).into()
    }

    fn settings_button(&self) -> Element<'_, Message> {
        picker::action("Settings", Message::OpenSettings).style(theme::primary_button).into()
    }

    fn maanim_button(&self) -> Element<'_, Message> {
        if self.anims.is_empty() {
            return picker::action("Missing MAANIM", Message::RemoveAnim)
                .on_press_maybe(None)
                .style(theme::neutral_button)
                .into();
        }

        if self.viewer.is_model_selected() {
            let label = self.confirm_wipe.confirm_label("Wipe MAANIM");
            return picker::action(label, Message::WipeAnims).style(theme::danger_button).into();
        }

        let armed = self.viewer.selected_anim().is_some_and(|anim| self.confirm_remove.armed_for(anim));
        let label = if armed { feedback::CONFIRM_LABEL } else { "Remove MAANIM" };

        picker::action(label, Message::RemoveAnim).style(theme::danger_button).into()
    }

    fn view_viewer<'a>(&'a self, settings: &'a Settings, app_state: &'a AppState) -> Element<'a, Message> {
        container(self.viewer.view(settings, &app_state.animation).map(Message::Viewer))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn settings_content(settings: &Settings) -> Element<'_, Message> {
    let toggles = viewer::debug_toggles(
        settings,
        Message::ToggleOrigin,
        Message::ToggleParts,
        Message::ToggleWorld,
    );

    container(toggles)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(Padding::from(SETTINGS_PADDING).top(SETTINGS_LIFT))
        .into()
}

fn centered<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).width(Length::Fill).align_x(Horizontal::Center).into()
}
