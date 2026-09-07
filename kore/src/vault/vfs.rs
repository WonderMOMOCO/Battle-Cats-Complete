//! Virtual File System
mod disk;
mod regional;
mod walk;

use std::env;
use std::ffi::OsStr;
use std::hash::{Hash, Hasher};
use std::fs;
use std::path::{Path, PathBuf};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use rustc_hash::{FxHashMap, FxHasher};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::domains::settings::{Settings, lang};

const MOUNT_GAME: &str = "game";

type MountKey = Box<str>;
type Index = FxHashMap<MountKey, MountedDir>;
type Sorted = Option<(u64, Arc<[MountKey]>)>;
type Printed = Option<(u64, u64)>;

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    path: PathBuf,
    mtime: u64,
    len: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct MountedDir {
    root: PathBuf,
    files: FxHashMap<MountKey, Entry>,
    dirs: FxHashMap<Box<str>, Vec<Box<str>>>,
    folders: FxHashMap<Box<str>, Vec<Box<str>>>,
    conflicts: Vec<Conflict>,
}

impl MountedDir {
    fn prune(&mut self, relative: &Path) -> Vec<MountKey> {
        let mut removed = Vec::new();

        if let Some(name) = relative.file_name().and_then(OsStr::to_str)
            && self.files.get(name).is_some_and(|entry| entry.path == relative)
        {
            self.files.remove(name);
            self.unlink(relative, name, true);

            return vec![name.into()];
        }

        let targets: Vec<MountKey> = self
            .dirs
            .keys()
            .filter(|dir| Path::new(dir.as_ref()).starts_with(relative))
            .cloned()
            .collect();

        for dir in targets {
            self.folders.remove(&dir);

            let Some(names) = self.dirs.remove(&dir) else {
                continue;
            };

            for name in names {
                if self.files.get(name.as_ref()).is_some_and(|entry| entry.path.starts_with(relative)) {
                    self.files.remove(name.as_ref());
                    removed.push(name);
                }
            }
        }

        if let Some(name) = relative.file_name().and_then(OsStr::to_str) {
            self.unlink(relative, name, false);
        }

        removed
    }

    fn unlink(&mut self, relative: &Path, name: &str, is_file: bool) {
        let Some(parent) = relative.parent() else {
            return;
        };

        let listing = if is_file {
            self.dirs.get_mut(parent.to_string_lossy().as_ref())
        } else {
            self.folders.get_mut(parent.to_string_lossy().as_ref())
        };

        if let Some(listing) = listing {
            listing.retain(|existing| existing.as_ref() != name);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub key: Box<str>,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Default)]
pub struct Listing {
    pub folders: Vec<Box<str>>,
    pub files: Vec<Box<str>>,
}

#[derive(Debug, thiserror::Error)]
pub enum VfsError {
    #[error("mount target is not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("path has no usable file name: {0}")]
    InvalidPath(PathBuf),
    #[error("no mount registered under key '{0}'")]
    UnknownMount(Box<str>),
    #[error("{path} does not sit under the '{root}' mount")]
    Unrooted { root: PathBuf, path: PathBuf },
    #[error("directory claims the reserved '{MOUNT_GAME}' mount key: {0}")]
    ReservedMount(PathBuf),
    #[error("failed to read {path}: {source}")]
    Walk { path: PathBuf, source: std::io::Error },
    #[error("virtual file system state is unavailable")]
    Unavailable,
}

pub trait Target {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R;
}

impl Target for &str {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R {
        run(&[self])
    }
}

impl Target for &String {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R {
        run(&[self.as_str()])
    }
}

impl<S: AsRef<str>> Target for &[S] {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R {
        run(&self.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
    }
}

impl<S: AsRef<str>, const N: usize> Target for &[S; N] {
    fn resolve<R>(self, run: impl FnOnce(&[&str]) -> R) -> R {
        run(&self.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
    }
}

pub trait Mount {
    fn mount(self, vfs: &Vfs) -> Result<Vec<Conflict>, VfsError>;
    fn unmount(self, vfs: &Vfs);
}

pub struct Vfs {
    mounts: RwLock<Index>,
    cache: RwLock<FxHashMap<MountKey, Arc<[u8]>>>,
    priority: RwLock<Vec<String>>,
    generation: AtomicU64,
    sorted: RwLock<Sorted>,
    printed: RwLock<Printed>,
}

struct Mutation<'a> {
    mounts: RwLockWriteGuard<'a, Index>,
    generation: &'a AtomicU64,
}

impl Deref for Mutation<'_> {
    type Target = Index;

