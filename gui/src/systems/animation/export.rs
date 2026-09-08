use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

use iced::futures::channel::mpsc::unbounded;
use iced::widget::{button, column, container, operation, pick_list, progress_bar, row, rule, scrollable, text, text_input, toggler, tooltip, Id};
use iced::{task, Alignment, Element, Length, Size, Task, Theme};
use tracing::trace;

use nyanko::graphics::rig::Animation;

use kore::common::job::ProgressCounter;
use kore::systems::addons::paths::{self, Presence};
use kore::systems::animation::export::{find_bounds, find_loop, leader, process, BoundsOutcome, EncoderStatus, ExportFormat, ExportMode, ExportRequest, FrameTiming, LoopStatus, ShowcaseLengths};
use kore::systems::animation::{playback_frames, Role};
use kore::domains::settings::Settings;

use crate::app::state::AnimState;
use crate::app::theme;
use crate::widget::{popup, section, smooth_scroll};

use super::data;
use super::diagnostics;
use super::offscreen::{self, Camera};
use super::overlay::Region;

const MODE_OPTIONS: [&str; 3] = ["Manual", "Loop", "Showcase"];
const FORMAT_OPTIONS: [&str; 8] = ["GIF", "WebP", "AVIF", "PNG", "MP4", "MKV", "WebM", "ZIP"];

const CONTENT_PADDING: f32 = 20.0;
const SECTION_SPACING: f32 = 14.0;
const POPUP_SIZE: Size = Size::new(320.0, 500.0 - SECTION_SPACING);
const ROW_SPACING: f32 = 6.0;
const FIELD_SPACING: f32 = 8.0;
const FIELD_LABEL_WIDTH: f32 = 82.0;
const NAME_INPUT_WIDTH: f32 = 170.0;
const SMALL_INPUT_WIDTH: f32 = 55.0;
const AXIS_INPUT_WIDTH: f32 = 100.0;
const AXIS_LABEL_WIDTH: f32 = 18.0;
const COMBO_WIDTH: f32 = 130.0;
const BUTTON_STATUS_GAP: f32 = 8.0;
const RULE_HEIGHT: f32 = 1.0;
const SCROLLBAR_GAP: f32 = 2.0;
const CONTROL_TEXT_SIZE: f32 = 13.0;

const DEFAULT_WALK_LEN: i32 = 90;
const DEFAULT_IDLE_LEN: i32 = 90;
const DEFAULT_KB_LEN: i32 = 60;
const DEFAULT_CULL: i32 = 100;
const PERCENT_MIN: i32 = 0;
const PERCENT_MAX: i32 = 100;

type JobKey = (String, Option<usize>);

enum JobResult {
    Completed,
    Terminated,
}

enum JobPhase {
    Running,
    Aborting,
    Done { result: JobResult, shown_at: Option<Instant> },
}

struct JobState {
    phase: JobPhase,
    abort: Arc<AtomicBool>,
    render_progress: Arc<AtomicI32>,
    rendered_frames: i32,
    encoded_frames: i32,
    total_frames: i32,
    encoder_handle: task::Handle,
}

enum SearchResult {
    Found,
    Terminated,
    Error(String),
}

enum SearchPhase {
    Running,
    Aborting,
    Done { result: SearchResult, shown_at: Option<Instant> },
}

struct SearchJob {
    phase: SearchPhase,
    abort: Arc<AtomicBool>,
    progress: Arc<ProgressCounter>,
    frames_searched: usize,
    start_time: Instant,
    task_handle: task::Handle,
}

struct ExportForm {
    frame_start: i32,
    frame_end: i32,
    max_frame: i32,
    frame_start_str: String,
    frame_end_str: String,
    export_mode: ExportMode,
    loop_supported: bool,
    loop_tolerance: i32,
    loop_tolerance_str: String,
    loop_min: i32,
    loop_min_str: String,
    loop_max: Option<i32>,
    loop_max_str: String,
    cull_percent: i32,
    cull_percent_str: String,
    showcase_walk_str: String,
    showcase_idle_str: String,
    showcase_attack_str: String,
    showcase_kb_str: String,
    showcase_walk_len: i32,
    showcase_idle_len: i32,
    detected_attack_len: i32,
    showcase_attack_len: i32,
    showcase_kb_len: i32,
    detected_walk_len: i32,
    detected_idle_len: i32,
    last_known_walk_default: i32,
    last_known_idle_default: i32,
    last_known_kb_default: i32,
    fps: i32,
    zoom: f32,
    region_x: f32,
    region_y: f32,
    region_w: f32,
    region_h: f32,
    region_x_str: String,
    region_y_str: String,
    region_w_str: String,
    region_h_str: String,
    file_name: String,
    name_prefix: String,
    format: ExportFormat,
    quality_percent: i32,
    quality_percent_str: String,
    compression_percent: i32,
    compression_percent_str: String,
    background: bool,
    debug: bool,
    user_bg_preference: bool,
}

impl Default for ExportForm {
    fn default() -> Self {
        Self {
            frame_start: 0,
            frame_end: 0,
            max_frame: 100,
            frame_start_str: String::new(),
            frame_end_str: String::new(),
            export_mode: ExportMode::Manual,
            loop_supported: false,
            loop_tolerance: 30,
            loop_tolerance_str: String::new(),
            loop_min: 15,
            loop_min_str: String::new(),
            loop_max: None,
            loop_max_str: String::new(),
            cull_percent: DEFAULT_CULL,
            cull_percent_str: String::new(),
            showcase_walk_str: String::new(),
            showcase_idle_str: String::new(),
            showcase_attack_str: String::new(),
            showcase_kb_str: String::new(),
            showcase_walk_len: DEFAULT_WALK_LEN,
            showcase_idle_len: DEFAULT_IDLE_LEN,
            detected_attack_len: 0,
            showcase_attack_len: 0,
            showcase_kb_len: DEFAULT_KB_LEN,
            detected_walk_len: DEFAULT_WALK_LEN,
            detected_idle_len: DEFAULT_IDLE_LEN,
            last_known_walk_default: DEFAULT_WALK_LEN,
            last_known_idle_default: DEFAULT_IDLE_LEN,
            last_known_kb_default: DEFAULT_KB_LEN,
            fps: 30,
            zoom: 1.0,
            region_x: 0.0,
            region_y: 0.0,
            region_w: 0.0,
            region_h: 0.0,
            region_x_str: String::new(),
            region_y_str: String::new(),
            region_w_str: String::new(),
            region_h_str: String::new(),
            file_name: String::new(),
            name_prefix: String::new(),
            format: ExportFormat::Gif,
            quality_percent: 80,
            quality_percent_str: String::new(),
            compression_percent: 30,
            compression_percent_str: String::new(),
            background: false,
            debug: false,
            user_bg_preference: false,
        }
    }
}

