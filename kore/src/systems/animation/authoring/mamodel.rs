use std::sync::Arc;

use nyanko::common::cell_value;
use nyanko::graphics::rig::{Alignment, Model, ModelPart, RigError};

const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];
pub(super) const DELIMITER: char = ',';
const VERSION_LINE: usize = 1;
const COUNT_LINE: usize = 2;
const GLOW_CELL: usize = 12;
const GLOW_VERSION: i32 = 2;
const UNITED_VERSION: i32 = 1;
const ALIGNED_VERSION: i32 = 3;
const SHEET_FIELD: usize = 1;
const PART_CELLS: usize = 13;
const GLOW_MODES: i32 = 3;
const NO_PARENT: i32 = -1;
const NOT_DRAWN: i32 = -1;

#[derive(Clone)]
pub(super) struct Line {
    pub(super) text: String,
    pub(super) end: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Version,
    Count,
    Parts,
    Units,
    AlignCount,
    Aligns,
    Skip,
}

struct Layout {
    roles: Vec<Option<Role>>,
    raws: Vec<Option<String>>,
    rows: Vec<Option<String>>,
    tails: [String; 2],
    present: [usize; 2],
}

#[derive(Clone)]
pub struct Mamodel {
    bom: bool,
    lines: Vec<Line>,
    roles: Vec<Option<Role>>,
    raws: Vec<Option<String>>,
    rows: Vec<Option<String>>,
    tails: [String; 2],
    present: [usize; 2],
    version: i32,
    parts: usize,
    aligns: usize,
    model: Arc<Model>,
}

impl Mamodel {
    pub fn parse(bytes: &[u8]) -> Result<Self, RigError> {
        let model = Model::parse(bytes)?;

        let bom = bytes.starts_with(&BOM);
        let body = String::from_utf8_lossy(if bom { &bytes[BOM.len()..] } else { bytes }).into_owned();

        let lines = split(&body);
        let Layout { roles, raws, rows, tails, present } = roles(&lines, &model);
        let (parts, aligns) = (model.parts.len(), model.alignment.len());
        let version = model.version;

        Ok(Self { bom, lines, roles, raws, rows, tails, present, version, parts, aligns, model: Arc::new(model) })
    }