    fn deref(&self) -> &Index {
        &self.mounts
    }
}

impl DerefMut for Mutation<'_> {
    fn deref_mut(&mut self) -> &mut Index {
        &mut self.mounts
    }
}

impl Drop for Mutation<'_> {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }
}

impl Vfs {
    pub fn new(settings: &Settings) -> Self {
        Self::with_priority(&settings.general.language_priority)
    }

    pub fn with_priority(order: &[String]) -> Self {
        let mut complete = order.to_vec();
        lang::ensure_complete_list(&mut complete);

        Self {
            mounts: RwLock::new(Index::default()),
            cache: RwLock::new(FxHashMap::default()),
            priority: RwLock::new(complete),
            generation: AtomicU64::new(0),
            sorted: RwLock::new(None),
            printed: RwLock::new(None),
        }
    }

    fn mutate(&self) -> Option<Mutation<'_>> {
        self.mounts
            .write()
            .ok()
            .map(|mounts| Mutation { mounts, generation: &self.generation })
    }

    pub(crate) fn detached() -> Self {
        Self::with_priority(&[])
    }

    pub fn priority(&self, order: &[String]) {
        let mut complete = order.to_vec();
        lang::ensure_complete_list(&mut complete);

        if let Ok(mut current) = self.priority.write() {
            *current = complete;
        }

        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    pub fn create<M: Mount>(&self, target: M) -> Result<Vec<Conflict>, VfsError> {
        target.mount(self)
    }

    pub fn destroy<M: Mount>(&self, target: M) {
        target.unmount(self);
    }

    pub fn find<T: Target>(&self, target: T) -> Option<PathBuf> {
        target.resolve(|names| self.first(names))
    }

    pub fn list<T: Target>(&self, target: T) -> Vec<PathBuf> {
        target.resolve(|names| self.collect(names))
    }

    pub fn locate(&self, filename: &str) -> Option<PathBuf> {
        let mounts = self.mounts.read().ok()?;

        resolve(&mounts, filename)
    }

    pub fn pristine<T: Target>(&self, target: T) -> Option<PathBuf> {
        target.resolve(|names| self.untouched(names))
    }

    pub fn originals<T: Target>(&self, target: T) -> Vec<PathBuf> {
        target.resolve(|names| self.gathered(names))
    }

    fn gathered(&self, filenames: &[&str]) -> Vec<PathBuf> {
        let Ok(order) = self.priority.read() else {
            return Vec::new();
        };

        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut found: Vec<PathBuf> =
            regional::interleaved(filenames, &order).filter_map(|candidate| vanilla(&mounts, &candidate)).collect();

        found.dedup();
        found
    }

    fn untouched(&self, filenames: &[&str]) -> Option<PathBuf> {
        let Ok(order) = self.priority.read() else {
            return None;
        };

        let Ok(mounts) = self.mounts.read() else {
            return None;
        };

        regional::interleaved(filenames, &order).find_map(|candidate| vanilla(&mounts, &candidate))
    }

    fn first(&self, filenames: &[&str]) -> Option<PathBuf> {
        let Ok(order) = self.priority.read() else {
            return None;
        };

        let Ok(mounts) = self.mounts.read() else {
            return None;
        };

        regional::interleaved(filenames, &order)
            .find_map(|candidate| modded(&mounts, &candidate))
            .or_else(|| regional::interleaved(filenames, &order).find_map(|candidate| vanilla(&mounts, &candidate)))
    }

    fn collect(&self, filenames: &[&str]) -> Vec<PathBuf> {
        let Ok(order) = self.priority.read() else {
            return Vec::new();
        };

        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut overrides = Vec::new();
        let mut originals = Vec::new();

        for candidate in regional::interleaved(filenames, &order) {
            if let Some(path) = modded(&mounts, &candidate) {
                overrides.push(path);
                continue;
            }

            if let Some(path) = vanilla(&mounts, &candidate) {
                originals.push(path);
            }
        }

        overrides.append(&mut originals);
        overrides.dedup();
        overrides
    }

    pub fn load(&self, filename: &str) -> Option<Arc<[u8]>> {
        if let Ok(cache) = self.cache.read()
            && let Some(bytes) = cache.get(filename)
        {
            return Some(Arc::clone(bytes));
        }

        let path = self.find(filename)?;
        let raw = fs::read(&path)
            .inspect_err(|err| warn!(file = filename, path = %path.display(), "vfs read failed: {}", err))
            .ok()?;

        let bytes = Arc::<[u8]>::from(raw);

        if let Ok(mut cache) = self.cache.write() {
            cache.insert(filename.into(), Arc::clone(&bytes));
        }

        Some(bytes)
    }

    pub fn refresh(&self, filename: &str) -> Option<Arc<[u8]>> {
        self.evict(filename);
        self.load(filename)
    }

    pub fn stripped(&self, filename: &str) -> Option<String> {
        let order = self.priority.read().ok()?;

        regional::stripped(filename, &order)
    }

    pub fn evict(&self, filename: &str) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(filename);
        }
    }

    pub fn purge(&self, filenames: &[Box<str>]) {
        if let Ok(mut cache) = self.cache.write() {
            for filename in filenames {
                cache.remove(filename.as_ref());
            }
        }
    }

    pub fn prune(&self, mount: &str, path: &Path) -> Vec<Box<str>> {
        let Some(mut mounts) = self.mutate() else {
            return Vec::new();
        };

        let Some(indexed) = mounts.get_mut(mount) else {
            return Vec::new();
        };

        let Some(relative) = within(&indexed.root, path) else {
            return Vec::new();
        };

        indexed.prune(&relative)
    }

    pub fn count(&self, mount: &str) -> usize {
        self.mounts
            .read()
            .map_or(0, |mounts| mounts.get(mount).map_or(0, |indexed| indexed.files.len()))
    }

    pub fn variants(&self, filename: &str) -> Vec<String> {
        let path = Path::new(filename);
        let Some(stem) = path.file_stem().and_then(OsStr::to_str) else {
            return Vec::new();
        };

        let extension = path.extension().and_then(OsStr::to_str).unwrap_or_default();
        let order = self.priority.read().map(|order| order.clone()).unwrap_or_default();

        let mut found: Vec<(usize, String)> = self
            .glob(stem)
            .into_iter()
            .filter_map(|name| {
                let candidate = Path::new(name.as_ref());
                let candidate_stem = candidate.file_stem().and_then(OsStr::to_str)?;

                if candidate.extension().and_then(OsStr::to_str).unwrap_or_default() != extension {
                    return None;
                }

                let suffix = candidate_stem.strip_prefix(stem)?;
                let code = match suffix {
                    "" => None,
                    other => Some(other.strip_prefix('_')?),
                };

                let rank = code.map_or(0, |code| {
                    order.iter().position(|entry| entry == code).map_or(usize::MAX, |index| index + 1)
                });

                Some((rank, name.to_string()))
            })
            .collect();

        found.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        found.dedup_by(|left, right| left.1 == right.1);

        found.into_iter().map(|(_, name)| name).collect()
    }

    pub fn glob(&self, prefix: &str) -> Vec<Box<str>> {
        let sorted = self.catalog();
        let start = sorted.partition_point(|name| name.as_ref() < prefix);

        sorted[start..].iter().take_while(|name| name.starts_with(prefix)).cloned().collect()
    }

    fn catalog(&self) -> Arc<[MountKey]> {
        let generation = self.generation.load(Ordering::Relaxed);

        if let Ok(cached) = self.sorted.read()
            && let Some((stamp, names)) = cached.as_ref()
            && *stamp == generation
        {
            return Arc::clone(names);
        }

        let Ok(mounts) = self.mounts.read() else {
            return Arc::from([]);
        };

        let mut names: Vec<MountKey> = mounts.values().flat_map(|indexed| indexed.files.keys()).cloned().collect();
        names.sort_unstable();
        names.dedup();

        let names: Arc<[MountKey]> = Arc::from(names);

        if let Ok(mut cached) = self.sorted.write() {
            *cached = Some((generation, Arc::clone(&names)));
        }

        names
    }

    pub fn conflicts(&self) -> Vec<Conflict> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut collisions: Vec<Conflict> = mounts.values().flat_map(|indexed| indexed.conflicts.iter().cloned()).collect();
        collisions.sort_by(|a, b| a.key.cmp(&b.key));
        collisions
    }

    pub fn mounted(&self) -> Vec<Box<str>> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut keys: Vec<Box<str>> = mounts.keys().cloned().collect();
        keys.sort_unstable_by(|a, b| {
            let rank = |key: &str| u8::from(key != MOUNT_GAME);
            rank(a).cmp(&rank(b)).then_with(|| a.cmp(b))
        });

        keys
    }

    pub fn relative(&self, mount: &str, path: &Path) -> Option<PathBuf> {
        let mounts = self.mounts.read().ok()?;
        let indexed = mounts.get(mount)?;

        within(&indexed.root, path)
    }

    pub fn fingerprint(&self) -> u64 {
        let generation = self.generation.load(Ordering::Relaxed);

        if let Ok(cached) = self.printed.read()
            && let Some((stamp, print)) = cached.as_ref()
            && *stamp == generation
        {
            return *print;
        }

        let Ok(mounts) = self.mounts.read() else {
            return 0;
        };

        let mut hasher = FxHasher::default();
        let mut keys: Vec<&MountKey> = mounts.keys().collect();
        keys.sort_unstable();

        for key in keys {
            let Some(indexed) = mounts.get(key) else { continue };

            key.hash(&mut hasher);

            let mut names: Vec<&MountKey> = indexed.files.keys().collect();
            names.sort_unstable();

            for name in &names {
                let Some(entry) = indexed.files.get(*name) else { continue };

                name.hash(&mut hasher);
                entry.path.hash(&mut hasher);
                entry.mtime.hash(&mut hasher);
                entry.len.hash(&mut hasher);
            }

            names.len().hash(&mut hasher);

            for conflict in &indexed.conflicts {
                conflict.key.hash(&mut hasher);
                conflict.paths.len().hash(&mut hasher);
            }
        }

        let print = hasher.finish();

        if let Ok(mut cached) = self.printed.write() {
            *cached = Some((generation, print));
        }

        print
    }

    pub fn indexed(&self, mount: &str, path: &Path) -> bool {
        let Ok(mounts) = self.mounts.read() else {
            return false;
        };

        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            return false;
        };

        mounts.get(mount).is_some_and(|indexed| {
            within(&indexed.root, path)
                .zip(indexed.files.get(name))
                .is_some_and(|(relative, entry)| entry.path == relative)
        })
    }

    pub fn contains(&self, mount: &str, dir: &Path, name: &str) -> bool {
        let Ok(mounts) = self.mounts.read() else {
            return false;
        };

        mounts
            .get(mount)
            .and_then(|indexed| indexed.dirs.get(dir.to_string_lossy().as_ref()))
            .is_some_and(|names| names.iter().any(|entry| entry.as_ref() == name))
    }

    pub fn any(&self, mount: &str, dir: &Path, keep: impl Fn(&str) -> bool) -> bool {
        let Ok(mounts) = self.mounts.read() else {
            return false;
        };

        mounts.get(mount).is_some_and(|indexed| descends(indexed, dir, &keep))
    }

    pub fn root(&self, mount: &str) -> Option<PathBuf> {
        let mounts = self.mounts.read().ok()?;

        mounts.get(mount).map(|indexed| indexed.root.clone())
    }

    pub fn stored(&self, mount: &str, name: &str) -> Option<PathBuf> {
        let mounts = self.mounts.read().ok()?;

        mounts.get(mount)?.files.get(name).map(|entry| entry.path.clone())
    }

    pub fn rooted(&self, mount: &str, name: &str) -> Option<PathBuf> {
        let mounts = self.mounts.read().ok()?;
        let indexed = mounts.get(mount)?;

        if let Some(entry) = indexed.files.get(name) {
            return Some(indexed.root.join(&entry.path));
        }

        indexed
            .conflicts
            .iter()
            .find(|conflict| conflict.key.as_ref() == name)
            .and_then(|conflict| conflict.paths.iter().min_by_key(|path| (path.components().count(), *path)))
            .cloned()
    }

    pub fn browse(&self, mount: &str, dir: &Path) -> Option<Listing> {
        let mounts = self.mounts.read().ok()?;
        let indexed = mounts.get(mount)?;
        let key = dir.to_string_lossy();

        let folders = indexed.folders.get(key.as_ref());
        let files = indexed.dirs.get(key.as_ref());

        if folders.is_none() && files.is_none() {
            return None;
        }

        let taken = |names: Option<&Vec<Box<str>>>| names.cloned().unwrap_or_default();

        Some(Listing { folders: taken(folders), files: taken(files) })
    }

    pub fn keys(&self, mount: &str) -> Vec<Box<str>> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        mounts
            .get(mount)
            .map_or_else(Vec::new, |indexed| indexed.files.keys().cloned().collect())
    }

    pub fn children(&self, dir: &Path) -> Vec<PathBuf> {
        self.matching(dir, "")
    }

    pub fn folders(&self, dir: &Path) -> Vec<PathBuf> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut paths = Vec::new();
        for mount in mounts.values() {
            let Ok(relative) = dir.strip_prefix(&mount.root) else {
                continue;
            };

            let key = relative.to_string_lossy();
            let Some(names) = mount.folders.get(key.as_ref()) else {
                continue;
            };

            for name in names {
                paths.push(dir.join(name.as_ref()));
            }
        }

        paths
    }

    pub fn matching(&self, dir: &Path, prefix: &str) -> Vec<PathBuf> {
        let Ok(mounts) = self.mounts.read() else {
            return Vec::new();
        };

        let mut paths = Vec::new();
        for mount in mounts.values() {
            let Ok(relative) = dir.strip_prefix(&mount.root) else {
                continue;
            };

            let key = relative.to_string_lossy();
            let Some(names) = mount.dirs.get(key.as_ref()) else {
                continue;
            };

            for name in names {
                if name.starts_with(prefix) {
                    paths.push(dir.join(name.as_ref()));
                }
            }
        }

        paths
    }

    pub fn persist(&self, hash: u64) {
        let encoded = self.mounts.read().ok().and_then(|mounts| disk::encode(&mounts, hash));

        let Some(bytes) = encoded else {
            return;
        };

        disk::store(&bytes);
    }

    pub fn hydrate(&self) -> Option<u64> {
        let (stored, index) = disk::load()?;
        let mut mounts = self.mutate()?;

        *mounts = index;
        Some(stored)
    }

}

