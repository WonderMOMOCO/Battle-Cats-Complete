use std::fmt;

use kore::domains::settings::EditorMode;

use super::combat::{cats, enemies};
use super::{combos, costs, itembuy, mapdata, mapdrops, schema::Subject, talents, unitbuy, unitlevel};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::editor) enum Rule {
    Plain,
    Opaque,
    Flag,
    Choice(&'static [Choice]),
    Gated(&'static [Choice], Gate),
    Percent,
    Floor(i32),
    Offset(i32),
    Ratio(i32, i32),
    Inverted(i32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::editor) struct Gate {
    pub(in crate::editor) field: &'static str,
    pub(in crate::editor) blocked: i32,
    pub(in crate::editor) reason: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::editor) struct Choice {
    raw: i32,
    label: &'static str,
}

impl Choice {
    pub(in crate::editor) const fn new(raw: i32, label: &'static str) -> Choice {
        Choice { raw, label }
    }

    pub(in crate::editor) fn raw(self) -> i32 {
        self.raw
    }

    pub(in crate::editor) fn label(self) -> &'static str {
        self.label
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::editor) enum Toggle {
    No,
    Yes,
}

impl Toggle {
    pub(in crate::editor) fn flip(self) -> Toggle {
        match self {
            Toggle::No => Toggle::Yes,
            Toggle::Yes => Toggle::No,
        }
    }

    pub(super) fn raw(self) -> i32 {
        match self {
            Toggle::No => 0,
            Toggle::Yes => 1,
        }
    }

    fn of(raw: i32) -> Option<Toggle> {
        match raw {
            0 => Some(Toggle::No),
            1 => Some(Toggle::Yes),
            _ => None,
        }
    }
}

impl fmt::Display for Toggle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Toggle::No => "No",
            Toggle::Yes => "Yes",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::editor) enum Face {
    Number,
    Danger,
    Toggle(Toggle),
    Choice(&'static [Choice], Choice),
    Disabled(&'static str),
}

pub(super) const PERCENT_FLOOR: i32 = 0;
pub(super) const PERCENT_CEILING: i32 = 100;

impl Rule {
    pub(in crate::editor) fn gate(self) -> Option<Gate> {
        match self {
            Rule::Gated(_, gate) => Some(gate),
            _ => None,
        }
    }

    pub(in crate::editor) fn to_display(self, raw: i32, values: EditorMode) -> i32 {
        if values == EditorMode::Raw {
            return raw;
        }

        match self {
            Rule::Offset(by) => raw.saturating_add(by),
            Rule::Ratio(numerator, denominator) => ratio(raw, numerator, denominator),
            Rule::Inverted(base) => base.saturating_sub(raw),
            _ => raw,
        }
    }

    pub(in crate::editor) fn to_raw(self, display: i32, values: EditorMode) -> i32 {
        if values == EditorMode::Raw {
            return display;
        }

        match self {
            Rule::Offset(by) => display.saturating_sub(by),
            Rule::Ratio(numerator, denominator) => ratio(display, denominator, numerator),
            Rule::Inverted(base) => base.saturating_sub(display),
            _ => display,
        }
    }

    pub(in crate::editor) fn signed(self, values: EditorMode) -> bool {
        if values == EditorMode::Raw {
            return true;
        }

        match self {
            Rule::Percent | Rule::Ratio(_, _) | Rule::Inverted(_) => false,
            Rule::Floor(least) => least < 0,
            _ => true,
        }
    }

    pub(in crate::editor) fn clamp(self, raw: i32, values: EditorMode) -> i32 {
        if values == EditorMode::Raw {
            return raw;
        }

        match self {
            Rule::Percent => raw.clamp(PERCENT_FLOOR, PERCENT_CEILING),
            Rule::Inverted(base) => raw.clamp(PERCENT_FLOOR, base),
            Rule::Ratio(_, _) => raw.max(PERCENT_FLOOR),
            Rule::Floor(least) => raw.max(least),
            _ => raw,
        }
    }

    pub(in crate::editor) fn face(self, raw: i32, values: EditorMode) -> Face {
        if values == EditorMode::Raw {
            return Face::Number;
        }

        match self {
            Rule::Plain
            | Rule::Percent
            | Rule::Floor(_)
            | Rule::Offset(_)
            | Rule::Ratio(_, _)
            | Rule::Inverted(_) => Face::Number,
            Rule::Opaque => Face::Danger,
            Rule::Flag => Toggle::of(raw).map_or(Face::Danger, Face::Toggle),
            Rule::Choice(options) | Rule::Gated(options, _) => options
                .iter()
                .find(|choice| choice.raw == raw)
                .map_or(Face::Danger, |choice| Face::Choice(options, *choice)),
        }
    }
}

fn ratio(value: i32, numerator: i32, denominator: i32) -> i32 {
    let scaled = f64::from(value) * f64::from(numerator) / f64::from(denominator);

    scaled.round() as i32
}

pub(super) fn note(subject: Subject, field: Option<&str>) -> Option<&'static str> {
    let field = field?;

    match subject {
        Subject::Cat => cats::note(field),
        Subject::Enemy => enemies::note(field),
        Subject::Buy
        | Subject::Curve
        | Subject::Talents
        | Subject::Costs
        | Subject::Combo
        | Subject::MapStage
        | Subject::MapDrops
        | Subject::DropChara
        | Subject::ItemBuy => None,
    }
}

pub(super) fn rule(subject: Subject, index: usize, field: Option<&str>, cells: &[i32]) -> Rule {
    match subject {
        Subject::Cat => lookup(field, cats::rule),
        Subject::Enemy => lookup(field, enemies::rule),
        Subject::Buy => lookup(field, unitbuy::rule),
        Subject::Curve => unitlevel::rule(),
        Subject::Talents => talents::rule(index, cells),
        Subject::Costs => costs::rule(index),
        Subject::Combo => lookup(field, combos::rule),
        Subject::MapStage => mapdata::rule(index, field, cells),
        Subject::MapDrops => mapdrops::rule(index),
        Subject::DropChara => mapdrops::chara_rule(index),
        Subject::ItemBuy => lookup(field, itembuy::rule),
    }
}

fn lookup(field: Option<&str>, find: fn(&str) -> Option<Rule>) -> Rule {
    field.and_then(find).unwrap_or(Rule::Opaque)
}

#[cfg(test)]
mod tests {
    use kore::domains::settings::EditorMode;

    use super::{rule, Rule};
    use crate::editor::figures::combat::{cats, enemies};
    use crate::editor::figures::schema::{self, SUBJECTS};

    #[test]
    fn a_quartered_range_is_typed_at_the_distance_it_shows() {
        let rule = Rule::Ratio(1, 4);

        assert_eq!(rule.to_display(1000, EditorMode::Resolved), 250);
        assert_eq!(rule.to_raw(250, EditorMode::Resolved), 1000);
        assert_eq!(rule.to_display(1000, EditorMode::Raw), 1000, "raw stays raw");
    }

    #[test]
    fn a_cost_is_typed_in_the_money_it_is_worth() {
        let rule = Rule::Ratio(3, 2);

        assert_eq!(rule.to_display(50, EditorMode::Resolved), 75);
        assert_eq!(rule.to_raw(75, EditorMode::Resolved), 50);
    }

    #[test]
    fn an_inverted_percent_reverses_in_both_directions() {
        let rule = Rule::Inverted(100);

        assert_eq!(rule.to_display(30, EditorMode::Resolved), 70);
        assert_eq!(rule.to_raw(70, EditorMode::Resolved), 30);
        assert_eq!(rule.clamp(140, EditorMode::Resolved), 100, "a percent cannot exceed its base");
    }

    #[test]
    fn cats_and_enemies_agree_on_the_columns_they_share() {
        for field in ["area_attack", "attack_count_state"] {
            assert_eq!(
                cats::rule(field),
                enemies::rule(field),
                "{field} is declared in both combat tables and the two copies have drifted",
            );
        }
    }

    #[test]
    fn no_scaled_column_is_a_flag() {
        for subject in SUBJECTS {
            for (index, entry) in schema::of(subject).order().iter().enumerate() {
                if !entry.scaled() {
                    continue;
                }

                assert_ne!(
                    rule(subject, index, Some(entry.field), &[]),
                    Rule::Flag,
                    "{subject:?}: {} carries a nyanko Scale, so it is a magnitude and never a flag",
                    entry.field,
                );
            }
        }
    }
}
