mod aim;
mod blank;

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use nyanko::graphics::tools::crash::Side;
use tracing::{debug, info, warn};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub use aim::{resolve, Aim, Roster};
pub use crate::domains::mods::patch_root;

use crate::common::architecture::{GAME, MODS, STUDIO};
use crate::domains::settings::FrameCount;
use crate::systems::animation::{self, Clip, ClipSet, Rigging};

pub use blank::SEED_SUFFIX;

const EXPORT_DIR: &str = "exports";
const SHEET_EXT: &str = "png";
const CUTS_EXT: &str = "imgcut";
const MODEL_EXT: &str = "mamodel";
const ANIM_EXT: &str = "maanim";
pub const DEFAULT_NAME: &str = "New Set";
pub const SEED_NAME: &str = "New";
const ATTACK_SLOT: &str = "02";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Sheet,
    Cuts,
    Model,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Sheet, Slot::Cuts, Slot::Model];

    pub fn label(self) -> &'static str {
        match self {
            Slot::Sheet => "PNG",
            Slot::Cuts => "IMGCUT",
            Slot::Model => "MAMODEL",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Slot::Sheet => SHEET_EXT,
            Slot::Cuts => CUTS_EXT,
            Slot::Model => MODEL_EXT,
        }
    }

    pub fn filter(self) -> &'static [&'static str] {
        match self {
            Slot::Sheet => &[SHEET_EXT],
            Slot::Cuts => &[CUTS_EXT],
            Slot::Model => &[MODEL_EXT],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Home {
    Game,
    Mod,
    Studio,
    Loose,
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Set {
    pub name: String,
    pub sheet: Option<PathBuf>,
    pub cuts: Option<PathBuf>,
    pub model: Option<PathBuf>,
    pub anims: Vec<PathBuf>,
}

impl Set {
    pub fn slot(&self, slot: Slot) -> Option<&Path> {
        match slot {
            Slot::Sheet => self.sheet.as_deref(),
            Slot::Cuts => self.cuts.as_deref(),
            Slot::Model => self.model.as_deref(),
        }
    }

    pub fn place(&mut self, slot: Slot, path: Option<PathBuf>) {
        match slot {
            Slot::Sheet => self.sheet = path,
            Slot::Cuts => self.cuts = path,
            Slot::Model => self.model = path,
        }
    }

    pub fn rigged(&self) -> bool {
        self.sheet.is_some() && self.cuts.is_some() && self.model.is_some()
    }

    pub fn files(&self) -> Vec<PathBuf> {
        Slot::ALL
            .into_iter()
            .filter_map(|slot| self.slot(slot).map(Path::to_path_buf))
            .chain(self.anims.iter().cloned())
            .collect()
    }

    pub fn key(&self, frames: FrameCount) -> String {
        let mut key = format!("{frames:?}|{}", self.name);

        for path in self.files() {
            key.push('|');
            key.push_str(&path.to_string_lossy());
        }

        key
    }

    pub fn rig_id(&self) -> String {
        let (Some(sheet), Some(cuts), Some(model)) = (&self.sheet, &self.cuts, &self.model) else {
            return String::new();
        };

        format!("{}|{}|{}", sheet.display(), cuts.display(), model.display())
    }

    pub fn home(&self) -> Home {
        self.files().first().map_or(Home::Loose, |path| home(path))
    }

    /// The unit stem the set's files are filed under, where the mount makes the name the address.
    pub fn addressed(&self) -> Option<String> {
        if !matches!(self.home(), Home::Game | Home::Mod) {
            return None;
        }

        let stem = self.model.as_deref()?.file_stem()?.to_str()?;

        stem_id(stem).map(|_| stem.to_owned())
    }

    pub fn clips(&self, frames: FrameCount) -> ClipSet {
        let (Some(sheet), Some(cuts), Some(model)) = (&self.sheet, &self.cuts, &self.model) else {
            return ClipSet::default();
        };

        let rig = Arc::new(Rigging {
            id: self.rig_id(),
            png: sheet.clone(),
            cut: cuts.clone(),
            model: model.clone(),
        });

        let unit = self.addressed();

        let mut clips: Vec<Clip> = self
            .anims
            .iter()
            .map(|anim| {
                let named = unit
                    .as_deref()
                    .and_then(|unit| slot_index(anim, unit))
                    .and_then(animation::standard);

                Clip {
                    name: named.map(|(name, _, _)| name.to_owned()),
                    slot: named.map(|(_, slot, _)| slot),
                    role: named.map(|(_, _, role)| role),
                    looping: frames.looping(),
                    rig: Arc::clone(&rig),
                    anim: Some(anim.clone()),
                }
            })
            .collect();

        clips.push(Clip::model(rig));

        ClipSet { name: self.name.clone(), clips, offsets: Vec::new() }
    }
}

pub fn root() -> PathBuf {
    PathBuf::from(STUDIO)
}

pub fn sets() -> Vec<String> {
    let Ok(entries) = fs::read_dir(root()) else {
        return Vec::new();
    };

    let mut listed: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .collect();

    listed.sort_by_key(|name| name.to_lowercase());
    listed
}

pub fn home(path: &Path) -> Home {
    let anchored = env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf());

    if anchored.starts_with(GAME) {
        return Home::Game;
    }

    if anchored.starts_with(MODS) {
        return Home::Mod;
    }

    if anchored.starts_with(STUDIO) {
        return Home::Studio;
    }

    Home::Loose
}

pub fn folder_name(set: &Set) -> Option<String> {
    let path = set.files().into_iter().find(|path| home(path) == Home::Studio)?;

    path.parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(str::to_owned)
}

pub fn pullable(set: &Set, unlocked: bool) -> bool {
    set.files().iter().any(|path| match home(path) {
        Home::Studio | Home::Mod => false,
        Home::Game => !unlocked,
        Home::Loose => true,
    })
}

pub fn load(name: &str) -> Set {
    let folder = root().join(name);
    let mut set = Set { name: name.to_owned(), ..Set::default() };

    let Ok(entries) = fs::read_dir(&folder) else {
        warn!(name, "Studio could not read the set folder");

        return set;
    };

    let mut found: Vec<PathBuf> =
        entries.flatten().map(|entry| entry.path()).filter(|path| path.is_file()).collect();

    found.sort();

    for path in found {
        match extension(&path).as_deref() {
            Some(SHEET_EXT) if set.sheet.is_none() => set.sheet = Some(path),
            Some(CUTS_EXT) if set.cuts.is_none() => set.cuts = Some(path),
            Some(MODEL_EXT) if set.model.is_none() => set.model = Some(path),
            Some(ANIM_EXT) => set.anims.push(path),
            _ => {}
        }
    }

    set
}

pub fn siblings(picked: &Path) -> Set {
    let stem = picked.with_extension("");
    let name = stem.file_name().and_then(OsStr::to_str).unwrap_or(DEFAULT_NAME).to_owned();

    let held = |ext: &str| {
        let path = stem.with_extension(ext);

        path.is_file().then_some(path)
    };

    let mut set = Set {
        name,
        sheet: held(SHEET_EXT),
        cuts: held(CUTS_EXT),
        model: held(MODEL_EXT),
        anims: Vec::new(),
    };

    if let Some(folder) = picked.parent() {
        set.anims = kin(folder, &stem);
    }

    set
}

fn kin(folder: &Path, stem: &Path) -> Vec<PathBuf> {
    let Some(base) = stem.file_name().and_then(OsStr::to_str) else {
        return Vec::new();
    };

    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };

    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| extension(path).as_deref() == Some(ANIM_EXT))
        .filter(|path| {
            path.file_stem().and_then(OsStr::to_str).is_some_and(|name| name.starts_with(base))
        })
        .collect();

    found.sort();
    found
}