impl ExportForm {
    fn with_settings(settings: &Settings) -> Self {
        Self {
            cull_percent: settings.animation.bounds_cull,
            showcase_walk_len: settings.animation.default_showcase_walk,
            showcase_idle_len: settings.animation.default_showcase_idle,
            showcase_kb_len: settings.animation.default_showcase_kb,
            last_known_walk_default: settings.animation.default_showcase_walk,
            last_known_idle_default: settings.animation.default_showcase_idle,
            last_known_kb_default: settings.animation.default_showcase_kb,
            ..Self::default()
        }
    }

    fn to_request(&self) -> ExportRequest {
        ExportRequest {
            timing: FrameTiming {
                mode: self.export_mode.clone(),
                frame_start: self.frame_start,
                frame_end: self.frame_end,
                loop_supported: self.loop_supported,
            },
            showcase: ShowcaseLengths {
                walk: self.showcase_walk_len,
                idle: self.showcase_idle_len,
                attack: self.showcase_attack_len,
                kb: self.showcase_kb_len,
            },
            file_name: (!self.file_name.trim().is_empty()).then(|| self.file_name.clone()),
            name_prefix: self.name_prefix.clone(),
            format: self.format.clone(),
            quality_percent: self.quality_percent as u32,
            compression_percent: self.compression_percent as u32,
            fps: self.fps as u32,
            width: self.region_w.round() as u32,
            height: self.region_h.round() as u32,
            background: self.background,
        }
    }
}

pub struct State {
    spec: popup::Spec,
    popup: popup::State,
    scroll: Id,
    offset: f32,
    exporter: ExportForm,
    jobs: HashMap<JobKey, JobState>,
    loop_job: Option<(JobKey, SearchJob)>,
    bounds_job: Option<(JobKey, SearchJob)>,
    synced_key: Option<JobKey>,
    scanned_showcase: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Scrolled(f32),
    SetMode(ExportMode),
    SetFormat(ExportFormat),
    SetFileName(String),
    SetStartFrame(String),
    SetEndFrame(String),
    SetLoopTolerance(String),
    SetLoopMin(String),
    SetLoopMax(String),
    SetCull(String),
    SetShowcaseWalk(String),
    SetShowcaseIdle(String),
    SetShowcaseAttack(String),
    SetShowcaseKb(String),
    SetRegionX(String),
    SetRegionY(String),
    SetRegionW(String),
    SetRegionH(String),
    SetQuality(String),
    SetCompression(String),
    ToggleBackground(bool),
    ToggleDebug(bool),
    BeginExport,
    AbortExport,
    Encoder(JobKey, EncoderStatus),
    SetCamera,
    UseBounds,
    AbortBounds,
    BoundsCalculated(JobKey, BoundsOutcome),
    FindLoop,
    AbortLoopSearch,
    LoopSearch(JobKey, LoopStatus),
}

impl State {
    pub(super) fn new(kind: popup::Kind) -> Self {
        Self {
            spec: popup::Spec::new(kind, POPUP_SIZE),
            popup: popup::State::default(),
            scroll: Id::unique(),
            offset: 0.0,
            exporter: ExportForm::default(),
            jobs: HashMap::new(),
            loop_job: None,
            bounds_job: None,
            synced_key: None,
            scanned_showcase: None,
        }
    }

