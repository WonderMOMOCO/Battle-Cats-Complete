use std::collections::HashMap;

const COMMENT: &str = "//";
const STORY_TAIL: usize = 2;
const NAME_COLUMN: usize = 1;
const STORY_LAST_NAMED: u32 = 47;
const STORY_PREFIXES: [&str; 4] = ["EC", "W", "Space", "Z"];

// The story chapters put one stage per line and read the first cell; every other chapter
// puts a whole map on one line and reads the cell at the stage index. Both the scan and the
// editor address a name through this, so the two cannot drift.
pub fn named_by_stage(prefix: &str) -> bool {
    STORY_PREFIXES.contains(&prefix)
}

pub fn stage_name_address(prefix: &str, map_id: u32, stage_id: u32) -> (u32, usize) {
    if !named_by_stage(prefix) {
        return (map_id, stage_id as usize);
    }

    let grouped = prefix == "EC" || (prefix == "Z" && map_id <= 2);

    match grouped && matches!(stage_id, 48..=50) {
        true => (STORY_LAST_NAMED, 0),
        false => (stage_id, 0),
    }
}

pub fn map_name_rows(body: &str, delimiter: char) -> HashMap<u32, usize> {
    let mut rows = HashMap::new();

    for (index, line) in body.lines().enumerate() {
        let Some(cells) = columns(line, delimiter) else { continue; };

        if cells.len() <= NAME_COLUMN {
            continue;
        }

        let Ok(map_id) = cells[0].trim().parse::<u32>() else { continue; };

        rows.insert(map_id, index);
    }

    rows
}

pub fn stage_name_rows(body: &str, delimiter: char) -> HashMap<u32, usize> {
    let mut scanned: Vec<(usize, String)> = Vec::new();

    for (index, line) in body.lines().enumerate() {
        let Some(cells) = columns(line, delimiter) else { continue; };

        scanned.push((index, cells.first().map(|cell| cell.trim().to_owned()).unwrap_or_default()));
    }

    let Some(dummy) = scanned.iter().position(|(_, leading)| placeholder(leading)) else {
        return scanned.into_iter().map(|(index, _)| (index as u32, index)).collect();
    };

    let valid: Vec<usize> = scanned.into_iter().take(dummy).map(|(index, _)| index).collect();

    if valid.len() < STORY_TAIL {
        return valid.into_iter().enumerate().map(|(key, index)| (key as u32, index)).collect();
    }

    let (backwards, forwards) = valid.split_at(valid.len() - STORY_TAIL);

    backwards
        .iter()
        .rev()
        .chain(forwards)
        .enumerate()
        .map(|(key, index)| (key as u32, *index))
        .collect()
}

fn columns(line: &str, delimiter: char) -> Option<Vec<&str>> {
    let clean = line.split_once(COMMENT).map_or(line, |(before, _)| before);
    let trimmed = clean.trim();

    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.split(delimiter).collect())
}

