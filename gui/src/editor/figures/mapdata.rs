use std::borrow::Cow;
use std::ops::Range;

use iced::alignment::Horizontal;
use iced::widget::{column, container, text};
use iced::{Element, Length};

use super::resolved::{Choice, Rule};
use super::schema::MAP_STAGE_LEAD;
use super::{cards, Draft, Message};

const NOTICE: &str = concat!(
    "The cells above apply to every stage of the map, and a setting of -1 turns its feature off. ",
    "Below them is this stage's own row, ending in its reward block -- a treasure pool or a score ",
    "ladder, whichever the row itself declares",
);

const NOTICE_SIZE: f32 = 11.0;
const CHROME_GAP: f32 = 8.0;

// A MapStageData file opens with two map-wide metadata lines and then one line per stage.
// nyanko enumerates every line below them and counts a blank or comment-only one as a stage
// index of its own, so a stage row is always at `MAP_HEADER_LINES + stage_id`; the corpus
// test below is what holds that true.
pub(crate) const MAP_HEADER_LINES: usize = 2;
#[cfg(test)]
const MAP_PATTERN_LINE: usize = 1;

// The map-wide cells sit above the scroll because they belong to the file rather than to
// the stage, and the stage's own row scrolls beneath them.
pub(super) fn view<'a>(draft: &'a Draft, width: f32, armed: bool) -> Element<'a, Message> {
    let lead = draft.lead();
    let chrome: Vec<usize> = (0..lead).collect();
    let shown: Vec<usize> = (lead..draft.len()).collect();

    let notice = container(text(NOTICE).size(NOTICE_SIZE).align_x(Horizontal::Center).style(text::secondary))
        .width(Length::Fill)
        .center_x(Length::Fill);

    let top = column![cards::band(draft, width, &chrome), notice].spacing(CHROME_GAP);

    cards::shell(
        Some(cards::header(top)),
        cards::grid(draft, width, &shown, None),
        cards::footer(vec![cards::sync(armed)]),
    )
}

// nyanko publishes `absent -1` for the whole header table, which is the value a *cell*
// reads as when the column is missing -- not the value the field resolves to. The two
// disagree for cost_type, whose struct default is Energy and whose own tests assert an
// omitted column keeps it, and 1,186 of 1,270 files stop before that column. Left at -1 the
// pick list cannot represent its own padding and falls back to a raw number card.
pub(super) fn fallback(field: &str) -> Option<i32> {
    matches!(field, "cost_type").then_some(0)
}

// Measured across the 1,288 shipped MapStageData and stageNormal files: the four setting
// ids and the map number all reach -1 and never go below it, user rank holds only 0 and
// 1600, and cost_type is declared by exactly one file, as Item.
fn named(field: &str) -> Option<Rule> {
    match field {
        "cost_type" => Some(Rule::Choice(&const {
            [Choice::new(0, "Energy"), Choice::new(1, "Item"), Choice::new(2, "Catamin")]
        })),

        "map_number" | "item_reward_setting" | "score_reward_setting" | "map_condition"
        | "stage_condition" => Some(Rule::Floor(-1)),

        "user_rank_threshold" => Some(Rule::Floor(0)),

        "map_pattern" => Some(Rule::Floor(-1)),

        "bgm_change_percent" => Some(Rule::Percent),
        "boss_track" => Some(Rule::Floor(-1)),
        "cost" | "xp" | "init_track" => Some(Rule::Floor(0)),

        _ => None,
    }
}

const DROP_FIRST: usize = 5;
const DROP_RULE: usize = 8;
const DROP_BLOCK: usize = 9;
const MULTI_DROP_WIDTH: usize = 9;
const TIMED_MARK: i32 = -2;
const TIMED_SPAN: Range<usize> = 8..15;
const SCORE_FIRST: usize = 16;
const BLOCK: usize = 3;

const DROP_PARTS: [&str; BLOCK] = ["Chance", "Item", "Amount"];
const SCORE_PARTS: [&str; BLOCK] = ["Points", "Item", "Amount"];

