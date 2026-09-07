use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::{space, Column};
use iced::widget::{mouse_area, operation, responsive, tooltip};
use iced::border::Border;
use iced::{widget, Color, Element, Font, Length, Padding, Point, Size, Task, Theme};
use tracing::{error, info, warn};

use kore::common::architecture;
use kore::common::preview::{self, Stamp};
use kore::domains::settings::{Faults, FrameCount, Scope, Settings, Shown, StudioSettings, Switch, Tier};
use kore::domains::studio as sets;
use kore::systems::animation::posing::{self, Gizmo, Probe};
use kore::systems::animation::authoring::{self as authoring, bound, Beat, Cadence, Imgcut, CUT_FIELDS, CUT_NAME_FIELD, ease_label, ease_takes_power, ease_value, key_label, kind_label, loop_label, nameable, Maanim, Mamodel, EASES, FIELDS, NAME_FIELD};
use image::RgbaImage;
use nyanko::graphics::rig::{Keyframe, Model, ModelPart, Opaque, Rig, SpriteCut};
use nyanko::graphics::tools::crash::{self, Side};
use nyanko::graphics::tools::timeline as curve;

use crate::app::state::{AnimState, StudioState};
use crate::app::theme;
use crate::editor::{self, Target};
use crate::systems::animation::{self as viewer, controls, overlay};

use crate::common::feedback::{self, Slot, LOCKED_NOTICE};
use crate::common::{dialog, fonts, glyphs};
use crate::common::row_window::{self, RowWindow};
use crate::widget::{branches, list_row, open_mark, picture, popup, slide, smooth_scroll, Guide, Slide, Tracer, SLIDE_DURATION};

mod blame;
mod documents;
mod gizmo;
mod history;
mod manage;
mod onion;
mod panel;
mod handle;
mod set;
mod shipout;
mod timeline;
mod tree;

use blame::{Alarm, Blame, Notice};
use documents::*;
use history::{History, Tag};
use panel::*;
use tree::*;

pub(crate) use sets::Set;
pub(crate) use shipout::Muster;

const PANEL_WIDTH: f32 = 372.0;
const PANEL_PADDING: f32 = 6.0;
const BODY_PADDING: f32 = 6.0;
const GAP: f32 = 6.0;
const ROW_GAP: f32 = 3.0;

const CLOSE_LABEL: &str = "\u{00d7}";
const CONFIRM_MARK: &str = "?";

const LABEL_SIZE: f32 = 12.0;
const CELL_SIZE: f32 = 12.0;
const CELL_PADDING: f32 = 3.0;
const INDEX_WIDTH: f32 = 22.0;
const ROW_HEIGHT: f32 = 21.0;
const MIN_TREE_ROWS: f32 = 8.0;
const ROW_SPACING: f32 = 0.0;
const ROW_PADDING: f32 = 6.0;
const INDENT: f32 = 11.0;

const TREE_TEXT_SIZE: f32 = 11.0;
const CHAR_WIDTH: f32 = TREE_TEXT_SIZE * 0.6;

const MARKER_SIZE: f32 = 18.0;
const MARKER_LINE_HEIGHT: f32 = ROW_HEIGHT / MARKER_SIZE;
const MARKER_WIDTH: f32 = 14.0;

const SCROLLBAR_WIDTH: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 2.0;
const SCROLLBAR_ALLOWANCE: f32 = SCROLLBAR_WIDTH + SCROLLBAR_MARGIN * 2.0;
const SCROLL_TAIL: f32 = 14.0;
const KEY_ROW_HEIGHT: f32 = 44.0;
const KEY_ROW_PAD: f32 = 3.0;
const KEY_ROW_INSET: f32 = 2.0;

const PLAYING_TICK: Duration = Duration::from_millis(16);
const RESTING_TICK: Duration = Duration::from_millis(200);
const RECALL_CAP: usize = 5;
const HISTORY_SETS: usize = 3;
const NOTICE_EXPIRY: Duration = Duration::from_secs(6);
const NOTICE_PAD_X: f32 = 7.0;
const NOTICE_PAD_Y: f32 = 7.0;
const NOTICE_OVERHANG: f32 = 4.0;
const NOTICE_TEXT_SIZE: f32 = 13.0;
const RENAME_DELAY: Duration = Duration::from_millis(700);
const KEY_HEAD_HEIGHT: f32 = 19.0;
const PAGE_DIALS: usize = 8;
const PAGE_BACK: &str = "\u{25c2}";
const PAGE_NEXT: &str = "\u{25b8}";
const PAGER_STEP: f32 = 26.0;
const DEBUG_WIDTH: f32 = 78.0;
const OPTION_WIDTH: f32 = 176.0;
const COMBO_WIDTH: f32 = 88.0;

const FOLDER_OPEN: &str = "\u{25be}";
const FOLDER_SHUT: &str = "\u{25b8}";
const EASE_WIDTH: f32 = 86.0;
const DROP_WIDTH: f32 = 18.0;
const STEP_WIDTH: f32 = 44.0;
const ACTIVE_TINT: f32 = 0.68;
const SEGMENT_TOP: f32 = 0.3;
const SEGMENT_BOTTOM: f32 = 0.88;
const FACT_ROW_HEIGHT: f32 = 21.0;
const FACT_ROW_PAD: f32 = 4.0;
const FACT_LABEL: f32 = 62.0;
const LOOP_WIDTH: f32 = 56.0;
const LOOP_CARD_PAD: f32 = 3.0;
const LOOP_DEFAULT: i32 = 1;
const LOOP_HINT: &str = "1";
const CELL_HINT: &str = "0";
const HEAD_HEIGHT: f32 = 22.0;
const FIELD_ROW_HEIGHT: f32 = 24.0;
const TREE_TOP: f32 = BODY_PADDING + PANEL_PADDING + HEAD_HEIGHT + ROW_GAP + 1.0 + ROW_GAP + theme::CONSOLE_BORDER_WIDTH;
const TREE_LEFT: f32 = BODY_PADDING + PANEL_PADDING + theme::CONSOLE_BORDER_WIDTH;
const TREE_RIGHT: f32 = BODY_PADDING + PANEL_WIDTH - PANEL_PADDING;
const NEST_BAND: f32 = 0.28;
const NEST_BORDER: f32 = 2.0;
const SEAM_HEIGHT: f32 = 2.0;
const CARRIED_TINT: f32 = 0.35;
const ALARM_TINT: f32 = 0.3;
const DRAG_HASTE: f32 = 2.0;
const GHOST_FILL: f32 = 0.22;
const GHOST_EDGE: f32 = 0.55;
const GHOST_INK: f32 = 0.7;
const GHOST_PAD: f32 = 3.0;
const CUT_CELL_WIDTH: f32 = 30.0;
const CUT_STEP_WIDTH: f32 = 46.0;
const FRAME_HINT: &str = "Right click & drag to set the cut";
const ALIGN_WIDTH: f32 = 36.0;
const BUFFER_MARK: char = '!';
const HEX_DIGITS: usize = 6;
const NOT_DRAWN: i32 = -1;

const LOADING_NOTICE: &str = "Loading animation\u{2026}";
const NO_SET_NOTICE: &str = "No set is loaded";
const NO_SET_HINT: &str = "Open Manage to import, pick or create a set";
const NO_CLIP_NOTICE: &str = "This clip has no channels to edit";
const UNREADABLE_NOTICE: &str = "This animation could not be read";
const EMPTY_TRACK_NOTICE: &str = "This channel holds no keyframes";
const NO_ENTRY_CHOSEN: &str = "Select a part or channel to edit";
const WRITE_FAILED_NOTICE: &str = "The last change could not be saved";
const NO_SPRITE_NOTICE: &str = "none";
const PAST_ATLAS_NOTICE: &str = "past the atlas";
const BLANK_CUT_NOTICE: &str = "nothing visible";
const WHOLE_CUT_NOTICE: &str = "the whole cut";
const UNNAMED_CUT_NOTICE: &str = "unnamed";
const NO_PART_NOTICE: &str = "This channel drives a part the model does not declare";
const LOST_PART_NOTICE: &str = "This part is no longer in the loaded model";
const SPRITE_KIND: i32 = 2;
const ALPHA_FLOOR: u8 = 8;
const ADRIFT_TINT: f32 = 0.35;
const OUT_OF_BOUNDS: &str = "Coordinate is out of bounds";
const NO_ATLAS_NOTICE: &str = "This atlas could not be read";
const NO_CUTS_NOTICE: &str = "This atlas declares no regions";
const NO_PART_CHOSEN: &str = "Select a part to edit its rest pose";
const NO_MODEL_NOTICE: &str = "This model could not be read";
const LOOSE_LABEL: &str = "Channels with no declared part";
const SHADOWED_MARK: &str = "overridden";
const SEPARATOR: &str = " \u{00b7} ";
const LABEL_ROOM: usize = 48;
const PART_FAULT: &str = "This part may cause a game crash";
const CHANNEL_FAULT: &str = "This channel may cause a game crash";
const TAINTED_FAULT: &str = "Child of this part may cause a game crash";
const TAINTED_HINT: &str = "Please find the child and resolve its error";
const SCALE_UNIT_DETAIL: &str =
    "The model's scale divisor is zero\nThe game fails to divide by zero";
const OPACITY_UNIT_DETAIL: &str =
    "The model's opacity divisor is zero\nThe game fails to divide by zero";
const ATTACK_LENGTH_DETAIL: &str =
    "The attack animation measures no frames\nThe game divides its frame counter by that length";
const ENDLESS_ATTACK_DETAIL: &str =
    "The attack animation never ends and reaches no frame past zero\nAn attacking enemy divides its frame counter by that";