impl Mount for &Path {
    fn mount(self, vfs: &Vfs) -> Result<Vec<Conflict>, VfsError> {
        let Some(key) = self.file_name().and_then(OsStr::to_str) else {
            return Err(VfsError::InvalidPath(self.to_path_buf()));
        };

        if key == MOUNT_GAME && !canonical(self) {
            warn!(
                path = %self.display(),
                "refusing to index a directory claiming the reserved '{}' mount key",
                MOUNT_GAME
            );
            return Err(VfsError::ReservedMount(self.to_path_buf()));
        }

        let mount = walk::walk(self)?;
        let conflicts = mount.conflicts.clone();

        let Some(mut mounts) = vfs.mutate() else {
            return Err(VfsError::Unavailable);
        };

        mounts.insert(key.into(), mount);
        Ok(conflicts)
    }

    fn unmount(self, vfs: &Vfs) {
        let Some(key) = self.file_name().and_then(OsStr::to_str) else {
            return;
        };

        if let Some(mut mounts) = vfs.mutate() {
            mounts.remove(key);
        }
    }
}

impl Mount for (&str, &Path) {
    fn mount(self, vfs: &Vfs) -> Result<Vec<Conflict>, VfsError> {
        let (key, file) = self;

        let Some(name) = file.file_name().and_then(OsStr::to_str) else {
            return Err(VfsError::InvalidPath(file.to_path_buf()));
        };

        let Some(mut mounts) = vfs.mutate() else {
            return Err(VfsError::Unavailable);
        };

        let Some(mount) = mounts.get_mut(key) else {
            return Err(VfsError::UnknownMount(key.into()));
        };

        let Some(relative) = within(&mount.root, file) else {
            return Err(VfsError::Unrooted { root: mount.root.clone(), path: file.to_path_buf() });
        };

        // A vanished file must never be indexed: it would shadow the game mount with a
        // dead path instead of letting resolution fall back to vanilla.
        let Some((mtime, len)) = walk::stat(file) else {
            return Err(VfsError::InvalidPath(file.to_path_buf()));
        };

        if let Some(parent) = relative.parent() {
            admit(mount.dirs.entry(parent.to_string_lossy().into()).or_default(), name);

            link(mount, parent);
        }

        let absolute = mount.root.join(&relative);

        if let Some(at) = mount.conflicts.iter().position(|conflict| conflict.key.as_ref() == name) {
            let paths = &mut mount.conflicts[at].paths;

            if !paths.contains(&absolute) {
                paths.push(absolute);
                paths.sort();
            }

            return Ok(vec![mount.conflicts[at].clone()]);
        }

        let shadowed = mount.files.get(name).filter(|entry| entry.path != relative).map(|entry| mount.root.join(&entry.path));

        let Some(previous) = shadowed else {
            mount.files.insert(name.into(), Entry { path: relative, mtime, len });

            return Ok(Vec::new());
        };

        // walk::merge applies, kept in step so an added file needs no remount.
        mount.files.remove(name);

        let mut paths = vec![previous, absolute];
        paths.sort();

        let conflict = Conflict { key: name.into(), paths };
        let at = mount.conflicts.partition_point(|existing| existing.key.as_ref() < name);

        mount.conflicts.insert(at, conflict.clone());

        Ok(vec![conflict])
    }

