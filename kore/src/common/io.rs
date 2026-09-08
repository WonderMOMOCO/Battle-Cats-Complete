pub mod cache;
pub mod json;

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::Vfs;

pub(crate) fn recase(destination: &Path) {
    let Some(dir) = destination.parent() else {
        return;
    };

    let Some(wanted) = destination.file_name().and_then(OsStr::to_str) else {
        return;
    };

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    let mut stray = None;

    for entry in entries.flatten() {
        let spelling = entry.file_name();
        let Some(current) = spelling.to_str() else {
            continue;
        };

        if current == wanted {
            return;
        }

        if stray.is_none() && current.eq_ignore_ascii_case(wanted) {
            stray = Some(entry.path());
        }
    }

    let Some(stray) = stray else {
        return;
    };

    match fs::rename(&stray, destination) {
        Ok(()) => info!(path = %stray.display(), wanted, "Corrected the spelling of a mod file"),
        Err(err) => warn!(path = %stray.display(), wanted, "Could not correct the spelling of a mod file: {}", err),
    }
}

pub(crate) fn hidden_temp(path: &Path) -> PathBuf {
    let Some(name) = path.file_name() else {
        return path.to_path_buf();
    };

    let mut hidden = OsString::from(".");
    hidden.push(name);
    hidden.push(".tmp");

    path.with_file_name(hidden)
}

pub(crate) const ASSET_IMG015_PATTERN: &str = r"^img015(?:_([a-z]{2}))?\.png$";
pub(crate) const ASSET_015CUT_PATTERN: &str = r"^img015(?:_([a-z]{2}))?\.imgcut$";
pub(crate) const ASSET_IMG022_PATTERN: &str = r"^img022(?:_([a-z]{2}))?\.png$";
pub(crate) const ASSET_022CUT_PATTERN: &str = r"^img022(?:_([a-z]{2}))?\.imgcut$";
pub(crate) const LOCALIZEABLE_PATTERN: &str = r"^localizable(?:_([a-z]{2}))?\.tsv$";
pub(crate) const PARAM_PATTERN: &str = r"^param\.tsv$";

pub(crate) const AUDIO_OGG_PATTERN: &str = r"^.+\.ogg$";
pub(crate) const AUDIO_CAF_PATTERN: &str = r"^.+\.caf$";

pub(crate) const GATYA_ITEM_D_PATTERN: &str = r"^gatyaitemD_(\d{2,3})_([fz])\.png$";
pub(crate) const GATYA_ITEM_BUY_PATTERN: &str = r"^Gatyaitembuy\.csv$";
pub(crate) const GATYA_ITEM_NAME_PATTERN: &str = r"^GatyaitemName(?:_([a-z]{2}))?\.csv$";


pub const APP_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("ja", "Japanese"),
    ("tw", "Taiwanese"),
    ("ko", "Korean"),
    ("es", "Spanish"),
    ("de", "German"),
    ("fr", "French"),
    ("it", "Italian"),
    ("th", "Thai"),
];

pub fn gatya_item_icon(vfs: &Vfs, id: i32) -> Option<PathBuf> {
    let names = [
        format!("gatyaitemD_{:03}_f.png", id),
        format!("gatyaitemD_{:02}_f.png", id),
        format!("gatyaitemD_{}_f.png", id),
    ];

    vfs.find(&names)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = env::temp_dir().join(format!("bcc-io-{name}-{}", std::process::id()));

            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("scratch root");

            Self(root)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_stray_spelling_is_moved_onto_the_name_we_are_about_to_write() {
        // Windows folds case, so a copy dropped in as "Uni000_f00.png" swallows a write aimed
        // at "uni000_f00.png" and keeps its own spelling. The game reads the exact name, so
        // the entry has to carry the name we asked for before the bytes land.
        let scratch = Scratch::new("recase");
        let dir = &scratch.0;
        let wanted = dir.join("uni000_f00.png");

        fs::write(dir.join("Uni000_f00.png"), "icon\n").expect("seed the stray spelling");

        recase(&wanted);

        let spellings: Vec<String> = fs::read_dir(dir)
            .expect("listing")
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .collect();

        assert_eq!(spellings, ["uni000_f00.png"]);
    }

    #[test]
    fn recase_never_touches_a_file_it_was_not_asked_about() {
        let scratch = Scratch::new("intact");
        let dir = &scratch.0;

        fs::write(dir.join("uni000_f00.png"), "exact\n").expect("seed");
        fs::write(dir.join("Uni000_f01.png"), "unrelated\n").expect("seed");

        // The name we want is already on disk: there is nothing to correct, and the file
        // sitting there must not be replaced by a neighbour.
        recase(&dir.join("uni000_f00.png"));

        // A different name entirely is not a stray spelling of this one.
        recase(&dir.join("uni000_f02.png"));

        let mut spellings: Vec<String> = fs::read_dir(dir)
            .expect("listing")
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .collect();

        spellings.sort();

        assert_eq!(spellings, ["Uni000_f01.png", "uni000_f00.png"]);
        assert_eq!(fs::read_to_string(dir.join("uni000_f00.png")).expect("read"), "exact\n");
    }
}
