pub mod authoring;
pub mod export;
pub mod posing;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nyanko::graphics::rig::Animation;
use serde::{Deserialize, Serialize};

use crate::Vfs;

const LOOP_CEILING: i32 = 999_999;

const MAANIM_EXT: &str = ".maanim";

const SLOT_WALK: usize = 0;
const SLOT_IDLE: usize = 1;
const SLOT_ATTACK: usize = 2;
const SLOT_KNOCKBACK: usize = 3;
pub const SLOT_MODEL: usize = 7;

const MODEL_NAME: &str = "Model";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Role {
    Walk,
    Idle,
    Attack,
    Knockback,
}

impl Role {
    pub(crate) fn loops(self) -> bool {
        matches!(self, Role::Walk | Role::Idle)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Loop {
    Exact,
    Frames,
    Auto,
    Continuous,
}

pub struct Rigging {
    pub id: String,
    pub png: PathBuf,
    pub cut: PathBuf,
    pub model: PathBuf,
}

pub struct Clip {
    pub name: Option<String>,
    pub slot: Option<usize>,
    pub role: Option<Role>,
    pub looping: Loop,
    pub rig: Arc<Rigging>,
    pub anim: Option<PathBuf>,
}

#[derive(Default)]
pub struct ClipSet {
    pub name: String,
    pub clips: Vec<Clip>,
    pub offsets: Vec<Offset>,
}

pub struct Offset {
    pub row: Option<usize>,
    pub name: &'static str,
}

impl Offset {
    pub fn none(name: &'static str) -> Self {
        Self { row: None, name }
    }

    pub fn at(row: usize, name: &'static str) -> Self {
        Self { row: Some(row), name }
    }
}

pub const RAW_OFFSET: &str = "Raw";
pub const NO_OFFSET: &str = "None";

/// Which alignment row places a rig in the viewer.
///
/// The engine has no unplaced mode — every drawn entity runs the anchor
/// transform, every frame — so the first row is the default and `Bare` is a
/// deliberate look behind it, never a fallback.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Placement {
    Bare,
    Row(usize),
}

impl Default for Placement {
    fn default() -> Self {
        Placement::Row(0)
    }
}

const COMBAT_OFFSET: &str = "Combat";

pub fn named_offsets(placement: &'static str) -> Vec<Offset> {
    vec![Offset::none(RAW_OFFSET), Offset::at(0, COMBAT_OFFSET), Offset::at(1, placement)]
}

impl Clip {
    pub fn model(rig: Arc<Rigging>) -> Self {
        Self {
            name: Some(MODEL_NAME.to_string()),
            slot: Some(SLOT_MODEL),
            role: None,
            looping: Loop::Frames,
            rig,
            anim: None,
        }
    }

    pub fn label(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }

        self.anim
            .as_deref()
            .and_then(Path::file_stem)
            .and_then(OsStr::to_str)
            .map_or_else(|| self.rig.id.clone(), str::to_string)
    }

    pub fn slug(&self) -> String {
        let Some(name) = &self.name else {
            return self.label();
        };

        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect()
    }
}

pub(crate) fn standard(index: usize) -> Option<(&'static str, usize, Role)> {
    match index {
        0 => Some(("Walk", SLOT_WALK, Role::Walk)),
        1 => Some(("Idle", SLOT_IDLE, Role::Idle)),
        2 => Some(("Attack", SLOT_ATTACK, Role::Attack)),
        3 => Some(("Knockback", SLOT_KNOCKBACK, Role::Knockback)),
        _ => None,
    }
}

pub(crate) fn rigging(vfs: &Vfs, id: &str, bases: &[String]) -> Option<Arc<Rigging>> {
    let resolve = |ext: &str| -> Option<PathBuf> {
        let names: Vec<String> = bases.iter().map(|base| format!("{}.{}", base, ext)).collect();
        vfs.find(names.as_slice())
    };

    Some(Arc::new(Rigging {
        id: id.to_string(),
        png: resolve("png")?,
        cut: resolve("imgcut")?,
        model: resolve("mamodel")?,
    }))
}

fn owns(suffix: &str) -> bool {
    suffix.starts_with('_') || suffix.bytes().all(|byte| byte.is_ascii_digit())
}

fn maanim_files(vfs: &Vfs, bases: &[String]) -> Vec<(String, String, PathBuf)> {
    let mut found: Vec<(String, String, PathBuf)> = Vec::new();

    for base in bases {
        for name in vfs.glob(base) {
            let canonical = vfs.stripped(&name).unwrap_or_else(|| name.to_string());

            let Some(suffix) = canonical.strip_prefix(base.as_str()).and_then(|rest| rest.strip_suffix(MAANIM_EXT))
            else {
                continue;
            };

            if !owns(suffix) {
                continue;
            }

            if found.iter().any(|(existing, _, _)| existing == suffix) {
                continue;
            }

            if let Some(path) = vfs.find(&canonical) {
                found.push((suffix.to_string(), canonical, path));
            }
        }
    }

    found.sort_by(|left, right| left.0.cmp(&right.0));
    found
}

