use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::warn;

use nyanko::graphics::rig::{Animation, BoundingBox, Model, Rig};
use nyanko::graphics::tools::part;

use kore::common::preview::{self, Stamp};
use kore::systems::animation::{cycle, loop_frame, restart, restart_frame, Clip, ClipSet, Loop, Offset, Placement, Rigging, Role, NO_OFFSET};

pub(super) const COLUMNS: usize = 4;
const DEFAULT_SLOTS: usize = 8;

#[derive(Default)]
pub struct State {
    pub held_unit: Option<Arc<Rig>>,
    pub current_anim: Option<Arc<Animation>>,

    clips: Vec<Clip>,
    slots: Vec<Option<usize>>,
    selected: Option<usize>,

    set_key: String,
    set_name: String,
    offsets: Vec<Offset>,
    placement: Placement,
    loaded_rig: String,
    failed_rig: String,
    loaded_clip: Option<usize>,
    bounds: Option<BoundingBox>,
    measured: Option<(String, Option<usize>, Option<usize>)>,
    cache: RigCache,
    mapped: RefCell<Option<Mapped>>,
}

struct Mapped {
    rig: Arc<Rig>,
    anim: Option<Arc<Animation>>,
    frame: i32,
    offset: Option<usize>,
    parts: Vec<part::PartFrame>,
}

impl Mapped {
    fn serves(&self, rig: &Arc<Rig>, anim: Option<&Arc<Animation>>, frame: i32, offset: Option<usize>) -> bool {
        let same = match (self.anim.as_ref(), anim) {
            (Some(held), Some(wanted)) => Arc::ptr_eq(held, wanted),
            (None, None) => true,
            _ => false,
        };

        same && self.frame == frame && self.offset == offset && Arc::ptr_eq(&self.rig, rig)
    }
}

impl State {
    pub fn export_name(&self) -> &str {
        &self.set_name
    }

    pub fn offset(&self) -> Option<usize> {
        let rows = self.available_rows();

        match self.placement {
            Placement::Bare => None,
            Placement::Row(row) if row < rows => Some(row),
            Placement::Row(_) => (rows > 0).then_some(0),
        }
    }

    pub fn offset_label(&self) -> String {
        self.offset().map_or_else(|| self.bare_label().to_owned(), |row| self.offset_name(row))
    }

    fn available_rows(&self) -> usize {
        self.held_unit.as_ref().map_or(0, |unit| unit.model.alignment.len())
    }

    fn bare_label(&self) -> &str {
        self.offsets.iter().find(|held| held.row.is_none()).map_or(NO_OFFSET, |held| held.name)
    }

    fn offset_name(&self, row: usize) -> String {
        self.offsets
            .iter()
            .find(|held| held.row == Some(row))
            .map_or_else(|| format!("Row {}", row), |held| held.name.to_owned())
    }

    pub fn offset_choices(&self) -> Vec<String> {
        std::iter::once(self.bare_label().to_owned())
            .chain((0..self.available_rows()).map(|row| self.offset_name(row)))
            .collect()
    }

    pub fn select_offset(&mut self, label: &str) {
        if label == self.bare_label() {
            self.placement = Placement::Bare;

            return;
        }

        if let Some(row) = (0..self.available_rows()).find(|row| self.offset_name(*row) == label) {
            self.placement = Placement::Row(row);
        }
    }

    pub fn selected_offset(&self) -> Placement {
        self.placement
    }

    pub fn restore_offset(&mut self, placement: Placement) {
        self.placement = placement;
    }

    fn served(&self, rig: &Arc<Rig>, anim: Option<&Arc<Animation>>, frame: i32, offset: Option<usize>) -> Option<Vec<part::PartFrame>> {
        let held = self.mapped.borrow();

        held.as_ref().filter(|held| held.serves(rig, anim, frame, offset)).map(|held| held.parts.clone())
    }