const UNKNOWN_DETAIL: &str = "The game's animation pass faults on this";
const FAULT_HINT: &str = "Manage faults under Option page 2";
const ATTACK_HINT: &str = "Only consider this fault for attacks";
const ROOT_MOVE_NOTICE: &str = "The root ignores its model X and Y\nSwitch the Gizmo to Channel to move it";
const ROOT_FIELD_HINT: &str = "The root ignores this column; key an X or Y channel instead";
const PINNED_NOTICE: &str = "The offset row is pinned to this part\nMoving it cancels out and the game draws it unmoved";
const PLACE_X_FIELD: usize = 4;
const PLACE_Y_FIELD: usize = 5;
const ENTITY_FAULT: &str = "This entity may cause a game crash\nPlease find the issue and resolve it";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    Atlas,
    #[default]
    Entity,
}

impl Mode {
    fn other(self) -> Mode {
        match self {
            Mode::Atlas => Mode::Entity,
            Mode::Entity => Mode::Atlas,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Mode::Atlas => "Atlas",
            Mode::Entity => "Entity",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Swap {
    Same,
    Fresh,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Span {
    Head,
    Tail,
    Only,
}

impl Span {
    fn tint(self) -> f32 {
        match self {
            Span::Head => SEGMENT_TOP,
            Span::Tail | Span::Only => SEGMENT_BOTTOM,
        }
    }
}

fn stepped<T: Copy + PartialEq>(all: &[T], held: T) -> T {
    let at = all.iter().position(|known| *known == held).map_or(0, |at| at + 1);

    all.get(at % all.len().max(1)).copied().unwrap_or(held)
}

fn spanned(at: usize, active: Option<usize>, rows: usize) -> Option<Span> {
    let anchor = active?;
    let reaching = anchor + 1;

    if reaching >= rows {
        return (at == anchor).then_some(Span::Only);
    }

    match at {
        _ if at == anchor => Some(Span::Head),
        _ if at == reaching => Some(Span::Tail),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Focus {
    #[default]
    Curve,
    Part,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Readout {
    Facts,
    #[default]
    Timeline,
}

impl Readout {
    const ALL: [Readout; 2] = [Readout::Timeline, Readout::Facts];

    fn label(self) -> &'static str {
        match self {
            Readout::Facts => "Table",
            Readout::Timeline => "Timeline",
        }
    }
}

impl std::fmt::Display for Readout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dial {
    Gizmo,
    Onion,
    Module,
    Fault,
    Rig,
    Hierarchy,
    Selected,
    Origin,
    Entity,
    World,
}

impl Dial {
    const ALL: [Dial; 10] = [
        Dial::Gizmo,
        Dial::Onion,
        Dial::Entity,
        Dial::Rig,
        Dial::Hierarchy,
        Dial::Selected,
        Dial::World,
        Dial::Origin,
        Dial::Module,
        Dial::Fault,
    ];

    fn pages() -> usize {
        Dial::ALL.len().div_ceil(PAGE_DIALS).max(1)
    }

    fn page(page: usize) -> &'static [Dial] {
        let from = (page * PAGE_DIALS).min(Dial::ALL.len());
        let to = (from + PAGE_DIALS).min(Dial::ALL.len());

        &Dial::ALL[from..to]
    }

    fn label(self) -> &'static str {
        match self {
            Dial::Gizmo => "Gizmo",
            Dial::Onion => "Onionskin",
            Dial::Module => "Module",
            Dial::Fault => "Fault",
            Dial::Rig => "Rig",
            Dial::Hierarchy => "Hierarchy",
            Dial::Selected => "Selected",
            Dial::Origin => "Origin",
            Dial::Entity => "Entity",
            Dial::World => "World",
        }
    }

    fn tier(self, anim: &StudioSettings) -> Option<Tier> {
        match self {
            Dial::Rig => Some(anim.rig),
            Dial::Hierarchy => Some(anim.hierarchy),
            Dial::Selected => Some(anim.selected),
            _ => None,
        }
    }

    fn set_tier(self, anim: &mut StudioSettings, tier: Tier) {
        match self {
            Dial::Rig => anim.rig = tier,
            Dial::Hierarchy => anim.hierarchy = tier,
            Dial::Selected => anim.selected = tier,
            _ => {}
        }
    }

    fn live(self, animated: bool) -> bool {
        self != Dial::Gizmo || animated
    }

    fn shown(self, anim: &StudioSettings) -> Option<Shown> {
        match self {
            Dial::Origin => Some(anim.origin),
            Dial::World => Some(anim.world),
            _ => None,
        }
    }

    fn set_shown(self, anim: &mut StudioSettings, shown: Shown) {
        match self {
            Dial::Origin => anim.origin = shown,
            Dial::World => anim.world = shown,
            _ => {}
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Frame,
    Value,
    Ease,
    Power,
}

impl Field {
    const ALL: [Field; 4] = [Field::Frame, Field::Value, Field::Ease, Field::Power];
    const TYPED: [Field; 3] = [Field::Frame, Field::Value, Field::Power];

    fn label(self) -> &'static str {
        match self {
            Field::Frame => "Frame",
            Field::Value => "Value",
            Field::Ease => "Ease",
            Field::Power => "Power",
        }
    }

    fn slot(self) -> usize {
        self as usize
    }

    fn default(self, neutral: i32) -> i32 {
        match self {
            Field::Value => neutral,
            _ => 0,
        }
    }
}

pub(crate) struct Offsets {
    pub(crate) rows: usize,
    pub(crate) addable: bool,
}

pub(crate) struct Channels {
    pub(crate) part: usize,
    pub(crate) label: String,
    pub(crate) present: Vec<(i32, usize)>,
    pub(crate) channelled: bool,
    pub(crate) locatable: bool,
}

#[derive(Clone)]
pub(crate) struct Plan {
    set: sets::Set,
    target_mod: Option<String>,
    clip: Option<String>,
}

pub(crate) fn slotted(anim: &Path) -> bool {
    matches!(sets::home(anim), sets::Home::Game | sets::Home::Mod) && sets::attack_slot(anim)
}

fn plan(set: sets::Set, target_mod: Option<String>, clip: Option<String>) -> Plan {
    Plan { set, target_mod, clip }
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Viewer(viewer::Message),
    Row(usize),
    Scrolled(f32, f32),
    StripScrolled(f32),
    Changed(usize, Field, String),
    LoopChanged(String),
    AddKey,
    DropKey(usize),
    Seek(usize),
    Bound(usize),
    EaseChanged(usize, i32),
    DropExpired,
    Tiered(Dial, Tier),
    Sighted(Dial, Shown),
    Scoped(Scope),
    Refused(&'static str),
    Module(Readout),
    Faulted(Faults),
    Page(usize),
    Cycle(Dial),
    OpenOnion,
    OnionPopup(popup::Message),
    Onioned(onion::Knob, String),
    Aimed(String),
    Ship,
    ShipExpired,
    ShipPopup(popup::Message),
    Onioning(Switch),
    Framed(usize, timeline::Window),
    Scrub(i32),
    Pick(usize),
    Switch(Mode),
    Press(usize),
    DragMove(Point),
    DragEnd,
    Picture(picture::Message),
    Carved(Option<Arc<Sheet>>),
    Cut(usize, usize, String),
    Frame(usize),
    Trim(usize),
    Slice(usize),
    Find(usize),
    DropCut(usize),
    DropCutExpired,
    Field(usize, String),
    OffsetChanged(usize, String),
    AddPart,
    AddCut,
    Undo,
    Persisted(u64, PathBuf, Option<Stamp>),
    Watched(u64, Arc<Vigil>),
    OpenManage,
    ManagePopup(popup::Message),
    Manage(manage::Message),
    Gizmo(gizmo::Turn),
    Handed(Gizmo),
    Export,
    Exported(bool, usize),
    ExportExpired,
}

#[derive(Default, Debug)]
pub struct Vigil {
    gone: bool,
    seen: Vec<(PathBuf, Option<Stamp>)>,
}

fn keep_watch(files: Vec<PathBuf>, open: Vec<PathBuf>, rigged: bool) -> Vigil {
    let gone = !rigged || !files.iter().all(|path| path.is_file());
    let seen = open.into_iter().map(|path| (path.clone(), preview::stamp(&path))).collect();

    Vigil { gone, seen }
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Drag {
    #[default]
    Idle,
    Pressed {
        row: usize,
        cargo: Cargo,
        since: Instant,
    },
    Moving {
        cargo: Cargo,
        at: Point,
        onto: Option<Landing>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cargo {
    Part(usize),
    Track(usize),
}

impl Drag {
    fn carrying(self) -> Option<Cargo> {
        match self {
            Drag::Moving { cargo, .. } => Some(cargo),
            _ => None,
        }
    }

    fn landing(self) -> Option<Landing> {
        match self {
            Drag::Moving { onto, .. } => onto,
            _ => None,
        }
    }

    fn ripe(self) -> bool {
        match self {
            Drag::Pressed { since, .. } => {
                since.elapsed() >= Duration::from_secs_f32(controls::HOLD_DELAY_SECS / DRAG_HASTE)
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Landing {
    Onto(usize),
    Seam(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Nest,
    Above,
    Below,
}

impl Landing {
    fn mark(self, row: usize, last: usize) -> Option<Mark> {
        match self {
            Landing::Onto(at) if at == row => Some(Mark::Nest),
            Landing::Seam(gap) if gap == row => Some(Mark::Above),
            Landing::Seam(gap) if gap == row + 1 && row == last => Some(Mark::Below),
            _ => None,
        }
    }
}

struct Recall {
    key: String,
    expanded: HashSet<usize>,
    clip: Option<String>,
    curve: Option<Held>,
    part: Option<usize>,
    focus: Focus,
}

pub struct State {
    unlocked: bool,
    session: Option<Session>,
    idle: viewer::State,
    recalled: Vec<Recall>,
    histories: Vec<(String, History)>,
    manage: manage::State,
    managing: bool,
    onioning: bool,
    shipping_to: bool,
    shed_onion: bool,
    mounted: Option<String>,
    aimed: String,
    aim: sets::Aim,
    ship_armed: Slot<()>,
    popup: popup::State,
    onion_popup: popup::State,
    ship_popup: popup::State,
    mode: Mode,
    readout: Readout,
    dials: usize,
    pending_fault: Option<Side>,
    flash: Option<Instant>,
    flash_text: String,
    notice_text: String,
    notice_hint: Option<&'static str>,
    raised: bool,
    lowered: Option<Instant>,
    exporting: bool,
    exported: Slot<bool>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            unlocked: false,
            session: None,
            idle: viewer::State::with_popup(popup::Kind::Animator).authoring(),
            recalled: Vec::new(),
            histories: Vec::new(),
            manage: manage::State::default(),
            managing: false,
            onioning: false,
            shipping_to: false,
            shed_onion: false,
            mounted: None,
            aimed: String::new(),
            aim: sets::Aim::Blank,
            ship_armed: Slot::default(),
            popup: popup::State::default(),
            onion_popup: popup::State::default(),
            ship_popup: popup::State::default(),
            mode: Mode::default(),
            readout: Readout::default(),
            dials: 0,
            pending_fault: None,
            flash: None,
            flash_text: String::new(),
            notice_text: String::new(),
            notice_hint: None,
            raised: false,
            lowered: None,
            exporting: false,
            exported: Slot::default(),
        }
    }
}

struct Showing {
    viewer: viewer::State,
    draft: Option<Draft>,
    pose: Option<Pose>,
    atlas: Option<Sheet>,
    opened: Option<PathBuf>,
    posed: Option<PathBuf>,
    cutting: Option<PathBuf>,
    rows: Vec<TreeRow>,
    widest: f32,
    listed: String,
}

impl Default for Showing {
    fn default() -> Self {
        Self {
            viewer: viewer::State::with_popup(popup::Kind::Animator).authoring(),
            draft: None,
            pose: None,
            atlas: None,
            opened: None,
            posed: None,
            cutting: None,
            rows: Vec::new(),
            widest: 0.0,
            listed: String::new(),
        }
    }
}

struct Session {
    plan: Plan,
    viewer: viewer::State,
    draft: Option<Draft>,
    pose: Option<Pose>,
    atlas: Option<Sheet>,
    mode: Mode,
    opened: Option<PathBuf>,
    posed: Option<PathBuf>,
    cutting: Option<PathBuf>,
    expanded: HashSet<usize>,
    wanted: Option<Held>,
    wanted_part: Option<usize>,
    rows: Vec<TreeRow>,
    widest: f32,
    listed: String,
    seeded: bool,
    scroll: f32,
    window: f32,
    scroll_id: widget::Id,
    strip_scroll: f32,
    strip_id: widget::Id,
    fields_id: widget::Id,
    picture: picture::State,
    framing: Option<usize>,
    slice: Option<usize>,
    carving: bool,
    slicing: Slot<usize>,
    gizmo: gizmo::State,
    drift: Vec<(usize, f32)>,
    winding: f32,
    drag: Drag,
    confirm: Slot<usize>,
    focus: Focus,
    timeline: timeline::State,
    loose_open: bool,
    history: History,
    key: String,
    keyed: Option<FrameCount>,
    primed: bool,
    watching: bool,
    watch_token: u64,
    gone: bool,
    entity: Scope,
    placed: Vec<viewer::Posed>,
    blame: Blame,
    alarms: Vec<usize>,
    faulting: Faults,
}

impl State {
    fn stash(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };

        let key = session.plan.set.name.clone();
        let history = std::mem::take(&mut session.history);
        let session = &*session;

        let held = Recall {
            key: session.plan.set.name.clone(),
            expanded: session.expanded.clone(),
            clip: session.viewer.selected_label(),
            curve: session.draft.as_ref().and_then(|draft| held_curve(&draft.doc, draft.track?)),
            part: session.pose.as_ref().and_then(|pose| pose.part),
            focus: session.focus,
        };

        self.recalled.retain(|known| known.key != held.key);
        self.recalled.push(held);

        while self.recalled.len() > RECALL_CAP {
            self.recalled.remove(0);
        }

        history::shelve(&mut self.histories, key, history, HISTORY_SETS);
    }

    fn recall_history(&mut self, name: &str) -> History {
        self.histories
            .iter()
            .position(|(held, _)| held == name)
            .map_or_else(History::default, |at| self.histories.remove(at).1)
    }

    pub(crate) fn begin(&mut self, mut plan: Plan) {
        self.flush_now();
        self.stash();

        let Showing {
            viewer,
            draft,
            pose,
            atlas,
            opened,
            posed,
            cutting,
            rows,
            widest,
            listed,
        } = self.session.take().map_or_else(Showing::default, |session| {
            let settled = session.viewer.loaded_rig() == plan.set.rig_id();
            let showing = session.showing();

            if settled {
                return showing;
            }

            Showing {
                viewer: showing.viewer,
                rows: showing.rows,
                widest: showing.widest,
                listed: showing.listed,
                ..Showing::default()
            }
        });

        let held = self.recalled.iter().find(|known| known.key == plan.set.name);
        let expanded = held.map(|held| held.expanded.clone()).unwrap_or_default();
        let seeded = held.is_some();

        let wanted = held.and_then(|held| held.curve);
        let wanted_part = held.and_then(|held| held.part);
        let focus = held.map_or_else(Focus::default, |held| held.focus);

        if let Some(clip) = held.and_then(|held| held.clip.clone()) {
            plan.clip = Some(clip);
        }

        let history = self.recall_history(&plan.set.name);

        self.session = Some(Session {
            plan,
            viewer,
            draft,
            pose,
            atlas,
            mode: self.mode,
            opened,
            posed,
            cutting,
            expanded,
            wanted,
            wanted_part,
            rows,
            widest,
            listed,
            seeded,
            scroll: 0.0,
            window: 0.0,
            scroll_id: widget::Id::unique(),
            strip_scroll: 0.0,
            strip_id: widget::Id::unique(),
            fields_id: widget::Id::unique(),
            picture: picture::State::default(),
            framing: None,
            slice: None,
            carving: false,
            slicing: Slot::default(),
            gizmo: gizmo::State::default(),
            drift: Vec::new(),
            winding: 1.0,
            drag: Drag::default(),
            confirm: Slot::default(),
            focus,
            timeline: timeline::State::default(),
            loose_open: false,
            history,
            key: String::new(),
            keyed: None,
            primed: false,
            watching: false,
            watch_token: 0,
            gone: false,
            entity: Scope::default(),
            placed: Vec::new(),
            blame: Blame::default(),
            alarms: Vec::new(),
            faulting: Faults::default(),
        });
    }

    pub(crate) fn adopt(
        &mut self,
        set: Set,
        target_mod: Option<String>,
        clip: Option<String>,
        copied: bool,
        side: Option<Side>,
    ) {
        let name = set.name.clone();

        self.pending_fault = side;

        match copied {
            true => self.manage.adopt(set.clone()),
            false => self.manage.seal(set.clone()),
        }

        self.begin(plan(set, target_mod, clip));

        if !copied {
            return;
        }

        self.raise(format!(
            "{}\nAnimation data copied to \"{}/{}\"",
            LOCKED_NOTICE,
            architecture::STUDIO,
            name
        ));
    }

    pub(crate) fn locate(&mut self, part: usize) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        let _ = session.spotlight(part);

        session.viewer.locate(part)
    }

    pub(crate) fn channels(&self, part: Option<usize>) -> Option<Channels> {
        let session = self.session.as_ref().filter(|session| session.mode == Mode::Entity)?;
        let pose = session.pose.as_ref()?;
        let part = part.filter(|at| *at < pose.doc.count())?;
        let wanted = i32::try_from(part).ok();

        let present = session.draft.as_ref().map_or_else(Vec::new, |draft| {
            draft
                .doc
                .tracks()
                .iter()
                .enumerate()
                .filter(|(_, track)| Some(track.part) == wanted)
                .map(|(at, track)| (track.kind, at))
                .collect()
        });

        Some(Channels {
            part,
            label: format!("Part {}", part),
            present,
            channelled: session.draft.is_some(),
            locatable: session.viewer.reachable(part),
        })
    }

    fn mount_of(plan: &Plan) -> String {
        plan.target_mod.clone().unwrap_or_else(|| match sets::folder_name(&plan.set) {
            Some(folder) => format!("{}/{}", architecture::STUDIO, folder),
            None => architecture::GAME.to_owned(),
        })
    }

    pub(crate) fn offsets(&self) -> Option<Offsets> {
        let session = self.session.as_ref().filter(|session| session.mode == Mode::Entity)?;
        let pose = session.pose.as_ref()?;

        Some(Offsets { rows: pose.doc.offsets(), addable: pose.doc.alignable() })
    }

    pub(crate) fn add_offset(&mut self) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Parts);

        let Some(pose) = session.pose.as_mut() else {
            return false;
        };

        if pose.doc.add_offset().is_none() {
            return false;
        }

        pose.backing.dirty = true;

        let settled = pose.persist_now();

        session.settle_pose();

        settled != Settled::Failed
    }

    pub(crate) fn drop_offset(&mut self, row: usize) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Parts);

        let Some(pose) = session.pose.as_mut() else {
            return false;
        };

        if !pose.doc.remove_offset(row) {
            return false;
        }

        pose.backing.dirty = true;

        let settled = pose.persist_now();

        session.settle_pose();

        settled != Settled::Failed
    }

    pub(crate) fn holder(&self, track: usize) -> Option<usize> {
        let session = self.session.as_ref().filter(|session| session.mode == Mode::Entity)?;
        let draft = session.draft.as_ref()?;

        usize::try_from(draft.doc.track(track)?.part).ok()
    }

    pub(crate) fn add_part(&mut self, parent: Option<usize>) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Parts);

        let Some(pose) = session.pose.as_mut() else {
            return false;
        };

        pose.grow(parent);

        let settled = pose.persist_now();

        if let Some(parent) = parent {
            session.expanded.insert(parent);
        }

        session.focus = Focus::Part;
        session.settle_pose();

        settled != Settled::Failed
    }

    pub(crate) fn drop_part(&mut self, part: usize) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Bulk);

        let Some(moved) = session.pose.as_mut().and_then(|pose| pose.doc.remove_part(part)) else {
            return false;
        };

        session.restructure(moved);

        true
    }

    pub(crate) fn add_channel(&mut self, part: usize, kind: i32) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Keys);

        let seeded = authoring::blank_curve(part, kind, session.viewer.rig().map(|rig| &rig.model));

        let Some(draft) = session.draft.as_mut() else {
            return false;
        };

        let at = draft.doc.tracks().len();
        draft.doc.insert(at, seeded);
        draft.retrack(at);
        draft.backing.dirty = true;

        session.focus = Focus::Curve;
        session.aim();
        session.relist();

        true
    }

