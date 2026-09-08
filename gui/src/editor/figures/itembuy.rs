use iced::Element;

use super::resolved::Rule;
use super::{Draft, Message};

// Gatyaitembuy.csv, one row per catalogue item below a header line. nyanko publishes the
// column table, and its own doc names the invariant this editor leans on: a row's position
// below the header is also the line naming it in GatyaitemName_<lang>.csv.
pub(crate) const ITEM_ID_COLUMN: usize = 3;

const NOTICE: &str = concat!(
    "One row per catalogue item, shared by every stage that drops it. The row's position ",
    "below the header is the line its name sits on in GatyaitemName",
);

const NOTICE_SIZE: f32 = 11.0;

// Measured over the 275 shipped rows: every id column reaches -1 and never goes below, the
// storage flag holds only 0 and 1, and the rest count up from nothing.
pub(super) fn rule(field: &str) -> Option<Rule> {
    match field {
        "reflect_or_storage" => Some(Rule::Flag),

        "stage_drop_item_id" | "sever_id" | "src_item_id" | "gatya_ticket_id" | "img_id" => {
            Some(Rule::Floor(-1))
        }

        "rarity" | "price" | "quantity" | "category" | "index" | "main_menu_type" => Some(Rule::Floor(0)),

        _ => None,
    }
}

// The row's final column is the game's own Japanese comment, which is text rather than a
// number. Showing only the published columns keeps it out of the grid, and `rebuild` writes
// every cell the line stored, so it survives untouched.
pub(super) fn view<'a>(draft: &'a Draft, width: f32, query: &'a str, armed: bool) -> Element<'a, Message> {
    super::unitbuy::bounded(draft, width, query, armed, Some(NOTICE), NOTICE_SIZE, draft.schema().known())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use nyanko::combat::Separator;
    use nyanko::common;
    use nyanko::files::GatyaItemBuy;

    use super::super::schema::{self, Subject};
    use super::super::Address;
    use super::*;

    fn catalogue() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.join(".claude/refs/game");
        let path = root.join("tables/Gatyaitembuy.csv");

        path.is_file().then_some(path)
    }

    // The editor finds an item's row by the id in its fourth column rather than by counting
    // past the header, and the row it lands on has to be the one nyanko parsed at that index
    // -- which is also, by nyanko's own doc, the line naming the item in GatyaitemName.
    #[test]
    fn an_item_is_found_by_its_own_column_at_the_line_that_names_it() {
        let Some(path) = catalogue() else { return };
        let Ok(bytes) = fs::read(&path) else { return };

        let parsed = GatyaItemBuy::parse(&bytes, None).expect("the shipped catalogue should parse");
        let text = common::scrub(&bytes);
        let delimiter = Separator::detect(&text).unwrap_or(Separator::Comma).char();
        let lines: Vec<String> = text.lines().map(str::to_owned).collect();
        let header = usize::from(lines.len() > parsed.len());

        assert!(parsed.len() > 200, "expected the shipped catalogue");

        let mut checked = 0;

        for (line, row) in parsed.iter().enumerate() {
            let Ok(id) = u32::try_from(row.stage_drop_item_id) else { continue };

            let found = Address::Column { at: ITEM_ID_COLUMN, id }.locate(&lines, delimiter);

            assert_eq!(
                found,
                Some(line + header),
                "item {id} is parsed at index {line} and must address the line below the header",
            );

            let held = super::super::split_row(&lines[line + header], delimiter, schema::of(Subject::ItemBuy));

            assert_eq!(held.cells[ITEM_ID_COLUMN], row.stage_drop_item_id, "item {id} column");
            assert_eq!(held.cells[0], row.rarity, "item {id} rarity");

            checked += 1;
        }

        assert!(checked > 200, "only {checked} rows agreed");
    }
}