pub fn vacant(wanted: &str) -> String {
    let base = sanitize(wanted);

    if !root().join(&base).exists() {
        return base;
    }

    (1..)
        .map(|at| format!("{}{}", base, at))
        .find(|name| !root().join(name).exists())
        .unwrap_or(base)
}

pub fn sanitize(wanted: &str) -> String {
    let cleaned: String = wanted
        .chars()
        .filter(|glyph| !matches!(glyph, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();

    match cleaned.trim() {
        "" => DEFAULT_NAME.to_owned(),
        trimmed => trimmed.to_owned(),
    }
}

pub fn adopt(name: &str, set: &Set) -> io::Result<Set> {
    let folder = root().join(name);

    fs::create_dir_all(&folder)?;

    let seat = |source: &Path| -> io::Result<PathBuf> {
        let file = source.file_name().unwrap_or_else(|| OsStr::new("asset"));
        let destination = folder.join(file);

        if source != destination {
            fs::copy(source, &destination)?;
        }

        Ok(destination)
    };

    let mut adopted = Set { name: name.to_owned(), ..Set::default() };

    for slot in Slot::ALL {
        if let Some(source) = set.slot(slot) {
            adopted.place(slot, Some(seat(source)?));
        }
    }

    for anim in &set.anims {
        adopted.anims.push(seat(anim)?);
    }

    info!(name, files = adopted.files().len(), "Studio adopted a set");

    Ok(adopted)
}

pub fn discard(name: &str) -> io::Result<()> {
    let held = Path::new(name);
    let ordinary = name == sanitize(name)
        && held.components().count() == 1
        && matches!(held.components().next(), Some(Component::Normal(_)));

    if !ordinary {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "that is not a set folder name"));
    }

    let folder = root().join(held);

    if !folder.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no set folder of that name"));
    }

    fs::remove_dir_all(&folder)?;
    info!(name, "Studio discarded a set");

    Ok(())
}

