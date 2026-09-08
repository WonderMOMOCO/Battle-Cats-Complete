use std::fs;
use std::path::{Path, PathBuf};

use nyanko::cat::unitid;
use nyanko::cat::unit::{Nyancombo, NyancomboData, NyancomboFilter, NyancomboParam, UnitExplanation};
use nyanko::combat::{Entity, Separator};
use tracing::trace;

use crate::domains::cat::files;
use crate::Vfs;

pub fn unitexplanation(vfs: &Vfs, cat_id: u32) -> UnitExplanation {
    trace!(cat_id = cat_id, "loading unit localized explanations");
    let mut final_explanation = UnitExplanation::default();
    let base_filename = format!("Unit_Explanation{}.csv", cat_id + 1);

    for file_path in vfs.list(&base_filename) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        let Ok(parsed_explanation) = UnitExplanation::parse(&bytes, Some(delimiter(&file_path))) else {
            continue;
        };

        for index in 0..4 {
            if final_explanation.names[index].is_none() && parsed_explanation.names[index].is_some() {
                final_explanation.names[index] = parsed_explanation.names[index].clone();
                final_explanation.descriptions[index] = parsed_explanation.descriptions[index].clone();
            }
        }
    }

    final_explanation
}

pub fn unitexplanation_source(vfs: &Vfs, cat_id: u32, form: usize) -> Option<PathBuf> {
    let base_filename = format!("Unit_Explanation{}.csv", cat_id + 1);
    let mut fallback = None;

    for file_path in vfs.list(&base_filename) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        let Ok(parsed) = UnitExplanation::parse(&bytes, Some(delimiter(&file_path))) else {
            continue;
        };

        if parsed.names.get(form).is_some_and(Option::is_some)
            || parsed.descriptions.get(form).is_some_and(Option::is_some)
        {
            return Some(file_path);
        }

        if fallback.is_none() {
            fallback = Some(file_path);
        }
    }

    fallback.or_else(|| vfs.find(&base_filename))
}

pub fn unitid(vfs: &Vfs, cat_id: i32) -> Option<Vec<Entity>> {
    trace!(cat_id = cat_id, "fetching individual unit battle layout");
    let file_name = files::stats_file(cat_id as u32);

    let resolved_path = vfs.find(&file_name)?;

    let bytes = fs::read(resolved_path).ok()?;

    unitid::parse(&bytes, None).ok()
}

pub(crate) enum ComboText {
    Name,
    Effect,
    Band,
}

impl ComboText {
    fn file(&self) -> &'static str {
        match self {
            ComboText::Name => files::NYANCOMBO_NAME,
            ComboText::Effect => files::NYANCOMBO_EFFECT,
            ComboText::Band => files::NYANCOMBO_BAND,
        }
    }

    fn separator(&self, path: &Path) -> Option<Separator> {
        match self {
            ComboText::Name => None,
            _ => Some(delimiter(path)),
        }
    }
}

pub(crate) fn nyancombodata(vfs: &Vfs) -> Vec<NyancomboData> {
    trace!("loading the cat combo table");

    let Some(file_path) = vfs.find(files::NYANCOMBO_DATA) else {
        return Vec::new();
    };

    let Ok(bytes) = fs::read(&file_path) else {
        return Vec::new();
    };

    NyancomboData::parse(&bytes, None).unwrap_or_default()
}

pub(crate) fn nyancombofilter(vfs: &Vfs) -> Vec<NyancomboFilter> {
    trace!("loading the cat combo filter groups");

    let Some(file_path) = vfs.find(files::NYANCOMBO_FILTER) else {
        return Vec::new();
    };

    let Ok(bytes) = fs::read(&file_path) else {
        return Vec::new();
    };

    NyancomboFilter::parse(&bytes, None).unwrap_or_default()
}