pub(super) fn label(index: usize, cells: &[i32]) -> Option<Cow<'static, str>> {
    tail(index, cells).map(|(label, _)| label)
}

pub(super) fn rule(index: usize, field: Option<&str>, cells: &[i32]) -> Rule {
    if let Some(field) = field {
        return named(field).unwrap_or(Rule::Opaque);
    }

    tail(index, cells).map_or(Rule::Opaque, |(_, rule)| rule)
}

// The reward block past nyanko's five published stage columns. Its length and meaning are
// decided by the row's own contents, so the walk is a mirror of `extract_treasure_drops` and
// `extract_timed_scores` and is pinned against nyanko by a probe test. Anything those two
// never read stays unlabelled and opaque rather than being given a name we invented.
fn tail(index: usize, cells: &[i32]) -> Option<(Cow<'static, str>, Rule)> {
    let column = index.checked_sub(MAP_STAGE_LEAD)?;
    let row = cells.get(MAP_STAGE_LEAD..)?;
    let width = row.len();

    if column < DROP_FIRST {
        return None;
    }

    // Measured: every one of the 6,129 reward-bearing rows ends in -1.
    if column + 1 == width {
        return Some((Cow::Borrowed("Row End"), Rule::Plain));
    }

    if let Some(part) = DROP_PARTS.get(column - DROP_FIRST) {
        return Some((Cow::Owned(format!("Drop 1 {part}")), triple_rule(column - DROP_FIRST)));
    }

    if timed(row) {
        if TIMED_SPAN.contains(&column) {
            return Some((Cow::Borrowed("Timed Marker"), Rule::Plain));
        }

        let offset = column.checked_sub(SCORE_FIRST)?;
        let block = offset / BLOCK;

        if block >= width.saturating_sub(SCORE_FIRST + 1) / BLOCK {
            return None;
        }

        let part = SCORE_PARTS[offset % BLOCK];

        return Some((Cow::Owned(format!("Score {} {part}", block + 1)), triple_rule(offset % BLOCK)));
    }

    if column == DROP_RULE {
        return (width > MULTI_DROP_WIDTH).then_some((Cow::Borrowed("Drop Rule"), Rule::Plain));
    }

    let offset = column.checked_sub(DROP_BLOCK)?;
    let block = offset / BLOCK + 1;

    if block >= width.saturating_sub(DROP_FIRST + 2) / BLOCK {
        return None;
    }

    let part = DROP_PARTS[offset % BLOCK];

    Some((Cow::Owned(format!("Drop {} {part}", block + 1)), triple_rule(offset % BLOCK)))
}

// A chance is a weight rather than a percentage -- measured 1 to 3,775 -- and a score
// reaches -1, so only the id and the quantity floor at zero.
fn triple_rule(part: usize) -> Rule {
    match part {
        0 => Rule::Floor(-1),
        _ => Rule::Floor(0),
    }
}