    pub(super) fn restore_scroll<M: 'static>(&self) -> Task<M> {
        operation::scroll_to(self.scroll.clone(), scrollable::AbsoluteOffset { x: 0.0, y: self.offset })
    }

    pub fn sync(&mut self, data: &data::State, settings: &Settings, anim_state: &AnimState) {
        self.check_settings_defaults(settings);

        let key = (data.export_name().to_string(), data.selected());

        if self.synced_key.as_ref() != Some(&key) {
            let unit_changed = self.synced_key.as_ref().is_none_or(|(id, _)| *id != key.0);
            self.synced_key = Some(key);

            if unit_changed {
                self.reset(settings, anim_state);
            }

            self.exporter.loop_supported = data.loop_supported();

            if self.exporter.export_mode == ExportMode::Loop && !self.exporter.loop_supported {
                self.exporter.export_mode = ExportMode::Manual;
                self.exporter.frame_start = 0;
                self.exporter.frame_end = 0;
                self.exporter.frame_start_str.clear();
                self.exporter.frame_end_str.clear();
            }

            if self.exporter.export_mode != ExportMode::Showcase {
                match &data.current_anim {
                    Some(anim) => {
                        self.exporter.max_frame = anim.declared_frames() - 1;
                        self.exporter.frame_start = 0;
                        self.exporter.frame_end = self.exporter.max_frame;
                    }
                    None => {
                        self.exporter.max_frame = 0;
                        self.exporter.frame_start = 0;
                        self.exporter.frame_end = 0;
                    }
                }
                self.exporter.frame_start_str.clear();
                self.exporter.frame_end_str.clear();
            }

            let slug = data.current_clip().map_or_else(|| "anim".to_string(), |clip| clip.slug());
            self.exporter.name_prefix = derive_name_prefix(data.export_name(), &slug);
        }

        self.maybe_scan_showcase(data, settings);
    }

    fn check_settings_defaults(&mut self, settings: &Settings) {
        let walk_mismatch = self.exporter.last_known_walk_default != settings.animation.default_showcase_walk;
        let idle_mismatch = self.exporter.last_known_idle_default != settings.animation.default_showcase_idle;
        let kb_mismatch = self.exporter.last_known_kb_default != settings.animation.default_showcase_kb;

        if !walk_mismatch && !idle_mismatch && !kb_mismatch {
            return;
        }

        self.exporter.last_known_walk_default = settings.animation.default_showcase_walk;
        self.exporter.last_known_idle_default = settings.animation.default_showcase_idle;
        self.exporter.last_known_kb_default = settings.animation.default_showcase_kb;

        if self.exporter.showcase_walk_str.is_empty() {
            self.exporter.showcase_walk_len = settings.animation.default_showcase_walk;
        }
        if self.exporter.showcase_idle_str.is_empty() {
            self.exporter.showcase_idle_len = settings.animation.default_showcase_idle;
        }
        if self.exporter.showcase_kb_str.is_empty() {
            self.exporter.showcase_kb_len = settings.animation.default_showcase_kb;
        }

        self.scanned_showcase = None;
    }

    fn maybe_scan_showcase(&mut self, data: &data::State, settings: &Settings) {
        if self.exporter.export_mode != ExportMode::Showcase {
            return;
        }

        let export_name = data.export_name();
        if self.scanned_showcase.as_deref() == Some(export_name) {
            return;
        }
        self.scanned_showcase = Some(export_name.to_string());

        let parse_anim = |role: Role| -> Option<Animation> {
            let bytes = fs::read(data.role_path(role)?).ok()?;
            Animation::parse(&bytes).ok()
        };

        if let Some(attack) = parse_anim(Role::Attack) {
            let total_attack_frames = attack.declared_frames();
            self.exporter.detected_attack_len = total_attack_frames;
            if self.exporter.showcase_attack_str.is_empty() {
                self.exporter.showcase_attack_len = total_attack_frames;
            }
        }

        if let Some(walk) = parse_anim(Role::Walk) {
            let walk_loop = playback_frames(&walk);
            let new_walk_length = if walk_loop <= 2 { 0 } else { settings.animation.default_showcase_walk };
            self.exporter.detected_walk_len = new_walk_length;

            if self.exporter.showcase_walk_str.is_empty()
                || self.exporter.showcase_walk_len == settings.animation.default_showcase_walk {
                self.exporter.showcase_walk_len = new_walk_length;
            }
        }

        if let Some(idle) = parse_anim(Role::Idle) {
            let idle_loop = playback_frames(&idle);
            let new_idle_length = if idle_loop <= 2 { 0 } else { settings.animation.default_showcase_idle };
            self.exporter.detected_idle_len = new_idle_length;

            if self.exporter.showcase_idle_str.is_empty()
                || self.exporter.showcase_idle_len == settings.animation.default_showcase_idle {
                self.exporter.showcase_idle_len = new_idle_length;
            }
        }
    }

    fn reset(&mut self, settings: &Settings, anim_state: &AnimState) {
        let previous_mode = self.exporter.export_mode.clone();
        self.exporter = ExportForm::with_settings(settings);
        self.exporter.export_mode = previous_mode;
        self.exporter.format = anim_state.last_export_format.clone();
        self.exporter.user_bg_preference = anim_state.export_background;
        self.exporter.background =
            is_forced_opaque(&self.exporter.format) || self.exporter.user_bg_preference;
        self.exporter.quality_percent = anim_state.last_export_quality.unwrap_or(80);
        self.exporter.quality_percent_str = anim_state.last_export_quality.map_or_else(String::new, |v| v.to_string());
        self.exporter.compression_percent = anim_state.last_export_compression.unwrap_or(30);
        self.exporter.compression_percent_str = anim_state.last_export_compression.map_or_else(String::new, |v| v.to_string());
        self.exporter.debug = anim_state.include_debug;
    }

    pub fn set_region(&mut self, region: Region) {
        self.exporter.region_x = region.x;
        self.exporter.region_y = region.y;
        self.exporter.region_w = region.w;
        self.exporter.region_h = region.h;
        self.exporter.region_x_str = region.x.to_string();
        self.exporter.region_y_str = region.y.to_string();
        self.exporter.region_w_str = region.w.to_string();
        self.exporter.region_h_str = region.h.to_string();
        self.exporter.zoom = 1.0;
    }

    pub fn camera_region(&self, open: bool) -> Option<Region> {
        if open && self.exporter.region_w > 0.1 && self.exporter.region_h > 0.1 {
            Some(Region {
                x: self.exporter.region_x,
                y: self.exporter.region_y,
                w: self.exporter.region_w,
                h: self.exporter.region_h,
            })
        } else {
            None
        }
    }

    pub(super) fn busy(&self) -> bool {
        !self.jobs.is_empty() || self.bounds_job.is_some()
    }

    pub fn tick(&mut self) {
        if let Some(key) = &self.synced_key
            && let Some(job) = self.jobs.get_mut(key) {
            job.rendered_frames = job.render_progress.load(Ordering::Relaxed);

            if let JobPhase::Done { shown_at, .. } = &mut job.phase
                && shown_at.is_none() {
                *shown_at = Some(Instant::now());
            }
        }

        self.jobs.retain(|_, job| {
            if let JobPhase::Done { shown_at: Some(at), .. } = &job.phase
                && at.elapsed().as_secs_f32() > 3.0 {
                job.encoder_handle.abort();
                return false;
            }
            true
        });

        if let Some((_, job)) = &mut self.bounds_job {
            job.frames_searched = job.progress.current();
        }

        advance_search(&mut self.loop_job, self.synced_key.as_ref());
        advance_search(&mut self.bounds_job, self.synced_key.as_ref());
    }

    pub fn update(
        &mut self,
        message: Message,
        data: &data::State,
        settings: &mut Settings,
        anim_state: &mut AnimState,
        open: &mut bool,
        marks: diagnostics::Shot,
    ) -> Task<Message> {
        match message {
            Message::Popup(msg) => {
                if self.popup.update(msg, self.spec) {
                    *open = false;
                }
            }
            Message::Scrolled(offset) => self.offset = offset,
            Message::SetMode(mode) => {
                if mode == ExportMode::Showcase {
                    self.exporter.showcase_walk_str.clear();
                    self.exporter.showcase_idle_str.clear();
                    self.exporter.showcase_attack_str.clear();
                    self.exporter.showcase_kb_str.clear();
                }
                self.exporter.export_mode = mode;
                self.maybe_scan_showcase(data, settings);
            }
            Message::SetFormat(format) => {
                self.exporter.format = format.clone();
                anim_state.last_export_format = format;
                self.exporter.background =
                    is_forced_opaque(&self.exporter.format) || self.exporter.user_bg_preference;
            }
            Message::SetFileName(name) => self.exporter.file_name = name,
            Message::SetStartFrame(value) => {
                self.exporter.frame_start_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.frame_start = 0;
                } else if let Ok(parsed) = value.trim().parse::<i32>() {
                    self.exporter.frame_start = parsed;
                }
            }
            Message::SetEndFrame(value) => {
                self.exporter.frame_end_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.frame_end = self.exporter.max_frame;
                } else if let Ok(parsed) = value.trim().parse::<i32>() {
                    self.exporter.frame_end = parsed;
                }
            }
            Message::SetLoopTolerance(value) => {
                if !is_typable_number(&value, false, false) {
                    return Task::none();
                }
                self.exporter.loop_tolerance_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.loop_tolerance = parsed;
                }
            }
            Message::SetCull(value) => {
                let Some((percent, text)) = percentage(&value, DEFAULT_CULL) else {
                    return Task::none();
                };

                self.exporter.cull_percent = percent;
                self.exporter.cull_percent_str = text;
                settings.animation.bounds_cull = percent;
            }
            Message::SetLoopMin(value) => {
                self.exporter.loop_min_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.loop_min = parsed;
                }
            }
            Message::SetLoopMax(value) => {
                self.exporter.loop_max = value.parse::<i32>().ok();
                self.exporter.loop_max_str = value;
            }
            Message::SetShowcaseWalk(value) => {
                self.exporter.showcase_walk_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.showcase_walk_len = self.exporter.detected_walk_len;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_walk_len = parsed;
                }
            }
            Message::SetShowcaseIdle(value) => {
                self.exporter.showcase_idle_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.showcase_idle_len = self.exporter.detected_idle_len;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_idle_len = parsed;
                }
            }
            Message::SetShowcaseAttack(value) => {
                self.exporter.showcase_attack_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.showcase_attack_len = self.exporter.detected_attack_len;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_attack_len = parsed;
                }
            }
            Message::SetShowcaseKb(value) => {
                self.exporter.showcase_kb_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.showcase_kb_len = settings.animation.default_showcase_kb;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_kb_len = parsed;
                }
            }
            Message::SetRegionX(value) => {
                if !is_typable_number(&value, true, true) {
                    return Task::none();
                }
                self.exporter.region_x_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.region_x = 0.0;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_x = parsed;
                }
            }
            Message::SetRegionY(value) => {
                if !is_typable_number(&value, true, true) {
                    return Task::none();
                }
                self.exporter.region_y_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.region_y = 0.0;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_y = parsed;
                }
            }
            Message::SetRegionW(value) => {
                if !is_typable_number(&value, false, true) {
                    return Task::none();
                }
                self.exporter.region_w_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.region_w = 0.0;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_w = parsed;
                }
            }
            Message::SetRegionH(value) => {
                if !is_typable_number(&value, false, true) {
                    return Task::none();
                }
                self.exporter.region_h_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.region_h = 0.0;
                } else if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_h = parsed;
                }
            }
            Message::SetQuality(value) => {
                let Some((percent, text)) = percentage(&value, PERCENT_MAX) else {
                    return Task::none();
                };

                self.exporter.quality_percent = percent;
                anim_state.last_export_quality = (!text.is_empty()).then_some(percent);
                self.exporter.quality_percent_str = text;
            }
            Message::SetCompression(value) => {
                let Some((percent, text)) = percentage(&value, PERCENT_MIN) else {
                    return Task::none();
                };

                self.exporter.compression_percent = percent;
                anim_state.last_export_compression = (!text.is_empty()).then_some(percent);
                self.exporter.compression_percent_str = text;
            }
            Message::ToggleDebug(enabled) => {
                self.exporter.debug = enabled;
                anim_state.include_debug = enabled;
            }
            Message::ToggleBackground(enabled) => {
                self.exporter.background = enabled;
                self.exporter.user_bg_preference = enabled;
                anim_state.export_background = enabled;
            }
            Message::BeginExport => {
                let Some(key) = self.synced_key.clone() else {
                    return Task::none();
                };

                if matches!(
                    self.jobs.get(&key).map(|job| &job.phase),
                    Some(JobPhase::Running | JobPhase::Aborting)
                ) {
                    return Task::none();
                }

                let Some(unit) = data.held_unit.clone() else {
                    return Task::none();
                };

                if self.exporter.region_w <= 0.1 || self.exporter.region_h <= 0.1 {
                    return Task::none();
                }

                self.exporter.region_w = self.exporter.region_w.round();
                self.exporter.region_h = self.exporter.region_h.round();

                let request = self.exporter.to_request();
                let config = process::build_config(&request);
                let total_frames = (config.end_frame - config.start_frame).abs() + 1;

                let timing = FrameTiming {
                    frame_start: config.start_frame,
                    frame_end: config.end_frame,
                    ..request.timing.clone()
                };

                let abort = Arc::new(AtomicBool::new(false));
                let render_progress = Arc::new(AtomicI32::new(0));

                let (frame_tx, frame_rx) = mpsc::channel();
                let (tx, rx) = unbounded();

                let worker_abort = abort.clone();
                thread::spawn(move || {
                    leader::run(config, frame_rx, |status| {
                        let _ = tx.unbounded_send(status);
                    }, &worker_abort);
                });

                offscreen::spawn(offscreen::Job {
                    unit,
                    animation: data.current_anim.clone(),
                    role_paths: data.role_paths(),
                    offset: data.offset(),
                    timing,
                    lengths: request.showcase,
                    camera: Camera {
                        region_x: self.exporter.region_x,
                        region_y: self.exporter.region_y,
                        zoom: self.exporter.zoom,
                    },
                    region_w: self.exporter.region_w,
                    region_h: self.exporter.region_h,
                    fps: self.exporter.fps,
                    background: self.exporter.background,
                    debug: self.exporter.debug.then_some(marks),
                    tx: frame_tx,
                    abort: abort.clone(),
                    progress: render_progress.clone(),
                });

                let map_key = key.clone();
                let (encoder_task, handle) = Task::stream(rx)
                    .map(move |status| Message::Encoder(map_key.clone(), status))
                    .abortable();

                if let Some(previous) = self.jobs.insert(key, JobState {
                    phase: JobPhase::Running,
                    abort,
                    render_progress,
                    rendered_frames: 0,
                    encoded_frames: 0,
                    total_frames,
                    encoder_handle: handle,
                }) {
                    previous.encoder_handle.abort();
                }

                return encoder_task;
            }
            Message::AbortExport => {
                if let Some(key) = &self.synced_key
                    && let Some(job) = self.jobs.get_mut(key)
                    && matches!(job.phase, JobPhase::Running) {
                    job.abort.store(true, Ordering::Relaxed);
                    job.phase = JobPhase::Aborting;
                }
            }
            Message::FindLoop => {
                let Some(key) = self.synced_key.clone() else {
                    return Task::none();
                };

                if matches!(
                    self.jobs.get(&key).map(|job| &job.phase),
                    Some(JobPhase::Running | JobPhase::Aborting)
                ) {
                    return Task::none();
                }

                if self.loop_job.as_ref().is_some_and(|(job_key, job)| {
                    *job_key == key && matches!(job.phase, SearchPhase::Running | SearchPhase::Aborting)
                }) {
                    return Task::none();
                }

                let Some(unit) = data.held_unit.clone() else {
                    return Task::none();
                };
                let Some(animation) = data.current_anim.clone() else {
                    return Task::none();
                };

                let tolerance = self.exporter.loop_tolerance as f32;
                let minimum = self.exporter.loop_min;
                let maximum = self.exporter.loop_max;

                let abort = Arc::new(AtomicBool::new(false));
                let (tx, rx) = unbounded();

                let worker_abort = abort.clone();
                thread::spawn(move || {
                    find_loop::search(&unit, &animation, tolerance, minimum, maximum, |status| {
                        let _ = tx.unbounded_send(status);
                    }, &worker_abort);
                });

                let map_key = key.clone();
                let (search_task, handle) = Task::stream(rx)
                    .map(move |status| Message::LoopSearch(map_key.clone(), status))
                    .abortable();

                if let Some((_, previous)) = self.loop_job.take() {
                    previous.abort.store(true, Ordering::Relaxed);
                    previous.task_handle.abort();
                }

                self.loop_job = Some((key, SearchJob {
                    phase: SearchPhase::Running,
                    abort,
                    progress: Arc::new(ProgressCounter::default()),
                    frames_searched: 0,
                    start_time: Instant::now(),
                    task_handle: handle,
                }));

                return search_task;
            }
            Message::AbortLoopSearch => {
                if let Some(key) = &self.synced_key
                    && let Some((job_key, job)) = &mut self.loop_job
                    && job_key == key
                    && matches!(job.phase, SearchPhase::Running) {
                    job.abort.store(true, Ordering::Relaxed);
                    job.phase = SearchPhase::Aborting;
                }
            }
            Message::LoopSearch(key, status) => {
                let Some((job_key, job)) = &mut self.loop_job else {
                    trace!("Dropping loop search event for missing job {:?}", key);
                    return Task::none();
                };

                if *job_key != key {
                    trace!("Dropping stale loop search event for {:?}", key);
                    return Task::none();
                }

                match status {
                    LoopStatus::Searching(frames) => job.frames_searched = frames,
                    LoopStatus::Found(start, end) => {
                        let was_aborting = matches!(job.phase, SearchPhase::Aborting);
                        job.phase = SearchPhase::Done {
                            result: if was_aborting { SearchResult::Terminated } else { SearchResult::Found },
                            shown_at: None,
                        };

                        if !was_aborting && self.synced_key.as_ref() == Some(&key) {
                            self.exporter.frame_start = start;
                            self.exporter.frame_end = end;
                            self.exporter.frame_start_str = start.to_string();
                            self.exporter.frame_end_str = end.to_string();
                        }
                    }
                    LoopStatus::Error(message) => {
                        let was_aborting = matches!(job.phase, SearchPhase::Aborting);
                        job.phase = SearchPhase::Done {
                            result: if was_aborting { SearchResult::Terminated } else { SearchResult::Error(message) },
                            shown_at: None,
                        };
                    }
                }
            }
            Message::Encoder(key, status) => {
                let Some(job) = self.jobs.get_mut(&key) else {
                    trace!("Dropping encoder event for pruned export job {:?}", key);
                    return Task::none();
                };

                match status {
                    EncoderStatus::Progress(progress) => job.encoded_frames = progress as i32,
                    EncoderStatus::Finished => {
                        let result = if matches!(job.phase, JobPhase::Aborting) {
                            JobResult::Terminated
                        } else {
                            JobResult::Completed
                        };
                        job.phase = JobPhase::Done { result, shown_at: None };
                    }
                }
            }
            Message::SetCamera => {}
            Message::UseBounds => {
                let Some(key) = self.synced_key.clone() else {
                    return Task::none();
                };

                if matches!(
                    self.jobs.get(&key).map(|job| &job.phase),
                    Some(JobPhase::Running | JobPhase::Aborting)
                ) {
                    return Task::none();
                }

                if self.bounds_job.as_ref().is_some_and(|(job_key, job)| {
                    *job_key == key && matches!(job.phase, SearchPhase::Running | SearchPhase::Aborting)
                }) {
                    return Task::none();
                }

                let Some(unit) = data.held_unit.clone() else {
                    return Task::none();
                };

                let tolerance = self.exporter.cull_percent as f32 / 100.0;
                let is_showcase = self.exporter.export_mode == ExportMode::Showcase;
                let scan_limit = self.exporter.frame_end.max(self.exporter.frame_start).max(0);
                let current_anim = data.current_anim.clone();
                let offset = data.offset();
                let role_paths = data.role_paths();
                let showcase_slots = [
                    (Role::Walk, self.exporter.showcase_walk_len),
                    (Role::Idle, self.exporter.showcase_idle_len),
                    (Role::Attack, self.exporter.showcase_attack_len),
                    (Role::Knockback, self.exporter.showcase_kb_len),
                ];

                let abort = Arc::new(AtomicBool::new(false));
                let progress = Arc::new(ProgressCounter::default());
                let (tx, rx) = unbounded();

                let worker_abort = abort.clone();
                let worker_progress = progress.clone();
                thread::spawn(move || {
                    let mut showcase_clips = Vec::new();

                    if is_showcase {
                        for (role, length) in showcase_slots {
                            if length <= 0 {
                                continue;
                            }

                            if let Some((_, path)) = role_paths.iter().find(|(known, _)| *known == role)
                                && let Ok(bytes) = fs::read(path)
                                && let Ok(anim) = Animation::parse(&bytes) {
                                showcase_clips.push((anim, length));
                            }
                        }
                    }

                    let clips: Vec<(&Animation, Option<i32>)> = if is_showcase {
                        showcase_clips.iter().map(|(anim, length)| (anim, Some(*length))).collect()
                    } else {
                        current_anim.iter().map(|anim| (anim.as_ref(), Some(scan_limit))).collect()
                    };

                    let outcome = find_bounds::search(&unit, &clips, tolerance, offset, &worker_progress, &worker_abort);
                    let _ = tx.unbounded_send(outcome);
                });

                let map_key = key.clone();
                let (bounds_task, handle) = Task::stream(rx)
                    .map(move |outcome| Message::BoundsCalculated(map_key.clone(), outcome))
                    .abortable();

                if let Some((_, previous)) = self.bounds_job.take() {
                    previous.abort.store(true, Ordering::Relaxed);
                    previous.task_handle.abort();
                }

                self.bounds_job = Some((key, SearchJob {
                    phase: SearchPhase::Running,
                    abort,
                    progress,
                    frames_searched: 0,
                    start_time: Instant::now(),
                    task_handle: handle,
                }));

                return bounds_task;
            }
            Message::AbortBounds => {
                if let Some(key) = &self.synced_key
                    && let Some((job_key, job)) = &mut self.bounds_job
                    && job_key == key
                    && matches!(job.phase, SearchPhase::Running) {
                    job.abort.store(true, Ordering::Relaxed);
                    job.phase = SearchPhase::Aborting;
                }
            }
            Message::BoundsCalculated(key, outcome) => {
                let Some((job_key, job)) = &mut self.bounds_job else {
                    trace!("Dropping bounds event for missing job {:?}", key);
                    return Task::none();
                };

                if *job_key != key {
                    trace!("Dropping stale bounds event for {:?}", key);
                    return Task::none();
                }

                let terminated = matches!(outcome, BoundsOutcome::Aborted)
                    || matches!(job.phase, SearchPhase::Aborting | SearchPhase::Done { result: SearchResult::Terminated, .. });
                let applies = !terminated && self.synced_key.as_ref() == Some(&key);

                let result = if terminated {
                    SearchResult::Terminated
                } else if let BoundsOutcome::Found(bounds) = outcome {
                    if applies {
                        self.exporter.region_x = bounds.min_x;
                        self.exporter.region_y = bounds.min_y;
                        self.exporter.region_w = bounds.width();
                        self.exporter.region_h = bounds.height();
                        self.exporter.region_x_str = bounds.min_x.to_string();
                        self.exporter.region_y_str = bounds.min_y.to_string();
                        self.exporter.region_w_str = bounds.width().to_string();
                        self.exporter.region_h_str = bounds.height().to_string();
                        self.exporter.zoom = 1.0;
                    }
                    SearchResult::Found
                } else {
                    if applies {
                        self.exporter.region_x = 0.0;
                        self.exporter.region_y = 0.0;
                        self.exporter.region_w = 0.0;
                        self.exporter.region_h = 0.0;
                        self.exporter.region_x_str = String::new();
                        self.exporter.region_y_str = String::new();
                        self.exporter.region_w_str = String::new();
                        self.exporter.region_h_str = String::new();
                        self.exporter.zoom = 1.0;
                    }
                    SearchResult::Error("Nothing to measure".to_string())
                };

                job.phase = SearchPhase::Done { result, shown_at: None };
            }
        }

        Task::none()
    }

    pub fn view(&self, window: Size) -> Element<'_, Message> {
        self.popup.view("Export Animation", self.spec, window, Message::Popup, move || self.content_view(), Some(popup::GLASS))
    }

    fn content_view(&self) -> Element<'_, Message> {
        let is_avif_missing = paths::avifenc_status() != Presence::Installed;
        let is_ffmpeg_missing = paths::ffmpeg_status() != Presence::Installed;
        let current_job = self.synced_key.as_ref().and_then(|key| self.jobs.get(key));
        let job_active = matches!(
            current_job.map(|job| &job.phase),
            Some(JobPhase::Running | JobPhase::Aborting)
        );
        let is_locked = job_active;

        let current_loop = self.loop_job.as_ref()
            .filter(|(key, _)| Some(key) == self.synced_key.as_ref())
            .map(|(_, job)| job);
        let loop_active = matches!(
            current_loop.map(|job| &job.phase),
            Some(SearchPhase::Running | SearchPhase::Aborting)
        );

        let current_bounds = self.bounds_job.as_ref()
            .filter(|(key, _)| Some(key) == self.synced_key.as_ref())
            .map(|(_, job)| job);
        let bounds_active = matches!(
            current_bounds.map(|job| &job.phase),
            Some(SearchPhase::Running | SearchPhase::Aborting)
        );

        let selected_mode = match self.exporter.export_mode {
            ExportMode::Manual => "Manual",
            ExportMode::Loop => "Loop",
            ExportMode::Showcase => "Showcase",
        };

        let mode_picker = field_row(
            "Mode",
            pick_list(&MODE_OPTIONS[..], Some(selected_mode), |selected: &str| {
                Message::SetMode(match selected {
                    "Loop" => ExportMode::Loop,
                    "Showcase" => ExportMode::Showcase,
                    _ => ExportMode::Manual,
                })
            })
                .width(Length::Fixed(COMBO_WIDTH))
                .style(theme::combo_box)
                .menu_style(theme::combo_box_menu),
        );

        let selected_format = match self.exporter.format {
            ExportFormat::Gif => "GIF",
            ExportFormat::WebP => "WebP",
            ExportFormat::Avif => "AVIF",
            ExportFormat::Png => "PNG",
            ExportFormat::Mp4 => "MP4",
            ExportFormat::Mkv => "MKV",
            ExportFormat::Webm => "WebM",
            ExportFormat::Zip => "ZIP",
        };

        let format_options: Vec<&str> = FORMAT_OPTIONS.iter().copied().filter(|format| {
            match *format {
                "AVIF" => !is_avif_missing,
                "MP4" | "MKV" | "WebM" | "PNG" => !is_ffmpeg_missing,
                _ => true,
            }
        }).collect();

        let format_picker = field_row(
            "Format",
            pick_list(format_options, Some(selected_format), |selected: &str| {
                Message::SetFormat(match selected {
                    "WebP" => ExportFormat::WebP,
                    "AVIF" => ExportFormat::Avif,
                    "PNG" => ExportFormat::Png,
                    "MP4" => ExportFormat::Mp4,
                    "MKV" => ExportFormat::Mkv,
                    "WebM" => ExportFormat::Webm,
                    "ZIP" => ExportFormat::Zip,
                    _ => ExportFormat::Gif,
                })
            })
                .width(Length::Fixed(COMBO_WIDTH))
                .style(theme::combo_box)
                .menu_style(theme::combo_box_menu),
        );

        let end_hint = self.exporter.max_frame.to_string();

        let input_section: Element<'_, Message> = match self.exporter.export_mode {
            ExportMode::Manual => field_row(
                "Frames",
                frame_range_row(
                    &self.exporter.frame_start_str,
                    &self.exporter.frame_end_str,
                    "0",
                    &end_hint,
                    Some(Message::SetStartFrame),
                    Some(Message::SetEndFrame),
                ),
            ),
            ExportMode::Loop => {
                let find_loop_button: Element<'_, Message> = match current_loop.map(|job| &job.phase) {
                    Some(SearchPhase::Running | SearchPhase::Aborting) => {
                        action_button("Abort Loop", theme::danger_button, Some(Message::AbortLoopSearch))
                    }
                    phase => {
                        let is_terminated = matches!(phase, Some(SearchPhase::Done { result: SearchResult::Terminated, .. }));
                        let label = if is_terminated { "Loop Terminated!" } else { "Find Loop" };
                        let style = if is_terminated { theme::danger_button } else { theme::neutral_button };
                        action_button(label, style, (!job_active).then_some(Message::FindLoop))
                    }
                };

                column![
                    field_row(
                        "Frames",
                        frame_range_row(
                            &self.exporter.frame_start_str,
                            &self.exporter.frame_end_str,
                            "0",
                            &end_hint,
                            Some(Message::SetStartFrame),
                            Some(Message::SetEndFrame),
                        ),
                    ),
                    field_row("Tolerance", small_input("30", &self.exporter.loop_tolerance_str, Message::SetLoopTolerance)),
                    field_row("Min Frames", small_input("15", &self.exporter.loop_min_str, Message::SetLoopMin)),
                    field_row("Max Frames", small_input("None", &self.exporter.loop_max_str, Message::SetLoopMax)),
                    find_loop_button,
                ].spacing(ROW_SPACING).into()
            }
            ExportMode::Showcase => {
                let walk_hint = self.exporter.detected_walk_len.to_string();
                let idle_hint = self.exporter.detected_idle_len.to_string();
                let attack_hint = self.exporter.detected_attack_len.to_string();
                let kb_hint = self.exporter.last_known_kb_default.to_string();

                column![
                    field_row("Walk", small_input(&walk_hint, &self.exporter.showcase_walk_str, Message::SetShowcaseWalk)),
                    field_row("Idle", small_input(&idle_hint, &self.exporter.showcase_idle_str, Message::SetShowcaseIdle)),
                    field_row("Attack", small_input(&attack_hint, &self.exporter.showcase_attack_str, Message::SetShowcaseAttack)),
                    field_row("Knockback", small_input(&kb_hint, &self.exporter.showcase_kb_str, Message::SetShowcaseKb)),
                ].spacing(ROW_SPACING).into()
            }
        };

        let bounds_button: Element<'_, Message> = match current_bounds.map(|job| &job.phase) {
            Some(SearchPhase::Running | SearchPhase::Aborting) => {
                action_button("Abort Bounds", theme::danger_button, Some(Message::AbortBounds))
            }
            phase => {
                let is_terminated = matches!(phase, Some(SearchPhase::Done { result: SearchResult::Terminated, .. }));
                let label = if is_terminated { "Terminated!" } else { "Use Bounds" };
                let style = if is_terminated { theme::danger_button } else { theme::neutral_button };
                action_button(label, style, (!is_locked).then_some(Message::UseBounds))
            }
        };

        let camera_buttons = row![
            action_button("Set Camera", theme::neutral_button, (!is_locked && !bounds_active).then_some(Message::SetCamera)),
            bounds_button,
        ].spacing(FIELD_SPACING);

        let camera_section = section(
            "Camera",
            Length::Fill,
            column![
                cull_row(&self.exporter.cull_percent_str),
                camera_buttons,
                row![
                    axis_input("X", &self.exporter.region_x_str, Message::SetRegionX),
                    axis_input("Y", &self.exporter.region_y_str, Message::SetRegionY),
                ].spacing(FIELD_SPACING),
                row![
                    axis_input("W", &self.exporter.region_w_str, Message::SetRegionW),
                    axis_input("H", &self.exporter.region_h_str, Message::SetRegionH),
                ].spacing(FIELD_SPACING),
            ].spacing(ROW_SPACING),
        );

        let (display_start, display_end) = if self.exporter.export_mode == ExportMode::Showcase {
            let total = self.exporter.showcase_walk_len
                + self.exporter.showcase_idle_len
                + self.exporter.showcase_attack_len
                + self.exporter.showcase_kb_len;
            (0, if total > 0 { total - 1 } else { 0 })
        } else {
            (self.exporter.frame_start, self.exporter.frame_end)
        };

        let range_part = if display_start == display_end {
            format!("{}f", display_start)
        } else {
            format!("{}f~{}f", display_start, display_end)
        };

        let prefix_display = if self.exporter.export_mode == ExportMode::Showcase {
            self.exporter.name_prefix.split('.').next()
                .map_or_else(|| "unit.showcase".to_string(), |first| format!("{}.showcase", first))
        } else {
            self.exporter.name_prefix.clone()
        };

        let name_hint = if prefix_display.is_empty() {
            "animation".to_string()
        } else {
            format!("{}.{}", prefix_display, range_part)
        };

        let output_section = section(
            "Output",
            Length::Fill,
            column![
                field_row(
                    "Name",
                    text_input(&name_hint, &self.exporter.file_name)
                        .on_input(Message::SetFileName)
                        .width(Length::Fixed(NAME_INPUT_WIDTH))
                        .style(theme::rounded_input),
                ),
                format_picker,
                field_row("Quality %", small_input("80", &self.exporter.quality_percent_str, Message::SetQuality)),
                field_row("Compress %", small_input("30", &self.exporter.compression_percent_str, Message::SetCompression)),
                row![
                    background_toggle(&self.exporter),
                    text("Background").size(CONTROL_TEXT_SIZE),
                ].spacing(FIELD_SPACING).align_y(Alignment::Center),
                row![
                    toggler(self.exporter.debug)
                        .on_toggle(Message::ToggleDebug)
                        .style(theme::ios_toggle),
                    text("Include Debug").size(CONTROL_TEXT_SIZE),
                ].spacing(FIELD_SPACING).align_y(Alignment::Center),
            ].spacing(ROW_SPACING),
        );

        let addons_section = section(
            "Add-Ons",
            Length::Fill,
            column![
                text("Tools that enhance the Exporter\nManage through Settings > Add-Ons").size(CONTROL_TEXT_SIZE),
                addon_badge("FFMPEG", !is_ffmpeg_missing),
                addon_badge("AVIFENC", !is_avif_missing),
            ].spacing(FIELD_SPACING),
        );

        let (ratio, status_label) = if let Some(job) = current_bounds.filter(|job| matches!(job.phase, SearchPhase::Running | SearchPhase::Aborting)) {
            (job.progress.fraction(), format!("Searching | {} frames", job.frames_searched))
        } else if let Some(job) = current_loop.filter(|job| matches!(job.phase, SearchPhase::Running | SearchPhase::Aborting)) {
            let pulse = job.start_time.elapsed().as_secs_f32() % 1.0;
            (pulse, format!("Searching | {} frames", job.frames_searched))
        } else if let Some(job) = current_job {
            let progress_ratio = |value: i32| (value as f32 / job.total_frames.max(1) as f32).min(1.0);

            match &job.phase {
                JobPhase::Running => {
                    if job.rendered_frames < job.total_frames {
                        let ratio = progress_ratio(job.rendered_frames);
                        (ratio, format!("Rendering | {}f/{}f ({}%)", job.rendered_frames, job.total_frames, (ratio * 100.0) as i32))
                    } else {
                        let ratio = progress_ratio(job.encoded_frames);
                        (ratio, format!("Encoding | {}f/{}f ({}%)", job.encoded_frames, job.total_frames, (ratio * 100.0) as i32))
                    }
                }
                JobPhase::Aborting => (1.0, "Aborting...".to_string()),
                JobPhase::Done { result: JobResult::Completed, .. } => (1.0, "Done".to_string()),
                JobPhase::Done { result: JobResult::Terminated, .. } => (1.0, "Ready".to_string()),
            }
        } else if let Some(SearchPhase::Done { result, .. }) = current_loop.map(|job| &job.phase) {
            let label = match result {
                SearchResult::Found => "Done".to_string(),
                SearchResult::Terminated => "Loop Terminated!".to_string(),
                SearchResult::Error(message) => message.clone(),
            };
            (1.0, label)
        } else if let Some(SearchPhase::Done { result, .. }) = current_bounds.map(|job| &job.phase) {
            let label = match result {
                SearchResult::Found => "Done".to_string(),
                SearchResult::Terminated => "Bounds Terminated!".to_string(),
                SearchResult::Error(message) => message.clone(),
            };
            (1.0, label)
        } else {
            (1.0, "Ready".to_string())
        };

        let export_button: Element<'_, Message> = match current_job.map(|job| &job.phase) {
            Some(JobPhase::Running) => action_button("Abort Export", theme::danger_button, Some(Message::AbortExport)),
            Some(JobPhase::Aborting) => action_button("Aborting...", theme::danger_button, None),
            phase => {
                let is_valid = self.exporter.region_w > 0.1 && self.exporter.region_h > 0.1;
                let is_terminated = matches!(phase, Some(JobPhase::Done { result: JobResult::Terminated, .. }));

                let label = if is_terminated {
                    "Export Terminated!"
                } else if is_valid {
                    "Begin Export"
                } else {
                    "No Camera Set"
                };

                let style = match (is_terminated, is_valid) {
                    (true, _) => theme::danger_button,
                    (false, true) => theme::primary_button,
                    (false, false) => theme::neutral_button,
                };
                let press = (is_valid && !is_locked && !loop_active && !bounds_active).then_some(Message::BeginExport);

                action_button(label, style, press)
            }
        };

        let scroll_content = column![
            section("Input", Length::Fill, column![mode_picker, input_section].spacing(ROW_SPACING)),
            camera_section,
            output_section,
            addons_section,
        ].spacing(SECTION_SPACING);

        let bottom_bar = column![
            export_button,
            column![
                theme::centered_text(status_label).size(CONTROL_TEXT_SIZE).width(Length::Fill),
                progress_bar(0.0..=1.0, ratio),
            ].spacing(ROW_SPACING),
        ].spacing(BUTTON_STATUS_GAP);

        container(
            column![
                column![
                    smooth_scroll(
                        scrollable(scroll_content)
                            .id(self.scroll.clone())
                            .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                            .height(Length::Fill)
                            .spacing(SCROLLBAR_GAP),
                    ),
                    container(rule::horizontal(RULE_HEIGHT)).width(Length::Fill),
                ].spacing(0).height(Length::Fill),
                bottom_bar,
            ].spacing(SECTION_SPACING)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(CONTENT_PADDING)
            .into()
    }
}