    fn unmount(self, vfs: &Vfs) {
        let (key, file) = self;

        let Some(name) = file.file_name().and_then(OsStr::to_str) else {
            return;
        };

        {
            let Some(mut mounts) = vfs.mutate() else {
                return;
            };

            let Some(mount) = mounts.get_mut(key) else {
                return;
            };

            let Some(relative) = within(&mount.root, file) else {
                return;
            };

            if let Some(parent) = relative.parent()
                && let Some(listing) = mount.dirs.get_mut(parent.to_string_lossy().as_ref())
            {
                listing.retain(|existing| existing.as_ref() != name);
            }

            let absolute = mount.root.join(&relative);

            if let Some(at) = mount.conflicts.iter().position(|conflict| conflict.key.as_ref() == name) {
                mount.conflicts[at].paths.retain(|path| path != &absolute);

                match mount.conflicts[at].paths.len() {
                    0 => drop(mount.conflicts.remove(at)),
                    // The last rival is gone, so the survivor becomes resolvable again.
                    1 => {
                        let survivor = mount.conflicts.remove(at).paths.remove(0);

                        if let Some(path) = within(&mount.root, &survivor)
                            && let Some((mtime, len)) = walk::stat(&survivor)
                        {
                            mount.files.insert(name.into(), Entry { path, mtime, len });
                        }
                    }
                    _ => {}
                }
            } else if mount.files.get(name).is_some_and(|entry| entry.path == relative) {
                mount.files.remove(name);
            }
        }

        vfs.evict(name);
    }
}