    pub(crate) fn drop_channel(&mut self, track: usize) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };

        session.remember(Tag::Keys);

        let Some(draft) = session.draft.as_mut() else {
            return false;
        };

        if draft.doc.remove(track).is_none() {
            return false;
        }

        draft.track = match draft.track {
            Some(held) if held == track => None,
            Some(held) if held > track => Some(held - 1),
            held => held,
        };

        draft.restate();
        draft.backing.dirty = true;

        session.aim();
        session.relist();

        true
    }

    pub(crate) fn sync_state(&self, state: &mut StudioState) {
        let set = self.manage.set();

        state.name = self.manage.name().to_owned();
        state.sheet = set.sheet.clone();
        state.cuts = set.cuts.clone();
        state.model = set.model.clone();
        state.anims = set.anims.clone();
        state.sealed = self.sealed();
        state.target_mod = self.session.as_ref().and_then(|session| session.plan.target_mod.clone());
        state.atlas = self.mode == Mode::Atlas;
        state.clip = self.session.as_ref().and_then(|session| session.viewer.selected_label());
    }

    pub(crate) fn restore_state(&mut self, state: &StudioState, mounted: Option<&str>) {
        self.mode = if state.atlas { Mode::Atlas } else { Mode::Entity };

        if state.sealed && state.target_mod.as_deref() != mounted {
            return;
        }

        let set = sets::Set {
            name: state.name.clone(),
            sheet: state.sheet.clone(),
            cuts: state.cuts.clone(),
            model: state.model.clone(),
            anims: state.anims.iter().filter(|path| path.is_file()).cloned().collect(),
        };

        if !set.rigged() || !set.files().iter().all(|path| path.is_file()) {
            return;
        }

        match state.sealed {
            true => self.manage.seal(set.clone()),
            false => self.manage.adopt(set.clone()),
        }

        self.begin(plan(set, state.target_mod.clone(), state.clip.clone()));
    }

    pub(crate) fn pace(&self) -> Option<Duration> {
        let swapping = self.lowered.is_some_and(|at| at.elapsed() < SLIDE_DURATION);

        let Some(session) = self.session.as_ref() else {
            return (self.managing || swapping).then_some(RESTING_TICK);
        };

        let live = swapping
            || session.viewer.playing()
            || session.viewer.holding()
            || session.draft.as_ref().is_some_and(|draft| draft.backing.busy())
            || session.pose.as_ref().is_some_and(|pose| pose.backing.busy())
            || session.atlas.as_ref().is_some_and(|atlas| atlas.backing.busy());

        Some(if live { PLAYING_TICK } else { RESTING_TICK })
    }

    pub(crate) fn flush_now(&mut self) {
        if let Some(session) = self.session.as_mut() {
            session.flush();
        }
    }

    pub(crate) fn update(
        &mut self,
        message: Message,
        settings: &mut Settings,
        anim: &mut AnimState,
    ) -> Task<Message> {
        self.unlocked = settings.files.unlock_game_mount;

        if let Some(side) = self.pending_fault.take()
            && settings.studio.auto_faults
        {
            settings.studio.faults = Faults::of(side);
        }

        let chromed = self.chrome(&message, settings);

        if std::mem::take(&mut self.shed_onion) {
            settings.studio.onion_arm(false);
        }

        if let Some(task) = chromed {
            self.settle_notice();

            return task;
        }

        let Some(session) = self.session.as_mut() else {
            self.settle_notice();

            return Task::none();
        };

        let faulting = settings.studio.faults;

        if session.faulting != faulting {
            session.faulting = faulting;
            session.relist();
        }

        if let Some(draft) = session.draft.as_mut() {
            let typing = matches!(&message, Message::Changed(at, field, _) if draft.buffering(*at, *field));

            if !typing {
                draft.resolve_buffer();
            }

            if !matches!(&message, Message::LoopChanged(_)) {
                draft.resolve_looping();
            }
        }

        if let Some(pose) = session.pose.as_mut() {
            let typing = match &message {
                Message::Field(at, _) => pose.buffering(Slotted::Cell(*at)),
                Message::OffsetChanged(axis, _) => pose.buffering(Slotted::Axis(*axis)),
                _ => false,
            };

            if !typing {
                pose.resolve_buffer();
            }
        }

        if let Some(atlas) = session.atlas.as_mut() {
            let typing = matches!(&message, Message::Cut(at, cell, _) if atlas.buffering(*at, *cell));

            if !typing {
                atlas.resolve_buffer();
            }
        }

        let task = match message {
            Message::Tick => {
                if session.vanished() {
                    self.manage.adopt(sets::Set::default());
                    self.stash();
                    self.session = None;

                    return Task::none();
                }

                if session.adrift() {
                    session.refresh();
                }

                let priming = session.sync(settings, anim);
                session.viewer.tick();
                session.reposed(settings.studio.entity);
                session.realign();
                session.repose();

                let carving = session.recarve();
                let flush =
                    session.draft.as_mut().map_or_else(Task::none, |draft| draft.persist_if_dirty());
                let shaped =
                    session.pose.as_mut().map_or_else(Task::none, |pose| pose.persist_if_dirty());
                let carved =
                    session.atlas.as_mut().map_or_else(Task::none, |atlas| atlas.persist_if_dirty());
                let watching = session.watched();

                Task::batch([priming, carving, flush, shaped, carved, watching])
            }
            Message::Viewer(msg) => {
                session.viewer.update(msg, settings, anim).map(Message::Viewer)
            }
            Message::Press(index) => {
                let cargo = session.rows.get(index).and_then(TreeRow::cargo);

                session.drag = match cargo {
                    Some(cargo) => Drag::Pressed { row: index, cargo, since: Instant::now() },
                    None => Drag::Idle,
                };

                Task::none()
            }
            Message::DragMove(at) => {
                let carried = match session.drag {
                    Drag::Moving { cargo, .. } => Some(cargo),
                    Drag::Pressed { cargo, .. } if session.drag.ripe() => Some(cargo),
                    _ => None,
                };

                if let Some(cargo) = carried {
                    session.drag = Drag::Moving { cargo, at, onto: session.landing(cargo, at) };
                }

                Task::none()
            }
            Message::DragEnd => {
                let settled = std::mem::take(&mut session.drag);

                match settled {
                    Drag::Pressed { row, .. } => {
                        session.select(row);

                        return Task::none();
                    }
                    Drag::Moving { onto: None, .. } => {}
                    Drag::Moving { cargo, onto: Some(onto), .. } => session.land(cargo, onto),
                    _ => {}
                }

                Task::none()
            }
            Message::Picture(picture::Message::Framed(outline)) => {
                let Some(at) = session.framing.take() else {
                    return Task::none();
                };

                session.remember(Tag::Cuts);

                let Some(atlas) = session.atlas.as_mut() else {
                    return Task::none();
                };

                let region =
                    [outline.x as i32, outline.y as i32, outline.width as i32, outline.height as i32];

                atlas.hidden = None;
                atlas.place(at, region);
                atlas.restate();

                atlas.persist_if_dirty()
            }
            Message::Picture(picture::Message::Cancelled) => {
                session.framing = None;
                session.redraw();

                Task::none()
            }
            Message::Picture(picture::Message::Picked(at)) => {
                let hit = session.atlas.as_ref().and_then(|atlas| atlas.over(at));

                session.reslice(hit);

                Task::none()
            }
            Message::Picture(msg) => {
                session.picture.update(msg);

                Task::none()
            }
            Message::Slice(at) => {
                session.reslice(Some(at));

                Task::none()
            }
            Message::Find(at) => {
                if let Some(atlas) = session.atlas.as_ref()
                    && let Some(centre) = atlas.find(at)
                {
                    session.picture.focus(&atlas.source, centre);
                }

                Task::none()
            }
            Message::Carved(carved) => {
                session.carving = false;
                session.atlas = carved.and_then(Arc::into_inner);
                session.framing = None;
                session.slice = None;
                session.picture.reset();

                Task::none()
            }
            Message::Cut(at, cell, value) => {
                session.remember(Tag::Cut(at, cell));

                let Some(atlas) = session.atlas.as_mut() else {
                    return Task::none();
                };

                atlas.edit(at, cell, &value);

                atlas.persist_if_dirty()
            }
            Message::Frame(at) => {
                session.framing = match session.framing {
                    Some(held) if held == at => None,
                    _ => Some(at),
                };

                session.redraw();

                Task::none()
            }
            Message::Trim(at) => {
                session.remember(Tag::Cuts);

                let region = session.atlas.as_ref().and_then(|atlas| atlas.opaque(at));

                let (Some(region), Some(atlas)) = (region, session.atlas.as_mut()) else {
                    return Task::none();
                };

                atlas.place(at, region);

                atlas.persist_if_dirty()
            }
            Message::DropCut(at) => {
                if !session.slicing.take(&at) {
                    return session.slicing.set(at, Message::DropCutExpired);
                }

                session.remember(Tag::Bulk);

                let Some(moved) = session.atlas.as_mut().and_then(|atlas| atlas.doc.remove_cut(at)) else {
                    return Task::none();
                };

                session.recut(moved);

                Task::none()
            }
            Message::DropCutExpired => {
                session.slicing.expire();

                Task::none()
            }
            Message::Row(index) => {
                session.select(index);

                Task::none()
            }
            Message::Scrolled(offset, window) => {
                session.scroll = offset;
                session.window = window;

                if let Drag::Moving { cargo, at, .. } = session.drag {
                    session.drag = Drag::Moving { cargo, at, onto: session.landing(cargo, at) };
                }

                Task::none()
            }
            Message::StripScrolled(offset) => {
                session.strip_scroll = offset;

                Task::none()
            }
            Message::Switch(mode) => {
                session.mode = mode;
                session.focus = Focus::Curve;
                session.framing = None;
                session.repose();
                session.aim();
                session.relist();

                session.recarve()
            }
            Message::Field(at, value) => {
                session.remember(Tag::Field(at));

                let Some(pose) = session.pose.as_mut() else {
                    return Task::none();
                };

                pose.edit(at, &value);

                pose.persist_if_dirty()
            }
            Message::OffsetChanged(axis, value) => {
                session.remember(Tag::Axis(axis));

                let Some(pose) = session.pose.as_mut() else {
                    return Task::none();
                };

                pose.shift(axis, &value);

                pose.persist_if_dirty()
            }
            Message::AddPart => {
                let parent = session.pose.as_ref().and_then(|pose| pose.part);

                self.add_part(parent);

                Task::none()
            }
            Message::Onioned(knob, typed) => {
                let typed = match knob.digits() {
                    true => typed,
                    false => typed.trim_start_matches('#').to_owned(),
                };

                let allowed = match knob.digits() {
                    true => typed.chars().all(|glyph| glyph.is_ascii_digit()),
                    false => {
                        typed.len() <= HEX_DIGITS && typed.chars().all(|glyph| glyph.is_ascii_hexdigit())
                    }
                };

                if allowed {
                    knob.set(&mut settings.studio, typed);
                }

                Task::none()
            }
            Message::Onioning(switch) => {
                settings.studio.onion_arm(switch.on());
                self.onioning = switch.on();

                Task::none()
            }
            Message::Tiered(dial, tier) => {
                dial.set_tier(&mut settings.studio, tier);

                Task::none()
            }
            Message::Sighted(dial, shown) => {
                dial.set_shown(&mut settings.studio, shown);

                Task::none()
            }
            Message::Scoped(scope) => {
                settings.studio.entity = scope;

                Task::none()
            }
            Message::Framed(part, window) => {
                session.timeline.seat(part, window);

                Task::none()
            }
            Message::Scrub(frame) => {
                session.viewer.scrub(frame as f32, settings.studio.scrub);

                Task::none()
            }
            Message::Pick(track) => {
                session.focus = Focus::Curve;

                if let Some(draft) = session.draft.as_mut() {
                    draft.retrack(track);
                }

                session.aim();

                Task::none()
            }
            Message::LoopChanged(value) => {
                session.remember(Tag::Loop);

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.set_looping(&value);

                let task = draft.persist_if_dirty();
                session.relist();

                task
            }
            Message::AddKey => {
                session.remember(Tag::Keys);

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.add_key();

                let task = draft.persist_if_dirty();
                session.relist();

                task
            }
            Message::Seek(at) => {
                if let Some(frame) = session.draft.as_ref().and_then(|draft| draft.key_at(at)) {
                    session.viewer.seek(frame as f32);
                }

                Task::none()
            }
            Message::Bound(at) => {
                if let Some((start, end)) = session.draft.as_ref().and_then(|draft| draft.key_span(at)) {
                    session.viewer.bound(start as f32, end as f32);
                }

                Task::none()
            }
            Message::DropKey(at) => {
                if !session.confirm.take(&at) {
                    return session.confirm.set(at, Message::DropExpired);
                }

                session.remember(Tag::Keys);

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.drop_key(at);

                let task = draft.persist_if_dirty();
                session.relist();

                task
            }
            Message::DropExpired => {
                session.confirm.expire();

                Task::none()
            }
            Message::EaseChanged(at, ease) => {
                session.remember(Tag::Ease(at));

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                draft.set_ease(at, ease);

                draft.persist_if_dirty()
            }
            Message::Changed(at, field, value) => {
                session.remember(Tag::Key(at, field));

                let Some(draft) = session.draft.as_mut() else {
                    return Task::none();
                };

                let moved = draft.edit(at, field, &value);
                let chase = moved
                    .and_then(|to| Field::TYPED.iter().position(|known| *known == field).map(|column| (to, column)))
                    .map(|(to, column)| operation::focus(draft.cursor(to, column)));

                let flush = draft.persist_if_dirty();

                match chase {
                    Some(chase) => Task::batch([flush, chase]),
                    None => flush,
                }
            }
            Message::Watched(token, watch) => {
                session.sighted(token, &watch);

                Task::none()
            }
            Message::Persisted(token, path, stamp) => {
                session.restamp();

                if let Some(atlas) = session.atlas.as_mut().filter(|atlas| atlas.backing.token == token) {
                    atlas.backing.settle(path, stamp);

                    return Task::none();
                }

                if let Some(pose) = session.pose.as_mut().filter(|pose| pose.backing.token == token) {
                    let saved = pose.backing.settle(path, stamp) == Settled::Saved;
                    let updated = pose.doc.shared();

                    if saved {
                        session.viewer.adopt_model(updated);
                    }

                    session.relist();

                    return Task::none();
                }

                let Some(draft) = session.draft.as_mut().filter(|draft| draft.backing.token == token) else {
                    return Task::none();
                };

                let saved = draft.backing.settle(path, stamp) == Settled::Saved;
                let updated = draft.doc.shared();

                if saved && let Some(showing) = session.viewer.selected_anim().cloned() {
                    session.viewer.adopt_anim(&showing, updated);
                }

                Task::none()
            }
            Message::Exported(..) | Message::ExportExpired => Task::none(),
            Message::AddCut => {
                session.remember(Tag::Cuts);

                let Some(atlas) = session.atlas.as_mut() else {
                    return Task::none();
                };

                let added = atlas.doc.add_cut();

                atlas.backing.dirty = true;
                atlas.restate();

                let task = atlas.persist_if_dirty();

                session.reslice(Some(added));

                task
            }
            Message::Undo => session.undo(),
            Message::Handed(hand) => {
                settings.studio.gizmo = hand;

                Task::none()
            }
            Message::Gizmo(gizmo::Turn::Halt) => {
                session.gizmo.show(false);

                Task::none()
            }
            Message::Gizmo(gizmo::Turn::Pick(part)) => {
                session.gizmo.show(false);

                if session.chosen_part() != Some(part) {
                    return session.spotlight(part);
                }

                session.shed();

                Task::none()
            }
            Message::Gizmo(gizmo::Turn::Seize(part)) => {
                let hand = session.hand(settings);
                let task = session.spotlight(part);

                session.grasp(part, gizmo::Grip::Move, hand);

                task
            }
            Message::Gizmo(gizmo::Turn::Begin(part, grip)) => {
                session.grasp(part, grip, session.hand(settings));

                Task::none()
            }
            Message::Gizmo(gizmo::Turn::Drag(sweep)) => session.haul(sweep, session.hand(settings)),
            Message::Gizmo(gizmo::Turn::Zoom(pixels)) => {
                session.viewer.zoom(pixels);
                session.viewer.store_camera(anim);

                Task::none()
            }
            Message::Gizmo(gizmo::Turn::Drop) => {
                session.gizmo.seize(None);

                Task::none()
            }
            Message::Gizmo(gizmo::Turn::Fade(step)) => session.tint(step, session.hand(settings)),
            Message::OpenManage
            | Message::OpenOnion
            | Message::Aimed(_)
            | Message::Ship
            | Message::ShipExpired
            | Message::ShipPopup(_)
            | Message::OnionPopup(_)
            | Message::ManagePopup(_)
            | Message::Manage(_)
            | Message::Module(_)
            | Message::Refused(_)
            | Message::Faulted(_)
            | Message::Page(_)
            | Message::Cycle(_)
            | Message::Export => Task::none(),
        };

        self.settle_notice();

        task
    }

    fn chrome(&mut self, message: &Message, settings: &mut Settings) -> Option<Task<Message>> {
        match message {
            Message::Module(readout) => {
                self.readout = *readout;

                Some(Task::none())
            }
            Message::Refused(notice) => {
                self.raise((*notice).to_owned());

                Some(Task::none())
            }
            Message::Faulted(faults) => {
                settings.studio.faults = *faults;

                Some(Task::none())
            }
            Message::Page(page) => {
                self.dials = (*page).min(Dial::pages().saturating_sub(1));

                Some(Task::none())
            }
            Message::Cycle(dial) => {
                let anim = &mut settings.studio;

                match dial {
                    Dial::Gizmo => anim.gizmo = stepped(&Gizmo::ALL, anim.gizmo),
                    Dial::Onion => {
                        let next = stepped(&Switch::ALL, anim.onion);

                        anim.onion_arm(next.on());
                        self.onioning = next.on();
                    }
                    Dial::Module => self.readout = stepped(&Readout::ALL, self.readout),
                    Dial::Fault => anim.faults = stepped(&Faults::ALL, anim.faults),
                    Dial::Entity => anim.entity = stepped(&Scope::ALL, anim.entity),
                    _ => match dial.tier(anim) {
                        Some(tier) => dial.set_tier(anim, stepped(&Tier::ALL, tier)),
                        None => {
                            let held = dial.shown(anim).unwrap_or_default();

                            dial.set_shown(anim, stepped(&Shown::ALL, held));
                        }
                    },
                }

                Some(Task::none())
            }
            Message::Tick => {
                if self.managing {
                    self.manage.restock();
                    self.settle_track();
                }

                None
            }
            Message::OpenManage => {
                self.managing = true;
                self.manage.restock();

                Some(Task::none())
            }
            Message::OpenOnion => {
                self.onioning = true;

                Some(Task::none())
            }
            Message::Aimed(typed) => {
                self.aimed = typed.clone();

                Some(Task::none())
            }
            Message::ShipPopup(msg) => {
                if self.ship_popup.update(msg.clone(), shipout::SPEC) {
                    self.shipping_to = false;
                }

                Some(Task::none())
            }
            Message::OnionPopup(msg) => {
                if self.onion_popup.update(msg.clone(), onion::SPEC) {
                    self.onioning = false;
                    self.shed_onion = true;
                }

                Some(Task::none())
            }
            Message::ManagePopup(msg) => {
                if self.popup.update(msg.clone(), manage::SPEC) {
                    self.managing = false;
                }

                Some(Task::none())
            }
            Message::Manage(msg) => Some(self.manage(msg.clone())),
            Message::Switch(mode) => {
                self.mode = *mode;

                None
            }
            Message::Export => {
                if self.exporting {
                    return Some(Task::none());
                }

                if self.mounted.is_some() {
                    self.shipping_to = true;
                    self.ship_armed.clear();

                    return Some(Task::none());
                }

                Some(self.ship(sets::Aim::Blank))
            }
            Message::Ship => {
                if self.exporting {
                    return Some(Task::none());
                }

                Some(self.ship(self.aim.clone()))
            }
            Message::ShipExpired => {
                self.ship_armed.expire();

                Some(Task::none())
            }
            Message::Exported(placed, stray) => {
                self.exporting = false;

                if *placed && *stray > 0 {
                    self.raise(format!(
                        "Failed to map {} files to Entity\nManual clean-up may be needed",
                        stray
                    ));
                }

                Some(self.exported.set(*placed, Message::ExportExpired))
            }
            Message::ExportExpired => {
                self.exported.expire();

                Some(Task::none())
            }
            _ => None,
        }
    }

    fn shipping(&self) -> Shipping {
        match (self.exporting, self.exported.get()) {
            (true, _) => Shipping::Running,
            (_, Some(true)) => Shipping::Placed,
            (_, Some(false)) => Shipping::Failed,
            _ => Shipping::Idle,
        }
    }

    fn wanted_notice(&self) -> Option<Notice> {
        if self.flash.is_some_and(|at| at.elapsed() < NOTICE_EXPIRY) {
            return Some((self.flash_text.clone(), None));
        }

        self.session
            .as_ref()
            .and_then(Session::alarm_notice)
            .or_else(|| self.session.as_ref().and_then(Session::pinned_notice))
            .or_else(|| self.session.as_ref().and_then(Session::entity_notice))
    }

    fn settle_notice(&mut self) {
        let wanted = self.wanted_notice();

        if self.raised && wanted.as_ref().is_some_and(|(text, _)| text == &self.notice_text) {
            return;
        }

        if self.raised {
            self.raised = false;
            self.lowered = Some(Instant::now());

            return;
        }

        let Some((text, sided)) = wanted else {
            return;
        };

        if self.lowered.is_some_and(|at| at.elapsed() < SLIDE_DURATION) {
            return;
        }

        self.notice_text = text;
        self.notice_hint = sided;
        self.raised = true;
        self.lowered = None;
    }

    fn raise(&mut self, text: String) {
        self.flash_text = text;
        self.flash = Some(Instant::now());
    }

    pub(crate) fn export_popup_visible(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.viewer.export_popup_open())
    }

    pub(crate) fn export_scroll_task<M: 'static>(&self) -> Task<M> {
        self.session.as_ref().map_or_else(Task::none, |session| session.viewer.export_scroll_task())
    }

    pub(crate) fn export_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.session
            .as_ref()
            .and_then(|session| session.viewer.export_popup_view(window))
            .map(|view| view.map(Message::Viewer))
    }

    pub(crate) fn expanded(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.viewer.is_expanded())
    }

    pub(crate) fn expanded_view<'a>(
        &'a self,
        settings: &'a Settings,
        anim: &'a AnimState,
    ) -> Option<Element<'a, Message>> {
        self.session
            .as_ref()
            .and_then(|session| session.viewer.expanded_view(settings, anim))
            .map(|view| view.map(Message::Viewer))
    }
}