    pub fn mapped(&self, frame: i32) -> Option<Vec<part::PartFrame>> {
        let rig = self.held_unit.as_ref()?;
        let anim = self.current_anim.as_ref();
        let offset = self.offset();

        if let Some(parts) = self.served(rig, anim, frame, offset) {
            return Some(parts);
        }

        let parts = part::resolve(rig, anim.map(Arc::as_ref), frame, offset).ok()?;

        *self.mapped.borrow_mut() = Some(Mapped {
            rig: Arc::clone(rig),
            anim: anim.cloned(),
            frame,
            offset,
            parts: parts.clone(),
        });

        Some(parts)
    }

    pub fn slots(&self) -> &[Option<usize>] {
        &self.slots
    }

    pub fn clips(&self) -> impl Iterator<Item = (usize, &Clip)> {
        self.clips.iter().enumerate()
    }

    pub fn clip(&self, index: usize) -> Option<&Clip> {
        self.clips.get(index)
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn current_clip(&self) -> Option<&Clip> {
        self.selected.and_then(|index| self.clips.get(index))
    }

    pub fn is_model(&self) -> bool {
        self.current_clip().is_some_and(|clip| clip.anim.is_none())
    }

    fn looping(&self) -> Loop {
        self.current_clip().map_or(Loop::Frames, |clip| clip.looping)
    }

    pub fn loop_supported(&self) -> bool {
        match self.looping() {
            Loop::Exact => true,
            Loop::Auto => self.current_anim.as_ref().is_some_and(|anim| cycle(anim).is_some()),
            Loop::Frames | Loop::Continuous => false,
        }
    }

    // Showcase composes the standard walk/idle/attack/knockback clips, so it only means
    // anything once those roles resolved. They resolve from a unit's own slot numbering, so
    // a Studio set in its own workspace and every Utilities rig carry none -- the export
    // would render the resting pose for every frame and read as "the animation is missing".
    pub fn showcasable(&self) -> bool {
        self.clips.iter().any(|clip| clip.role.is_some() && clip.anim.is_some())
    }

    pub fn role_paths(&self) -> Vec<(Role, PathBuf)> {
        self.clips
            .iter()
            .filter_map(|clip| Some((clip.role?, clip.anim.clone()?)))
            .collect()
    }

    pub fn role_path(&self, role: Role) -> Option<&PathBuf> {
        self.clips
            .iter()
            .find(|clip| clip.role == Some(role))
            .and_then(|clip| clip.anim.as_ref())
    }

    pub fn loop_bound(&self) -> Option<i32> {
        self.current_anim.as_ref().map_or(Some(0), |anim| match self.looping() {
            Loop::Exact => cycle(anim).map(|frames| frames - 1),
            Loop::Frames => Some(anim.declared_frames() - 1),
            Loop::Auto => restart(anim).map(|frames| frames - 1),
            Loop::Continuous => None,
        })
    }

    pub fn playback_frame(&self, frame: f32) -> f32 {
        let Some(anim) = self.current_anim.as_ref() else { return frame };

        match self.looping() {
            Loop::Exact | Loop::Continuous => frame,
            Loop::Frames => loop_frame(anim, frame),
            Loop::Auto => restart_frame(anim, frame),
        }
    }

    pub fn trailed(&self, frame: f32) -> Option<f32> {
        let seated = self.playback_frame(frame);

        if seated >= 0.0 {
            return Some(seated);
        }

        self.current_anim.as_ref().map(|anim| loop_frame(anim, seated))
    }

    pub fn playhead_key(&self) -> (&str, Option<usize>) {
        (&self.loaded_rig, self.selected)
    }

    pub fn bounds(&self) -> Option<BoundingBox> {
        self.bounds
    }

    pub fn measure(&mut self, tolerance: f32) {
        let offset = self.offset();
        let fresh = self.measured.as_ref().is_some_and(|(rig, clip, row)| {
            rig == &self.loaded_rig && *clip == self.loaded_clip && *row == offset
        });

        if fresh {
            return;
        }

        self.measured = Some((self.loaded_rig.clone(), self.loaded_clip, offset));
        self.bounds = self.held_unit.as_ref().and_then(|unit| {
            let anims: Vec<&Animation> = self.current_anim.as_deref().into_iter().collect();

            unit.calculate_bounds(&anims, tolerance, None, offset)
        });
    }

    pub fn loaded_rig(&self) -> &str {
        &self.loaded_rig
    }

    pub fn resolved(&self) -> bool {
        self.held_unit.is_some() || !self.failed_rig.is_empty()
    }

    pub fn selected_label(&self) -> Option<String> {
        self.current_clip().map(Clip::label)
    }

    pub fn select_label(&mut self, label: &str) {
        if let Some(index) = self.clips.iter().position(|clip| clip.label() == label) {
            self.select(index);
        }
    }

    pub fn select(&mut self, index: usize) {
        if self.selected == Some(index) || index >= self.clips.len() {
            return;
        }

        self.selected = Some(index);
        self.load_active();
    }

    pub fn selected_model(&self) -> Option<&Path> {
        self.current_clip().map(|clip| clip.rig.model.as_path())
    }

    pub fn anim_paths(&self) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = Vec::with_capacity(self.clips.len());

        for path in self.clips.iter().filter_map(|clip| clip.anim.as_ref()) {
            if !found.contains(path) {
                found.push(path.clone());
            }
        }

        found
    }