pub fn rename(from: &str, to: &str) -> io::Result<()> {
    let (source, destination) = (root().join(from), root().join(to));

    if source == destination {
        return Ok(());
    }

    fs::rename(&source, &destination)?;
    debug!(from, to, "Studio renamed a set folder");

    Ok(())
}

pub fn seed(name: &str) -> io::Result<Set> {
    let folder = root().join(name);

    fs::create_dir_all(&folder)?;

    let seated = |stem: &str, extension: &str| folder.join(format!("{}.{}", stem, extension));

    let sheet = seated(name, SHEET_EXT);
    let cuts = seated(name, CUTS_EXT);
    let model = seated(name, MODEL_EXT);
    let anim = seated(&format!("{}{}", name, blank::SEED_SUFFIX), ANIM_EXT);

    fs::write(&sheet, blank::sheet())?;
    fs::write(&cuts, blank::cuts(name))?;
    fs::write(&model, blank::model())?;
    fs::write(&anim, blank::track())?;

    info!(name, "Studio seeded a new set");

    Ok(Set { name: name.to_owned(), sheet: Some(sheet), cuts: Some(cuts), model: Some(model), anims: vec![anim] })
}

pub fn seed_track(set: &Set) -> io::Result<PathBuf> {
    let Some(model) = set.model.as_deref() else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "the set holds no model"));
    };

    let folder = model.parent().unwrap_or_else(|| Path::new(""));
    let base = stem_of(model);

    let path = (0..)
        .map(|at| folder.join(format!("{}{:02}.{}", base, at, ANIM_EXT)))
        .find(|path| !path.exists())
        .unwrap_or_else(|| folder.join(format!("{}.{}", base, ANIM_EXT)));

    fs::write(&path, blank::track())?;
    info!(path = %path.display(), "Studio seeded a new track");

    Ok(path)
}

pub fn rename_track(track: &Path, wanted: &str) -> io::Result<PathBuf> {
    let folder = track.parent().unwrap_or_else(|| Path::new(""));
    let destination = folder.join(format!("{}.{}", sanitize(wanted), ANIM_EXT));

    if destination == track {
        return Ok(destination);
    }

    if destination.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "a track of that name is already here"));
    }

    fs::rename(track, &destination)?;
    debug!(from = %track.display(), to = %destination.display(), "Studio renamed a track");

    Ok(destination)
}

pub fn export(set: &Set, named: Option<&str>) -> io::Result<PathBuf> {
    let files = set.files();

    if files.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "the set holds no files"));
    }

    let folder = PathBuf::from(EXPORT_DIR);

    fs::create_dir_all(&folder)?;

    let stem = match named.map(str::trim).filter(|held| !held.is_empty()) {
        Some(held) => sanitize(held),
        None => match set.name.trim() {
            "" => set.model.as_deref().map_or_else(|| DEFAULT_NAME.to_owned(), stem_of),
            held => sanitize(held),
        },
    };

    let mut path = folder.join(format!("{}.zip", stem));
    let mut counter = 1;

    while path.exists() {
        path = folder.join(format!("{}{}.zip", stem, counter));
        counter += 1;
    }

    let mut zip = ZipWriter::new(File::create(&path)?);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for file in &files {
        let Some(name) = file.file_name().and_then(OsStr::to_str) else {
            continue;
        };

        let body = fs::read(file)?;

        zip.start_file(name, options)?;
        io::Write::write_all(&mut zip, &body)?;
    }

    zip.finish()?;
    info!(path = %path.display(), files = files.len(), "Studio exported a set");

    Ok(path)
}