impl Session {
    fn vanished(&self) -> bool {
        self.gone
    }

    pub(super) fn reposed(&mut self, entity: Scope) {
        self.entity = entity;
        self.placed = self.viewer.posed(entity);
    }

    fn watched(&mut self) -> Task<Message> {
        if self.watching {
            return Task::none();
        }

        let open: Vec<PathBuf> = [
            self.draft.as_ref().map(|held| held.backing.read_from.clone()),
            self.pose.as_ref().map(|held| held.backing.read_from.clone()),
            self.atlas.as_ref().map(|held| held.backing.read_from.clone()),
        ]
        .into_iter()
        .flatten()
        .collect();

        let files = self.plan.set.files();
        let rigged = self.plan.set.rigged();

        self.watching = true;

        let token = self.watch_token;

        Task::perform(
            smol::unblock(move || Arc::new(keep_watch(files, open, rigged))),
            move |watch| Message::Watched(token, watch),
        )
    }

    fn restamp(&mut self) {
        self.watch_token = self.watch_token.wrapping_add(1);
        self.watching = false;
    }

    fn sighted(&mut self, token: u64, watch: &Vigil) {
        self.watching = false;

        if token != self.watch_token {
            return;
        }

        self.gone = watch.gone;

        for (path, stamp) in &watch.seen {
            if let Some(draft) = self.draft.as_mut() {
                draft.backing.sighted(path, *stamp);
            }

            if let Some(pose) = self.pose.as_mut() {
                pose.backing.sighted(path, *stamp);
            }

            if let Some(atlas) = self.atlas.as_mut() {
                atlas.backing.sighted(path, *stamp);
            }
        }
    }

