pub mod export;
pub mod import;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::common::{architecture, io};
use crate::{Conflict, Vault, Vfs, VfsError};

use super::mods::export::ExportState;
use super::mods::import::ModImportState;

const MODS_ROOT: &str = "mods";

pub const METADATA: &str = "metadata.json";

pub const ICON: &str = "icon.png";

pub const PATCH: &str = "patch";

pub fn taken(mods_root: &Path, candidate: &str) -> bool {
    if candidate.eq_ignore_ascii_case(architecture::GAME) {
        return true;
    }

    let Ok(entries) = fs::read_dir(mods_root) else {
        return false;
    };

    entries.flatten().any(|entry| {
        entry.file_name().to_str().is_some_and(|name| name.eq_ignore_ascii_case(candidate))
    })
}

type Sweep = (Option<PathBuf>, Vec<PathBuf>);

fn descends(entry: &fs::DirEntry, path: &Path) -> bool {
    entry.file_type().map_or_else(
        |_| path.is_dir(),
        |kind| if kind.is_symlink() { path.is_dir() } else { kind.is_dir() },
    )
}

fn sweep(dir: &Path, filename: &str) -> Sweep {
    let mut hit = None;
    let mut next = Vec::new();

    let Ok(entries) = fs::read_dir(dir) else { return (hit, next) };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(OsStr::to_str) else { continue };

        if descends(&entry, &path) {
            next.push(path);
            continue;
        }

        if name == filename {
            hit = Some(path);
        }
    }

    (hit, next)
}

pub fn locate(mod_dir: &Path, filename: &str) -> Option<PathBuf> {
    let mut level = vec![mod_dir.to_path_buf()];

    while !level.is_empty() {
        let scanned: Vec<Sweep> = level.par_iter().map(|dir| sweep(dir, filename)).collect();

        let mut found: Option<PathBuf> = None;
        let mut next = Vec::new();

        for (hit, descend_into) in scanned {
            if let Some(path) = hit
                && found.as_ref().is_none_or(|existing| path < *existing)
            {
                found = Some(path);
            }

            next.extend(descend_into);
        }

        if found.is_some() {
            return found;
        }

        level = next;
    }

    None
}

pub fn enable(vault: &Vault, name: &str) -> Result<Vec<Conflict>, VfsError> {
    let path = Path::new(MODS_ROOT).join(name);
    let conflicts = vault.vfs.create(path.as_path())?;

    let keys = vault.vfs.keys(name);
    debug!(mod_name = name, files = keys.len(), "mod mounted, evicting shadowed game content");

    vault.purge(&keys);

    Ok(conflicts)
}

pub fn disable(vault: &Vault, name: &str) {
    let path = Path::new(MODS_ROOT).join(name);
    let keys = vault.vfs.keys(name);

    vault.vfs.destroy(path.as_path());

    debug!(mod_name = name, files = keys.len(), "mod unmounted, evicting stale mod content");

    vault.purge(&keys);
}

pub fn find(vfs: &Vfs, mod_name: &str, source: &Path) -> Option<PathBuf> {
    let name = source.file_name().and_then(OsStr::to_str)?;

    vfs.rooted(mod_name, name)
}

pub fn find_all<'a>(vfs: &Vfs, mod_name: &str, names: impl IntoIterator<Item = &'a str>) -> FxHashMap<&'a str, PathBuf> {
    names
        .into_iter()
        .filter_map(|name| Some((name, vfs.rooted(mod_name, name)?)))
        .collect()
}

pub fn patch_root(mod_name: &str) -> PathBuf {
    let root = Path::new(MODS_ROOT).join(mod_name);
    let patch = root.join(PATCH);

    if patch.is_dir() { patch } else { root }
}

fn slot(mod_name: &str, name: &OsStr) -> PathBuf {
    let root = Path::new(MODS_ROOT).join(mod_name);
    let patch = root.join(PATCH);

    if patch.is_dir() { patch.join(name) } else { root.join(name) }
}

fn copy_into(mod_name: &str, source: &Path, destination: PathBuf) -> Result<PathBuf, std::io::Error> {
    io::recase(&destination);

    fs::copy(source, &destination)?;

    debug!(mod_name, path = %destination.display(), "Copied a file into a mod");

    Ok(destination)
}

