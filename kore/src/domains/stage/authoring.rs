use std::io::Cursor;

use image::{ImageFormat, RgbaImage};

// The engine allocates five hundred map slots per category, read off the decompiled code
// rather than inferred from the shipped files -- the busiest category ships 436 of them.
pub const MAPS_PER_CATEGORY: u32 = 500;

// A stage's number is a two digit field in every filename that carries one, so a map cannot
// address a hundredth stage. The Labyrinth uses all hundred, which is what tells us the
// ceiling is the engine's and not just the widest map anyone shipped.
pub const STAGES_PER_MAP: u32 = 100;

const PLATE_WIDTH: u32 = 256;
const PLATE_HEIGHT: u32 = 64;

pub struct Asset {
    pub name: String,
    pub bytes: Vec<u8>,
}

// A placeholder is transparent rather than absent: the engine faults on a missing asset, and
// a blank one is honest about carrying no art of its own.
pub fn blank_plate(name: &str) -> Vec<Asset> {
    vec![
        Asset { name: name.to_owned(), bytes: blank_png(PLATE_WIDTH, PLATE_HEIGHT) },
        Asset { name: cut_name(name), bytes: imgcut(name, PLATE_WIDTH, PLATE_HEIGHT) },
    ]
}

pub fn cut_name(png: &str) -> String {
    match png.strip_suffix(".png") {
        Some(stem) => format!("{stem}.imgcut"),
        None => format!("{png}.imgcut"),
    }
}

fn blank_png(width: u32, height: u32) -> Vec<u8> {
    let mut held = Cursor::new(Vec::new());
    let sheet = RgbaImage::new(width, height);

    let _ = sheet.write_to(&mut held, ImageFormat::Png);

    held.into_inner()
}

// The shipped cuts are five lines: the marker, a version, the sheet the cuts address, how
// many there are, then one `x,y,w,h,` per cut.
fn imgcut(sheet: &str, width: u32, height: u32) -> Vec<u8> {
    format!("[imgcut]\n0\n{sheet}\n1\n0,0,{width},{height},\n").into_bytes()
}

// A battleground the engine can load: the header row, the config row, and one spawn holding
// enemy 000 so the stage is not empty. `stage.rs`'s own scan reads these back.
pub fn blank_battleground() -> Vec<u8> {
    b"0,0\n1,1000,30,300,0,10,0,0,0\n2,1,0,300,100,0,100,0,100,0,0,0\n".to_vec()
}

// One stage row of a MapStageData table: cost, xp, the two music ids and the change point,
// then a single empty treasure slot closed by the row terminator.
pub fn blank_stage_row() -> String {
    "30,500,1,100,1,0,0,1,-1".to_owned()
}

pub fn blank_map_option(global: u32) -> String {
    format!("{global},0,0,100,150,200,300,0,0,0,0,0,0,0,0,0,0,1,0,")
}

// The two metadata lines and one stage row. A table with no stage rows is an `EmptyFile` to
// nyanko's parser, and a map with no stages never reaches the list at all, so a new map is
// born holding its first one.
pub fn blank_map_data(global: u32) -> Vec<u8> {
    format!("{global},-1,-1,-1,-1,0,0\n0\n{}\n", blank_stage_row()).into_bytes()
}

pub fn next_free(taken: &[u32], ceiling: u32) -> Option<u32> {
    (0..ceiling).find(|id| !taken.contains(id))
}

// A new file's name comes from a sibling that already exists rather than from a naming rule,
// so a category whose files do not follow the obvious pattern still lands in the right place.
// Stories of Legend is the standing example: its maps are `N` but its stages are `RN`.
//
// Every name in this family spells its map in the first run of digits and, where it has one,
// its stage in the second -- `stageRN000_00.csv`, `mapsn000_00_v_en.png`, `MapStageDataN_000.csv`.
// The runs are replaced in place, so each keeps the width and the surrounding text it had.
pub fn renumbered(sibling: &str, map: u32, stage: Option<u32>) -> Option<String> {
    let (stem, extension) = sibling.rsplit_once('.')?;
    let held = runs(stem);

    let mut rebuilt = stem.to_owned();
    let wanted = [Some(map), stage];

    for (at, (start, width)) in held.iter().enumerate().rev() {
        let Some(Some(value)) = wanted.get(at) else { continue };

        rebuilt.replace_range(*start..start + width, &format!("{value:0width$}", width = width));
    }

    held.first().map(|_| format!("{rebuilt}.{extension}"))
}

