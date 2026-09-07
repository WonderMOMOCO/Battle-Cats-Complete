use std::collections::BTreeMap;
use std::fs;

use nyanko::cat::unit::UnitExplanation;
use nyanko::chapter::map::MapName;
use nyanko::enemy::{EnemyName, EnemyPictureBook};
use rayon::prelude::*;

use crate::common::region;
use crate::domains::settings::lang;
use crate::Vfs;

use super::{FileDelta, Diff, Status};

const EXPLANATION: &str = "Unit_Explanation";
const ENEMY_NAMES: &str = "Enemyname";
const ENEMY_BOOK: &str = "EnemyPictureBook";
const MAP_NAMES: &str = "Map_Name";
const CSV: &str = ".csv";
const TSV: &str = ".tsv";
const CODE_LENGTH: usize = 2;

type Spoken = BTreeMap<(u32, Option<usize>), Vec<String>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Table {
    Names,
    Book,
}

const FORMS: usize = 4;

pub struct Localized {
    pub id: u32,
    pub form: Option<usize>,
    pub languages: Vec<String>,
}

pub(crate) fn mineable(filename: &str) -> bool {
    explains(filename) || describes(filename) || charts(filename)
}

pub(crate) fn explains(filename: &str) -> bool {
    explanation(filename).is_some()
}

pub(crate) fn describes(filename: &str) -> bool {
    roster(filename).is_some()
}

pub(crate) fn charts(filename: &str) -> bool {
    atlas(filename).is_some()
}

pub fn cats(diff: &Diff, vfs: &Vfs) -> Vec<Localized> {
    let arrivals: Vec<((u32, Option<usize>), String)> = diff
        .files
        .par_iter()
        .filter_map(|delta| {
            let (id, code) = explanation(&delta.file)?;
            let bytes = quarried(vfs, &delta.file)?;
            let after = String::from_utf8_lossy(&bytes);
            let now = named(after.as_bytes());

            let was = match delta.status {
                Status::Baseline => [false; FORMS],
                Status::Changed => named(restore(&after, delta).as_bytes()),
            };

            let spoken: Vec<((u32, Option<usize>), String)> = (0..FORMS)
                .filter(|form| now[*form] && !was[*form])
                .map(|form| ((id, Some(form)), code.clone()))
                .collect();

            Some(spoken)
        })
        .flatten()
        .collect();

    let mut found: Spoken = BTreeMap::new();

    for (key, code) in &arrivals {
        record(&mut found, *key, code);
    }

    gather(found)
}

pub fn enemies(diff: &Diff, vfs: &Vfs) -> Vec<Localized> {
    let mut found: Spoken = BTreeMap::new();

    for delta in &diff.files {
        let Some((table, code)) = roster(&delta.file) else {
            continue;
        };

        let speaks = |line: &str| reads(table, line, &delta.file);

        match delta.status {
            Status::Baseline => {
                let Some(bytes) = quarried(vfs, &delta.file) else {
                    continue;
                };

                for (index, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
                    if speaks(line) {
                        record(&mut found, (index as u32, None), &code);
                    }
                }
            }
            Status::Changed => {
                for row in &delta.rows {
                    let Ok(id) = row.key.parse::<u32>() else {
                        continue;
                    };

                    if row.after.as_deref().is_some_and(&speaks)
                        && !row.before.as_deref().is_some_and(&speaks)
                    {
                        record(&mut found, (id, None), &code);
                    }
                }
            }
        }
    }

    gather(found)
}

pub fn stages(diff: &Diff, vfs: &Vfs) -> Vec<Localized> {
    let mut found: Spoken = BTreeMap::new();

    for delta in &diff.files {
        let Some(code) = atlas(&delta.file) else {
            continue;
        };

        let named = |line: &str| titled(line, &delta.file);

        match delta.status {
            Status::Baseline => {
                let Some(bytes) = quarried(vfs, &delta.file) else {
                    continue;
                };

                for map in titles(&bytes, &delta.file) {
                    record(&mut found, (map, None), &code);
                }
            }
            Status::Changed => {
                for row in &delta.rows {
                    let Some(map) = row.after.as_deref().and_then(&named) else {
                        continue;
                    };

                    if row.before.as_deref().and_then(&named).is_none() {
                        record(&mut found, (map, None), &code);
                    }
                }
            }
        }
    }

    gather(found)
}

fn restore(current: &str, delta: &FileDelta) -> String {
    let mut lines: Vec<&str> = Vec::new();

    for (index, line) in current.lines().enumerate() {
        let key = index.to_string();

        match delta.rows.iter().find(|row| row.key == key) {
            Some(row) => lines.extend(row.before.as_deref()),
            None => lines.push(line),
        }
    }

    for row in delta.rows.iter().filter(|row| row.after.is_none()) {
        lines.extend(row.before.as_deref());
    }

    lines.join("\n")
}

fn quarried(vfs: &Vfs, filename: &str) -> Option<Vec<u8>> {
    fs::read(vfs.pristine(filename)?).ok()
}

fn record(found: &mut Spoken, key: (u32, Option<usize>), code: &str) {
    let spoken = found.entry(key).or_default();

    if !spoken.iter().any(|held| held == code) {
        spoken.push(code.to_string());
    }
}

fn gather(found: Spoken) -> Vec<Localized> {
    let order = lang::default_priority();
    let rank = |code: &String| order.iter().position(|entry| entry == code).unwrap_or(usize::MAX);

    found
        .into_iter()
        .map(|((id, form), mut languages)| {
            languages.sort_by(|left, right| rank(left).cmp(&rank(right)).then_with(|| left.cmp(right)));

            Localized { id, form, languages }
        })
        .collect()
}

