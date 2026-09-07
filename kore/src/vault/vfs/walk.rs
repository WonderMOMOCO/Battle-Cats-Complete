use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rayon::prelude::*;
use rustc_hash::FxHashMap;
use tracing::{trace, warn};

use crate::common::junk;

use super::{Conflict, Entry, MountKey, MountedDir, VfsError};

#[derive(Default)]
struct Collected {
    files: FxHashMap<MountKey, Entry>,
    dirs: FxHashMap<Box<str>, Vec<Box<str>>>,
    folders: FxHashMap<Box<str>, Vec<Box<str>>>,
    conflicts: FxHashMap<MountKey, Vec<PathBuf>>,
}

pub(super) fn walk(root: &Path) -> Result<MountedDir, VfsError> {
    if !root.is_dir() {
        return Err(VfsError::NotADirectory(root.to_path_buf()));
    }

    fs::read_dir(root).map_err(|source| VfsError::Walk { path: root.to_path_buf(), source })?;

    let collected = scan(root, root);

    let mut conflicts: Vec<Conflict> = collected
        .conflicts
        .into_iter()
        .map(|(key, mut paths)| {
            paths.sort();
            Conflict {
                key,
                paths: paths.iter().map(|relative| root.join(relative)).collect(),
            }
        })
        .collect();

    conflicts.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(MountedDir {
        root: root.to_path_buf(),
        files: collected.files,
        dirs: collected.dirs,
        folders: collected.folders,
        conflicts,
    })
}

pub(super) fn stat(path: &Path) -> Option<(u64, u64)> {
    path.metadata().ok().map(|meta| (modified(&meta), meta.len()))
}

fn modified(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| u64::try_from(since.as_nanos()).unwrap_or(u64::MAX))
}

fn scan(root: &Path, dir: &Path) -> Collected {
    let mut collected = Collected::default();
    let mut listing = Vec::new();
    let mut nested = Vec::new();
    let mut subdirs = Vec::new();

    let Ok(entries) = fs::read_dir(dir).inspect_err(|err| warn!(path = %dir.display(), "vfs walk skipped a directory: {}", err)) else {
        return collected;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.file_name().and_then(|name| name.to_str()).is_some_and(junk::ignored) {
            trace!(path = %path.display(), "vfs walk skipped a hidden or junk entry");
            continue;
        }

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                nested.push(Box::<str>::from(name));
            }

            subdirs.push(path);
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };

        let (mtime, len) = entry.metadata().map_or((0, 0), |meta| (modified(&meta), meta.len()));

        listing.push(Box::<str>::from(name));
        collected.files.insert(
            Box::from(name),
            Entry { path: relative.to_path_buf(), mtime, len },
        );
    }

    if let Ok(relative) = dir.strip_prefix(root) {
        let key = Box::<str>::from(relative.to_string_lossy());

        listing.sort_unstable();
        nested.sort_unstable();

        collected.dirs.insert(key.clone(), listing);
        collected.folders.insert(key, nested);
    }

    let branches: Vec<Collected> = subdirs.par_iter().map(|sub| scan(root, sub)).collect();

    for branch in branches {
        merge(&mut collected, branch);
    }

    collected
}

fn merge(into: &mut Collected, from: Collected) {
    for (key, entry) in from.files {
        if let Some(recorded) = into.conflicts.get_mut(&key) {
            recorded.push(entry.path);
            continue;
        }

        let Some(previous) = into.files.remove(&key) else {
            into.files.insert(key, entry);
            continue;
        };

        into.conflicts.insert(key, vec![previous.path, entry.path]);
    }

    for (key, mut paths) in from.conflicts {
        if let Some(previous) = into.files.remove(&key) {
            paths.push(previous.path);
        }

        into.conflicts.entry(key).or_default().append(&mut paths);
    }

    into.dirs.extend(from.dirs);
    into.folders.extend(from.folders);
}