fn runs(stem: &str) -> Vec<(usize, usize)> {
    let glyphs: Vec<(usize, char)> = stem.char_indices().collect();
    let mut held = Vec::new();
    let mut at = 0;

    while at < glyphs.len() {
        if !glyphs[at].1.is_ascii_digit() {
            at += 1;
            continue;
        }

        let start = glyphs[at].0;
        let first = at;

        while at < glyphs.len() && glyphs[at].1.is_ascii_digit() {
            at += 1;
        }

        held.push((start, at - first));
    }

    held
}

// Row surgery on the shared tables. The files are written back as raw lines, so a table one
// of these touches keeps every other row exactly as it was.
pub fn line_of(body: &str, delimiter: char, at: usize, id: u32) -> Option<usize> {
    body.lines().position(|line| cell(line, delimiter, at) == Some(id))
}

pub fn with_line(body: &str, at: usize, text: &str) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();

    while lines.len() < at {
        lines.push(String::new());
    }

    lines.insert(at.min(lines.len()), text.to_owned());

    joined(lines)
}

pub fn without_line(body: &str, at: usize) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();

    if at < lines.len() {
        lines.remove(at);
    }

    joined(lines)
}

// A table addressed by line -- StageName's maps, MapStageData's stages -- is emptied in
// place, never closed up. Removing the line would renumber every row below it, which is the
// whole map's data sliding one seat up rather than one map going away.
pub fn blanked_line(body: &str, at: usize) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();

    if let Some(held) = lines.get_mut(at) {
        held.clear();
    }

    joined(lines)
}

pub fn blanked_cell(body: &str, delimiter: char, line: usize, cell: usize) -> String {
    with_cell(body, delimiter, line, cell, "")
}

// A stage's name is a cell of its map's line, so adding one widens that line rather than
// adding another.
pub fn with_cell(body: &str, delimiter: char, line: usize, cell: usize, text: &str) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();

    while lines.len() <= line {
        lines.push(String::new());
    }

    let Some(held) = lines.get_mut(line) else {
        return joined(lines);
    };

    let mut fields: Vec<String> = held.split(delimiter).map(str::to_owned).collect();

    while fields.len() <= cell {
        fields.push(String::new());
    }

    fields[cell] = text.to_owned();
    *held = fields.join(&delimiter.to_string());

    joined(lines)
}

pub fn without_cell(body: &str, delimiter: char, line: usize, cell: usize) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();

    let Some(held) = lines.get_mut(line) else {
        return joined(lines);
    };

    let mut fields: Vec<String> = held.split(delimiter).map(str::to_owned).collect();

    if cell < fields.len() {
        fields.remove(cell);
    }

    *held = fields.join(&delimiter.to_string());

    joined(lines)
}

fn cell(line: &str, delimiter: char, at: usize) -> Option<u32> {
    line.split(delimiter).nth(at)?.trim().parse().ok()
}

fn joined(lines: Vec<String>) -> String {
    let mut body = lines.join("\n");
    body.push('\n');

    body
}

#[cfg(test)]
mod tests {
    use super::*;

    // The name of a new file is lifted from one that already exists, so a category whose
    // stage prefix differs from its map prefix keeps working.
    #[test]
    fn a_sibling_names_the_new_file() {
        assert_eq!(renumbered("stageRN000_00.csv", 4, Some(7)).as_deref(), Some("stageRN004_07.csv"));
        assert_eq!(renumbered("stageRN048_12.csv", 100, Some(3)).as_deref(), Some("stageRN100_03.csv"));
        assert_eq!(renumbered("MapStageDataN_000.csv", 12, None).as_deref(), Some("MapStageDataN_012.csv"));
    }

    #[test]
    fn a_name_with_no_stage_field_is_left_short() {
        assert_eq!(renumbered("mapname000_v_en.png", 9, Some(4)).as_deref(), Some("mapname009_v_en.png"));
        assert_eq!(renumbered("Map_option.csv", 3, None), None, "a name with no number cannot be renumbered");
    }

    #[test]
    fn a_localized_plate_keeps_its_suffix() {
        assert_eq!(
            renumbered("mapsn000_00_v_en.png", 3, Some(11)).as_deref(),
            Some("mapsn003_11_v_en.png"),
        );
        assert_eq!(renumbered("mapname000_v_en.png", 9, None).as_deref(), Some("mapname009_v_en.png"));
    }