    pub fn selected_sheet(&self) -> Option<&Path> {
        self.current_clip().map(|clip| clip.rig.png.as_path())
    }

    pub fn selected_cuts(&self) -> Option<&Path> {
        self.current_clip().map(|clip| clip.rig.cut.as_path())
    }

    pub fn adopt_model(&mut self, model: Arc<Model>) {
        let Some(unit) = self.held_unit.as_deref().filter(|unit| unit.model != *model) else {
            return;
        };

        let mut fresh = Rig::clone(unit);
        fresh.model = Model::clone(&model);

        let fresh = Arc::new(fresh);

        let stamp = self.current_clip().map(|clip| RigStamp::of(&clip.rig)).unwrap_or_default();

        self.held_unit = Some(Arc::clone(&fresh));
        self.cache.insert(&self.loaded_rig, fresh, stamp);
        self.measured = None;
        self.bounds = None;
    }

    pub fn adopt_anim(&mut self, path: &Path, anim: Arc<Animation>) {
        if let Some(stamp) = preview::stamp(path) {
            self.cache.store_anim(&self.loaded_rig, path, stamp, anim.clone());
        }

        let showing = self
            .selected
            .and_then(|index| self.clips.get(index))
            .and_then(|clip| clip.anim.as_deref());

        if showing == Some(path) {
            self.current_anim = Some(anim);
        }
    }

    pub fn invalidate_paths(&mut self) {
        self.set_key.clear();
        self.loaded_rig.clear();
        self.failed_rig.clear();
        self.cache.clear();
    }

    pub fn reset_display(&mut self) {
        self.held_unit = None;
        self.current_anim = None;
        self.clips.clear();
        self.slots.clear();
        self.selected = None;
        self.set_key.clear();
        self.set_name.clear();
        self.offsets.clear();
        self.loaded_rig.clear();
        self.failed_rig.clear();
        self.loaded_clip = None;
        self.bounds = None;
        self.measured = None;
        self.mapped.replace(None);
    }

    pub fn sync(&mut self, key: &str, build: impl FnOnce() -> ClipSet) {
        self.prepare(key, build);
        self.load_active();
    }

    pub fn preload_request(&mut self, key: &str, build: impl FnOnce() -> ClipSet) -> Option<PreloadRequest> {
        self.prepare(key, build);
        self.build_request()
    }