pub(crate) fn nyancomboparam(vfs: &Vfs) -> Vec<NyancomboParam> {
    trace!("loading the cat combo effect magnitudes");

    let Some(file_path) = vfs.find(files::NYANCOMBO_PARAM) else {
        return Vec::new();
    };

    let Ok(bytes) = fs::read(&file_path) else {
        return Vec::new();
    };

    NyancomboParam::parse(&bytes, None).unwrap_or_default()
}

pub fn nyancombo_source(vfs: &Vfs, line: usize) -> Option<PathBuf> {
    let table = ComboText::Name;
    let mut fallback = None;

    for file_path in vfs.list(table.file()) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        if Nyancombo::parse_row(&bytes, line, table.separator(&file_path))
            .is_some_and(|entry| entry.text.is_some())
        {
            return Some(file_path);
        }

        if fallback.is_none() {
            fallback = Some(file_path);
        }
    }

    fallback.or_else(|| vfs.find(table.file()))
}

pub(crate) fn nyancombo(vfs: &Vfs, table: ComboText) -> Vec<Option<String>> {
    trace!(file = table.file(), "merging localized cat combo text");
    let mut merged: Vec<Option<String>> = Vec::new();

    for file_path in vfs.list(table.file()) {
        let Ok(bytes) = fs::read(&file_path) else {
            continue;
        };

        let Ok(lines) = Nyancombo::parse(&bytes, table.separator(&file_path)) else {
            continue;
        };

        if lines.len() > merged.len() {
            merged.resize(lines.len(), None);
        }

        for (line, entry) in lines.into_iter().enumerate() {
            if let Some(slot) = merged.get_mut(line)
                && slot.is_none()
            {
                *slot = entry.text;
            }
        }
    }

    merged
}

fn delimiter(path: &Path) -> Separator {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();

    crate::common::region::text_separator(&name)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = env::temp_dir().join(format!("bcc-combo-{name}-{}", std::process::id()));

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
    fn every_region_in_priority_order_fills_the_holes_the_one_above_it_leaves() {
        let scratch = Scratch::new("names");
        let root = &scratch.0;

        // Every region but Japan blanks a few entries and stops a line short, and Japanese
        // is only the usual filler, not a special case: whichever table comes next in the
        // priority order gets to answer.
        fs::write(root.join("Nyancombo_en.csv"), "Cat Army\n\n\nRed Strike\n").expect("seed en");
        fs::write(root.join("Nyancombo_ja.csv"), "ニャンコ軍団\nキンギンパワー\n\nレッドストライク\nパックマンフィーバー\n").expect("seed ja");
        fs::write(root.join("Nyancombo_tw.csv"), "貓咪軍團\n金銀力量\n只有台灣\n紅色打擊\n").expect("seed tw");

        let vfs = Vfs::with_priority(&[
            "".to_string(),
            "en".to_string(),
            "ja".to_string(),
            "tw".to_string(),
            "--".to_string(),
        ]);
        vfs.create(root.as_path()).expect("mount the scratch dir");

        let merged = nyancombo(&vfs, ComboText::Name);

        assert_eq!(
            merged,
            vec![
                Some("Cat Army".to_string()),
                Some("キンギンパワー".to_string()),
                Some("只有台灣".to_string()),
                Some("Red Strike".to_string()),
                Some("パックマンフィーバー".to_string()),
            ]
        );
    }

    #[test]
    fn the_band_suffix_keeps_the_space_that_separates_it() {
        let scratch = Scratch::new("bands");
        let root = &scratch.0;

        fs::write(root.join("Nyancombo2_en.csv"), " (Sm)|\n (M)|\n Activated|\n").expect("seed en");

        let vfs = Vfs::with_priority(&["".to_string(), "en".to_string(), "--".to_string()]);
        vfs.create(root.as_path()).expect("mount the scratch dir");

        let bands = nyancombo(&vfs, ComboText::Band);

        assert_eq!(bands.first(), Some(&Some(" (Sm)".to_string())));
        assert_eq!(bands.get(2), Some(&Some(" Activated".to_string())));
    }
}