    #[test]
    fn a_cut_answers_to_its_own_sheet() {
        assert_eq!(cut_name("mapsn003_11_v_en.png"), "mapsn003_11_v_en.imgcut");

        let held = String::from_utf8(imgcut("a.png", 8, 4)).expect("the cut is text");

        assert!(held.starts_with("[imgcut]\n0\na.png\n1\n"), "{held}");
        assert!(held.contains("0,0,8,4,"), "{held}");
    }

    #[test]
    fn a_blank_plate_is_a_readable_transparent_png() {
        let held = blank_plate("mapsn000_00_v_en.png");

        assert_eq!(held.len(), 2, "a plate is its sheet and its cut");

        let decoded = image::load_from_memory(&held[0].bytes).expect("the placeholder should decode");

        assert_eq!(decoded.width(), PLATE_WIDTH);
        assert_eq!(decoded.height(), PLATE_HEIGHT);
    }

    // The engine faults on a malformed file as readily as on a missing one, so every blank
    // this module writes is read back through nyanko's own parser.
    #[test]
    fn a_blank_battleground_loads_with_one_spawn() {
        use nyanko::chapter::stage::Battleground;

        let held = Battleground::parse(blank_battleground(), None).expect("the blank should parse");

        assert_eq!(held.entries.len(), 1, "a new stage is born with one enemy, not none");
        assert!(held.width > 0, "and a battlefield with a width");
    }

    #[test]
    fn a_blank_map_table_loads_with_one_stage() {
        use nyanko::chapter::stage::MapStageData;

        let held = MapStageData::parse(blank_map_data(3000), None).expect("the blank should parse");

        assert_eq!(held.header.map_number, 3000, "the header names the map it was made for");
        assert_eq!(held.entries.len(), 1, "a table with no stage rows is an EmptyFile to nyanko");
    }

    #[test]
    fn a_blank_option_row_fills_every_published_column() {
        let held = blank_map_option(3000);
        let cells: Vec<&str> = held.split(',').collect();

        assert_eq!(cells.first().copied(), Some("3000"));
        assert_eq!(cells.len(), 20, "Map_option's header names twenty columns");
    }

    #[test]
    fn a_row_lands_without_disturbing_its_neighbours() {
        let body = "a\nb\nc\n";

        assert_eq!(with_line(body, 1, "x"), "a\nx\nb\nc\n");
        assert_eq!(without_line(body, 1), "a\nc\n");
        assert_eq!(with_line(body, 5, "x"), "a\nb\nc\n\n\nx\n", "a gap is padded, never skipped");
    }

    // A stage name is a cell of its map's line, so it widens that line rather than adding one.
    // Closing a line up would move every row below it into the seat above, which reads as the
    // map keeping its name while its contents shift.
    #[test]
    fn emptying_a_row_leaves_every_later_row_where_it_was() {
        let body = "map0\nmap1\nmap2\n";

        assert_eq!(blanked_line(body, 1), "map0\n\nmap2\n");
        assert_eq!(without_line(body, 1), "map0\nmap2\n", "the compacting form still exists for id keyed tables");
        assert_eq!(blanked_cell("a|b|c\n", '|', 0, 1), "a||c\n");
    }

    #[test]
    fn a_cell_widens_its_own_line_only() {
        let body = "one|two\nthree\n";

        assert_eq!(with_cell(body, '|', 0, 2, "new"), "one|two|new\nthree\n");
        assert_eq!(with_cell(body, '|', 1, 2, "new"), "one|two\nthree||new\n");
        assert_eq!(without_cell(body, '|', 0, 0), "two\nthree\n");
    }

    #[test]
    fn a_row_is_found_by_the_id_it_carries() {
        let body = "stageID,x\n0,a\n7,b\n";

        assert_eq!(line_of(body, ',', 0, 7), Some(2));
        assert_eq!(line_of(body, ',', 0, 9), None);
    }

    #[test]
    fn the_first_gap_is_the_next_free_slot() {
        assert_eq!(next_free(&[0, 1, 3], STAGES_PER_MAP), Some(2));
        assert_eq!(next_free(&[], MAPS_PER_CATEGORY), Some(0));
        assert_eq!(next_free(&(0..STAGES_PER_MAP).collect::<Vec<u32>>(), STAGES_PER_MAP), None);
    }
}