fn descends(indexed: &MountedDir, dir: &Path, keep: &impl Fn(&str) -> bool) -> bool {
    let key = dir.to_string_lossy();

    if indexed.dirs.get(key.as_ref()).is_some_and(|names| names.iter().any(|name| keep(name))) {
        return true;
    }

    indexed
        .folders
        .get(key.as_ref())
        .is_some_and(|names| names.iter().any(|name| descends(indexed, &dir.join(name.as_ref()), keep)))
}

fn admit(listing: &mut Vec<Box<str>>, name: &str) {
    if let Err(at) = listing.binary_search_by(|existing| existing.as_ref().cmp(name)) {
        listing.insert(at, name.into());
    }
}

fn link(mount: &mut MountedDir, dir: &Path) {
    let mut current = Some(dir);

    while let Some(branch) = current {
        let Some(parent) = branch.parent() else {
            return;
        };

        let Some(name) = branch.file_name().and_then(OsStr::to_str) else {
            return;
        };

        admit(mount.folders.entry(parent.to_string_lossy().into()).or_default(), name);

        current = Some(parent);
    }
}

fn within(root: &Path, file: &Path) -> Option<PathBuf> {
    if let Ok(relative) = file.strip_prefix(root) {
        return Some(relative.to_path_buf());
    }

    let absolute = env::current_dir().ok()?.join(root);

    if let Ok(relative) = file.strip_prefix(&absolute) {
        return Some(relative.to_path_buf());
    }

    file.strip_prefix(fs::canonicalize(&absolute).ok()?).ok().map(Path::to_path_buf)
}

