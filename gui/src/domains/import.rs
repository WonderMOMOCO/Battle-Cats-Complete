use std::env;
use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use iced::futures::channel::mpsc;
use iced::task;
use iced::widget::{
    button, column, container, pick_list, progress_bar, row, rule, scrollable, text, Space,
};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use smol::Timer;
use tracing::{info, trace, warn};

use kore::systems::addons::paths::{self, Presence};
use kore::common::job::{JobEvent, JobOutcome, ProgressCounter};
use kore::common::region::Region;
use kore::domains::import::{
    android, pack, raw, AdbImportType, AdbTarget, DataConfigState, ImportMode, ImportSubTab,
};
use kore::domains::settings::Settings;

use crate::app::state::AppState;
use crate::app::theme;
use crate::common::dialog;
use crate::common::watcher;
use crate::widget::ConsoleState;

const PATH_BUDGET: usize = 22;
const ELLIPSIS: &str = "\u{2026}";
const RULE_PADDING: f32 = 8.0;
const PROGRESS_TICK: Duration = Duration::from_millis(16);

#[derive(Debug, Clone)]
pub enum Message {
    ImportJobSelected(ImportSubTab),
    AdbImportTypeChanged(usize),
    AdbRegionChangedEmu(usize),
    AdbRegionChangedDec(AdbTarget),
    ImportModeChanged(ImportMode),
    SelectDecryptFolder,
    DecryptFolderPicked(Option<PathBuf>),
    SelectImportData,
    ImportDataPicked(Option<PathBuf>),
    TriggerImportJob,
    AbortImportJob,
    ImportJob(JobEvent),
    ImportBannerExpired,
    ImportProgressTick,
    ImportConsoleScrolled(scrollable::Viewport),
}

#[derive(Clone, Copy, PartialEq)]
enum Banner {
    Completed,
    Aborted,
}

#[derive(Default)]
struct JobSlot {
    running: bool,
    aborting: bool,
    log: String,
    displayed_progress: f32,
    progress_counter: Arc<ProgressCounter>,
    console: ConsoleState,
    banner: Option<Banner>,
    abort: Arc<AtomicBool>,
    job_handle: Option<task::Handle>,
    banner_handle: Option<task::Handle>,
}

impl JobSlot {
    fn begin(&mut self) {
        self.running = true;
        self.aborting = false;
        self.log.clear();
        self.displayed_progress = 1.0;
        self.progress_counter.reset(0);
        self.banner = None;
        self.abort.store(false, Ordering::Relaxed);
        if let Some(handle) = self.banner_handle.take() {
            handle.abort();
        }
    }

    fn request_abort(&mut self) {
        self.aborting = true;
        self.abort.store(true, Ordering::Relaxed);
    }

    fn target_progress(&self) -> f32 {
        if !self.running || self.progress_counter.total() == 0 {
            return 1.0;
        }

        self.progress_counter.fraction()
    }

    fn advance_progress(&mut self) {
        self.displayed_progress = self.target_progress();
    }

    fn apply(&mut self, event: JobEvent, label: &str, expired: Message) -> Task<Message> {
        match event {
            JobEvent::Log(line) => {
                if !self.log.is_empty() {
                    self.log.push('\n');
                }
                self.log.push_str(&line);
                self.console.snap_to_bottom()
            }
            JobEvent::Progress { .. } => Task::none(),
            JobEvent::Finished(outcome) => {
                self.running = false;
                self.aborting = false;
                self.abort.store(false, Ordering::Relaxed);
                self.displayed_progress = 1.0;
                self.job_handle = None;

                match outcome {
                    JobOutcome::Completed => {
                        info!("{} job completed.", label);
                        self.show_banner(Banner::Completed, expired)
                    }
                    JobOutcome::Aborted => {
                        info!("{} job aborted.", label);
                        self.show_banner(Banner::Aborted, expired)
                    }
                    JobOutcome::Failed(message) => {
                        warn!("{} job failed: {}", label, message);
                        if !self.log.is_empty() {
                            self.log.push('\n');
                        }
                        self.log.push_str(&format!("Error: {}", message));
                        self.console.snap_to_bottom()
                    }
                }
            }
        }
    }