fn timed(row: &[i32]) -> bool {
    row.len() > TIMED_SPAN.end && row[TIMED_SPAN].iter().all(|cell| *cell == TIMED_MARK)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use nyanko::chapter::stage::{CostType, MapStageData, RewardStructure};
    use nyanko::combat::Separator;
    use nyanko::common;

    use super::super::schema::{self, Subject};
    use super::*;

    const HEADER_WIDTH: usize = 7;
    const STAGE_FIRST: usize = HEADER_WIDTH + 1;

    fn wanted_width() -> usize {
        5
    }

    fn corpus() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.join(".claude/refs/game/stages");

        root.is_dir().then_some(root)
    }

    fn walk(root: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(root) else { return };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                walk(&path, found);
                continue;
            }

            let named = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            let wanted = named.starts_with("MapStageData") || named.starts_with("stageNormal");

            if wanted && named.ends_with(".csv") {
                found.push(path);
            }
        }
    }

    // Only the cells the line really holds are compared. Past them the editor pads with the
    // column's published `absent` default, which is nyanko's declared value for a missing
    // column and deliberately not the same as the struct field's Default.
    fn cells(line: &str, delimiter: char, first: usize, len: usize) -> (Vec<i32>, usize) {
        let row = super::super::split_span(line, delimiter, schema::of(Subject::MapStage), first, len);

        (row.cells, row.stored)
    }

    fn shared(read: &[i32], wanted: &[i32], stored: usize) -> (Vec<i32>, Vec<i32>) {
        let width = wanted.len().min(stored);

        (read[..width].to_vec(), wanted[..width].to_vec())
    }

    // The pick list has to be able to represent the padding the editor writes into a file
    // that stops short of the cost_type column, which 1,186 of 1,270 of them do.
    #[test]
    fn an_absent_cost_type_resolves_to_the_scheme_the_engine_assumes() {
        let schema = schema::of(Subject::MapStage);
        let Some(index) = schema.index_of("cost_type") else {
            panic!("nyanko no longer publishes cost_type");
        };

        assert_eq!(schema.fallback(index), 0, "an omitted cost_type is Energy, not the -1 cell sentinel");

        let short = "19,194,-1,-1,-1,0";
        let (cells, _) = cells(short, ',', 0, HEADER_WIDTH);

        assert_eq!(cells.get(index), Some(&0), "the padded cell must be a value the pick list can show");

        let held = MapStageData::parse(format!("{short}\n0\n10,1,1,1,1\n"), None)
            .expect("the fixture should parse");

        assert_eq!(held.header.cost_type, CostType::Energy, "nyanko agrees the absent column is Energy");
    }

    // Each labelled reward cell is given a value of its own and the row handed to nyanko, so
    // the name the card shows is checked against the field nyanko actually parses it into
    // rather than against a reading of its extractor.
    fn probe(row: &[i32]) -> Vec<(String, i32)> {
        let cells: Vec<i32> = std::iter::repeat_n(0, MAP_STAGE_LEAD).chain(row.iter().copied()).collect();

        (MAP_STAGE_LEAD..cells.len())
            .filter_map(|index| Some((label(index, &cells)?.into_owned(), cells[index])))
            .collect()
    }

    fn parsed(row: &[i32]) -> RewardStructure {
        let cells: Vec<String> = row.iter().map(i32::to_string).collect();
        let body = format!("0\n0\n{}\n", cells.join(","));

        MapStageData::parse(body, None)
            .ok()
            .and_then(|held| held.entries.into_iter().next())
            .map(|entry| entry.rewards)
            .unwrap_or_default()
    }

    #[test]
    fn a_treasure_row_names_the_pool_nyanko_parses() {
        //     cost xp trk pct bos | c1 i1 a1 | rule | c2 i2 a2 | c3 i3 a3 | end
        let row = [30, 900, 3, 99, 33, 34, 55, 3, -3, 33, 56, 4, 32, 57, 5, -1];
        let named = probe(&row);

        let RewardStructure::Treasure { drop_rule, drops } = parsed(&row) else {
            panic!("nyanko no longer reads this row as a treasure pool");
        };

        assert_eq!(drop_rule, -3);
        assert_eq!(drops.len(), 3, "three drop blocks");

        for (at, drop) in drops.iter().enumerate() {
            let wanted = [
                (format!("Drop {} Chance", at + 1), drop.chance as i32),
                (format!("Drop {} Item", at + 1), drop.item_id as i32),
                (format!("Drop {} Amount", at + 1), drop.amount as i32),
            ];

            for (label, value) in wanted {
                assert!(
                    named.contains(&(label.clone(), value)),
                    "{label} should name the cell holding {value}, found {named:?}",
                );
            }
        }

        assert!(named.contains(&("Drop Rule".to_owned(), -3)));
        assert!(named.contains(&("Row End".to_owned(), -1)));
    }

    #[test]
    fn a_timed_row_names_the_ladder_nyanko_parses() {
        //     cost xp trk pct bos | c1 i1 a1 | -2 x7 | ? | s1 i1 a1 | s2 i2 a2 | end
        let row = [
            30, 900, 3, 99, 33, 1, 1028, 1, -2, -2, -2, -2, -2, -2, -2, 1, 7000, 6, 50, 4500, 7, 10, -1,
        ];
        let named = probe(&row);

        let RewardStructure::Timed(scores) = parsed(&row) else {
            panic!("nyanko no longer reads this row as a score ladder");
        };

        assert_eq!(scores.len(), 2, "two score blocks");

        for (at, score) in scores.iter().enumerate() {
            let wanted = [
                (format!("Score {} Points", at + 1), score.score as i32),
                (format!("Score {} Item", at + 1), score.item_id as i32),
                (format!("Score {} Amount", at + 1), score.amount as i32),
            ];

            for (label, value) in wanted {
                assert!(
                    named.contains(&(label.clone(), value)),
                    "{label} should name the cell holding {value}, found {named:?}",
                );
            }
        }

        assert_eq!(named.iter().filter(|(label, _)| label == "Timed Marker").count(), TIMED_SPAN.len());
        assert!(named.contains(&("Row End".to_owned(), -1)));
    }

    // A card's label box is sized once from the static table, so a reward name must not wrap
    // wider than the widest column the table already publishes.
    #[test]
    fn no_reward_name_is_wider_than_the_published_columns() {
        let widest = (0..schema::of(Subject::MapStage).known())
            .map(|index| schema::of(Subject::MapStage).label(index).chars().count())
            .max()
            .unwrap_or(0);

        for name in ["Drop 10 Chance", "Score 10 Points", "Timed Marker", "Drop Rule", "Row End"] {
            assert!(name.chars().count() <= widest, "{name} is wider than {widest} characters");
        }
    }

    // The editor addresses a stage by arithmetic rather than by scanning, so the arithmetic
    // has to be right on every shipped file: 9,083 of their table lines are blank or comment
    // only, and nyanko spends a stage index on each one.
    #[test]
    fn a_stage_row_sits_where_the_editor_addresses_it() {
        let Some(root) = corpus() else { return };

        let mut files = Vec::new();
        walk(&root, &mut files);

        assert!(files.len() > 1000, "expected the shipped map corpus, found {}", files.len());

        let mut checked = 0;

        for path in &files {
            let Ok(bytes) = fs::read(path) else { continue };
            let Ok(parsed) = MapStageData::parse(&bytes, None) else { continue };

            let body = common::scrub(&bytes);
            let delimiter = Separator::detect(&body).unwrap_or(Separator::Comma).char();
            let lines: Vec<&str> = body.lines().collect();

            for (stage, entry) in parsed.entries.iter().enumerate() {
                let held = lines.get(MAP_HEADER_LINES + stage).copied().unwrap_or_default();
                let (read, stored) = cells(held, delimiter, STAGE_FIRST, wanted_width());

                let wanted = [
                    entry.cost as i32,
                    entry.xp as i32,
                    entry.init_track as i32,
                    entry.bgm_change_percent as i32,
                    i32::from(entry.boss_track),
                ];

                let (read, wanted) = shared(&read, &wanted, stored);

                assert_eq!(
                    read, wanted,
                    "{}: line {} does not hold stage {stage}",
                    path.display(),
                    MAP_HEADER_LINES + stage,
                );
            }

            let (head, stored) = cells(lines.first().copied().unwrap_or_default(), delimiter, 0, HEADER_WIDTH);
            let header = &parsed.header;
            let cost_type = match header.cost_type {
                CostType::Energy => 0,
                CostType::Item => 1,
                CostType::Catamin => 2,
                _ => -1,
            };

            let declared = [
                header.map_number,
                header.item_reward_setting,
                header.score_reward_setting,
                header.map_condition,
                header.stage_condition,
                header.user_rank_threshold,
                cost_type,
            ];

            let (head, declared) = shared(&head, &declared, stored);

            assert_eq!(
                head, declared,
                "{}: line 0 is not the map header",
                path.display(),
            );

            let (pattern, _) =
                cells(lines.get(MAP_PATTERN_LINE).copied().unwrap_or_default(), delimiter, HEADER_WIDTH, 1);

            assert_eq!(
                pattern.first().copied(),
                Some(header.map_pattern),
                "{}: line {MAP_PATTERN_LINE} is not the map pattern",
                path.display(),
            );

            checked += 1;
        }

        assert!(checked > 1000, "only {checked} files agreed");
    }
}