    fn prepare(&mut self, key: &str, build: impl FnOnce() -> ClipSet) {
        if self.set_key != key {
            let previous = self.current_clip().map(Clip::label);
            let set = build();

            self.set_key = key.to_string();
            self.set_name = set.name;
            self.clips = set.clips;
            self.offsets = set.offsets;
            self.loaded_clip = None;
            let requests: Vec<Request> = self
                .clips
                .iter()
                .map(|clip| Request { slot: clip.slot, trailing: clip.anim.is_none() })
                .collect();

            self.slots = place(&requests);
            self.selected = previous.and_then(|label| self.clips.iter().position(|clip| clip.label() == label));
        }

        self.select_valid();
    }

    fn select_valid(&mut self) {
        if self.selected.is_some_and(|index| index < self.clips.len()) {
            return;
        }

        self.selected = self.slots.iter().find_map(|slot| *slot);

        if self.selected.is_none() {
            self.held_unit = None;
            self.current_anim = None;
            self.loaded_clip = None;
        }
    }

    fn build_request(&mut self) -> Option<PreloadRequest> {
        let index = self.selected?;
        let rig_id = self.clips.get(index)?.rig.id.clone();

        if self.is_loaded(&rig_id) || self.failed_rig == rig_id || self.apply_cached(index) {
            return None;
        }

        let clip = self.clips.get(index)?;

        Some(PreloadRequest {
            rig_id,
            png: clip.rig.png.clone(),
            cut: clip.rig.cut.clone(),
            model: clip.rig.model.clone(),
            anim: clip.anim.clone(),
        })
    }

    pub fn apply_preload(&mut self, result: PreloadResult) {
        let wanted = self
            .current_clip()
            .is_some_and(|clip| clip.rig.id == result.rig_id && !self.is_loaded(&result.rig_id));

        let Some(unit) = result.unit else {
            if wanted {
                self.loaded_rig = result.rig_id.clone();
                self.failed_rig = result.rig_id;
                self.held_unit = None;
                self.current_anim = None;
                self.loaded_clip = None;
            }
            return;
        };

        self.cache.insert(&result.rig_id, unit.clone(), result.stamps.rig.clone());

        if let (Some(path), Some(stamp), Some(anim)) =
            (&result.anim_path, result.stamps.anim, &result.anim)
        {
            self.cache.store_anim(&result.rig_id, path, stamp, anim.clone());
        }

        if !wanted {
            return;
        }

        self.held_unit = Some(unit);
        self.loaded_rig = result.rig_id;
        self.failed_rig.clear();

        if let Some(index) = self.selected {
            self.load_anim(index);
        }
    }

    fn is_loaded(&self, rig_id: &str) -> bool {
        self.loaded_rig == rig_id && self.held_unit.is_some()
    }

    fn apply_cached(&mut self, index: usize) -> bool {
        let Some(clip) = self.clips.get(index) else {
            return false;
        };

        let rig_id = clip.rig.id.clone();
        let stamp = RigStamp::of(&clip.rig);

        let Some(unit) = self.cache.lookup(&rig_id, &stamp) else {
            return false;
        };

        self.held_unit = Some(unit);
        self.loaded_rig = rig_id;
        self.failed_rig.clear();
        self.load_anim(index);
        true
    }

    fn load_active(&mut self) {
        let Some(index) = self.selected else {
            return;
        };

        let Some(rig) = self.clips.get(index).map(|clip| clip.rig.clone()) else {
            return;
        };

        let rig_id = rig.id.clone();

        if self.is_loaded(&rig_id) {
            if self.loaded_clip != Some(index) {
                self.load_anim(index);
            }
            return;
        }

        if self.failed_rig == rig_id || self.apply_cached(index) {
            return;
        }

        let stamp = RigStamp::of(&rig);
        let sources = (std::fs::read(&rig.png), std::fs::read(&rig.cut), std::fs::read(&rig.model));

        let loaded_unit = match sources {
            (Ok(png), Ok(cut), Ok(model)) => Rig::parse(&png, &cut, &model).ok(),
            _ => None,
        };

        match loaded_unit {
            Some(unit) => {
                let unit = Arc::new(unit);
                self.cache.insert(&rig_id, unit.clone(), stamp);
                self.held_unit = Some(unit);
                self.loaded_rig = rig_id;
                self.failed_rig.clear();
                self.load_anim(index);
            }
            None => {
                self.loaded_rig = rig_id.clone();
                self.failed_rig = rig_id;
                self.held_unit = None;
                self.current_anim = None;
                self.loaded_clip = None;
            }
        }
    }