    fn show_banner(&mut self, banner: Banner, expired: Message) -> Task<Message> {
        self.banner = Some(banner);
        if let Some(handle) = self.banner_handle.take() {
            handle.abort();
        }

        let (banner_task, handle) = Task::perform(
            async {
                Timer::after(Duration::from_secs(2)).await;
            },
            move |_| expired,
        )
        .abortable();

        self.banner_handle = Some(handle);
        banner_task
    }
}

fn job_outcome(result: Result<(), String>, abort: &AtomicBool) -> JobOutcome {
    if abort.load(Ordering::Relaxed) {
        return JobOutcome::Aborted;
    }

    result.map_or_else(JobOutcome::Failed, |()| JobOutcome::Completed)
}

#[derive(Default)]
pub struct State {
    pub config: DataConfigState,
    pub import_censored: String,
    pub decrypt_censored: String,
    import: JobSlot,
    import_succeeded: bool,
}

impl State {
    pub(crate) fn enter(&mut self) -> Task<Message> {
        self.import.console.restick()
    }

    pub(crate) fn take_import_success(&mut self) -> bool {
        std::mem::take(&mut self.import_succeeded)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.import.running {
            iced::time::every(PROGRESS_TICK).map(|_| Message::ImportProgressTick)
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState) -> Task<Message> {
        match message {
            Message::ImportJobSelected(job) => {
                trace!("Selected import sub-job");
                app_state.data.selected_import = Some(job);
            }
            Message::AdbImportTypeChanged(idx) => {
                app_state.data.adb_import_type_idx = idx;
            }
            Message::AdbRegionChangedEmu(idx) => {
                app_state.data.adb_region_idx = idx;
            }
            Message::AdbRegionChangedDec(target) => {
                self.config.adb_target = target;
            }
            Message::ImportModeChanged(mode) => {
                self.config.import_mode = mode;
            }
            Message::SelectDecryptFolder => {
                return Task::perform(dialog::folder(), Message::DecryptFolderPicked);
            }
            Message::DecryptFolderPicked(Some(folder_path)) => {
                self.config.decrypt_path = folder_path.to_string_lossy().to_string();
                self.decrypt_censored = censor_path(&self.config.decrypt_path);
                info!("Selected decrypt folder: {}", self.decrypt_censored);
            }
            Message::SelectImportData => {
                return match self.config.import_mode {
                    ImportMode::Zip => Task::perform(
                        dialog::file("Archive", &["zst", "tar", "zip"]),
                        Message::ImportDataPicked,
                    ),
                    ImportMode::Folder => Task::perform(dialog::folder(), Message::ImportDataPicked),
                };
            }
            Message::ImportDataPicked(Some(file_path)) => {
                self.config.import_path = file_path.to_string_lossy().to_string();
                self.import_censored = censor_path(&self.config.import_path);
                info!("Selected import data path: {}", self.import_censored);
            }
            Message::DecryptFolderPicked(None) | Message::ImportDataPicked(None) => {}
            Message::TriggerImportJob => {
                info!("Starting import job.");
                return self.trigger_import_job(settings, app_state);
            }
            Message::AbortImportJob => {
                warn!("Aborting import job.");
                self.import.request_abort();
            }
            Message::ImportJob(event) => {
                if let JobEvent::Finished(outcome) = &event {
                    watcher::resume();
                    self.import_succeeded |= matches!(outcome, JobOutcome::Completed);
                }

                return self.import.apply(event, "Import", Message::ImportBannerExpired);
            }
            Message::ImportBannerExpired => {
                self.import.banner = None;
                self.import.banner_handle = None;
            }
            Message::ImportProgressTick => {
                self.import.advance_progress();
            }
            Message::ImportConsoleScrolled(viewport) => {
                self.import.console.on_scroll(viewport);
            }
        }
        Task::none()
    }

    pub fn view<'a>(&'a self, app_state: &'a AppState) -> Element<'a, Message> {
        let content = self.view_import(app_state);
        let progress_section = self.view_progress_and_console();

        column![
            content,
            Space::new().height(RULE_PADDING),
            rule::horizontal(1),
            Space::new().height(RULE_PADDING),
            progress_section
        ]
            .spacing(0)
            .padding(20)
            .into()
    }

    fn view_import<'a>(&'a self, app_state: &'a AppState) -> Element<'a, Message> {
        let is_running = self.import.running;
        let adb_installed = paths::adb_status() == Presence::Installed;

        let android_btn = button(
            text("Android")
                .size(16)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
            .style(move |t: &Theme, status| theme::toggle_button(t, status, app_state.data.selected_import == Some(ImportSubTab::Emulator)))
            .on_press_maybe(if !is_running && adb_installed {
                Some(Message::ImportJobSelected(ImportSubTab::Emulator))
            } else {
                None
            });

        let import_types = vec!["All Content", "Update Only"];
        let current_type = if app_state.data.adb_import_type_idx == 1 {
            "Update Only"
        } else {
            "All Content"
        };
        let type_picker = pick_list(import_types, Some(current_type), |sel| {
            Message::AdbImportTypeChanged(if sel == "Update Only" { 1 } else { 0 })
        }).style(theme::combo_box).menu_style(theme::combo_box_menu);

        let emu_regions = vec!["Global", "Japan", "Taiwan", "Korea", "All Regions"];
        let emu_selected = emu_regions
            .get(app_state.data.adb_region_idx)
            .copied()
            .unwrap_or("Global");
        let emu_region_picker = pick_list(emu_regions, Some(emu_selected), |sel| {
            let idx = match sel {
                "Japan" => 1,
                "Taiwan" => 2,
                "Korea" => 3,
                "All Regions" => 4,
                _ => 0,
            };
            Message::AdbRegionChangedEmu(idx)
        }).style(theme::combo_box).menu_style(theme::combo_box_menu);

        let mut android_col = column![
            android_btn,
            if adb_installed {
                text("Import directly via Bridge").size(14)
            } else {
                text("Requires Android Bridge Add-On").size(14).style(text::danger)
            },
            Space::new().height(10),
        ];

        if adb_installed {
            android_col = android_col
                .push(row![text("Type: "), type_picker].align_y(Alignment::Center).spacing(5))
                .push(Space::new().height(10))
                .push(row![text("Region: "), emu_region_picker].align_y(Alignment::Center).spacing(5));
        }
        let android_col = android_col.spacing(5).width(Length::FillPortion(1));

        let pack_btn = button(
            text("Pack")
                .size(16)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
            .style(move |t: &Theme, status| theme::toggle_button(t, status, app_state.data.selected_import == Some(ImportSubTab::Decrypt)))
            .on_press_maybe(if !is_running {
                Some(Message::ImportJobSelected(ImportSubTab::Decrypt))
            } else {
                None
            });

        let dec_regions = vec!["Global", "Japan", "Taiwan", "Korea", "All Regions"];
        let dec_selected = match self.config.adb_target {
            AdbTarget::Specific(Region::En) => "Global",
            AdbTarget::Specific(Region::Ja) => "Japan",
            AdbTarget::Specific(Region::Tw) => "Taiwan",
            AdbTarget::Specific(Region::Ko) => "Korea",
            AdbTarget::All => "All Regions",
        };
        let dec_region_picker = pick_list(dec_regions, Some(dec_selected), |sel| {
            let target = match sel {
                "Japan" => AdbTarget::Specific(Region::Ja),
                "Taiwan" => AdbTarget::Specific(Region::Tw),
                "Korea" => AdbTarget::Specific(Region::Ko),
                "All Regions" => AdbTarget::All,
                _ => AdbTarget::Specific(Region::En),
            };
            Message::AdbRegionChangedDec(target)
        }).style(theme::combo_box).menu_style(theme::combo_box_menu);

        let pack_folder_label = if self.decrypt_censored.is_empty() {
            "None selected"
        } else {
            &self.decrypt_censored
        };

        let pack_col = column![
            pack_btn,
            text("Decrypt external pack files").size(14),
            Space::new().height(10),
            row![text("Region: "), dec_region_picker].align_y(Alignment::Center).spacing(5),
            Space::new().height(10),
            row![
                button("Select Folder").on_press_maybe(if !is_running {
                    Some(Message::SelectDecryptFolder)
                } else {
                    None
                }),
                text(pack_folder_label).width(Length::Fill).wrapping(text::Wrapping::None)
            ]
            .align_y(Alignment::Center)
            .spacing(10)
        ]
            .spacing(5)
            .width(Length::FillPortion(1));

        let raw_btn = button(
            text("Raw")
                .size(16)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
            .style(move |t: &Theme, status| theme::toggle_button(t, status, app_state.data.selected_import == Some(ImportSubTab::Sort)))
            .on_press_maybe(if !is_running {
                Some(Message::ImportJobSelected(ImportSubTab::Sort))
            } else {
                None
            });

        let modes = vec!["Folder", "Archive"];
        let current_mode = match self.config.import_mode {
            ImportMode::Folder => "Folder",
            _ => "Archive",
        };
        let mode_picker = pick_list(modes, Some(current_mode), |sel| {
            Message::ImportModeChanged(if sel == "Folder" {
                ImportMode::Folder
            } else {
                ImportMode::Zip
            })
        }).style(theme::combo_box).menu_style(theme::combo_box_menu);

        let raw_folder_label = if self.import_censored.is_empty() {
            "None selected"
        } else {
            &self.import_censored
        };

        let raw_col = column![
            raw_btn,
            text("Sort archive or raw files").size(14),
            Space::new().height(10),
            row![text("Source: "), mode_picker].align_y(Alignment::Center).spacing(5),
            Space::new().height(10),
            row![
                button("Select Data").on_press_maybe(if !is_running {
                    Some(Message::SelectImportData)
                } else {
                    None
                }),
                text(raw_folder_label).width(Length::Fill).wrapping(text::Wrapping::None)
            ]
            .align_y(Alignment::Center)
            .spacing(10)
        ]
            .spacing(5)
            .width(Length::FillPortion(1));

        let sections_row = row![android_col, pack_col, raw_col].spacing(20);

        let show_success = self.import.banner == Some(Banner::Completed);
        let show_aborted = self.import.banner == Some(Banner::Aborted);
        let is_aborting = is_running && self.import.aborting;

        let (button_text, can_run) = match app_state.data.selected_import {
            Some(ImportSubTab::Emulator) => {
                let is_installed = paths::adb_status() == Presence::Installed;
                (
                    if is_installed { "Start Job" } else { "Bridge Missing" },
                    is_installed,
                )
            }
            Some(ImportSubTab::Decrypt) => {
                let has_path = !self.config.decrypt_path.is_empty();
                (
                    if has_path { "Start Job" } else { "Select Source Folder" },
                    has_path,
                )
            }
            Some(ImportSubTab::Sort) => {
                let has_path = !self.config.import_path.is_empty();
                (
                    if has_path { "Start Job" } else { "Select Source Data" },
                    has_path,
                )
            }
            None => ("Select a Job", false),
        };

        let action_btn = if show_success {
            button(theme::button_label("Job Complete!").size(18))
                .style(theme::success_button)
                .width(Length::Fixed(300.0))
                .on_press_maybe(if can_run { Some(Message::TriggerImportJob) } else { None })
        } else if show_aborted {
            button(theme::button_label("Job Aborted!").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
                .on_press_maybe(if can_run { Some(Message::TriggerImportJob) } else { None })
        } else if is_aborting {
            button(theme::button_label("Aborting Job...").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
        } else if is_running {
            button(theme::button_label("Abort Job").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
                .on_press(Message::AbortImportJob)
        } else {
            button(theme::button_label(button_text).size(18))
                .style(move |t: &Theme, status| theme::toggle_button(t, status, can_run))
                .width(Length::Fixed(300.0))
                .on_press_maybe(if can_run { Some(Message::TriggerImportJob) } else { None })
        };

        let action_row = container(action_btn).width(Length::Fill).center_x(Length::Fill);

        column![
            sections_row,
            Space::new().height(RULE_PADDING),
            rule::horizontal(1),
            Space::new().height(RULE_PADDING),
            action_row
        ]
            .into()
    }

    fn view_progress_and_console(&self) -> Element<'_, Message> {
        let slot = &self.import;

        let progress = progress_bar(0.0..=1.0, slot.displayed_progress);

        let console_area = slot.console.view(&slot.log, Message::ImportConsoleScrolled);

        column![
            progress,
            Space::new().height(RULE_PADDING),
            rule::horizontal(1),
            Space::new().height(RULE_PADDING),
            console_area
        ]
            .into()
    }

    fn trigger_import_job(&mut self, settings: &Settings, app_state: &AppState) -> Task<Message> {
        if self.import.running {
            return Task::none();
        }
        let Some(job) = app_state.data.selected_import else {
            return Task::none();
        };

        watcher::suspend();
        self.import.begin();

        let (tx, rx) = mpsc::unbounded();
        let abort = self.import.abort.clone();
        let progress_counter = self.import.progress_counter.clone();
        let import_config = settings.import_config();

        match job {
            ImportSubTab::Emulator => {
                let mode = if app_state.data.adb_import_type_idx == 1 {
                    AdbImportType::Update
                } else {
                    AdbImportType::All
                };
                let region = match app_state.data.adb_region_idx {
                    0 => AdbTarget::Specific(Region::En),
                    1 => AdbTarget::Specific(Region::Ja),
                    2 => AdbTarget::Specific(Region::Tw),
                    3 => AdbTarget::Specific(Region::Ko),
                    _ => AdbTarget::All,
                };
                let thread_tx = tx.clone();
                let spawn_result = thread::Builder::new()
                    .name("android_import_worker".to_string())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        let emit = |event: JobEvent| {
                            let _ = thread_tx.unbounded_send(event);
                        };
                        let result = android::run(mode, region, import_config, emit, &abort, &progress_counter);
                        emit(JobEvent::Finished(job_outcome(result, &abort)));
                    });

                if spawn_result.is_err() {
                    warn!("Failed to spawn android import worker thread.");
                    let _ = tx.unbounded_send(JobEvent::Finished(JobOutcome::Failed(
                        "Failed to spawn worker thread".to_string(),
                    )));
                }
            }
            ImportSubTab::Decrypt => {
                let folder_path = self.config.decrypt_path.clone();

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = pack::run(
                        &folder_path,
                        ImportMode::Folder,
                        import_config,
                        emit,
                        &abort,
                        &progress_counter,
                    );
                    emit(JobEvent::Finished(job_outcome(result, &abort)));
                });
            }
            ImportSubTab::Sort => {
                let data_path = self.config.import_path.clone();
                let lang_priority = settings.general.language_priority.clone();

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = raw::run(&data_path, import_config, emit, &abort, &lang_priority, &progress_counter);
                    emit(JobEvent::Finished(job_outcome(result, &abort)));
                });
            }
        }

        let (stream_task, handle) = Task::stream(rx).abortable();
        self.import.job_handle = Some(handle);
        stream_task.map(Message::ImportJob)
    }

}