fn advance_search(slot: &mut Option<(JobKey, SearchJob)>, synced_key: Option<&JobKey>) {
    if let Some((job_key, job)) = slot {
        if matches!(job.phase, SearchPhase::Aborting) {
            job.phase = SearchPhase::Done { result: SearchResult::Terminated, shown_at: None };
        }

        if Some(&*job_key) == synced_key
            && let SearchPhase::Done { shown_at, .. } = &mut job.phase
            && shown_at.is_none() {
            *shown_at = Some(Instant::now());
        }
    }

    let expired = slot.as_ref().is_some_and(|(_, job)| {
        matches!(&job.phase, SearchPhase::Done { shown_at: Some(at), .. } if at.elapsed().as_secs_f32() > 3.0)
    });

    if expired
        && let Some((_, job)) = slot.take() {
        job.task_handle.abort();
    }
}

fn cull_row<'a>(value: &'a str) -> Element<'a, Message> {
    row![
        text("Auto-Bounds Cull Faint %").size(CONTROL_TEXT_SIZE),
        small_input(&DEFAULT_CULL.to_string(), value, Message::SetCull),
    ]
        .spacing(FIELD_SPACING)
        .align_y(Alignment::Center)
        .into()
}

fn field_row<'a>(label: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        text(label).size(CONTROL_TEXT_SIZE).width(Length::Fixed(FIELD_LABEL_WIDTH)),
        control.into(),
    ]
        .spacing(FIELD_SPACING)
        .align_y(Alignment::Center)
        .into()
}

