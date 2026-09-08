use std::sync::LazyLock;

use iced::Element;

use super::resolved::Rule;
use super::{Draft, Message};

// DropItem.csv, measured across the 134 shipped rows. nyanko hand-parses this file and
// publishes no column table, so the layout is restated here and pinned against its parser.
const MAP_ID: usize = 0;
const CROWNS: usize = 4;
const STAGES: usize = 8;
const MATERIALS: usize = 8;

const CROWN_FIRST: usize = 1;
const STAGE_FIRST: usize = CROWN_FIRST + CROWNS;
const DUD: usize = STAGE_FIRST + STAGES;
pub(crate) const MATERIAL_FIRST: usize = DUD + 1;
const ZOMBIE_FIRST: usize = MATERIAL_FIRST + MATERIALS;

pub(crate) const WIDTH: usize = ZOMBIE_FIRST + MATERIALS;

// The crown multipliers are the only cells in the file written with a decimal point; the
// five the corpus ships are 0.5, 0.75, 1, 1.25 and 1.5.
const CROWN_PLACES: u32 = 2;
const CROWN_UNIT: i32 = 100;

const NOTICE: &str = concat!(
    "One row per map. The drop chances share a hundred with No Drop, and a stage's count is ",
    "multiplied by the crown difficulty's own figure before it is awarded",
);

const NOTICE_SIZE: f32 = 11.0;

static LABELS: LazyLock<Vec<String>> = LazyLock::new(|| {
    (0..WIDTH)
        .map(|index| match index {
            MAP_ID => "Map ID".to_owned(),
            _ if index < STAGE_FIRST => format!("{}\u{2605} Multiplier", index - CROWN_FIRST + 1),
            _ if index < DUD => format!("Stage {} Drops", index - STAGE_FIRST + 1),
            DUD => "No Drop %".to_owned(),
            _ if index < ZOMBIE_FIRST => format!("Material {} %", index - MATERIAL_FIRST + 1),
            _ => format!("Z Material {} %", index - ZOMBIE_FIRST + 1),
        })
        .collect()
});

pub(super) fn label(index: usize) -> &'static str {
    LABELS.get(index).map_or("Column", String::as_str)
}

pub(super) fn decimals(index: usize) -> u32 {
    match (CROWN_FIRST..STAGE_FIRST).contains(&index) {
        true => CROWN_PLACES,
        false => 0,
    }
}

// A multiplier the file leaves out is a plain one, which is a hundred hundredths. Every
// other column counts up from nothing.
pub(super) fn fallback(index: usize) -> i32 {
    match (CROWN_FIRST..STAGE_FIRST).contains(&index) {
        true => CROWN_UNIT,
        false => 0,
    }
}

// Measured: No Drop runs 33-60 and the eight material chances 0-67, the two adding up to a
// hundred on all but one shipped row, so both are percentages. A stage's cell holds only 3
// or 4 across the whole file, which is a count of drops rather than the item identifier
// nyanko's doc comment names -- the Materials panel reads it as a count too.
pub(super) fn rule(index: usize) -> Rule {
    match index {
        MAP_ID => Rule::Floor(0),
        _ if index < STAGE_FIRST => Rule::Floor(0),
        _ if index < DUD => Rule::Floor(0),
        _ => Rule::Percent,
    }
}

pub(super) fn view<'a>(draft: &'a Draft, width: f32, query: &'a str, armed: bool) -> Element<'a, Message> {
    super::unitbuy::searched(draft, width, query, armed, Some(NOTICE), NOTICE_SIZE)
}

// drop_chara.csv, one row per unit a stage can unlock. Measured: no shipped stage references
// more than one of them, so the row is addressed outright rather than switched between.
pub(super) const CHARA_WIDTH: usize = 3;

const CHARA_NOTICE: &str =
    "One row per unit a stage clear can unlock. The drop id is the item id the stage's reward block awards";

const CHARA_LABELS: [&str; CHARA_WIDTH] = ["Drop ID", "Gift Slot", "Unit ID"];

pub(super) fn chara_label(index: usize) -> &'static str {
    CHARA_LABELS.get(index).copied().unwrap_or("Column")
}

// The file's own placeholder rows carry -1 in the leading cell, so it floors there; the
// other two count up from nothing.
pub(super) fn chara_rule(index: usize) -> Rule {
    match index {
        0 => Rule::Floor(-1),
        _ => Rule::Floor(0),
    }
}