    fn showing(self) -> Showing {
        Showing {
            viewer: self.viewer,
            draft: self.draft,
            pose: self.pose,
            atlas: self.atlas,
            opened: self.opened,
            posed: self.posed,
            cutting: self.cutting,
            rows: self.rows,
            widest: self.widest,
            listed: self.listed,
        }
    }

    fn sync(&mut self, settings: &Settings, anim: &AnimState) -> Task<Message> {
        let set = self.plan.set.clone();
        let frames = settings.studio.frame_count;
        let mut priming = Task::none();

        if self.keyed != Some(frames) {
            self.keyed = Some(frames);
            self.key = set.key(frames);
        }

        if std::mem::replace(&mut self.primed, true) {
            self.viewer.sync(&self.key, || set.clips(frames), settings, anim);
        } else {
            priming = self.viewer.preload(&self.key, || set.clips(frames), anim).map(Message::Viewer);

            if let Some(label) = self.plan.clip.as_deref() {
                self.viewer.select_label(label);
            }
        }

        let selected = self.viewer.selected_anim().cloned();

        if selected == self.opened && !self.draft.as_ref().is_some_and(|draft| draft.backing.drifted()) {
            if self.listed != self.viewer.loaded_rig() {
                self.relist();
            }

            return priming;
        }

        let wanted = self
            .draft
            .as_ref()
            .and_then(|draft| held_curve(&draft.doc, draft.track?))
            .or_else(|| self.wanted.take());

        self.wanted = None;
        self.opened = selected.clone();
        self.draft = selected.as_deref().and_then(Draft::load);

        if let Some(wanted) = wanted {
            self.recover(wanted);
        }

        self.aim();
        self.relist();

        priming
    }