fn censor_path(path_string: &str) -> String {
    if path_string.is_empty() || path_string == "No source selected" {
        return String::new();
    }

    let mut clean = path_string.to_owned();

    if let Ok(username) = env::var("USERNAME").or_else(|_| env::var("USER"))
        && !username.is_empty()
    {
        clean = clean.replace(&username, "***");
    }

    let parts: Vec<String> = Path::new(&clean)
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();

    if parts.is_empty() {
        return shorten(&clean, PATH_BUDGET);
    }

    let separator = MAIN_SEPARATOR.to_string();
    let kept = parts.len().min(2);
    let joined = parts[parts.len() - kept..].join(&separator);
    let deeper = parts.len() > kept;

    match deeper {
        true => format!("{}{}{}", ELLIPSIS, separator, shorten(&joined, PATH_BUDGET - 2)),
        false => shorten(&joined, PATH_BUDGET),
    }
}

fn shorten(text: &str, room: usize) -> String {
    let length = text.chars().count();

    if length <= room {
        return text.to_owned();
    }

    if room < 4 {
        return ELLIPSIS.to_owned();
    }

    let keep = room - 1;
    let head = keep / 2;
    let tail = keep - head;

    format!(
        "{}{}{}",
        text.chars().take(head).collect::<String>(),
        ELLIPSIS,
        text.chars().skip(length - tail).collect::<String>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deep_path_is_cut_to_the_budget_the_column_can_hold() {
        // The label sits beside a button in a third of an 800px window; anything
        // longer wraps into the next column.
        let shown = censor_path("/home/somebody/Downloads/battle cats data/apk extraction/files");

        assert!(shown.chars().count() <= PATH_BUDGET, "{}", shown);
        assert!(shown.starts_with(ELLIPSIS), "{}", shown);
        assert!(shown.ends_with("files"), "the leaf stays readable: {}", shown);
    }

    #[test]
    fn a_short_path_is_left_whole() {
        assert_eq!(censor_path("/tmp/data"), format!("tmp{}data", MAIN_SEPARATOR));
        assert_eq!(censor_path(""), "");
    }
}