pub fn stem_id(stem: &str) -> Option<i32> {
    let (head, tail) = stem.split_once('_')?;
    let known = aim::FORMS.iter().any(|form| tail.starts_with(*form)) || tail.starts_with(aim::ENEMY_SUFFIX);

    (known && tail.len() == 1).then(|| head.parse::<i32>().ok())?
}

fn slot_index(anim: &Path, unit: &str) -> Option<usize> {
    anim.file_stem()?.to_str()?.strip_prefix(unit)?.parse().ok()
}

pub fn stem_side(stem: &str) -> Option<Side> {
    let (_, tail) = stem.split_once('_')?;

    if tail.len() != 1 {
        return None;
    }

    if tail.starts_with(aim::ENEMY_SUFFIX) {
        return Some(Side::Enemy);
    }

    aim::FORMS.iter().any(|form| tail.starts_with(*form)).then_some(Side::Cat)
}

pub fn attack_slot(anim: &Path) -> bool {
    anim.file_stem()
        .and_then(OsStr::to_str)
        .and_then(|stem| stem.strip_suffix(ATTACK_SLOT))
        .is_some_and(|unit| stem_id(unit).is_some())
}

pub fn occupied(set: &Set, target: &Aim, root: &Path) -> Vec<String> {
    let (Some(stem), true) = (target.stem(), set.model.is_some()) else {
        return Vec::new();
    };

    aim::plan(set, &stem)
        .seats
        .into_iter()
        .map(|(_, seat)| seat)
        .filter(|seat| root.join(seat).exists())
        .collect()
}

pub struct Landed {
    pub model: PathBuf,
    pub stray: usize,
}

pub fn install(set: &Set, target: &Aim, root: &Path) -> io::Result<Landed> {
    let (Some(stem), Some(unit)) = (target.stem(), target.unit()) else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "the target names no entity"));
    };

    if set.model.is_none() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "the set holds no model"));
    }

    let plan = aim::plan(set, &stem);

    fs::create_dir_all(root)?;

    for (source, seat) in &plan.seats {
        let landed = root.join(seat);
        let body = fs::read(source)?;

        let body = match seat.ends_with(MODEL_EXT) {
            true => aim::restamped(&body, unit).unwrap_or(body),
            false => body,
        };

        let body = match seat.ends_with(CUTS_EXT) {
            true => aim::repointed(&body, &stem),
            false => body,
        };

        fs::write(&landed, body)?;
    }

    if plan.stray > 0 {
        warn!(root = %root.display(), stem, stray = plan.stray, "Studio could not map every track onto the entity");
    }

    info!(root = %root.display(), stem, unit, files = plan.seats.len(), "Studio installed a set onto an entity");

    Ok(Landed { model: root.join(format!("{stem}.{MODEL_EXT}")), stray: plan.stray })
}

fn extension(path: &Path) -> Option<String> {
    path.extension().and_then(OsStr::to_str).map(str::to_lowercase)
}