    fn reseat(&mut self, set: sets::Set) {
        let rigged = |set: &sets::Set| (set.sheet.clone(), set.cuts.clone(), set.model.clone());
        let moved = rigged(&self.plan.set) != rigged(&set);

        self.plan.set = set;
        self.keyed = None;

        if !moved {
            return;
        }

        self.wanted = self.draft.as_ref().and_then(|draft| held_curve(&draft.doc, draft.track?));
        self.wanted_part = self.pose.as_ref().and_then(|pose| pose.part);
        self.plan.clip = self.viewer.selected_label();

        self.history.clear();
        self.primed = false;

        self.opened = None;
        self.posed = None;
        self.cutting = None;
    }

    fn select(&mut self, index: usize) {
        let Some(row) = self.rows.get(index) else {
            return;
        };

        let (part, track, bucket) = (row.part, row.track, row.bucket);

        if bucket {
            self.loose_open = !self.loose_open;
            self.relist();

            return;
        }

        if let Some(track) = track {
            self.focus = Focus::Curve;

            if let Some(draft) = self.draft.as_mut() {
                draft.retrack(track);
            }

            self.aim();

            return;
        }

        let Some(part) = part else {
            return;
        };

        let held =
            self.focus == Focus::Part && self.pose.as_ref().is_some_and(|pose| pose.part == Some(part));

        self.focus = Focus::Part;

        if let Some(pose) = self.pose.as_mut() {
            pose.pick(part);
        }

        self.aim();

        if held && !self.expanded.remove(&part) {
            self.expanded.insert(part);
        }

        self.relist();
    }

    fn spotlight(&mut self, part: usize) -> Task<Message> {
        self.focus = Focus::Part;
        self.unfold(part);

        if let Some(pose) = self.pose.as_mut() {
            pose.pick(part);
        }

        self.aim();
        self.relist();

        self.reveal(part)
    }

    fn shed(&mut self) {
        self.focus = Focus::Part;

        if let Some(pose) = self.pose.as_mut() {
            pose.part = None;
            pose.restate();
        }

        self.aim();
        self.relist();
    }

    fn unfold(&mut self, part: usize) {
        let Some(pose) = self.pose.as_ref() else {
            return;
        };

        let mut at = part;

        for _ in 0..pose.doc.count() {
            self.expanded.insert(at);

            let Some(parent) = pose.doc.field(at, 0).and_then(|parent| usize::try_from(parent).ok()) else {
                return;
            };

            if parent == at || parent >= pose.doc.count() {
                return;
            }

            at = parent;
        }
    }