fn placeholder(cell: &str) -> bool {
    let clean = cell.trim();

    clean.eq_ignore_ascii_case("dammy")
        || clean.eq_ignore_ascii_case("<tbd>")
        || clean == "預備"
        || clean == "예비"
        || clean == "予備"
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use nyanko::chapter::map::MapName;
    use nyanko::chapter::stage::StageName;
    use nyanko::common;

    use super::*;

    // The editor writes raw lines, so it has to find a name's line itself. The delimiter
    // is the one the prose editor writes with, filename-driven rather than sniffed, so the
    // scan and the write can never disagree; nyanko sniffs, and these tests are what prove
    // the two land on the same character for every shipped file.
    fn delimiter(name: &str) -> char {
        let japanese = name.rsplit_once('_').is_some_and(|(_, tail)| tail.starts_with("ja"));

        if japanese { ',' } else { '|' }
    }

    fn corpus() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.join(".claude/refs/game/stages");

        root.is_dir().then_some(root)
    }

    fn walk(root: &Path, prefix: &str, found: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(root) else { return; };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                walk(&path, prefix, found);
                continue;
            }

            let named = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();

            if named.starts_with(prefix) && named.ends_with(".csv") {
                found.push(path);
            }
        }
    }

    fn cells(line: &str, delimiter: char) -> Vec<String> {
        let clean = line.split_once(COMMENT).map_or(line, |(before, _)| before);

        clean.trim().split(delimiter).map(|cell| cell.trim().to_owned()).collect()
    }

    // The story files reorder their lines: everything above the "dammy" marker counts
    // backwards except the last two, which count forwards. Getting that wrong is silent and
    // would rename the wrong stage, so it is pinned against nyanko on every shipped file.
    #[test]
    fn the_stage_name_scan_agrees_with_nyanko_on_every_shipped_file() {
        let Some(root) = corpus() else { return; };

        let mut files = Vec::new();
        walk(&root, "StageName", &mut files);

        assert!(files.len() > 100, "expected the shipped name corpus, found {}", files.len());

        let mut checked = 0;

        for path in &files {
            let Ok(bytes) = fs::read(path) else { continue };
            let Ok(parsed) = StageName::parse(&bytes, None) else { continue };

            let named = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            let body = common::scrub(&bytes);
            let delimiter = delimiter(named);
            let lines: Vec<&str> = body.lines().collect();
            let rows = stage_name_rows(&body, delimiter);

            for (key, entry) in &parsed.entries {
                let Some(index) = rows.get(key) else {
                    panic!("{}: nyanko keyed {key}, the scan found no line", path.display());
                };

                let Some(line) = lines.get(*index) else {
                    panic!("{}: line {index} for key {key} is past the end", path.display());
                };

                assert_eq!(
                    cells(line, delimiter),
                    entry.names,
                    "{}: line {index} does not hold the names nyanko keyed under {key}",
                    path.display(),
                );
            }

            checked += 1;
        }

        assert!(checked > 100, "only {checked} files agreed");
    }

    // Empire of Cats reuses stage 47's name for 48, 49 and 50, and only there. Getting
    // this wrong renames a neighbouring stage rather than failing, so it is pinned.
    #[test]
    fn the_story_chapters_read_one_name_per_line() {
        assert_eq!(stage_name_address("EC", 0, 12), (12, 0));
        assert_eq!(stage_name_address("EC", 0, 49), (47, 0));
        assert_eq!(stage_name_address("Z", 1, 49), (47, 0));

        // Zombie maps above 2 borrow no story chapter, so 48..=50 stay themselves.
        assert_eq!(stage_name_address("Z", 5, 49), (49, 0));

        // Every other chapter holds a whole map on one line and reads the stage's cell.
        assert_eq!(stage_name_address("V", 3, 7), (3, 7));
    }

    // Map names are a plain id-to-line lookup, but the scan keeps rows nyanko drops for
    // holding an empty name -- an untranslated map is exactly the row a modder opens.
    #[test]
    fn the_map_name_scan_agrees_with_nyanko_on_every_shipped_file() {
        let Some(root) = corpus() else { return; };

        let mut files = Vec::new();
        walk(&root, "Map_Name", &mut files);

        assert!(files.len() > 5, "expected the shipped map name corpus, found {}", files.len());

        let mut checked = 0;
        let mut blanks = 0;

        for path in &files {
            let Ok(bytes) = fs::read(path) else { continue };
            let Ok(parsed) = MapName::parse(&bytes, None) else { continue };

            let named = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            let body = common::scrub(&bytes);
            let delimiter = delimiter(named);
            let lines: Vec<&str> = body.lines().collect();
            let rows = map_name_rows(&body, delimiter);

            for (map_id, name) in &parsed.names {
                let Some(index) = rows.get(map_id) else {
                    panic!("{}: nyanko keyed {map_id}, the scan found no line", path.display());
                };

                let Some(line) = lines.get(*index) else {
                    panic!("{}: line {index} for map {map_id} is past the end", path.display());
                };

                let held = cells(line, delimiter);

                assert_eq!(held.first().map(String::as_str), Some(map_id.to_string().as_str()), "{}", path.display());
                assert_eq!(held.get(NAME_COLUMN).map(String::as_str), Some(name.as_str()), "{}", path.display());
            }

            blanks += rows.len() - parsed.names.len();
            checked += 1;
        }

        assert!(checked > 5, "only {checked} files agreed");
        assert!(blanks > 0, "the shipped corpus carries untranslated map rows and the scan should keep them");
    }
}
