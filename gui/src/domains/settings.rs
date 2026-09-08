mod addons;
mod disk;
mod snapshot;
mod exceptions;
pub(crate) mod general;
mod keys;
mod pem;

use iced::mouse::Interaction;
use iced::widget::{
    column, container, mouse_area, pick_list, row, rule, scrollable, text, text_input, Space, Stack,
};
use iced::{Alignment, Element, Length, Size, Task};

use kore::domains::cat::files as cat_files;
use kore::domains::settings::{lang, nightly, ContextScope, EditorMode, Utf8Mode};
use kore::domains::settings::{
    ExportBehavior, FrameCount, ImportStructure, ScrubBehavior, Settings as CoreSettings,
    SidebarBehavior,
};

use crate::app::theme;
use crate::common::feedback::NIGHTLY_ONLY_NOTICE;
use crate::app::UpdateStatus;
use crate::widget::{combo_row, hover_hint, list_row, smooth_scroll, toggle_row};

const SECTION_SPACING: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BannerForm(usize);

impl std::fmt::Display for BannerForm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(cat_files::form_name(self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    General,
    Cats,
    Enemies,
    Stages,
    Mods,
    Files,
    Import,
    Mining,
    Studio,
    Utilities,
    Animation,
    AddOns,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(Tab),
    General(general::Message),
    PreferredBannerSelected(usize),
    ToggleInvalidCats(bool),
    ToggleExpandSpirit(bool),
    DefaultLevelChanged(String),
    ToggleAutoLevel(bool),
    ToggleBumpUltra(bool),
    ToggleInvalidEnemies(bool),
    ToggleAutoFaults(bool),
    ScrubBehaviorSelected(ScrubBehavior),
    SidebarBehaviorSelected(SidebarBehavior),
    ExportBehaviorSelected(ExportBehavior),
    ToggleKeyValidation(bool),
    ToggleIgnoreModifiedApp(bool),
    ImportStructureSelected(ImportStructure),
    FrameCountSelected(FrameCount),
    StudioFrameCountSelected(FrameCount),
    Keys(keys::Message),
    OpenKeysPopup,
    Exceptions(exceptions::Message),
    Disk(disk::Message),
    Snapshot(snapshot::Message),
    Pem(pem::Message),
    Addons(addons::Message),
    ToggleUnlockGameMount(bool),
    Utf8ModeSelected(Utf8Mode),
    ContextScopeSelected(ContextScope),
    EditorModeSelected(EditorMode),
    ToggleAutoCamera(bool),
    ShowcaseWalkChanged(String),
    ShowcaseIdleChanged(String),
    ShowcaseKbChanged(String),
}

pub struct State {
    pub active_tab: Tab,

    pub default_cat_level_buffer: String,
    pub showcase_walk_buffer: String,
    pub showcase_idle_buffer: String,
    pub showcase_kb_buffer: String,

    general: general::State,
    keys: keys::State,
    exceptions: exceptions::State,
    pem: pem::State,
    addons: addons::State,
    disk: disk::State,
    snapshot: snapshot::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_tab: Tab::General,
            default_cat_level_buffer: "1".to_string(),
            showcase_walk_buffer: "0".to_string(),
            showcase_idle_buffer: "0".to_string(),
            showcase_kb_buffer: "0".to_string(),
            general: general::State::default(),
            keys: keys::State::default(),
            exceptions: exceptions::State::default(),
            pem: pem::State::default(),
            addons: addons::State::default(),
            disk: disk::State::default(),
            snapshot: snapshot::State::default(),
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, core_settings: &mut CoreSettings) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                match tab {
                    Tab::General => {
                        lang::ensure_complete_list(&mut core_settings.general.language_priority);
                        nightly::settle(&mut core_settings.general.enable_nightly);
                    }
                    Tab::Cats => self.default_cat_level_buffer = core_settings.cat_data.default_level.to_string(),
                    Tab::Files => return self.disk.update(disk::Message::Refresh).map(Message::Disk),
                    Tab::Mining => {
                        return self.snapshot.update(snapshot::Message::Refresh).map(Message::Snapshot);
                    }
                    Tab::Animation => {
                        self.showcase_walk_buffer = core_settings.animation.default_showcase_walk.to_string();
                        self.showcase_idle_buffer = core_settings.animation.default_showcase_idle.to_string();
                        self.showcase_kb_buffer = core_settings.animation.default_showcase_kb.to_string();
                    }
                    _ => {}
                }
                Task::none()
            }

            Message::General(msg) => self.general.update(msg, core_settings).map(Message::General),

            Message::PreferredBannerSelected(val) => {
                core_settings.cat_data.preferred_banner_form = val;
                Task::none()
            }
            Message::ToggleInvalidCats(val) => {
                core_settings.cat_data.show_invalid_cats = val;
                Task::none()
            }
            Message::ToggleExpandSpirit(val) => {
                core_settings.cat_data.expand_spirit_details = val;
                Task::none()
            }
            Message::DefaultLevelChanged(val) => {
                self.default_cat_level_buffer = val.clone();
                if let Ok(parsed) = val.parse::<i32>() {
                    core_settings.cat_data.default_level = parsed;
                }
                Task::none()
            }
            Message::ToggleAutoLevel(val) => {
                core_settings.cat_data.auto_level_calculations = val;
                Task::none()
            }
            Message::ToggleBumpUltra(val) => {
                core_settings.cat_data.bump_ultra_60 = val;
                Task::none()
            }

            Message::ToggleAutoFaults(val) => {
                core_settings.studio.auto_faults = val;
                Task::none()
            }

            Message::ScrubBehaviorSelected(val) => {
                core_settings.studio.scrub = val;
                Task::none()
            }
            Message::ToggleInvalidEnemies(val) => {
                core_settings.enemy_data.show_invalid_enemies = val;
                Task::none()
            }

            Message::FrameCountSelected(val) => {
                core_settings.utilities.frame_count = val;
                Task::none()
            }

            Message::StudioFrameCountSelected(val) => {
                core_settings.studio.frame_count = val;
                Task::none()
            }

            Message::SidebarBehaviorSelected(val) => {
                core_settings.stages.sidebar_behavior = val;
                Task::none()
            }

            Message::ExportBehaviorSelected(val) => {
                core_settings.mods.export_behavior = val;
                Task::none()
            }
            Message::Pem(msg) => self.pem.update(msg).map(Message::Pem),

            Message::ToggleKeyValidation(val) => {
                core_settings.game_data.enforce_key_validation = val;
                Task::none()
            }
            Message::ToggleIgnoreModifiedApp(val) => {
                core_settings.game_data.ignore_modified_app = val;
                Task::none()
            }
            Message::ImportStructureSelected(structure) => {
                core_settings.game_data.import_structure = structure;
                Task::none()
            }
            Message::Keys(msg) => self.keys.update(msg).map(Message::Keys),
            Message::OpenKeysPopup => self.keys.update(keys::Message::Open).map(Message::Keys),
            Message::Exceptions(msg) => self.exceptions.update(msg).map(Message::Exceptions),
            Message::Disk(msg) => self.disk.update(msg).map(Message::Disk),
            Message::Snapshot(msg) => self.snapshot.update(msg).map(Message::Snapshot),

            Message::Addons(msg) => self.addons.update(msg).map(Message::Addons),

            Message::Utf8ModeSelected(mode) => {
                core_settings.files.utf8_mode = mode;
                Task::none()
            }
            Message::ContextScopeSelected(scope) => {
                core_settings.files.context_scope = scope;
                Task::none()
            }
            Message::EditorModeSelected(mode) => {
                core_settings.files.editor_mode = mode;
                Task::none()
            }
            Message::ToggleUnlockGameMount(val) => {
                core_settings.files.unlock_game_mount = val;
                Task::none()
            }
            Message::ToggleAutoCamera(val) => {
                core_settings.animation.auto_set_camera_region = val;
                Task::none()
            }
            Message::ShowcaseWalkChanged(val) => {
                self.showcase_walk_buffer = val.clone();
                if let Ok(parsed) = val.parse::<i32>() {
                    core_settings.animation.default_showcase_walk = parsed;
                }
                Task::none()
            }
            Message::ShowcaseIdleChanged(val) => {
                self.showcase_idle_buffer = val.clone();
                if let Ok(parsed) = val.parse::<i32>() {
                    core_settings.animation.default_showcase_idle = parsed;
                }
                Task::none()
            }
            Message::ShowcaseKbChanged(val) => {
                self.showcase_kb_buffer = val.clone();
                if let Ok(parsed) = val.parse::<i32>() {
                    core_settings.animation.default_showcase_kb = parsed;
                }
                Task::none()
            }
        }
    }

    pub fn take_language_change(&mut self) -> bool {
        self.general.take_language_change()
    }

    pub fn keys_popup_open(&self) -> bool {
        self.keys.is_open
    }

    pub fn exceptions_popup_open(&self) -> bool {
        self.exceptions.is_open
    }

    pub fn pem_popup_open(&self) -> bool {
        self.pem.is_open
    }

    pub fn keys_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.keys.is_open.then(|| self.keys.view(window).map(Message::Keys))
    }

    pub fn exceptions_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.exceptions.is_open.then(|| self.exceptions.view(window).map(Message::Exceptions))
    }

    pub fn pem_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.pem.is_open.then(|| self.pem.view(window).map(Message::Pem))
    }

    pub fn view<'a>(&'a self, core_settings: &'a CoreSettings, updater_status: &'a UpdateStatus) -> Element<'a, Message> {
        let tab_area: Element<'a, Message> = if self.active_tab == Tab::About {
            container(self.view_about()).width(Length::Fill).height(Length::Fill).padding(15).into()
        } else {
            smooth_scroll(
                scrollable(container(self.view_tab_content(core_settings, updater_status)).padding(15))
                    .width(Length::Fill)
                    .height(Length::Fill)
            ).into()
        };

        let main_content = row![self.view_sidebar(), tab_area].height(Length::Fill);

        let mut layers: Vec<Element<'a, Message>> = vec![main_content.into()];

        if self.active_tab == Tab::General && self.general.is_dragging() {
            layers.push(
                mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                    .interaction(Interaction::Grabbing)
                    .on_move(|point| Message::General(general::Message::LanguageDragMove(point)))
                    .on_release(Message::General(general::Message::LanguageDragEnd))
                    .into()
            );
        }

        Stack::with_children(layers).into()
    }

    fn view_sidebar<'a>(&'a self) -> Element<'a, Message> {
        const SIDEBAR_WIDTH: f32 = 110.0;

        let tabs = [
            (Tab::General, "General"),
            (Tab::Cats, "Cats"),
            (Tab::Enemies, "Enemies"),
            (Tab::Stages, "Stages"),
            (Tab::Mods, "Mods"),
            (Tab::Files, "Files"),
            (Tab::Import, "Import"),
            (Tab::Mining, "Mining"),
            (Tab::Studio, "Studio"),
            (Tab::Utilities, "Utilities"),
            (Tab::Animation, "Animation"),
            (Tab::AddOns, "Add-Ons"),
            (Tab::About, "About"),
        ];

        let mut tab_list = column![].spacing(4);

        for (tab_enum, label) in tabs {
            let is_active = self.active_tab == tab_enum;
            let row_content = container(theme::button_label(label).size(14)).padding([8, 12]).width(Length::Fill);

            tab_list = tab_list.push(list_row(row_content, is_active, true, Length::Fill, Message::TabSelected(tab_enum)));
        }

        container(smooth_scroll(scrollable(tab_list).width(Length::Fill).height(Length::Fill)))
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill)
            .padding(8)
            .style(theme::list_panel_container)
            .into()
    }

    fn view_tab_content<'a>(&'a self, core_settings: &'a CoreSettings, updater_status: &'a UpdateStatus) -> Element<'a, Message> {
        match self.active_tab {
            Tab::General => column![
                header_section(text("Keys & IV").size(24), self.view_keys(core_settings)),
                self.general.view(core_settings, updater_status).map(Message::General),
            ].spacing(SECTION_SPACING).into(),
            Tab::Cats => self.view_cats(core_settings),
            Tab::Enemies => self.view_enemies(core_settings),
            Tab::Stages => self.view_stages(core_settings),
            Tab::Mods => self.view_mods(core_settings),
            Tab::Files => self.view_files(core_settings),
            Tab::Import => self.view_import(core_settings),
            Tab::Mining => self.view_mining(),
            Tab::Studio => Self::view_studio(core_settings),
            Tab::Utilities => self.view_utilities(core_settings),
            Tab::Animation => self.view_animation(core_settings),
            Tab::AddOns => self.addons.view().map(Message::Addons),
            Tab::About => self.view_about(),
        }
    }

    fn view_cats<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let banner_options: Vec<BannerForm> = (0..cat_files::FORM_COUNT).map(BannerForm).collect();

        let list_content = column![
            row![
                text("Preferred Banner Form"),
                pick_list(
                    banner_options,
                    Some(BannerForm(core_settings.cat_data.preferred_banner_form)),
                    |form| Message::PreferredBannerSelected(form.0),
                ).style(theme::combo_box).menu_style(theme::combo_box_menu),
            ].spacing(10).align_y(Alignment::Center),

            toggle_row(core_settings.cat_data.show_invalid_cats, text("Show Invalid Cats"), Some(Message::ToggleInvalidCats)),
        ].spacing(10);

        let ability_content = toggle_row(core_settings.cat_data.expand_spirit_details, text("Expand Spirit Details by Default"), Some(Message::ToggleExpandSpirit));

        let level_content = column![
            row![
                text("Default Level"),
                text_input("Level", &self.default_cat_level_buffer)
                    .on_input_maybe((!core_settings.cat_data.auto_level_calculations).then_some(Message::DefaultLevelChanged))
                    .width(Length::Fixed(60.0))
                    .style(theme::rounded_input),
            ].spacing(10).align_y(Alignment::Center),

            hover_hint(
                toggle_row(core_settings.cat_data.auto_level_calculations, text("Auto Level Calculations"), Some(Message::ToggleAutoLevel)),
                "Automatically calculates the max reasonable level for a unit based on their level caps",
            ),

            hover_hint(
                toggle_row(core_settings.cat_data.bump_ultra_60, text("Lv60 For Ultra"), Some(Message::ToggleBumpUltra)),
                "Automatically bumps the level to 60 (if not higher already) when an Ultra Form or Ultra Talent is selected",
            ),
        ].spacing(10);

        column![
            header_section(text("Cat List").size(24), list_content),
            header_section(text("Ability Display").size(24), ability_content),
            header_section(text("Level Display").size(24), level_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_enemies<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let list_content = toggle_row(core_settings.enemy_data.show_invalid_enemies, text("Show Invalid Enemies"), Some(Message::ToggleInvalidEnemies));

        column![
            header_section(text("Enemy List").size(24), list_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_stages<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let sidebar_options = vec!["Cover", "Push"];
        let current_sidebar = match core_settings.stages.sidebar_behavior {
            SidebarBehavior::Cover => "Cover",
            SidebarBehavior::Push => "Push",
        };

        let list_content = row![
            text("Sidebar Behavior"),
            pick_list(
                sidebar_options,
                Some(current_sidebar),
                |val| {
                    let behavior = match val {
                        "Cover" => SidebarBehavior::Cover,
                        _ => SidebarBehavior::Push,
                    };
                    Message::SidebarBehaviorSelected(behavior)
                }
            ).style(theme::combo_box).menu_style(theme::combo_box_menu),
        ].spacing(10).align_y(Alignment::Center);

        column![
            header_section(text("Stage List").size(24), list_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_mods<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let export_options = vec!["Automatic", "Create", "Update"];
        let current_export = match core_settings.mods.export_behavior {
            ExportBehavior::Automatic => "Automatic",
            ExportBehavior::Create => "Create",
            ExportBehavior::Update => "Update",
        };

        let export_content = column![
            theme::sized_button("Manage PEM", theme::MANAGE_BUTTON_WIDTH, theme::primary_button).on_press(Message::Pem(pem::Message::Open)),
            row![
                hover_hint(
                    text("Export Behavior"),
                    "Determines whether to scan and automatically choose, always create a new APK, or always overwrite the input APK.",
                ),
                pick_list(
                    export_options,
                    Some(current_export),
                    |val| {
                        let behavior = match val {
                            "Automatic" => ExportBehavior::Automatic,
                            "Create" => ExportBehavior::Create,
                            _ => ExportBehavior::Update,
                        };
                        Message::ExportBehaviorSelected(behavior)
                    }
                ).style(theme::combo_box).menu_style(theme::combo_box_menu),
            ].spacing(10).align_y(Alignment::Center),
        ].spacing(10);

        column![
            header_section(text("Export").size(24), export_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_keys<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        column![
            theme::sized_button("Manage Keys", theme::MANAGE_BUTTON_WIDTH, theme::primary_button).on_press(Message::Keys(keys::Message::Open)),
            hover_hint(
                toggle_row(core_settings.game_data.enforce_key_validation, text("Enforce Key Validation"), Some(Message::ToggleKeyValidation)),
                "Prevents decryption/encryption if the cryptographic keys don't match the known official file hashes\nTurn this off only if the game keys have changed and you haven't updated BCC yet",
            ),
        ].spacing(10).into()
    }

    fn view_import<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let management_content = column![
            theme::sized_button("Manage Exceptions", theme::MANAGE_BUTTON_WIDTH, theme::primary_button).on_press(Message::Exceptions(exceptions::Message::Open)),

            hover_hint(
                row![
                    text("Import Structure"),
                    pick_list(
                        ImportStructure::ALL,
                        Some(core_settings.game_data.import_structure),
                        Message::ImportStructureSelected,
                    ).style(theme::combo_box).menu_style(theme::combo_box_menu),
                ].spacing(10).align_y(Alignment::Center),
                core_settings.game_data.import_structure.hint(),
            ),

            hover_hint(
                toggle_row(core_settings.game_data.ignore_modified_app, text("Ignore Modified App"), Some(Message::ToggleIgnoreModifiedApp)),
                "Imports modded versions of the app with Vanilla package names as if they are Vanilla intalls, bypassing the import refusal",
            ),
        ].spacing(10);

        header_section(text("Management").size(24), management_content)
    }

    fn view_mining(&self) -> Element<'_, Message> {
        header_section(text("Snapshot").size(24), self.snapshot.view().map(Message::Snapshot))
    }

    fn view_files<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let mount_row = hover_hint(
            toggle_row(
                core_settings.files.unlock_game_mount,
                text("Unlock \"game\" Mount"),
                Some(Message::ToggleUnlockGameMount),
            ),
            "Allows editing vanilla game files in place\nPrefer creating a Mod so the original data stays intact",
        );

        let utf8_mode = core_settings.files.utf8_mode;

        let utf8_mode_row = combo_row(
            "UTF-8 Mode",
            utf8_mode.hint(),
            Utf8Mode::ALL,
            Some(utf8_mode),
            Some(Message::Utf8ModeSelected),
        );

        let nightly = core_settings.general.enable_nightly;
        let scope = core_settings.files.context_scope;

        let scope_row = combo_row(
            "Context Scope",
            if nightly { scope.hint() } else { NIGHTLY_ONLY_NOTICE },
            ContextScope::ALL,
            Some(scope),
            nightly.then_some(Message::ContextScopeSelected),
        );

        let editor_mode = core_settings.files.editor_mode;

        let editor_mode_row = combo_row(
            "Editor Mode",
            if nightly { editor_mode.hint() } else { NIGHTLY_ONLY_NOTICE },
            EditorMode::ALL,
            Some(editor_mode),
            nightly.then_some(Message::EditorModeSelected),
        );

        column![
            header_section(text("Disk").size(24), self.disk.view().map(Message::Disk)),
            header_section(text("Viewer").size(24), utf8_mode_row),
            header_section(text("Editor").size(24), column![scope_row, editor_mode_row, mount_row].spacing(10)),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_studio<'a>(core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let frames = frame_count_row(core_settings.studio.frame_count, Message::StudioFrameCountSelected);

        let information = hover_hint(
            toggle_row(
                core_settings.studio.auto_faults,
                text("Auto Set Fault"),
                Some(Message::ToggleAutoFaults),
            ),
            "Studio checks a rig against the side it is installed on, picked under Option page 2\nTurn this on to have opening a cat or an enemy pick that side for you, rather than leaving it where you set it",
        );

        let timeline = hover_hint(
            row![
                text("Scrub Behavior"),
                pick_list(
                    ScrubBehavior::ALL,
                    Some(core_settings.studio.scrub),
                    Message::ScrubBehaviorSelected,
                )
                .style(theme::combo_box)
                .menu_style(theme::combo_box_menu),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            "Pause stops playback when the playhead is clicked to a new frame\nRetain leaves it playing, carrying on from where it landed",
        );

        column![
            header_section(text("Animation").size(24), column![frames, information].spacing(10)),
            header_section(text("Timeline").size(24), timeline),
        ]
        .spacing(SECTION_SPACING)
        .into()
    }

    fn view_utilities<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let animation_content = frame_count_row(core_settings.utilities.frame_count, Message::FrameCountSelected);

        column![header_section(text("Animation").size(24), animation_content)]
            .spacing(SECTION_SPACING)
            .into()
    }

    fn view_animation<'a>(&'a self, core_settings: &'a CoreSettings) -> Element<'a, Message> {
        let exporter_content = column![
            hover_hint(
                toggle_row(core_settings.animation.auto_set_camera_region, text("Auto-Set Camera Region"), Some(Message::ToggleAutoCamera)),
                "Automatically calculates a Units tight bounding box when exporting\nThis setting may cause lag spikes on some devices",
            ),
        ].spacing(10);

        let showcase_content = column![
            row![text("Walk Frames"), text_input("0", &self.showcase_walk_buffer).on_input(Message::ShowcaseWalkChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
            row![text("Idle Frames"), text_input("0", &self.showcase_idle_buffer).on_input(Message::ShowcaseIdleChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
            row![text("KB Frames"), text_input("0", &self.showcase_kb_buffer).on_input(Message::ShowcaseKbChanged).width(Length::Fixed(60.0)).style(theme::rounded_input)].spacing(10).align_y(Alignment::Center),
        ].spacing(10);

        column![
            header_section(text("Exporter").size(24), exporter_content),
            header_section(text("Showcase").size(18), showcase_content),
        ].spacing(SECTION_SPACING).into()
    }

    fn view_about<'a>(&'a self) -> Element<'a, Message> {
        let license_text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../kore/assets/licenses.txt"));

        let header = column![
            text("About Battle Cats Complete").size(32),
            text("A high-performance Battle Cats toolkit built by Omochi"),
            text("Open Source & Legal Info").size(20),
        ].spacing(10);

        let legal_area = container(
            smooth_scroll(
                scrollable(text(license_text).size(11))
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
        ).width(Length::Fill).height(Length::Fill);

        column![
            column![header, rule::horizontal(1)].spacing(15),
            legal_area,
        ].spacing(0).height(Length::Fill).into()
    }
}

fn frame_count_row<'a>(selected: FrameCount, on_select: impl Fn(FrameCount) -> Message + 'a) -> Element<'a, Message> {
    combo_row(
        "Frame Count Handling",
        "Automatic bounds an animation by its own looping data, and leaves one the file never ends unbounded\nContinuous leaves every animation unbounded, as the game itself plays them",
        FrameCount::ALL,
        Some(selected),
        Some(on_select),
    )
}

fn header_section<'a, M: 'a>(header: impl Into<Element<'a, M>>, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    column![header.into(), content.into()].spacing(10).into()
}