    fn load_anim(&mut self, index: usize) {
        self.loaded_clip = Some(index);

        let Some(path) = self.clips.get(index).and_then(|clip| clip.anim.clone()) else {
            self.current_anim = None;
            return;
        };

        if let Some(anim) = self.cache.anim(&self.loaded_rig, &path) {
            self.current_anim = Some(anim);
            return;
        }

        let stamp = preview::stamp(&path);
        let parsed = std::fs::read(&path)
            .ok()
            .and_then(|bytes| Animation::parse(&bytes).ok())
            .map(Arc::new);

        if let (Some(stamp), Some(anim)) = (stamp, &parsed) {
            self.cache.store_anim(&self.loaded_rig, &path, stamp, anim.clone());
        }

        self.current_anim = parsed;
    }
}

#[derive(Clone, Copy)]
pub(super) struct Request {
    pub slot: Option<usize>,
    pub trailing: bool,
}

fn place(requests: &[Request]) -> Vec<Option<usize>> {
    let mut slots: Vec<Option<usize>> = vec![None; DEFAULT_SLOTS];
    let settled = |request: &Request| !request.trailing;

    let mut anchored: Vec<(usize, usize)> = requests
        .iter()
        .enumerate()
        .filter(|(_, request)| settled(request))
        .filter_map(|(index, request)| Some((request.slot?, index)))
        .collect();
    anchored.sort_by_key(|(slot, _)| *slot);

    for (slot, index) in anchored {
        settle(&mut slots, slot, 1, index);
    }

    for (index, _) in requests.iter().enumerate().filter(|(_, request)| settled(request) && request.slot.is_none()) {
        settle(&mut slots, 0, 1, index);
    }

    for (index, request) in requests.iter().enumerate().filter(|(_, request)| request.trailing) {
        settle(&mut slots, request.slot.unwrap_or(0), COLUMNS, index);
    }

    while !slots.len().is_multiple_of(COLUMNS) {
        slots.push(None);
    }

    slots
}

fn settle(slots: &mut Vec<Option<usize>>, from: usize, stride: usize, index: usize) {
    let mut at = from;

    loop {
        while at >= slots.len() {
            slots.push(None);
        }

        if slots[at].is_none() {
            slots[at] = Some(index);
            return;
        }

        at += stride;
    }
}

pub struct PreloadRequest {
    rig_id: String,
    png: PathBuf,
    cut: PathBuf,
    model: PathBuf,
    anim: Option<PathBuf>,
}

impl PreloadRequest {
    pub fn run(self) -> PreloadResult {
        let stamp = RigStamp {
            png: preview::stamp(&self.png),
            cut: preview::stamp(&self.cut),
            model: preview::stamp(&self.model),
        };
        let stamps = Box::new(Stamps { rig: stamp, anim: self.anim.as_deref().and_then(preview::stamp) });

        let unit = match (std::fs::read(&self.png), std::fs::read(&self.cut), std::fs::read(&self.model)) {
            (Ok(png_bytes), Ok(cut_bytes), Ok(model_bytes)) => Rig::parse(&png_bytes, &cut_bytes, &model_bytes).ok(),
            _ => None,
        };

        if unit.is_none() {
            warn!("Animation preload failed for {}", self.rig_id);
        }

        let anim = self
            .anim
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| Animation::parse(&bytes).ok());

        PreloadResult {
            rig_id: self.rig_id,
            anim_path: self.anim,
            unit: unit.map(Arc::new),
            anim: anim.map(Arc::new),
            stamps,
        }
    }
}