    pub fn shared(&self) -> Arc<Model> {
        Arc::clone(&self.model)
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn count(&self) -> usize {
        self.model.parts.len()
    }

    pub fn field(&self, part: usize, at: usize) -> Option<i32> {
        self.model.parts.get(part).filter(|_| at < PART_CELLS).map(|part| field(part, at))
    }

    pub fn set_field(&mut self, part: usize, at: usize, value: i32) -> bool {
        if at >= PART_CELLS {
            return false;
        }

        let Some(held) = Arc::make_mut(&mut self.model).parts.get_mut(part) else {
            return false;
        };

        if field(held, at) == value {
            return false;
        }

        set_field(held, at, value);

        true
    }

    pub fn name(&self, part: usize) -> Option<&str> {
        self.model.parts.get(part).map(|part| part.name.as_str())
    }

    pub fn restamp(&mut self, unit: i32) -> usize {
        let borrowed: Vec<usize> = (0..self.count())
            .filter(|part| self.field(*part, SHEET_FIELD).is_some_and(|id| id >= 0 && id != unit))
            .collect();

        borrowed.iter().filter(|part| self.set_field(**part, SHEET_FIELD, unit)).count()
    }

    pub fn set_name(&mut self, part: usize, name: &str) -> bool {
        let Some(part) = Arc::make_mut(&mut self.model).parts.get_mut(part) else {
            return false;
        };

        if part.name == name {
            return false;
        }

        part.name = name.to_owned();

        true
    }

    pub fn parent(&self, part: usize) -> Option<usize> {
        self.model.parts.get(part).and_then(|part| usize::try_from(part.parent).ok())
    }

    pub fn offsets(&self) -> usize {
        self.model.alignment.len()
    }

    pub fn alignable(&self) -> bool {
        self.model.version >= UNITED_VERSION && self.roles.contains(&Some(Role::Version))
    }

    pub fn add_offset(&mut self) -> Option<usize> {
        if !self.alignable() {
            return None;
        }

        let model = Arc::make_mut(&mut self.model);
        let anchor = model.alignment.last().map_or(0, |row| row.part);

        model.version = model.version.max(ALIGNED_VERSION);
        model.alignment.push(Alignment { part: anchor, ..Alignment::default() });
        self.rows.push(None);

        Some(model.alignment.len() - 1)
    }

    pub fn remove_offset(&mut self, row: usize) -> bool {
        if row >= self.model.alignment.len() {
            return false;
        }

        Arc::make_mut(&mut self.model).alignment.remove(row);

        if row < self.rows.len() {
            self.rows.remove(row);
        }

        true
    }

    pub fn offset(&self, row: usize) -> Option<(i32, i32)> {
        self.model.alignment.get(row).map(|row| (row.x, row.y))
    }

    pub fn offset_name(&self, row: usize) -> Option<&str> {
        self.model.alignment.get(row).map(|row| row.name.as_str())
    }

    pub fn set_offset(&mut self, row: usize, axis: usize, value: i32) -> bool {
        let Some(row) = Arc::make_mut(&mut self.model).alignment.get_mut(row) else {
            return false;
        };

        let cell = if axis == 0 { &mut row.x } else { &mut row.y };

        if *cell == value {
            return false;
        }

        *cell = value;

        true
    }

    pub fn add_part(&mut self, parent: Option<usize>) -> usize {
        let seeded = blank_part(parent, &self.model);
        let model = Arc::make_mut(&mut self.model);

        model.parts.push(seeded);
        self.raws.push(None);

        model.parts.len() - 1
    }

    pub fn remove_part(&mut self, at: usize) -> Option<Vec<Option<usize>>> {
        let count = self.model.parts.len();

        if at >= count {
            return None;
        }

        let inherited = self.model.parts[at].parent;
        let moved: Vec<Option<usize>> =
            (0..count).map(|old| (old != at).then(|| old - usize::from(old > at))).collect();

        let model = Arc::make_mut(&mut self.model);

        for part in model.parts.iter_mut() {
            if i32::try_from(at) == Ok(part.parent) {
                part.parent = inherited;
            }
        }

        model.parts.remove(at);
        self.raws.remove(at);

        for part in model.parts.iter_mut() {
            part.parent = remap(&moved, part.parent);
        }

        Some(moved)
    }

    pub fn retarget_sprites(&mut self, moved: &[Option<usize>]) -> bool {
        let shifted = self.model.parts.iter().any(|part| {
            usize::try_from(part.sprite)
                .ok()
                .and_then(|at| moved.get(at))
                .is_some_and(|landed| landed.and_then(|at| i32::try_from(at).ok()) != Some(part.sprite))
        });

        if !shifted {
            return false;
        }

        for part in Arc::make_mut(&mut self.model).parts.iter_mut() {
            let Some(landed) = usize::try_from(part.sprite).ok().and_then(|at| moved.get(at)) else {
                continue;
            };

            part.sprite = landed.and_then(|at| i32::try_from(at).ok()).unwrap_or(NOT_DRAWN);
        }

        true
    }

    pub fn reparent(&mut self, at: usize, parent: Option<usize>) -> bool {
        let wanted = parent.and_then(|at| i32::try_from(at).ok()).unwrap_or(NO_PARENT);

        if self.model.parts.get(at).is_none_or(|part| part.parent == wanted) {
            return false;
        }

        if parent.is_some_and(|parent| self.descends(parent, at)) {
            return false;
        }

        if let Some(part) = Arc::make_mut(&mut self.model).parts.get_mut(at) {
            part.parent = wanted;
        }

        true
    }

    pub fn descends(&self, mut at: usize, of: usize) -> bool {
        for _ in 0..self.model.parts.len() {
            if at == of {
                return true;
            }

            let Some(parent) = self.parent(at).filter(|parent| *parent != at) else {
                return false;
            };

            at = parent;
        }

        false
    }

    pub fn write(&self) -> Vec<u8> {
        let mut body = String::with_capacity(self.lines.iter().map(|line| line.text.len() + 2).sum());
        let seam = self.seam();

        for (at, line) in self.lines.iter().enumerate() {
            match self.roles.get(at).copied().flatten() {
                Some(Role::Skip) => continue,
                Some(Role::Parts) => self.push_parts(&mut body, &line.end),
                Some(Role::Aligns) => self.push_aligns(&mut body, &line.end),
                Some(Role::AlignCount) => {
                    let seeding = self.aligns == 0 && !self.model.alignment.is_empty();
                    let end = if line.end.is_empty() && seeding { "\n" } else { &line.end };

                    body.push_str(&self.render(Role::AlignCount, &line.text));
                    body.push_str(end);

                    if seeding {
                        self.push_aligns(&mut body, end);
                    }
                }
                Some(role) => {
                    body.push_str(&self.render(role, &line.text));
                    body.push_str(&line.end);
                }
                None => {
                    body.push_str(&line.text);
                    body.push_str(&line.end);
                }
            }

            if Some(at) == seam {
                self.push_block(&mut body);
            }
        }

        if self.creating() && seam.is_none() {
            terminate(&mut body);
            body.push_str(&cells(None, &self.units(), None));
            body.push('\n');

            self.push_block(&mut body);
        }

        let mut bytes = Vec::with_capacity(body.len() + BOM.len());

        if self.bom {
            bytes.extend_from_slice(&BOM);
        }

        bytes.extend_from_slice(body.as_bytes());
        bytes
    }

    fn creating(&self) -> bool {
        let grown = self.model.alignment.len() > self.aligns || self.model.version > self.version;

        grown && !self.roles.contains(&Some(Role::AlignCount))
    }

    fn seam(&self) -> Option<usize> {
        if !self.creating() {
            return None;
        }

        self.roles.iter().position(|role| *role == Some(Role::Units))
    }

    fn push_block(&self, body: &mut String) {
        terminate(body);

        body.push_str(&self.model.alignment.len().to_string());
        body.push('\n');

        self.push_aligns(body, "\n");
    }

    fn push_parts(&self, body: &mut String, end: &str) {
        let count = kept(&self.model.parts, self.parts, self.present[0]);
        let last = count.saturating_sub(1);
        let end = if end.is_empty() { "\n" } else { end };

        for (at, part) in self.model.parts.iter().enumerate().take(count) {
            let raw = self.raws.get(at).and_then(Option::as_deref);
            let mut values: Vec<i32> = (0..PART_CELLS).map(|cell| field(part, cell)).collect();

            if self.model.version < GLOW_VERSION && part.glow == 0 {
                values[GLOW_CELL] = read(raw.and_then(|text| text.split(DELIMITER).nth(GLOW_CELL)));
            }

            body.push_str(&cells(raw, &values, Some(part.name.as_str())));
            body.push_str(if at == last { &self.tails[0] } else { end });
        }
    }

    fn push_aligns(&self, body: &mut String, end: &str) {
        let count = kept(&self.model.alignment, self.aligns, self.present[1]);
        let last = count.saturating_sub(1);
        let end = if end.is_empty() { "\n" } else { end };

        for (at, row) in self.model.alignment.iter().enumerate().take(count) {
            let raw = self.rows.get(at).and_then(Option::as_deref);
            let values = [row.part, row.unknown_1, row.x, row.y, row.unknown_4, row.unknown_5];

            body.push_str(&cells(raw, &values, Some(row.name.as_str())));
            body.push_str(if at == last { &self.tails[1] } else { end });
        }
    }

    fn render(&self, role: Role, raw: &str) -> String {
        match role {
            Role::Version if cell_value(raw) == self.model.version => raw.to_owned(),
            Role::Version => self.model.version.to_string(),
            Role::Count => count_line(raw, self.parts, self.model.parts.len()),
            Role::AlignCount => count_line(raw, self.aligns, self.model.alignment.len()),
            Role::Units => self.units_row(raw),
            _ => raw.to_owned(),
        }
    }

    fn units_row(&self, raw: &str) -> String {
        cells(Some(raw), &self.units(), None)
    }

    fn units(&self) -> Vec<i32> {
        let mut values = vec![self.model.scale_unit, self.model.angle_unit, self.model.opacity_unit];

        if let Some(extra) = self.model.unknown_3 {
            values.push(extra);
        }

        values
    }
}

pub const FIELDS: [&str; 14] = [
    "Parent", "Sheet ID", "Sprite", "Z Order", "X", "Y", "Pivot X", "Pivot Y", "Scale X", "Scale Y",
    "Angle", "Opacity", "Glow", "Name",
];

pub const NAME_FIELD: usize = PART_CELLS;

pub fn bound(model: &Model, cuts: usize, at: usize, value: i32) -> i32 {
    let ceiling = |count: usize| i32::try_from(count).unwrap_or(i32::MAX).saturating_sub(1);

    match at {
        0 => value.clamp(NO_PARENT, ceiling(model.parts.len())),
        2 => value.clamp(NOT_DRAWN, ceiling(cuts).max(NOT_DRAWN)),
        11 => value.clamp(0, model.opacity_unit.max(0)),
        12 => value.clamp(0, GLOW_MODES),
        _ => value,
    }
}

pub fn nameable(text: &str) -> bool {
    !text.chars().any(|glyph| matches!(glyph, ',' | '|' | '\t' | '\n' | '\r'))
}

pub fn defaults(model: &Model) -> [i32; PART_CELLS] {
    let mut cells = [0; PART_CELLS];

    cells[0] = NO_PARENT;
    cells[1] = drawn_id(model);
    cells[8] = model.scale_unit;
    cells[9] = model.scale_unit;
    cells[11] = model.opacity_unit;

    cells
}

fn blank_part(parent: Option<usize>, model: &Model) -> ModelPart {
    let anchor = parent.and_then(|at| model.parts.get(at));
    let cells = defaults(model);
    let mut part = ModelPart::default();

    for (at, value) in cells.iter().enumerate() {
        set_field(&mut part, at, *value);
    }

    part.parent = parent.and_then(|at| i32::try_from(at).ok()).unwrap_or(NO_PARENT);
    part.z = anchor.map_or(cells[3], |anchor| anchor.z.saturating_add(1));

    part
}

fn drawn_id(model: &Model) -> i32 {
    model.parts.iter().map(|part| part.id).find(|id| *id != NOT_DRAWN).unwrap_or(NOT_DRAWN)
}

fn field(part: &ModelPart, at: usize) -> i32 {
    match at {
        0 => part.parent,
        1 => part.id,
        2 => part.sprite,
        3 => part.z,
        4 => part.x,
        5 => part.y,
        6 => part.pivot_x,
        7 => part.pivot_y,
        8 => part.scale_x,
        9 => part.scale_y,
        10 => part.angle,
        11 => part.opacity,
        _ => part.glow,
    }
}

fn set_field(part: &mut ModelPart, at: usize, value: i32) {
    let cell = match at {
        0 => &mut part.parent,
        1 => &mut part.id,
        2 => &mut part.sprite,
        3 => &mut part.z,
        4 => &mut part.x,
        5 => &mut part.y,
        6 => &mut part.pivot_x,
        7 => &mut part.pivot_y,
        8 => &mut part.scale_x,
        9 => &mut part.scale_y,
        10 => &mut part.angle,
        11 => &mut part.opacity,
        12 => &mut part.glow,
        _ => return,
    };

    *cell = value;
}

fn remap(moved: &[Option<usize>], parent: i32) -> i32 {
    let Ok(at) = usize::try_from(parent) else {
        return parent;
    };

    match moved.get(at) {
        Some(Some(landed)) => i32::try_from(*landed).unwrap_or(NO_PARENT),
        Some(None) => NO_PARENT,
        None => parent,
    }
}

pub(super) fn split(body: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut rest = body;

    while !rest.is_empty() {
        let (chunk, tail) = rest.split_once('\n').unwrap_or((rest, ""));
        let trimmed = chunk.trim_end_matches(['\r', '\n']);
        let text = if trimmed.is_empty() { chunk } else { trimmed };

        lines.push(Line {
            text: text.to_owned(),
            end: rest[text.len()..rest.len() - tail.len()].to_owned(),
        });

        rest = tail;
    }

    lines
}

fn roles(lines: &[Line], model: &Model) -> Layout {
    let mut roles = vec![None; lines.len()];
    let mut cursor = COUNT_LINE;

    if VERSION_LINE < lines.len() {
        roles[VERSION_LINE] = Some(Role::Version);
    }

    if cursor < lines.len() {
        roles[cursor] = Some(Role::Count);
    }

    cursor += 1;

    let (raws, held, tail) = block(lines, cursor, model.parts.len(), Role::Parts, &mut roles);
    cursor += model.parts.len();

    let mut rows = Vec::new();
    let mut aligned = 0;
    let mut trailing = String::from("\n");

    if model.version >= UNITED_VERSION {
        if cursor < lines.len() {
            roles[cursor] = Some(Role::Units);
        }

        cursor += 1;
    }

    if model.version >= ALIGNED_VERSION {
        if cursor < lines.len() {
            roles[cursor] = Some(Role::AlignCount);
        }

        cursor += 1;

        (rows, aligned, trailing) = block(lines, cursor, model.alignment.len(), Role::Aligns, &mut roles);
    }

    Layout { roles, raws, rows, tails: [tail, trailing], present: [held, aligned] }
}

pub(super) fn block(
    lines: &[Line],
    cursor: usize,
    count: usize,
    lead: Role,
    roles: &mut [Option<Role>],
) -> (Vec<Option<String>>, usize, String) {
    let mut raws = Vec::with_capacity(count);
    let mut present = 0;
    let mut tail = String::from("\n");

    for at in 0..count {
        let Some(line) = lines.get(cursor + at) else {
            raws.push(None);

            continue;
        };

        roles[cursor + at] = Some(if at == 0 { lead } else { Role::Skip });
        tail = line.end.clone();
        raws.push(Some(line.text.clone()));
        present = at + 1;
    }

    (raws, present, tail)
}

pub(super) fn kept<T: Default + PartialEq>(held: &[T], parsed: usize, present: usize) -> usize {
    if held.len() != parsed || present >= parsed {
        return held.len();
    }

    let fallback = T::default();
    let repeated = present.checked_sub(1).and_then(|at| held.get(at)).unwrap_or(&fallback);

    match held[present..].iter().all(|row| row == repeated) {
        true => present,
        false => held.len(),
    }
}

fn cells(raw: Option<&str>, values: &[i32], name: Option<&str>) -> String {
    let fresh = raw.is_none();
    let raw: Vec<&str> = raw
        .filter(|text| !text.is_empty())
        .map_or_else(Vec::new, |text| text.split(DELIMITER).collect());

    let mut count = match fresh {
        true => values.len() + usize::from(name.is_some_and(|name| !name.is_empty())),
        false => raw.len(),
    };

    for (at, value) in values.iter().enumerate() {
        if read(raw.get(at).copied()) != *value {
            count = count.max(at + 1);
        }
    }

    if let Some(name) = name
        && raw.get(values.len()).copied().unwrap_or_default().trim() != name
    {
        count = count.max(values.len() + 1);
    }

    let written: Vec<String> = (0..count)
        .map(|at| match (values.get(at), name) {
            (Some(value), _) => keep(raw.get(at).copied(), *value),
            (None, Some(name)) if at == values.len() => raw
                .get(at)
                .filter(|text| text.trim() == name)
                .map_or_else(|| name.to_owned(), |text| (*text).to_owned()),
            _ => raw.get(at).copied().unwrap_or_default().to_owned(),
        })
        .collect();

    written.join(&DELIMITER.to_string())
}

fn terminate(body: &mut String) {
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
}

fn count_line(raw: &str, was: usize, now: usize) -> String {
    if was == now {
        return raw.to_owned();
    }

    let declared = usize::try_from(cell_value(raw)).unwrap_or(was);

    declared.saturating_add(now).saturating_sub(was).to_string()
}

pub(super) fn read(cell: Option<&str>) -> i32 {
    cell.map_or(0, cell_value)
}

pub(super) fn keep(cell: Option<&str>, value: i32) -> String {
    cell.filter(|text| read(Some(text)) == value)
        .map_or_else(|| value.to_string(), |text| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[modelanim:model]\n3\n2\n-1,0,0,0,0,0,0,0,1000,1000,0,1000,0,body\n0,0,1,400,10,-20,5,5,1000,1000,0,1000,0,\t head \t\n1000,3600,1000\n1\n0,0,-30,120,1000,1000,align\n";

    fn round_trip(source: &str) -> String {
        let doc = Mamodel::parse(source.as_bytes()).expect("the sample parses");

        String::from_utf8(doc.write()).expect("the output is text")
    }

    #[test]
    fn an_untouched_model_writes_back_byte_for_byte() {
        assert_eq!(round_trip(SAMPLE), SAMPLE);
    }

    #[test]
    fn a_zero_padded_number_survives_because_the_raw_field_is_kept() {
        // Nine vanilla files pad their integers; rewriting them numerically would
        // change bytes the engine never asked us to touch.
        let padded = SAMPLE.replace("0,0,1,400,", "0,000,001,0400,");

        assert_eq!(round_trip(&padded), padded);
    }

    #[test]
    fn a_byte_order_mark_and_carriage_returns_are_both_kept() {
        let windows: String = SAMPLE.replace('\n', "\r\n");
        let with_bom: Vec<u8> = BOM.iter().copied().chain(windows.bytes()).collect();
        let doc = Mamodel::parse(&with_bom).expect("the sample parses");

        assert_eq!(doc.write(), with_bom);
    }

    #[test]
    fn a_line_past_the_alignment_block_is_kept_verbatim() {
        let odd = format!("{}999,junk\n", SAMPLE);

        assert_eq!(round_trip(&odd), odd);
    }

    #[test]
    fn a_blank_line_is_a_row_like_any_other() {
        // nyanko reads it as the units row rather than skipping it, so the writer has
        // to agree on where the blocks after it start.
        let blanked = SAMPLE.replace("1000,3600,1000\n", "\n1000,3600,1000\n");
        let doc = Mamodel::parse(blanked.as_bytes()).expect("the sample parses");

        assert_eq!(doc.model().scale_unit, 0);
        assert_eq!(String::from_utf8(doc.write()).expect("the output is text"), blanked);
    }

    #[test]
    fn a_count_larger_than_the_rows_present_writes_back_the_rows_it_had() {
        // 17 modder files declare one; nyanko repeats the last row it read to fill the
        // gap, and materialising those repeats would rewrite a file nobody edited.
        let truncated = "[modelanim:model]\n1\n4\n-1,0,0,0,0,0,0,0,1000,1000,0,1000,0,body\n";

        assert_eq!(round_trip(truncated), truncated);
    }

    #[test]
    fn editing_one_field_rewrites_only_that_field() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        assert!(doc.set_field(1, 3, 500));

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("0,0,1,500,10,-20,5,5,1000,1000,0,1000,0,\t head \t\n"));
        assert!(written.contains("-1,0,0,0,0,0,0,0,1000,1000,0,1000,0,body\n"));
    }