fn percentage(input: &str, empty: i32) -> Option<(i32, String)> {
    if !is_typable_number(input, false, false) {
        return None;
    }

    if input.trim().is_empty() {
        return Some((empty, String::new()));
    }

    let parsed = input.parse::<i32>().unwrap_or(PERCENT_MAX);
    let clamped = parsed.clamp(PERCENT_MIN, PERCENT_MAX);
    let text = if clamped == parsed { input.to_string() } else { clamped.to_string() };

    Some((clamped, text))
}

fn is_typable_number(value: &str, allow_negative: bool, allow_decimal: bool) -> bool {
    let body = if allow_negative { value.strip_prefix('-').unwrap_or(value) } else { value };

    if !allow_decimal {
        return body.chars().all(|c| c.is_ascii_digit());
    }

    let mut dot_seen = false;
    for c in body.chars() {
        if c == '.' {
            if dot_seen {
                return false;
            }
            dot_seen = true;
        } else if !c.is_ascii_digit() {
            return false;
        }
    }
    true
}

fn small_input<'a>(hint: &str, value: &'a str, on_input: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    text_input(hint, value)
        .on_input(on_input)
        .width(Length::Fixed(SMALL_INPUT_WIDTH))
        .style(theme::rounded_input)
        .into()
}

fn axis_input<'a>(label: &'a str, value: &'a str, on_input: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    row![
        text(label).size(CONTROL_TEXT_SIZE).width(Length::Fixed(AXIS_LABEL_WIDTH)),
        text_input("0", value)
            .on_input(on_input)
            .width(Length::Fixed(AXIS_INPUT_WIDTH))
            .style(theme::rounded_input),
    ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
}