pub(crate) fn maanims(vfs: &Vfs, bases: &[String]) -> Vec<(String, PathBuf)> {
    maanim_files(vfs, bases).into_iter().map(|(suffix, _, path)| (suffix, path)).collect()
}

pub struct RigFiles {
    pub key: String,
    pub sheet: String,
    pub cuts: String,
    pub model: String,
    pub anims: Vec<(String, String)>,
}

impl RigFiles {
    pub fn names(&self) -> Vec<String> {
        let rig = [self.sheet.as_str(), self.cuts.as_str(), self.model.as_str()];

        rig.into_iter().map(str::to_owned).chain(self.anims.iter().map(|(_, name)| name.clone())).collect()
    }
}

pub(crate) fn rig_files(vfs: &Vfs, id: &str, bases: &[String]) -> Option<RigFiles> {
    let resolve = |ext: &str| -> Option<String> {
        let names: Vec<String> = bases.iter().map(|base| format!("{}.{}", base, ext)).collect();
        let path = vfs.find(names.as_slice())?;

        path.file_name().and_then(OsStr::to_str).map(str::to_owned)
    };

    let anims = maanim_files(vfs, bases).into_iter().map(|(suffix, name, _)| (suffix, name)).collect();

    Some(RigFiles {
        key: id.to_string(),
        sheet: resolve("png")?,
        cuts: resolve("imgcut")?,
        model: resolve("mamodel")?,
        anims,
    })
}

pub fn loop_frame(animation: &Animation, frame: f32) -> f32 {
    seat(playback_frames(animation), frame)
}

pub fn restart(animation: &Animation) -> Option<i32> {
    cycle(animation).or_else(|| (animation.length() > 0).then(|| animation.declared_frames()))
}

pub fn restart_frame(animation: &Animation, frame: f32) -> f32 {
    restart(animation).map_or(frame, |frames| seat(frames, frame))
}

fn seat(frames: i32, frame: f32) -> f32 {
    if frames > 0 { frame.rem_euclid(frames as f32) } else { frame }
}

pub fn playback_frames(animation: &Animation) -> i32 {
    cycle(animation).unwrap_or_else(|| animation.declared_frames())
}

pub fn cycle(animation: &Animation) -> Option<i32> {
    animation.loop_frames().filter(|frames| *frames <= LOOP_CEILING)
}

#[inline(always)]
pub fn multiply_mat3(matrix_a: &[f32; 9], matrix_b: &[f32; 9]) -> [f32; 9] {
    [
        matrix_a[0]*matrix_b[0] + matrix_a[3]*matrix_b[1] + matrix_a[6]*matrix_b[2],
        matrix_a[1]*matrix_b[0] + matrix_a[4]*matrix_b[1] + matrix_a[7]*matrix_b[2],
        matrix_a[2]*matrix_b[0] + matrix_a[5]*matrix_b[1] + matrix_a[8]*matrix_b[2],

        matrix_a[0]*matrix_b[3] + matrix_a[3]*matrix_b[4] + matrix_a[6]*matrix_b[5],
        matrix_a[1]*matrix_b[3] + matrix_a[4]*matrix_b[4] + matrix_a[7]*matrix_b[5],
        matrix_a[2]*matrix_b[3] + matrix_a[5]*matrix_b[4] + matrix_a[8]*matrix_b[5],

        matrix_a[0]*matrix_b[6] + matrix_a[3]*matrix_b[7] + matrix_a[6]*matrix_b[8],
        matrix_a[1]*matrix_b[6] + matrix_a[4]*matrix_b[7] + matrix_a[7]*matrix_b[8],
        matrix_a[2]*matrix_b[6] + matrix_a[5]*matrix_b[7] + matrix_a[8]*matrix_b[8],
    ]
}