fn stem_of(path: &Path) -> String {
    path.file_stem().map_or_else(|| DEFAULT_NAME.to_owned(), |stem| stem.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use crate::systems::animation::authoring::Maanim;

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = env::temp_dir().join(format!("bcc-studio-{}-{}", name, std::process::id()));

        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the scratch folder is made");

        root
    }

    #[test]
    fn a_seeded_track_takes_the_next_free_ordinal_and_parses_back() {
        let root = scratch("seed-track");
        let model = root.join("Rig.mamodel");

        fs::write(&model, blank::model()).expect("the model is written");
        fs::write(root.join("Rig00.maanim"), blank::track()).expect("the first track is written");

        let set = Set { model: Some(model), ..Set::default() };
        let seeded = seed_track(&set).expect("a track is seeded");

        assert_eq!(seeded.file_name().and_then(OsStr::to_str), Some("Rig01.maanim"));
        assert!(Maanim::parse(&fs::read(&seeded).expect("the seed reads")).is_ok());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn renaming_a_track_keeps_the_extension_and_refuses_a_collision() {
        let root = scratch("rename-track");
        let held = root.join("Rig00.maanim");
        let taken = root.join("Taken.maanim");

        fs::write(&held, blank::track()).expect("the track is written");
        fs::write(&taken, blank::track()).expect("the rival is written");

        assert!(rename_track(&held, "Taken").is_err(), "an occupied name is refused");
        assert!(held.is_file(), "and the original is left alone");

        let moved = rename_track(&held, "Walk Cycle").expect("the track is renamed");

        assert_eq!(moved, root.join("Walk Cycle.maanim"));
        assert!(!held.exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discarding_refuses_anything_that_is_not_one_folder_under_studio() {
        // remove_dir_all is not something to hand a traversal.
        for name in ["..", ".", "", "../..", "a/b"] {
            let err = discard(name).expect_err("the name is refused");

            assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "{name} slipped through");
        }

        assert_eq!(
            discard("a set that is not there").map_err(|err| err.kind()),
            Err(io::ErrorKind::NotFound)
        );
    }

    #[test]
    fn a_name_loses_path_separators_but_keeps_the_rest() {
        assert_eq!(sanitize("my/set"), "myset");
        assert_eq!(sanitize("  spaced  "), "spaced");
        assert_eq!(sanitize("   "), DEFAULT_NAME);
    }

    #[test]
    fn a_set_lists_every_anim_by_file_name_and_claims_no_roles() {
        // Studio is an editor, not an entity browser. A file called 044_f02.maanim is not
        // promised to be an attack, so nothing here guesses one.
        let set = Set {
            name: "044_f".to_owned(),
            sheet: Some(PathBuf::from("studio/a/044_f.png")),
            cuts: Some(PathBuf::from("studio/a/044_f.imgcut")),
            model: Some(PathBuf::from("studio/a/044_f.mamodel")),
            anims: vec![
                PathBuf::from("studio/a/044_f00.maanim"),
                PathBuf::from("studio/a/044_f02.maanim"),
                PathBuf::from("studio/a/044_f_zombie00.maanim"),
            ],
        };

        let clips = set.clips(FrameCount::Automatic).clips;
        let labelled: Vec<String> = clips.iter().map(Clip::label).collect();

        assert_eq!(labelled, vec!["044_f00", "044_f02", "044_f_zombie00", "Model"]);
        assert!(clips.iter().all(|clip| clip.role.is_none()), "no clip claims a role");

        // Slots are what pinned a clip to a fixed grid cell; without them the buttons
        // follow the file order exactly.
        assert!(clips.iter().filter(|clip| clip.anim.is_some()).all(|clip| clip.slot.is_none()));
    }

    #[test]
    fn a_set_is_only_rigged_once_all_three_assets_are_there() {
        let mut set = Set { sheet: Some(PathBuf::from("a.png")), ..Set::default() };

        assert!(!set.rigged());

        set.cuts = Some(PathBuf::from("a.imgcut"));
        assert!(!set.rigged());

        set.model = Some(PathBuf::from("a.mamodel"));
        assert!(set.rigged());
    }

    #[test]
    fn every_mount_is_told_apart() {
        assert_eq!(home(Path::new("game/cats/044/f/anim/044_f.mamodel")), Home::Game);
        assert_eq!(home(Path::new("mods/MyMod/044_f.mamodel")), Home::Mod);
        assert_eq!(home(Path::new("studio/Test/044_f.mamodel")), Home::Studio);
        assert_eq!(home(Path::new("/tmp/somewhere/044_f.mamodel")), Home::Loose);
    }

    #[test]
    fn only_an_unwritable_set_has_to_be_pulled_into_studio() {
        // A mod is writable by definition, `studio/` already is, and `game` is only
        // when the mount is unlocked.
        let seat = |path: &str| Set {
            sheet: Some(PathBuf::from(path)),
            cuts: Some(PathBuf::from(path)),
            model: Some(PathBuf::from(path)),
            ..Set::default()
        };

        assert!(!pullable(&seat("studio/a/x.png"), false));
        assert!(!pullable(&seat("mods/MyMod/x.png"), false));
        assert!(pullable(&seat("game/x.png"), false));
        assert!(!pullable(&seat("game/x.png"), true));
        assert!(pullable(&seat("/tmp/x.png"), true));
    }

    #[test]
    fn only_a_units_third_slot_counts_as_the_attack_animation() {
        // The engine reaches the unguarded modulo through an entity's attacking state,
        // so the slot only exists for a unit. bgEffect_057_02 ships with a zero length
        // and never attacks anything; reading it as slot two flags a vanilla file.
        assert!(attack_slot(Path::new("game/cats/000/f/anim/000_f02.maanim")));
        assert!(attack_slot(Path::new("game/enemies/044/anim/044_e02.maanim")));

        assert!(!attack_slot(Path::new("game/stages/backgrounds/effects/057/bgEffect_057_02.maanim")));
        assert!(!attack_slot(Path::new("game/cats/000/f/anim/000_f01.maanim")));
        assert!(!attack_slot(Path::new("game/raw/rain.maanim")));
    }
}