#[derive(Clone)]
pub struct PreloadResult {
    rig_id: String,
    anim_path: Option<PathBuf>,
    unit: Option<Arc<Rig>>,
    anim: Option<Arc<Animation>>,
    stamps: Box<Stamps>,
}

impl std::fmt::Debug for PreloadResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreloadResult")
            .field("rig_id", &self.rig_id)
            .field("anim", &self.anim_path)
            .field("loaded", &self.unit.is_some())
            .finish()
    }
}

const CACHE_CAP: usize = 12;

#[derive(Clone)]
struct Stamps {
    rig: RigStamp,
    anim: Option<Stamp>,
}

#[derive(Clone, Default, PartialEq, Eq)]
struct RigStamp {
    png: Option<Stamp>,
    cut: Option<Stamp>,
    model: Option<Stamp>,
}

impl RigStamp {
    fn of(rig: &Rigging) -> Self {
        Self {
            png: preview::stamp(&rig.png),
            cut: preview::stamp(&rig.cut),
            model: preview::stamp(&rig.model),
        }
    }

    fn whole(&self) -> bool {
        self.png.is_some() && self.cut.is_some() && self.model.is_some()
    }
}

#[derive(Default)]
struct RigCache {
    slots: Vec<CacheSlot>,
}

struct CacheSlot {
    id: String,
    unit: Arc<Rig>,
    stamp: RigStamp,
    anims: Vec<(PathBuf, Stamp, Arc<Animation>)>,
}

impl RigCache {
    fn lookup(&mut self, id: &str, stamp: &RigStamp) -> Option<Arc<Rig>> {
        if !stamp.whole() {
            return None;
        }

        let pos = self.slots.iter().position(|slot| slot.id == id && slot.stamp == *stamp)?;
        let slot = self.slots.remove(pos);
        let unit = slot.unit.clone();
        self.slots.push(slot);
        Some(unit)
    }

    fn anim(&self, id: &str, path: &Path) -> Option<Arc<Animation>> {
        let slot = self.slots.iter().find(|slot| slot.id == id)?;
        let stamp = preview::stamp(path)?;

        slot.anims
            .iter()
            .find(|(known, held, _)| known == path && *held == stamp)
            .map(|(_, _, anim)| anim.clone())
    }

    fn insert(&mut self, id: &str, unit: Arc<Rig>, stamp: RigStamp) {
        if let Some(pos) = self.slots.iter().position(|slot| slot.id == id) {
            let mut slot = self.slots.remove(pos);
            slot.unit = unit;
            slot.stamp = stamp;
            self.slots.push(slot);
            return;
        }

        if self.slots.len() >= CACHE_CAP {
            self.slots.remove(0);
        }

        self.slots.push(CacheSlot { id: id.to_string(), unit, stamp, anims: Vec::new() });
    }

    fn store_anim(&mut self, id: &str, path: &Path, stamp: Stamp, anim: Arc<Animation>) {
        let Some(slot) = self.slots.iter_mut().find(|slot| slot.id == id) else {
            return;
        };

        let seat = (path.to_path_buf(), stamp, anim);

        match slot.anims.iter_mut().find(|(known, _, _)| known == path) {
            Some(held) => *held = seat,
            None => slot.anims.push(seat),
        }
    }

    fn clear(&mut self) {
        self.slots.clear();
    }
}

#[cfg(test)]
mod tests {
    use kore::systems::animation::SLOT_MODEL;

    use super::*;

    fn rigging() -> Arc<Rigging> {
        Arc::new(Rigging {
            id: "test".to_owned(),
            png: PathBuf::from("t.png"),
            cut: PathBuf::from("t.imgcut"),
            model: PathBuf::from("t.mamodel"),
        })
    }

    fn clip(role: Option<Role>, anim: Option<&str>) -> Clip {
        Clip {
            name: None,
            slot: None,
            role,
            looping: Loop::Auto,
            rig: rigging(),
            anim: anim.map(PathBuf::from),
        }
    }