#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::{AnimModification, Animation, Keyframe};

    use super::{cycle, owns, playback_frames, restart, restart_frame};

    fn curve(loop_count: i32, first: i32, last: i32) -> AnimModification {
        AnimModification {
            part: 0,
            kind: 0,
            loop_count,
            min_value: 0,
            max_value: 0,
            name: String::new(),
            keyframes: vec![
                Keyframe { frame: first, value: 0, ease: 0, ease_power: 0 },
                Keyframe { frame: last, value: 0, ease: 0, ease_power: 0 },
            ],
        }
    }

    fn animation(curves: Vec<AnimModification>) -> Animation {
        Animation { version: 1, modifications: curves }
    }

    #[test]
    fn a_play_once_curve_has_no_cycle_and_plays_its_declared_frames() {
        // An attack holds a loop_count of one, which is what makes the viewer show "???".
        let attack = animation(vec![curve(1, 0, 129)]);

        assert_eq!(cycle(&attack), None);
        assert_eq!(playback_frames(&attack), 130);
    }

    #[test]
    fn a_repeating_curve_keeps_its_cycle_as_a_frame_count() {
        // Spans 8 and 12 realign after 24 frames, and frame 24 is frame 0 again,
        // so the cycle is 24 frames rather than 25.
        let walk = animation(vec![curve(-1, 0, 8), curve(-1, 0, 12)]);

        assert_eq!(cycle(&walk), Some(24));
        assert_eq!(playback_frames(&walk), 24);
    }

    #[test]
    fn a_cycle_past_the_display_ceiling_falls_back_to_the_declared_frames() {
        // 9973 and 9967 are coprime, so the crate reports 99,400,891 frames. The
        // ceiling is ours: six digits is what the viewer's frame field can show.
        let absurd = animation(vec![curve(-1, 0, 9_973), curve(-1, 0, 9_967)]);

        assert_eq!(absurd.loop_frames(), Some(99_400_891));
        assert_eq!(cycle(&absurd), None);
        assert_eq!(playback_frames(&absurd), absurd.declared_frames());
    }

    // 872_f00 in the shipped corpus: 103 modifications, 99 of them replaying forever over
    // spans of 15, 18, 45, 60 and 200, and four that play exactly once. The play-once four
    // are why loop_frames gives up, and the forever ninety-nine are why frame 200 is not a
    // seam - so restarting there snaps every one of them mid-motion.
    #[test]
    fn a_timeline_that_never_ends_has_no_restart_to_snap_to() {
        let walk = animation(vec![curve(-1, 0, 60), curve(-1, 0, 15), curve(1, 0, 200)]);

        assert_eq!(walk.length(), -1);
        assert_eq!(walk.loop_frames(), None);
        assert_eq!(walk.declared_frames(), 200);

        assert_eq!(restart(&walk), None);
        assert_eq!(restart_frame(&walk, 260.0), 260.0);

        // The old bound wrapped at the declared count, which is what snapped.
        assert_eq!(playback_frames(&walk), 200);
    }

    #[test]
    fn a_timeline_that_ends_still_restarts_at_its_declared_length() {
        let attack = animation(vec![curve(1, 0, 89)]);

        assert_eq!(attack.length(), 90);
        assert_eq!(restart(&attack), Some(90));
        assert_eq!(restart_frame(&attack, 90.0), 0.0);
    }

    #[test]
    fn a_timeline_with_a_cycle_restarts_on_the_cycle() {
        let walk = animation(vec![curve(-1, 0, 8), curve(-1, 0, 12)]);

        assert_eq!(restart(&walk), Some(24));
        assert_eq!(restart_frame(&walk, 25.0), 1.0);
    }

    #[test]
    fn a_looping_animation_no_longer_replays_its_first_frame() {
        let walk = animation(vec![curve(-1, 0, 16)]);

        // 0..=15 are distinct; frame 16 renders as frame 0, so it must wrap there.
        assert_eq!(super::loop_frame(&walk, 15.0), 15.0);
        assert_eq!(super::loop_frame(&walk, 16.0), 0.0);
    }

    #[test]
    fn the_authored_range_and_the_playback_bound_agree_without_a_cycle() {
        // The exporter sizes its Frames row from declared_frames - 1 and the viewer
        // sizes its slider from playback_frames - 1. Those must land on the same
        // number, or the two controls disagree about the same animation.
        let attack = animation(vec![curve(1, 0, 15)]);
        assert_eq!(attack.declared_frames() - 1, playback_frames(&attack) - 1);

        // A held pose is one frame, so index zero from both directions.
        let knockback = animation(vec![curve(-1, 0, 0)]);
        assert_eq!(knockback.declared_frames() - 1, playback_frames(&knockback) - 1);

        // A looping walk ends one frame before its furthest keyframe, since that
        // frame renders as frame zero again.
        let walk = animation(vec![curve(-1, 0, 16)]);
        assert_eq!(walk.declared_frames() - 1, 15);
        assert_eq!(playback_frames(&walk) - 1, 15);
    }

    #[test]
    fn a_longer_name_sharing_the_prefix_is_not_ours() {
        // 008_charaawa.maanim starts with 008_c but belongs to a different unit.
        assert!(!owns("haraawa"));
        assert!(!owns("haraawa_zombie00"));
    }

    #[test]
    fn numbered_and_underscored_suffixes_are_ours() {
        assert!(owns("00"));
        assert!(owns("14"));
        assert!(owns("_zombie02"));
        assert!(owns(""));
    }
}