fn frame_range_row<'a, FMin, FMax>(
    min: &'a str,
    max: &'a str,
    min_hint: &str,
    max_hint: &str,
    on_min: Option<FMin>,
    on_max: Option<FMax>,
) -> Element<'a, Message>
where
    FMin: Fn(String) -> Message + 'a,
    FMax: Fn(String) -> Message + 'a,
{
    row![
        text_input(min_hint, min).on_input_maybe(on_min).width(Length::Fixed(SMALL_INPUT_WIDTH)).style(theme::rounded_input),
        text("~").size(CONTROL_TEXT_SIZE),
        text_input(max_hint, max).on_input_maybe(on_max).width(Length::Fixed(SMALL_INPUT_WIDTH)).style(theme::rounded_input),
    ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
}

fn action_button<'a>(
    label: &'a str,
    style: fn(&Theme, button::Status) -> button::Style,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    button(theme::button_label(label).size(CONTROL_TEXT_SIZE))
        .width(Length::Fill)
        .padding([6, 10])
        .style(style)
        .on_press_maybe(on_press)
        .into()
}

fn is_forced_opaque(format: &ExportFormat) -> bool {
    matches!(format, ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm)
}

fn background_toggle(exporter: &ExportForm) -> Element<'_, Message> {
    if is_forced_opaque(&exporter.format) {
        tooltip(
            toggler(true).style(theme::ios_toggle),
            container(text("This video format requires a background")).padding(6).style(container::bordered_box),
            tooltip::Position::Top,
        )
        .into()
    } else {
        toggler(exporter.background).on_toggle(Message::ToggleBackground).style(theme::ios_toggle).into()
    }
}

fn addon_badge(label: &str, installed: bool) -> Element<'_, Message> {
    let status = if installed { format!("{label} Installed") } else { format!("{label} Missing") };

    container(theme::centered_text(status).size(CONTROL_TEXT_SIZE))
        .width(Length::Fill)
        .padding(6)
        .style(move |theme: &Theme| theme::status_badge(theme, installed))
        .into()
}

fn derive_name_prefix(raw_id: &str, type_string: &str) -> String {
    let id_parts: Vec<&str> = raw_id.split('_').collect();
    let mut clean_id = id_parts.first().copied().unwrap_or("").to_string();

    if id_parts.len() >= 2 && !clean_id.is_empty() && clean_id.chars().all(char::is_numeric) {
        let form_number = match id_parts[1].chars().next() {
            Some('f') => 1,
            Some('c') => 2,
            Some('s') => 3,
            Some('u') => 4,
            _ => 0,
        };
        if form_number > 0 {
            clean_id = format!("{}-{}", clean_id, form_number);
        }
    }

    format!("{}.{}", clean_id, type_string)
}