    fn reveal(&mut self, part: usize) -> Task<Message> {
        let Some(row) = self.rows.iter().position(|row| row.part == Some(part) && row.track.is_none())
        else {
            return Task::none();
        };

        let seat = row as f32 * ROW_HEIGHT;
        let window = self.window.max(ROW_HEIGHT * MIN_TREE_ROWS);

        if seat >= self.scroll && seat + ROW_HEIGHT <= self.scroll + window {
            return Task::none();
        }

        let wanted = (seat - window / 2.0).max(0.0);

        self.scroll = wanted;

        operation::scroll_to(
            self.scroll_id.clone(),
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: wanted },
        )
    }

    fn recover(&mut self, wanted: Held) {
        let found = self.draft.as_ref().and_then(|draft| locate_curve(&draft.doc, &wanted));

        if let Some(at) = found
            && let Some(draft) = self.draft.as_mut()
        {
            draft.retrack(at);
            self.focus = Focus::Curve;

            return;
        }

        let Ok(part) = usize::try_from(wanted.part) else {
            return;
        };

        if let Some(pose) = self.pose.as_mut().filter(|pose| part < pose.doc.count()) {
            pose.pick(part);
            self.focus = Focus::Part;
        }
    }

    fn remember(&mut self, tag: Tag) {
        let anchor = match tag.subject() {
            history::Subject::Anim => self.draft.as_ref().map(|draft| draft.backing.read_from.clone()),
            history::Subject::Model => self.pose.as_ref().map(|pose| pose.backing.read_from.clone()),
            history::Subject::Cuts => self.atlas.as_ref().map(|atlas| atlas.backing.read_from.clone()),
            history::Subject::Rig => None,
        };

        if !self.history.wanted(tag, anchor.as_deref()) {
            return;
        }

        let shot = match (tag.subject(), anchor) {
            (history::Subject::Anim, Some(path)) => {
                self.draft.as_ref().map(|draft| history::Shot::Anim(path, draft.doc.clone()))
            }
            (history::Subject::Model, Some(path)) => {
                self.pose.as_ref().map(|pose| history::Shot::Model(path, pose.doc.clone()))
            }
            (history::Subject::Cuts, Some(path)) => {
                self.atlas.as_ref().map(|atlas| history::Shot::Cuts(path, atlas.doc.clone()))
            }
            (history::Subject::Rig, _) => {
                self.flush();

                Some(history::Shot::Rig(self.bytes()))
            }
            _ => None,
        };

        if let Some(shot) = shot {
            self.history.push(tag, shot);
        }
    }

    fn bytes(&self) -> Vec<(PathBuf, Vec<u8>)> {
        self.plan
            .set
            .files()
            .into_iter()
            .filter_map(|path| fs::read(&path).ok().map(|body| (path, body)))
            .collect()
    }

    fn undo(&mut self) -> Task<Message> {
        let Some(shot) = self.history.pop() else {
            return Task::none();
        };

        match shot {
            history::Shot::Anim(path, doc) => {
                if let Some(draft) = self.draft.as_mut().filter(|draft| draft.backing.read_from == path) {
                    draft.doc = doc;
                    draft.retrack_clamped();
                    draft.backing.dirty = true;

                    let task = draft.persist_if_dirty();
                    self.relist();

                    return task;
                }

                self.rewrite(&path, &doc.write());
            }
            history::Shot::Model(path, doc) => {
                if let Some(pose) = self.pose.as_mut().filter(|pose| pose.backing.read_from == path) {
                    pose.doc = doc;
                    pose.reseat();
                    pose.restate();
                    pose.backing.dirty = true;

                    let task = pose.persist_if_dirty();

                    self.settle_pose();

                    return task;
                }

                self.rewrite(&path, &doc.write());
            }
            history::Shot::Cuts(path, doc) => {
                if let Some(atlas) = self.atlas.as_mut().filter(|atlas| atlas.backing.read_from == path) {
                    atlas.doc = doc;
                    atlas.restate();
                    atlas.backing.dirty = true;

                    return atlas.persist_if_dirty();
                }

                self.rewrite(&path, &doc.write());
            }
            history::Shot::Rig(files) => {
                self.flush();

                for (path, body) in files {
                    self.rewrite(&path, &body);
                }

                self.draft = None;
                self.pose = None;
                self.atlas = None;
            }
        }

        self.reload();

        Task::none()
    }

    fn rewrite(&self, path: &Path, body: &[u8]) {
        let Some(stamp) = preview::stamp(path) else {
            warn!(path = %path.display(), "Studio could not stamp a file it is undoing");

            return;
        };

        write_now(path, body, stamp);
    }

    fn flush(&mut self) {
        if let Some(draft) = self.draft.as_mut() {
            draft.persist_now();
        }

        if let Some(pose) = self.pose.as_mut() {
            pose.persist_now();
        }

        if let Some(atlas) = self.atlas.as_mut() {
            atlas.persist_now();
        }
    }

    fn realign(&mut self) {
        let row = self.viewer.offset();

        if let Some(pose) = self.pose.as_mut() {
            pose.aim(row);
        }
    }

    fn repose(&mut self) {
        let Some(path) = self.viewer.selected_model().map(Path::to_path_buf) else {
            return;
        };

        let stale = Some(&path) != self.posed.as_ref()
            || self.pose.as_ref().is_some_and(|pose| pose.backing.drifted());

        if !stale {
            return;
        }

        let path = Some(path);

        let held = self.pose.as_ref().and_then(|pose| pose.part).or(self.wanted_part.take());

        self.posed = path.clone();
        self.pose = path.as_deref().and_then(Pose::load);

        if let Some(pose) = self.pose.as_mut()
            && let Some(at) = held.filter(|at| *at < pose.doc.count())
        {
            pose.pick(at);
        }

        self.aim();
        self.relist();
    }

    fn landing(&self, cargo: Cargo, at: Point) -> Option<Landing> {
        if at.x < TREE_LEFT || at.x > TREE_RIGHT || at.y < TREE_TOP || self.rows.is_empty() {
            return None;
        }

        let travelled = at.y - TREE_TOP + self.scroll;

        if travelled < 0.0 {
            return None;
        }

        let row = ((travelled / ROW_HEIGHT).floor() as usize).min(self.rows.len() - 1);
        let within = (travelled - row as f32 * ROW_HEIGHT) / ROW_HEIGHT;

        let Cargo::Part(part) = cargo else {
            return self.rehome(row).map(|_| Landing::Onto(row));
        };

        let landing = match within {
            _ if (NEST_BAND..1.0 - NEST_BAND).contains(&within) => Landing::Onto(row),
            _ if within < NEST_BAND => Landing::Seam(row),
            _ => Landing::Seam(row + 1),
        };

        self.settle(part, landing).map(|_| landing)
    }

    fn rehome(&self, row: usize) -> Option<usize> {
        let row = self.rows.get(row)?;

        row.part.or(row.owner)
    }

    fn settle(&self, part: usize, onto: Landing) -> Option<Option<usize>> {
        let pose = self.pose.as_ref()?;

        let parent = match onto {
            Landing::Onto(row) => Some(self.rows.get(row)?.part?),
            Landing::Seam(gap) => self.seam(gap, pose),
        };

        match parent.filter(|anchor| pose.doc.descends(*anchor, part)) {
            Some(_) => None,
            None => Some(parent),
        }
    }

    fn seam(&self, gap: usize, pose: &Pose) -> Option<usize> {
        let above = gap.checked_sub(1).and_then(|row| self.rows.get(row)).and_then(|row| row.part)?;
        let below = self.rows.get(gap).and_then(|row| row.part);

        match below.and_then(|below| pose.doc.parent(below)) {
            Some(parent) if parent == above => Some(above),
            _ => pose.doc.parent(above),
        }
    }

    fn land(&mut self, cargo: Cargo, onto: Landing) {
        let part = match cargo {
            Cargo::Part(part) => part,
            Cargo::Track(track) => return self.rehouse(track, onto),
        };

        let Some(parent) = self.settle(part, onto) else {
            return;
        };

        if self.pose.as_ref().is_none_or(|pose| pose.doc.parent(part) == parent) {
            return;
        }

        self.remember(Tag::Parts);

        let Some(pose) = self.pose.as_mut() else {
            return;
        };

        if !pose.doc.reparent(part, parent) {
            return;
        }

        pose.backing.dirty = true;

        pose.persist_now();

        if let Some(parent) = parent {
            self.expanded.insert(parent);
        }

        if let Some(pose) = self.pose.as_mut() {
            pose.pick(part);
        }

        self.settle_pose();
    }

    fn rehouse(&mut self, track: usize, onto: Landing) {
        let Landing::Onto(row) = onto else {
            return;
        };

        let Some(wanted) = self.rehome(row).and_then(|part| i32::try_from(part).ok()) else {
            return;
        };

        if self.draft.as_ref().is_none_or(|draft| draft.doc.track(track).is_none_or(|held| held.part == wanted)) {
            return;
        }

        self.remember(Tag::Keys);

        let Some(draft) = self.draft.as_mut() else {
            return;
        };

        let Some(held) = draft.doc.edit(track) else {
            return;
        };

        held.part = wanted;
        draft.restate();
        draft.backing.dirty = true;

        if let Ok(part) = usize::try_from(wanted) {
            self.expanded.insert(part);
        }

        self.aim();
        self.relist();
    }

    fn settle_pose(&mut self) {
        if let Some(model) = self.pose.as_ref().map(|pose| pose.doc.shared()) {
            self.viewer.adopt_model(model);
        }

        self.aim();
        self.relist();
    }

    fn restructure(&mut self, moved: Vec<Option<usize>>) {
        let Some(pose) = self.pose.as_mut() else {
            return;
        };

        pose.backing.dirty = true;
        pose.persist_now();

        self.wanted_part = self
            .pose
            .as_ref()
            .and_then(|pose| pose.part)
            .and_then(|at| moved.get(at).copied().flatten());

        if let Some(draft) = self.draft.as_mut() {
            draft.persist_now();
        }

        for path in self.viewer.anim_paths() {
            retarget_file(&path, &moved);
        }

        self.expanded = self
            .expanded
            .iter()
            .filter_map(|at| moved.get(*at).copied().flatten())
            .collect();

        self.resplice();
        self.settle_pose();
    }

    fn adrift(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| draft.backing.drifted())
            || self.pose.as_ref().is_some_and(|pose| pose.backing.drifted())
            || self.atlas.as_ref().is_some_and(|atlas| atlas.backing.drifted())
    }

    fn reload(&mut self) {
        self.cutting = None;
        self.resplice();
    }

    pub(super) fn refresh(&mut self) {
        self.draft = None;
        self.pose = None;
        self.atlas = None;
        self.reload();
    }

    fn resplice(&mut self) {
        self.plan.clip = self.viewer.selected_label();
        self.opened = None;
        self.posed = None;
        self.wanted = None;
        self.primed = false;
        self.viewer.invalidate_paths();
        self.repose();
    }

    fn recarve(&mut self) -> Task<Message> {
        if self.mode != Mode::Atlas || self.carving {
            return Task::none();
        }

        let Some(path) = self.viewer.selected_cuts().map(Path::to_path_buf) else {
            return Task::none();
        };

        let stale = Some(&path) != self.cutting.as_ref()
            || self.atlas.as_ref().is_some_and(|atlas| atlas.backing.drifted());

        if !stale {
            return Task::none();
        }

        self.cutting = Some(path.clone());

        let opened = Backing::open(&path);
        let art = self.viewer.selected_sheet().map(fs::read).and_then(Result::ok);

        let Some(((backing, bytes), art)) = opened.zip(art) else {
            self.atlas = None;

            return Task::none();
        };

        self.carving = true;

        Task::perform(
            smol::unblock(move || Sheet::assemble(backing, bytes, art).map(Arc::new)),
            Message::Carved,
        )
    }

    fn reslice(&mut self, at: Option<usize>) {
        let held = self.slice;
        self.slice = at.filter(|at| held != Some(*at));

        self.redraw();
    }

    fn redraw(&mut self) {
        let (picked, hidden) = (self.slice, self.framing);

        if let Some(atlas) = self.atlas.as_mut() {
            atlas.picked = picked;
            atlas.hidden = hidden;
            atlas.restate();
        }
    }

    fn recut(&mut self, moved: Vec<Option<usize>>) {
        let Some(atlas) = self.atlas.as_mut() else {
            return;
        };

        atlas.backing.dirty = true;
        atlas.picked = None;
        atlas.hidden = None;
        atlas.restate();
        atlas.persist_now();

        if let Some(pose) = self.pose.as_mut()
            && pose.doc.retarget_sprites(&moved)
        {
            pose.backing.dirty = true;
            pose.persist_now();
        }

        if let Some(draft) = self.draft.as_mut() {
            draft.persist_now();
        }

        for path in self.viewer.anim_paths() {
            revalue_file(&path, &moved);
        }

        self.framing = None;
        self.slice = None;
        self.resplice();
    }

    fn chosen(&self) -> Option<usize> {
        match (self.mode, self.focus) {
            (Mode::Atlas, _) => None,
            (_, Focus::Part) => self.pose.as_ref().and_then(|pose| pose.part),
            (_, Focus::Curve) => {
                self.draft.as_ref().and_then(Draft::part).and_then(|part| usize::try_from(part).ok())
            }
        }
    }

    fn entity_notice(&self) -> Option<Notice> {
        (self.mode == Mode::Entity && !self.blame.quiet())
            .then(|| (ENTITY_FAULT.to_owned(), None))
    }

    fn alarm_notice(&self) -> Option<Notice> {
        if self.mode != Mode::Entity || self.blame.quiet() {
            return None;
        }

        match self.focus {
            Focus::Part => self.blame.notice(self.pose.as_ref().and_then(|pose| pose.part), None),
            Focus::Curve => {
                self.blame.notice(None, self.draft.as_ref().and_then(|draft| draft.track))
            }
        }
    }

    fn unit(&self) -> Option<i32> {
        let model = self.plan.set.model.as_deref()?;

        if !matches!(sets::home(model), sets::Home::Game | sets::Home::Mod) {
            return None;
        }

        sets::stem_id(model.file_stem()?.to_str()?)
    }

    fn slotted(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| slotted(&draft.backing.read_from))
    }

    fn alarming(&self) -> Vec<usize> {
        let (Some(side), Some(rig)) = (self.faulting.side(), self.viewer.rig()) else {
            return Vec::new();
        };

        let mut found = Vec::new();

        for (index, clip) in self.viewer.clips() {
            let Some(path) = clip.anim.as_deref() else {
                if self.blame.rigged() {
                    found.push(index);
                }

                continue;
            };

            let open = self
                .draft
                .as_ref()
                .filter(|draft| draft.backing.read_from == path)
                .map(|draft| draft.doc.shared());

            let held = match open {
                Some(held) => Some(held),
                None => fs::read(path)
                    .ok()
                    .and_then(|bytes| Maanim::parse(&bytes).ok())
                    .map(|doc| doc.shared()),
            };

            let Some(anim) = held else {
                continue;
            };

            let attacking = slotted(path) || self.faulting.attacking();
            let faulted = !crash::anim_faults(&anim, &rig.model).is_empty()
                || (attacking && !crash::attack_faults(&anim, side).is_empty());

            if faulted {
                found.push(index);
            }
        }

        found
    }

    fn relist(&mut self) {
        if self.viewer.loaded_rig() != self.plan.set.rig_id() {
            return;
        }

        self.seed();

        let tracks = match self.mode {
            Mode::Entity => self.draft.as_ref().map(|draft| &draft.doc),
            Mode::Atlas => None,
        };

        let model = self.viewer.rig().map(|rig| &rig.model);

        self.blame = match (model, self.faulting.side()) {
            (Some(model), Some(side)) => {
                let anim = tracks.map(Maanim::shared);
                let slotted = self.slotted();
                let forced = self.faulting.attacking() && !slotted;

                Blame::of(model, anim.as_deref(), self.unit(), side, slotted || forced, forced)
            }
            _ => Blame::default(),
        };

        self.alarms = self.alarming();

        let listed = listing(tracks, model, &self.expanded, self.loose_open, &self.blame);

        self.listed = self.viewer.loaded_rig().to_owned();
        self.widest = listed.iter().map(TreeRow::span).fold(0.0, f32::max);
        self.rows = listed;
    }

    fn seed(&mut self) {
        if self.seeded {
            return;
        }

        let Some(model) = self.viewer.rig().map(|rig| &rig.model) else {
            return;
        };

        self.seeded = true;

        if let [only] = roots(model).as_slice() {
            self.expanded.insert(*only);
        }
    }

    fn aim(&mut self) {
        let cuts = self.viewer.rig().map_or(0, |rig| rig.sheet.cuts.len());

        if let Some(pose) = self.pose.as_mut() {
            pose.cuts = cuts;
        }

        self.realign();

        let part = self.chosen();
        let kind = self.draft.as_ref().and_then(|draft| draft.curve()).map(|track| track.kind);

        let neutral = match (part, kind) {
            (Some(part), Some(kind)) => {
                authoring::neutral_value(kind, part, self.viewer.rig().map(|rig| &rig.model))
            }
            _ => 0,
        };

        if let Some(draft) = self.draft.as_mut() {
            draft.neutral = neutral;
            draft.hint = neutral.to_string();
            draft.restate();
        }

        self.viewer.set_highlight(part);
    }
}