    fn seeded(key: &str, clips: Vec<Clip>) -> State {
        let mut held = State::default();

        held.sync(key, || ClipSet { name: key.to_owned(), clips, offsets: Vec::new() });

        held
    }

    // Showcase composes the four standard role clips. A Studio set in its own workspace and
    // every Utilities rig carry no roles at all, so the export rendered the resting pose for
    // every frame and read as "the animation never loaded".
    #[test]
    fn a_rig_with_no_roles_cannot_be_showcased() {
        let unit = seeded("unit", vec![
            clip(Some(Role::Walk), Some("000_f00.maanim")),
            clip(Some(Role::Idle), Some("000_f01.maanim")),
        ]);

        assert!(unit.showcasable(), "a rig whose slots resolved is a unit we can compose");

        let loose = seeded("loose", vec![clip(None, Some("something.maanim")), clip(None, None)]);

        assert!(!loose.showcasable(), "no role resolved means we do not know what any clip is");

        let modelled = seeded("modelled", vec![clip(Some(Role::Walk), None)]);

        assert!(!modelled.showcasable(), "a role with no file on disk cannot be loaded either");
    }

    #[test]
    fn a_label_no_row_answers_to_leaves_the_choice_alone() {
        // A rig with no alignment block offers only the bare entry, so a label carried
        // over from another rig matches nothing. Clearing the row on that persists "no
        // placement", which is a view the engine never draws.
        let mut held = State::default();

        held.select_offset("Combat");

        assert_eq!(held.selected_offset(), Placement::Row(0));
    }

    #[test]
    fn the_bare_entry_still_clears_it() {
        let mut held = State::default();

        assert_eq!(held.selected_offset(), Placement::Row(0), "the row the engine always applies");

        held.select_offset(NO_OFFSET);

        assert_eq!(held.selected_offset(), Placement::Bare);
    }

    fn at(slot: usize) -> Request {
        Request { slot: Some(slot), trailing: false }
    }

    fn free() -> Request {
        Request { slot: None, trailing: false }
    }

    fn model() -> Request {
        Request { slot: Some(SLOT_MODEL), trailing: true }
    }

    #[test]
    fn requested_slots_win_before_walkers() {
        // walk/idle/attack/kb ask for 0-3; burrow/surface/spirit have no request and walk from 0.
        let slots = place(&[at(0), at(1), at(2), at(3), model(), free(), free(), free()]);

        assert_eq!(slots, vec![Some(0), Some(1), Some(2), Some(3), Some(5), Some(6), Some(7), Some(4)]);
    }

    #[test]
    fn collisions_walk_forward_in_numerical_order() {
        // Two clips want slot 2; the later one settles on 3, pushing the slot-3 request to 4.
        let slots = place(&[at(3), at(2), at(2)]);

        assert_eq!(slots[2..5], [Some(1), Some(2), Some(0)]);
    }

    #[test]
    fn a_request_past_the_default_grid_grows_it() {
        let slots = place(&[at(0), at(1), at(2), at(3), model(), at(9)]);

        assert_eq!(slots.len(), 12);
        assert_eq!(slots[9], Some(5));
        assert_eq!(slots[8], None);
    }

    #[test]
    fn the_model_settles_last_and_keeps_the_trailing_column() {
        // Eight walkers fill 0-7, so the model steps a whole row rather than landing at 8.
        let mut requests: Vec<Request> = std::iter::repeat_n(free(), 8).collect();
        requests.push(model());

        let slots = place(&requests);

        assert_eq!(slots.len(), 12);
        assert_eq!(slots[11], Some(8));
        assert_eq!(slots[8..11], [None, None, None]);
    }

    #[test]
    fn overflow_grows_the_grid_in_whole_rows() {
        let requests: Vec<Request> = std::iter::repeat_n(free(), 9).chain([model()]).collect();
        let slots = place(&requests);

        assert_eq!(slots.len(), 12);
        assert_eq!(slots[8], Some(8));
        assert_eq!(slots[11], Some(9));
        assert_eq!(slots[9..11], [None, None]);
    }
}