fn code(rest: &str) -> Option<String> {
    match rest {
        "" => Some(String::new()),
        other => other
            .strip_prefix('_')
            .filter(|code| code.len() == CODE_LENGTH && code.chars().all(|glyph| glyph.is_ascii_lowercase()))
            .map(str::to_string),
    }
}

fn explanation(filename: &str) -> Option<(u32, String)> {
    let body = filename.strip_prefix(EXPLANATION)?.strip_suffix(CSV)?;
    let cut = body.find(|glyph: char| !glyph.is_ascii_digit()).unwrap_or(body.len());
    let (digits, rest) = body.split_at(cut);

    Some((digits.parse::<u32>().ok()?.checked_sub(1)?, code(rest)?))
}

fn roster(filename: &str) -> Option<(Table, String)> {
    let named = filename.strip_prefix(ENEMY_NAMES).and_then(|body| body.strip_suffix(TSV));

    if let Some(rest) = named {
        return Some((Table::Names, code(rest)?));
    }

    let described = filename.strip_prefix(ENEMY_BOOK).and_then(|body| body.strip_suffix(CSV))?;

    Some((Table::Book, code(described)?))
}

fn atlas(filename: &str) -> Option<String> {
    code(filename.strip_prefix(MAP_NAMES)?.strip_suffix(CSV)?)
}

fn titles(bytes: &[u8], filename: &str) -> Vec<u32> {
    MapName::parse(bytes, Some(region::text_separator(filename)))
        .map_or_else(|_| Vec::new(), |held| held.names.into_keys().collect())
}

fn titled(line: &str, filename: &str) -> Option<u32> {
    titles(line.as_bytes(), filename).first().copied()
}

fn reads(table: Table, line: &str, filename: &str) -> bool {
    match table {
        Table::Names => EnemyName::parse(line).is_ok_and(|rows| rows.first().is_some_and(|row| row.name.is_some())),
        Table::Book => EnemyPictureBook::parse(line, Some(region::text_separator(filename)))
            .is_ok_and(|rows| rows.first().is_some_and(|row| row.description.is_some())),
    }
}

fn named(bytes: &[u8]) -> [bool; FORMS] {
    UnitExplanation::parse(bytes, None)
        .map_or([false; FORMS], |held| held.names.map(|name| name.is_some()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_localized_table_is_recognised_only_in_its_own_family() {
        assert_eq!(explanation("Unit_Explanation301_ko.csv"), Some((300, "ko".to_string())));
        assert_eq!(explanation("Unit_Explanation301.csv"), Some((300, String::new())));
        assert!(explanation("Unit_Explanation0.csv").is_none(), "there is no unit before the first");
        assert!(explanation("SkillDescriptions.csv").is_none());

        assert!(matches!(roster("Enemyname_en.tsv"), Some((Table::Names, code)) if code == "en"));
        assert!(matches!(roster("EnemyPictureBook.csv"), Some((Table::Book, code)) if code.is_empty()));

        // The sibling tables share the picture book's prefix but are not the picture book.
        assert!(roster("EnemyPictureBook2_en.csv").is_none());
        assert!(roster("EnemyPictureBookQuestion.csv").is_none());
    }

    // A form arriving appends a line to the explanation file, which looks exactly like a row
    // gaining a name. The unit was already named in this language, so it is not a localization.
    #[test]
    fn a_new_form_is_not_a_new_language() {
        let arrived = FileDelta {
            file: "Unit_Explanation301_en.csv".to_string(),
            from: "en".to_string(),
            region: "en".to_string(),
            status: Status::Changed,
            rows_before: 3,
            rows_after: 4,
            rows: vec![super::super::RowDelta {
                key: "3".to_string(),
                before: None,
                after: Some("Ultra Cat|An ultra form.".to_string()),
            }],
        };

        let after = "Normal Cat|A cat.\nEvolved Cat|B cat.\nTrue Cat|C cat.\nUltra Cat|An ultra form.";

        assert_eq!(named(after.as_bytes()), [true; FORMS]);
        assert_eq!(
            named(restore(after, &arrived).as_bytes()),
            [true, true, true, false],
            "the earlier forms were already named before the ultra form landed"
        );

        let fresh = FileDelta {
            rows: vec![super::super::RowDelta {
                key: "0".to_string(),
                before: None,
                after: Some("Normal Cat|A cat.".to_string()),
            }],
            ..arrived
        };

        assert_eq!(named(restore("Normal Cat|A cat.", &fresh).as_bytes()), [false; FORMS]);
    }

    // nyanko owns what counts as a placeholder; the section only asks it the question.
    // Map_Name spells an unlocalized map out as a present row with an empty name, and the
    // English table is pipe-separated while the Japanese one is not.
    #[test]
    fn an_empty_map_name_is_not_a_localization() {
        assert!(matches!(atlas("Map_Name_en.csv"), Some(code) if code == "en"));
        assert!(atlas("Map_option.csv").is_none());

        assert_eq!(titled("4081|Pac-Man Collab EX", "Map_Name_en.csv"), Some(4081));
        assert_eq!(titled("4081|", "Map_Name_en.csv"), None, "a named-but-empty row is not localized");
        assert_eq!(titled("4081,\u{30d1}\u{30c3}\u{30af}\u{30de}\u{30f3}", "Map_Name_ja.csv"), Some(4081));
    }

    #[test]
    fn a_placeholder_row_is_not_a_localization() {
        assert!(reads(Table::Names, "Ragnarok", "Enemyname_en.tsv"));
        assert!(!reads(Table::Names, "ダミー", "Enemyname_en.tsv"));
        assert!(!reads(Table::Names, "   ", "Enemyname_en.tsv"));

        assert!(named(b"Ragnarok|A fallen angel.")[0]);
        assert!(!named(b"301|")[0], "a bare identifier is not a name");
    }
}