pub fn place(mod_name: &str, source: &Path, name: &str) -> Result<PathBuf, std::io::Error> {
    let root = Path::new(MODS_ROOT).join(mod_name);
    let destination = locate(&root, name).unwrap_or_else(|| slot(mod_name, OsStr::new(name)));

    copy_into(mod_name, source, destination)
}

pub fn ensure(vfs: &Vfs, mod_name: &str, source: &Path) -> Result<PathBuf, std::io::Error> {
    find(vfs, mod_name, source).map_or_else(|| adopt(mod_name, source), Ok)
}

pub fn ensure_as(vfs: &Vfs, mod_name: &str, source: &Path, name: &str) -> Result<PathBuf, std::io::Error> {
    vfs.rooted(mod_name, name)
        .map_or_else(|| copy_into(mod_name, source, slot(mod_name, OsStr::new(name))), Ok)
}

pub fn adopt(mod_name: &str, source: &Path) -> Result<PathBuf, std::io::Error> {
    let Some(name) = source.file_name() else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "the source path has no file name"));
    };

    copy_into(mod_name, source, slot(mod_name, name))
}

fn default_source() -> String {
    "Battle Cats Complete".to_string()
}

fn default_package() -> String {
    "".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMetadata {
    #[serde(default)] pub title: String,
    #[serde(default)] pub author: String,
    #[serde(default)] pub version: String,
    #[serde(default)] pub description: String,
    #[serde(default = "default_package")] pub package: String,
    #[serde(default = "default_source")] pub source: String,
}

impl Default for ModMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            author: String::new(),
            version: String::new(),
            description: String::new(),
            package: default_package(),
            source: default_source(),
        }
    }
}

impl ModMetadata {
    pub fn load<P: AsRef<Path>>(mod_folder_path: P) -> Self {
        let Some(meta_path) = locate(mod_folder_path.as_ref(), METADATA) else {
            return Self::default();
        };

        fs::read_to_string(meta_path).map_or_else(|_| Self::default(), |data| serde_json::from_str(&data).unwrap_or_default())
    }

    pub fn destination<P: AsRef<Path>>(mod_folder_path: P) -> PathBuf {
        let root = mod_folder_path.as_ref();

        locate(root, METADATA).unwrap_or_else(|| root.join(METADATA))
    }

    pub fn write(&self, meta_path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = meta_path.parent()
            && !parent.exists()
        {
            let _ = fs::create_dir_all(parent);
        }

        let data = serde_json::to_string_pretty(self)?;
        fs::write(meta_path, data)
    }

    pub fn save<P: AsRef<Path>>(&self, mod_folder_path: P) -> Result<(), std::io::Error> {
        self.write(&Self::destination(mod_folder_path))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModData {
    pub folder_name: String,
    pub enabled: bool,
    #[serde(skip)] pub metadata: ModMetadata,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModDataState {
    pub search_query: String,
    pub selected_mod: Option<String>,
    #[serde(skip)] pub loaded_mods: Vec<ModData>,
    #[serde(skip)] pub rename_buffer: String,
    pub import: ModImportState,
    pub export: ExportState,
}

impl ModDataState {
    pub fn refresh_mods(&mut self) {
        let mods_dir = Path::new(MODS_ROOT);
        if !mods_dir.exists() { return; }

        let mut current_folders = std::collections::HashSet::new();

        if let Ok(entries) = fs::read_dir(mods_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let folder_name = entry.file_name().to_string_lossy().to_string();
                    current_folders.insert(folder_name.clone());

                    if !self.loaded_mods.iter().any(|m| m.folder_name == folder_name) {
                        let metadata = ModMetadata::load(mods_dir.join(&folder_name));

                        self.loaded_mods.push(ModData {
                            folder_name,
                            enabled: false,
                            metadata,
                        });
                    }
                }
            }
        }

        self.loaded_mods.retain(|m| current_folders.contains(&m.folder_name));
    }

    pub fn active_mod(&self) -> Option<String> {
        self.loaded_mods.iter().find(|m| m.enabled).map(|m| m.folder_name.clone())
    }
}