fn resolve(mounts: &Index, name: &str) -> Option<PathBuf> {
    modded(mounts, name).or_else(|| vanilla(mounts, name))
}

fn modded(mounts: &Index, name: &str) -> Option<PathBuf> {
    mounts
        .iter()
        .filter(|(key, _)| key.as_ref() != MOUNT_GAME)
        .find_map(|(_, mount)| mount.files.get(name).map(|entry| mount.root.join(&entry.path)))
}

fn vanilla(mounts: &Index, name: &str) -> Option<PathBuf> {
    mounts
        .iter()
        .filter(|(key, _)| key.as_ref() == MOUNT_GAME)
        .find_map(|(_, mount)| mount.files.get(name).map(|entry| mount.root.join(&entry.path)))
}

fn canonical(path: &Path) -> bool {
    path.components().count() == 1 && path.file_name() == Some(OsStr::new(MOUNT_GAME))
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = env::temp_dir().join(format!("bcc-vfs-{name}-{}", std::process::id()));

            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("patch")).expect("scratch mount");

            Self(root)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_listing_comes_back_sorted_without_browse_having_to_sort_it() {
        // browse runs for every open directory on every tree rebuild, and the Files page
        // can hold 150k names, so it must not re-sort what the index already knows. The
        // index is kept in order on the way in instead; this is the invariant that buys.
        let scratch = Scratch::new("sorted");
        let root = &scratch.0;
        let patch = root.join("patch");

        fs::create_dir_all(patch.join("zeta")).expect("nested dir");
        fs::create_dir_all(patch.join("alpha")).expect("nested dir");

        for name in ["unit099.csv", "unit001.csv", "unit050.csv"] {
            fs::write(patch.join(name), "0\n").expect("seed file");
        }

        for dir in ["zeta", "alpha"] {
            fs::write(patch.join(dir).join("held.csv"), "0\n").expect("seed file");
        }

        let vfs = Vfs::with_priority(&[]);
        vfs.create(root.as_path()).expect("mount the scratch dir");

        let mount = root.file_name().and_then(OsStr::to_str).expect("mount key");
        let listing = vfs.browse(mount, Path::new("patch")).expect("listing");

        let files: Vec<&str> = listing.files.iter().map(Box::as_ref).collect();
        let folders: Vec<&str> = listing.folders.iter().map(Box::as_ref).collect();

        assert_eq!(files, ["unit001.csv", "unit050.csv", "unit099.csv"]);
        assert_eq!(folders, ["alpha", "zeta"]);
    }

    #[test]
    fn indexed_separates_a_modified_file_from_a_brand_new_one() {
        let scratch = Scratch::new("indexed");
        let root = &scratch.0;
        let known = root.join("patch").join("unit044.csv");

        fs::write(&known, "0,1,2\n").expect("seed file");

        let vfs = Vfs::with_priority(&[]);
        vfs.create(root.as_path()).expect("mount the scratch dir");

        let mount = root.file_name().and_then(OsStr::to_str).expect("mount key");

        // An edit to an already-indexed file takes the O(1) path.
        assert!(vfs.indexed(mount, &known));

        // A file that appeared after the walk is unknown, so it must force a remount.
        let fresh = root.join("patch").join("unit045.csv");
        fs::write(&fresh, "0\n").expect("seed a late arrival");
        assert!(!vfs.indexed(mount, &fresh));

        // Same name, different directory: still not what the index holds.
        assert!(!vfs.indexed(mount, &root.join("unit044.csv")));

        // Unknown mounts never claim a file.
        assert!(!vfs.indexed("nope", &known));
    }

    #[test]
    fn an_incrementally_updated_index_fingerprints_like_a_cold_walk() {
        let scratch = Scratch::new("fingerprint");
        let root = &scratch.0;
        let target = root.join("patch").join("unit044.csv");

        fs::write(&target, "0,1,2\n").expect("seed file");
        fs::write(root.join("patch").join("t_unit.csv"), "a\n").expect("seed sibling");

        let live = Vfs::with_priority(&[]);
        live.create(root.as_path()).expect("mount");

        let mount = root.file_name().and_then(OsStr::to_str).expect("mount key");
        let before = live.fingerprint();

        // Edit on disk, then apply only the per-file update the watcher would.
        fs::write(&target, "0,9,9,9\n").expect("rewrite file");
        live.create((mount, target.as_path())).expect("incremental update");

        let after = live.fingerprint();
        assert_ne!(before, after, "an edit has to move the fingerprint or the cache never invalidates");

        // The whole design trusts the live index instead of re-walking: a cold walk of the
        // same directory must produce the identical fingerprint, or the key written during
        // a session will not match the one the next launch computes.
        let cold = Vfs::with_priority(&[]);
        cold.create(root.as_path()).expect("cold walk");

        assert_eq!(after, cold.fingerprint(), "incremental index diverged from a fresh walk");
    }

    #[test]
    fn a_deleted_file_cannot_be_reindexed_into_a_dead_entry() {
        let scratch = Scratch::new("deleted");
        let root = &scratch.0;
        let target = root.join("patch").join("unit044.csv");

        fs::write(&target, "0,1,2\n").expect("seed file");

        let vfs = Vfs::with_priority(&[]);
        vfs.create(root.as_path()).expect("mount");

        let mount = root.file_name().and_then(OsStr::to_str).expect("mount key");
        fs::remove_file(&target).expect("delete the file");

        // The name is still indexed, so an existence check is the caller's job, but the
        // incremental mount must refuse rather than insert a zeroed entry, which would shadow
        // the game mount with a dead path and read as "missing" instead of falling back.
        assert!(vfs.indexed(mount, &target));
        assert!(vfs.create((mount, target.as_path())).is_err());
        assert_eq!(vfs.rooted(mount, "unit044.csv"), Some(target.clone()));

        // A remount is what actually drops it, restoring the fallback.
        vfs.create(root.as_path()).expect("remount");
        assert_eq!(vfs.rooted(mount, "unit044.csv"), None);
    }

    #[test]
    fn adding_and_removing_a_rival_copy_needs_no_remount() {
        let scratch = Scratch::new("rivals");
        let root = &scratch.0;
        let shallow = root.join("unit044.csv");
        let deep = root.join("patch").join("unit044.csv");

        fs::write(&shallow, "one\n").expect("seed");

        let vfs = Vfs::with_priority(&[]);
        vfs.create(root.as_path()).expect("mount");

        let mount = root.file_name().and_then(OsStr::to_str).expect("mount key");
        assert!(vfs.indexed(mount, &shallow));

        // A second copy appears: neither resolves, exactly as a cold walk would decide.
        fs::write(&deep, "two\n").expect("seed rival");
        vfs.create((mount, deep.as_path())).expect("index the rival");

        assert!(!vfs.indexed(mount, &shallow), "a contested name is no longer a single entry");
        assert_eq!(vfs.conflicts().len(), 1);

        let cold = Vfs::with_priority(&[]);
        cold.create(root.as_path()).expect("cold walk");
        assert_eq!(vfs.fingerprint(), cold.fingerprint(), "incremental add diverged from a fresh walk");

        // Remove the rival: the survivor has to become resolvable again.
        fs::remove_file(&deep).expect("delete rival");
        vfs.destroy((mount, deep.as_path()));

        assert!(vfs.conflicts().is_empty());
        assert_eq!(vfs.rooted(mount, "unit044.csv"), Some(shallow.clone()));

        let settled = Vfs::with_priority(&[]);
        settled.create(root.as_path()).expect("cold walk");
        assert_eq!(vfs.fingerprint(), settled.fingerprint(), "incremental delete diverged from a fresh walk");
    }

    #[test]
    fn rooted_falls_back_to_the_shallowest_conflicting_copy() {
        let scratch = Scratch::new("rooted");
        let root = &scratch.0;

        fs::write(root.join("unit044.csv"), "shallow\n").expect("seed the shallow copy");
        fs::write(root.join("patch").join("unit044.csv"), "deep\n").expect("seed the deep copy");

        let vfs = Vfs::with_priority(&[]);
        vfs.create(root.as_path()).expect("mount the scratch dir");

        let mount = root.file_name().and_then(OsStr::to_str).expect("mount key");

        // A duplicated name is dropped from `files` and recorded as a conflict, so the
        // editor would resolve nothing and adopt a third copy without this fallback.
        assert_eq!(vfs.rooted(mount, "unit044.csv"), Some(root.join("unit044.csv")));

        // A conflicted name is still not a single indexed entry, so edits to it force a remount.
        assert!(!vfs.indexed(mount, &root.join("unit044.csv")));
    }
}