impl State {
    fn ship(&mut self, aim: sets::Aim) -> Task<Message> {
        let set = self.manage.set().clone();
        let Some(root) = self.mounted.as_deref().map(sets::patch_root) else {
            return self.zip(set);
        };

        if aim.stem().is_none() {
            return self.zip(set);
        }

        if !sets::occupied(&set, &aim, &root).is_empty() && !self.ship_armed.armed_for(&()) {
            return self.ship_armed.set((), Message::ShipExpired);
        }

        self.exporting = true;
        self.exported.clear();
        self.ship_armed.clear();
        self.shipping_to = false;

        Task::perform(smol::unblock(move || planted(&set, &aim, &root)), |(placed, stray)| {
            Message::Exported(placed, stray)
        })
    }

    fn zip(&mut self, set: sets::Set) -> Task<Message> {
        let named = match &self.aim {
            sets::Aim::Zip(name) => Some(name.clone()),
            _ => None,
        };

        self.exporting = true;
        self.exported.clear();
        self.shipping_to = false;

        Task::perform(smol::unblock(move || shipped(&set, named.as_deref())), |placed| {
            Message::Exported(placed, 0)
        })
    }
}

fn planted(set: &sets::Set, aim: &sets::Aim, root: &std::path::Path) -> (bool, usize) {
    match sets::install(set, aim, root) {
        Ok(landed) => {
            info!(path = %landed.model.display(), "Studio installed a set onto an entity");

            (true, landed.stray)
        }
        Err(err) => {
            error!(set = %set.name, "Studio could not install the set: {}", err);

            (false, 0)
        }
    }
}

fn shipped(set: &sets::Set, named: Option<&str>) -> bool {
    match sets::export(set, named) {
        Ok(path) => {
            info!(path = %path.display(), "Studio exported a set");

            true
        }
        Err(err) => {
            error!(set = %set.name, "Studio could not export the set: {}", err);

            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_segment_spans_the_key_we_left_and_the_one_we_are_reaching() {
        // Both rows light up; the lighter one is the key being left and carries the
        // weaker tint, so the pair reads in the direction the pose is travelling.
        assert_eq!(spanned(0, Some(0), 3), Some(Span::Head));
        assert_eq!(spanned(1, Some(0), 3), Some(Span::Tail));
        assert_eq!(spanned(2, Some(0), 3), None);

        assert!(Span::Head.tint() < Span::Tail.tint());
    }

    #[test]
    fn the_final_and_only_keys_span_themselves_rather_than_orphaning() {
        // The last key holds its pose to the end, and a lone key is the whole track,
        // so neither has a partner to reach, but neither may go unmarked either.
        assert_eq!(spanned(2, Some(2), 3), Some(Span::Only));
        assert_eq!(spanned(0, Some(0), 1), Some(Span::Only));
        assert_eq!(spanned(0, None, 3), None);
    }
}