pub(super) fn chara_view<'a>(draft: &'a Draft, width: f32, armed: bool) -> Element<'a, Message> {
    super::unitbuy::searched(draft, width, "", armed, Some(CHARA_NOTICE), NOTICE_SIZE)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use nyanko::chapter::map::DropItem;
    use nyanko::combat::Separator;
    use nyanko::common;

    use super::super::schema::{self, Subject};
    use super::*;

    fn corpus() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.join(".claude/refs/game/stages");

        root.is_dir().then_some(root)
    }

    // The file covers a hundred-odd maps, keyed by the routed global id, so a story chapter's
    // 3000-3008 reaches no row. The menu is gated on the scanner having found the row, but the
    // failure it prevents is a dead "Edit in app", so the address itself is pinned here.
    #[test]
    fn only_a_map_the_file_carries_opens_a_draft() {
        let Some(root) = corpus() else { return };
        let path = root.join("DropItem.csv");
        let vfs = kore::Vfs::with_priority(&[]);

        let opened = |id: u32| {
            let plan = super::super::plan(
                Subject::MapDrops,
                super::super::Address::Keyed(id),
                "probe".to_owned(),
                &path,
                None,
                kore::domains::settings::EditorMode::Resolved,
            );

            super::super::Draft::load(plan, &vfs).is_some()
        };

        assert!(opened(0), "map 0 is the file's first row");
        assert!(opened(1), "and map 1 its second");
        assert!(!opened(3000), "a story chapter's routed id reaches no row, so no Edit may be offered");
    }

    // nyanko hand-parses DropItem and publishes no column table, so the layout restated here
    // is checked against what its parser actually reads out of every shipped row.
    #[test]
    fn every_column_lands_where_nyanko_reads_it() {
        let Some(root) = corpus() else { return };
        let path = root.join("DropItem.csv");
        let Ok(bytes) = fs::read(&path) else { return };

        let parsed = DropItem::parse(&bytes, None).expect("the shipped table should parse");
        let text = common::scrub(&bytes);
        let delimiter = Separator::detect(&text).unwrap_or(Separator::Comma).char();
        let schema = schema::of(Subject::MapDrops);

        assert!(parsed.map_drops.len() > 100, "expected the shipped map rows");

        let mut checked = 0;

        for line in text.lines().skip(1) {
            let row = super::super::split_row(line, delimiter, schema);
            let Some(id) = row.cells.first().copied().and_then(|id| u32::try_from(id).ok()) else {
                continue;
            };

            let Some(entry) = parsed.map_drops.get(&id) else { continue };

            assert_eq!(entry.map_id, id, "the leading cell addresses the row");

            for (at, held) in entry.crown_multipliers.iter().enumerate() {
                let cell = row.cells[CROWN_FIRST + at];

                assert_eq!(
                    cell,
                    (held * CROWN_UNIT as f32).round() as i32,
                    "map {id} crown {at}: {held} should read as hundredths",
                );
                assert_eq!(decimals(CROWN_FIRST + at), CROWN_PLACES);
            }

            for (at, held) in entry.stage_drops.iter().enumerate() {
                assert_eq!(row.cells[STAGE_FIRST + at], *held as i32, "map {id} stage {at}");
            }

            assert_eq!(row.cells[DUD], entry.dud_chance as i32, "map {id} dud chance");

            for (at, held) in entry.material_drops.iter().enumerate() {
                // The eight zombie slots are only present on the wider rows.
                if MATERIAL_FIRST + at >= row.stored {
                    assert_eq!(*held, 0, "map {id} material {at} is absent, so nyanko reads nothing");
                    continue;
                }

                assert_eq!(row.cells[MATERIAL_FIRST + at], *held as i32, "map {id} material {at}");
            }

            assert_eq!(rebuilt(&row, delimiter), numeric(line), "map {id} must survive an untouched read");

            checked += 1;
        }

        assert!(checked > 100, "only {checked} rows agreed");
    }

    fn numeric(line: &str) -> &str {
        line.split_once("//").map_or(line, |(before, _)| before).trim_end()
    }

    fn rebuilt(row: &super::super::Row, delimiter: char) -> String {
        row.written[..row.stored].join(&delimiter.to_string())
    }
}