    #[test]
    fn renaming_a_part_drops_the_padding_it_no_longer_matches() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        assert!(doc.set_name(1, "arm"));

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.ends_with("0,1000,0,arm\n1000,3600,1000\n1\n0,0,-30,120,1000,1000,align\n"));
    }

    #[test]
    fn an_added_part_lands_after_the_block_and_updates_the_count() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");

        assert_eq!(doc.add_part(Some(0)), 2);

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("\n3\n-1,0,0,"), "{}", written);
        assert!(written.contains("\t head \t\n0,0,0,1,0,0,0,0,1000,1000,0,1000,0\n1000,3600,1000\n"), "{}", written);
    }

    #[test]
    fn moving_the_alignment_row_rewrites_only_its_own_columns() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        assert!(doc.set_offset(0, 0, 44));

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.ends_with("0,0,44,120,1000,1000,align\n"));
    }

    #[test]
    fn a_first_offset_lands_under_a_count_line_that_had_no_rows() {
        let bare = SAMPLE.replace("1\n0,0,-30,120,1000,1000,align\n", "0\n");
        let mut doc = Mamodel::parse(bare.as_bytes()).expect("the sample parses");

        assert_eq!(doc.add_offset(), Some(0));
        assert!(round_trip_of(&doc).ends_with("1000,3600,1000\n1\n0,0,0,0,0,0\n"), "{}", round_trip_of(&doc));
    }

    #[test]
    fn a_model_below_the_aligned_revision_gains_the_block_and_the_revision_with_it() {
        // Revision 2 declares no block, so the first offset has to raise the revision as
        // well as write the rows; 2 to 3 changes nothing else the reader looks at.
        let old = "[modelanim:model]\n2\n1\n-1,0,0,0,0,0,0,0,1000,1000,0,1000,0,body\n1000,3600,1000\n";
        let mut doc = Mamodel::parse(old.as_bytes()).expect("the sample parses");

        assert_eq!(round_trip(old), old, "an untouched one is left alone");
        assert_eq!(doc.add_offset(), Some(0));

        let written = round_trip_of(&doc);

        assert!(written.starts_with("[modelanim:model]\n3\n1\n"), "{}", written);
        assert!(written.ends_with("1000,3600,1000\n1\n0,0,0,0,0,0\n"), "{}", written);
        assert_eq!(Model::parse(&written).map(|held| held.alignment.len()), Ok(1));
    }

    #[test]
    fn a_file_that_stops_before_its_offset_count_still_gains_one() {
        let cut = "[modelanim:model]\n3\n1\n-1,0,0,0,0,0,0,0,1000,1000,0,1000,0,body\n0,0,0\n";
        let mut doc = Mamodel::parse(cut.as_bytes()).expect("the sample parses");

        assert_eq!(round_trip(cut), cut, "an untouched one is left alone");
        assert_eq!(doc.add_offset(), Some(0));
        assert!(round_trip_of(&doc).ends_with("body\n0,0,0\n1\n0,0,0,0,0,0\n"), "{}", round_trip_of(&doc));
    }

    #[test]
    fn a_created_block_lands_where_the_reader_looks_not_at_the_end_of_the_file() {
        // The old seed declared revision 1 and still trailed a block the reader never
        // reached. Appending after that made the dead lines live on the next parse and
        // buried the rows the editor had just written.
        let seeded = "[modelanim:model]\n1\n1\n-1,0,0,0,0,0,32,32,1000,1000,0,1000,0,box\n1000,3600,1000\n2\n0,0,9,9,0,0,combat\n0,0,9,9,0,0,gacha\n";
        let mut doc = Mamodel::parse(seeded.as_bytes()).expect("the sample parses");

        assert_eq!(round_trip(seeded), seeded, "an untouched one is left alone");
        assert_eq!(doc.add_offset(), Some(0));
        assert!(doc.set_offset(0, 0, 32) && doc.set_offset(0, 1, 64));

        let written = round_trip_of(&doc);
        let back = Model::parse(&written).expect("it reads back");

        assert_eq!(back.alignment.len(), 1, "{}", written);
        assert_eq!((back.alignment[0].x, back.alignment[0].y), (32, 64), "{}", written);
        assert!(written.contains("0,0,9,9,0,0,combat"), "the old lines are still kept: {}", written);
    }

    #[test]
    fn a_model_with_no_units_row_of_its_own_declares_no_offsets() {
        // Revision 3 makes the engine read a units row this file never wrote, so raising
        // it to reach the block would change every part's scale.
        let old = SAMPLE.replace("model]\n3\n", "model]\n0\n");
        let mut doc = Mamodel::parse(old.as_bytes()).expect("the sample parses");

        assert!(!doc.alignable());
        assert_eq!(doc.add_offset(), None);
    }

    #[test]
    fn a_new_offset_names_the_part_the_row_before_it_does() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");

        assert_eq!(doc.add_offset(), Some(1));
        assert!(round_trip_of(&doc).ends_with("0,0,-30,120,1000,1000,align\n0,0,0,0,0,0\n"));
    }

    #[test]
    fn removing_the_last_offset_takes_the_block_and_its_count_down_with_it() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");

        assert!(doc.remove_offset(0));
        assert!(!doc.remove_offset(0), "there is nothing left to take");
        assert!(round_trip_of(&doc).ends_with("1000,3600,1000\n0\n"), "{}", round_trip_of(&doc));
    }

    fn round_trip_of(doc: &Mamodel) -> String {
        String::from_utf8(doc.write()).expect("the output is text")
    }

    #[test]
    fn a_row_with_no_name_field_does_not_grow_one() {
        let nameless = SAMPLE.replace(",0,body\n", ",0\n");

        assert_eq!(round_trip(&nameless), nameless);
    }

    #[test]
    fn a_delimiter_can_never_enter_a_name() {
        // A bar would flip nyanko's reader to pipe mode and mis-parse the file.
        assert!(nameable("left arm"));
        assert!(!nameable("left,arm"));
        assert!(!nameable("left|arm"));
        assert!(!nameable("left\narm"));
    }

    #[test]
    fn a_new_part_inherits_the_unit_id_and_draws_in_front_of_its_parent() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        let added = doc.add_part(Some(1));

        assert_eq!(doc.field(added, 0), Some(1));
        assert_eq!(doc.field(added, 1), Some(0), "the unit id is taken from the first drawn part");
        assert_eq!(doc.field(added, 2), Some(0));
        assert_eq!(doc.field(added, 3), Some(401), "one layer in front of its parent's 400");
        assert_eq!(doc.field(added, 8), Some(1000));
        assert_eq!(doc.field(added, 11), Some(1000));
    }

    #[test]
    fn removing_a_part_hands_its_children_to_its_own_parent_and_renumbers() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");
        doc.add_part(Some(1));

        let moved = doc.remove_part(1).expect("the part exists");

        assert_eq!(moved, vec![Some(0), None, Some(1)]);
        assert_eq!(doc.count(), 2);
        assert_eq!(doc.field(1, 0), Some(0), "the orphan inherits the removed part's own parent");
    }

    #[test]
    fn a_part_cannot_be_reparented_under_its_own_descendant() {
        let mut doc = Mamodel::parse(SAMPLE.as_bytes()).expect("the sample parses");

        assert!(!doc.reparent(0, Some(1)), "part 1 already hangs off part 0");
        assert!(!doc.reparent(0, Some(0)), "nor can a part hang off itself");
    }

    #[test]
    fn only_the_fields_the_engine_bounds_are_clamped() {
        let model = Model { parts: vec![ModelPart::default(); 4], opacity_unit: 1000, ..Model::default() };

        assert_eq!(bound(&model, 9, 0, 12), 3, "parent stops at the last part");
        assert_eq!(bound(&model, 9, 0, -7), -1, "and at the root sentinel");
        assert_eq!(bound(&model, 9, 2, 40), 8, "sprite stops at the last cut");
        assert_eq!(bound(&model, 9, 11, 4000), 1000, "opacity stops at its own unit");
        assert_eq!(bound(&model, 9, 12, 9), 3, "glow stops at the last blending mode");
        assert_eq!(bound(&model, 9, 3, i32::MAX), i32::MAX, "z order has no bound at all");
        assert_eq!(bound(&model, 9, 4, i32::MIN), i32::MIN, "and neither does an offset");
    }
}
