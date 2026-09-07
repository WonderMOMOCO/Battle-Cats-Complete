use std::sync::Arc;

use nyanko::common::{cell_value, declared_rows};
use nyanko::graphics::rig::{RigError, SpriteCut};

use super::mamodel::{kept, keep, read, split, Line, DELIMITER};

const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];
const CUT_CELLS: usize = 4;
const NAME_LINE: usize = 2;
const COUNT_LINE: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Named,
    Count,
    Cuts,
    Skip,
}

#[derive(Clone)]
pub struct Imgcut {
    bom: bool,
    lines: Vec<Line>,
    roles: Vec<Option<Role>>,
    raws: Vec<Option<String>>,
    tail: String,
    present: usize,
    declared: usize,
    sheet: String,
    cuts: Arc<Vec<SpriteCut>>,
}

impl Imgcut {
    pub fn parse(bytes: &[u8]) -> Result<Self, RigError> {
        let bom = bytes.starts_with(&BOM);
        let body = String::from_utf8_lossy(if bom { &bytes[BOM.len()..] } else { bytes }).into_owned();

        let lines = split(&body);
        let sheet = lines.get(NAME_LINE).map(|line| line.text.trim().to_owned()).unwrap_or_default();

        let count = lines.get(COUNT_LINE).map_or(0, |line| cell_value(&line.text));
        let declared = declared_rows(count).ok_or(RigError::CountTooLarge)?;

        if declared == 0 {
            return Err(RigError::NoSpriteCuts);
        }

        let mut roles = vec![None; lines.len()];

        for (at, role) in [(NAME_LINE, Role::Named), (COUNT_LINE, Role::Count)] {
            if at < lines.len() {
                roles[at] = Some(role);
            }
        }

        let mut cuts = Vec::with_capacity(declared);
        let mut raws = Vec::with_capacity(declared);
        let mut present = 0;
        let mut tail = String::from("\n");
        let mut carried = SpriteCut::default();

        for at in 0..declared {
            if let Some(line) = lines.get(COUNT_LINE + 1 + at) {
                roles[COUNT_LINE + 1 + at] = Some(if at == 0 { Role::Cuts } else { Role::Skip });
                tail = line.end.clone();
                present = at + 1;

                let row: Vec<&str> = line.text.split(DELIMITER).collect();

                carried = SpriteCut {
                    x: read(row.first().copied()),
                    y: read(row.get(1).copied()),
                    width: read(row.get(2).copied()),
                    height: read(row.get(3).copied()),
                    name: row.get(CUT_CELLS).copied().unwrap_or_default().trim().to_owned(),
                };

                raws.push(Some(line.text.clone()));
            } else {
                raws.push(None);
            }

            cuts.push(carried.clone());
        }

        Ok(Self { bom, lines, roles, raws, tail, present, declared, sheet, cuts: Arc::new(cuts) })
    }

    pub fn sheet(&self) -> &str {
        &self.sheet
    }

    pub(crate) fn set_sheet(&mut self, name: &str) -> bool {
        if self.sheet == name {
            return false;
        }

        self.sheet = name.to_owned();

        true
    }

    pub fn count(&self) -> usize {
        self.cuts.len()
    }

    pub fn cut(&self, at: usize) -> Option<&SpriteCut> {
        self.cuts.get(at)
    }

    pub fn field(&self, at: usize, cell: usize) -> Option<i32> {
        self.cuts.get(at).filter(|_| cell < CUT_CELLS).map(|cut| field(cut, cell))
    }

    pub fn set_field(&mut self, at: usize, cell: usize, value: i32) -> bool {
        if cell >= CUT_CELLS {
            return false;
        }

        let Some(cut) = Arc::make_mut(&mut self.cuts).get_mut(at) else {
            return false;
        };

        if field(cut, cell) == value {
            return false;
        }

        set_field(cut, cell, value);

        true
    }

    pub fn place(&mut self, at: usize, region: [i32; CUT_CELLS]) -> bool {
        let placed: Vec<bool> = (0..CUT_CELLS).map(|cell| self.set_field(at, cell, region[cell])).collect();

        placed.into_iter().any(|changed| changed)
    }

    pub fn name(&self, at: usize) -> Option<&str> {
        self.cuts.get(at).map(|cut| cut.name.as_str())
    }

    pub fn set_name(&mut self, at: usize, name: &str) -> bool {
        let Some(cut) = Arc::make_mut(&mut self.cuts).get_mut(at) else {
            return false;
        };

        if cut.name == name {
            return false;
        }

        cut.name = name.to_owned();

        true
    }

    pub fn add_cut(&mut self) -> usize {
        let cuts = Arc::make_mut(&mut self.cuts);
        cuts.push(SpriteCut::default());
        self.raws.push(None);

        cuts.len() - 1
    }

    pub fn remove_cut(&mut self, at: usize) -> Option<Vec<Option<usize>>> {
        let count = self.cuts.len();

        if at >= count || count == 1 {
            return None;
        }

        let moved: Vec<Option<usize>> =
            (0..count).map(|old| (old != at).then(|| old - usize::from(old > at))).collect();

        Arc::make_mut(&mut self.cuts).remove(at);
        self.raws.remove(at);

        Some(moved)
    }

    pub fn write(&self) -> Vec<u8> {
        let mut body = String::with_capacity(self.lines.iter().map(|line| line.text.len() + 2).sum());

        for (at, line) in self.lines.iter().enumerate() {
            match self.roles.get(at).copied().flatten() {
                Some(Role::Skip) => continue,
                Some(Role::Cuts) => self.push_cuts(&mut body, &line.end),
                Some(Role::Count) => {
                    body.push_str(&count_line(&line.text, self.declared, self.cuts.len()));
                    body.push_str(&line.end);
                }
                Some(Role::Named) => {
                    body.push_str(&keep_text(&line.text, &self.sheet));
                    body.push_str(&line.end);
                }
                None => {
                    body.push_str(&line.text);
                    body.push_str(&line.end);
                }
            }
        }

        let mut bytes = Vec::with_capacity(body.len() + BOM.len());

        if self.bom {
            bytes.extend_from_slice(&BOM);
        }

        bytes.extend_from_slice(body.as_bytes());
        bytes
    }

    fn push_cuts(&self, body: &mut String, end: &str) {
        let count = kept(&self.cuts, self.declared, self.present);
        let last = count.saturating_sub(1);
        let end = if end.is_empty() { "\n" } else { end };

        for (at, cut) in self.cuts.iter().enumerate().take(count) {
            let raw = self.raws.get(at).and_then(Option::as_deref);
            let values = [cut.x, cut.y, cut.width, cut.height];

            body.push_str(&cells(raw, &values, &cut.name));
            body.push_str(if at == last { &self.tail } else { end });
        }
    }
}

pub const CUT_FIELDS: [&str; 5] = ["X", "Y", "W", "H", "Name"];
pub const CUT_NAME_FIELD: usize = CUT_CELLS;


fn field(cut: &SpriteCut, cell: usize) -> i32 {
    match cell {
        0 => cut.x,
        1 => cut.y,
        2 => cut.width,
        _ => cut.height,
    }
}

fn set_field(cut: &mut SpriteCut, cell: usize, value: i32) {
    let held = match cell {
        0 => &mut cut.x,
        1 => &mut cut.y,
        2 => &mut cut.width,
        3 => &mut cut.height,
        _ => return,
    };

    *held = value;
}

fn cells(raw: Option<&str>, values: &[i32; CUT_CELLS], name: &str) -> String {
    let fresh = raw.is_none();
    let raw: Vec<&str> = raw
        .filter(|text| !text.is_empty())
        .map_or_else(Vec::new, |text| text.split(DELIMITER).collect());

    let mut count = match fresh {
        true => CUT_CELLS + usize::from(!name.is_empty()),
        false => raw.len(),
    };

    for (at, value) in values.iter().enumerate() {
        if read(raw.get(at).copied()) != *value {
            count = count.max(at + 1);
        }
    }

    if raw.get(CUT_CELLS).copied().unwrap_or_default().trim() != name {
        count = count.max(CUT_CELLS + 1);
    }

    let written: Vec<String> = (0..count)
        .map(|at| match values.get(at) {
            Some(value) => keep(raw.get(at).copied(), *value),
            None if at == CUT_CELLS => raw
                .get(at)
                .filter(|text| text.trim() == name)
                .map_or_else(|| name.to_owned(), |text| (*text).to_owned()),
            None => raw.get(at).copied().unwrap_or_default().to_owned(),
        })
        .collect();

    written.join(&DELIMITER.to_string())
}

fn count_line(raw: &str, was: usize, now: usize) -> String {
    if was == now {
        return raw.to_owned();
    }

    let declared = usize::try_from(cell_value(raw)).unwrap_or(was);

    declared.saturating_add(now).saturating_sub(was).to_string()
}

fn keep_text(raw: &str, text: &str) -> String {
    match raw.trim() == text {
        true => raw.to_owned(),
        false => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[imgcut]\n2\n044_f.png\n3\n0,0,64,64,head\n64,0,32,48,\t arm \t\n0,64,128,16\n";

    fn round_trip(source: &str) -> String {
        let doc = Imgcut::parse(source.as_bytes()).expect("the sample parses");

        String::from_utf8(doc.write()).expect("the output is text")
    }

    #[test]
    fn an_untouched_cut_list_writes_back_byte_for_byte() {
        assert_eq!(round_trip(SAMPLE), SAMPLE);
    }

    #[test]
    fn zero_padding_and_carriage_returns_both_survive() {
        let padded = SAMPLE.replace("64,0,32,48", "064,000,032,048").replace('\n', "\r\n");

        assert_eq!(round_trip(&padded), padded);
    }

    #[test]
    fn a_file_that_ends_without_a_newline_is_not_given_one() {
        // 240 vanilla cut lists end on their last row; adding a newline rewrites them all.
        let bare = SAMPLE.trim_end_matches('\n');

        assert_eq!(round_trip(bare), bare);
    }

    #[test]
    fn placing_a_cut_rewrites_only_its_own_row() {
        let mut doc = Imgcut::parse(SAMPLE.as_bytes()).expect("the sample parses");

        assert!(doc.place(1, [10, 20, 30, 40]));

        let written = round_trip_of(&doc);

        assert!(written.contains("\n10,20,30,40,\t arm \t\n"), "{}", written);
        assert!(written.contains("\n0,0,64,64,head\n"), "{}", written);
    }

    #[test]
    fn a_row_with_no_name_does_not_grow_one() {
        let mut doc = Imgcut::parse(SAMPLE.as_bytes()).expect("the sample parses");

        assert!(doc.set_field(2, 2, 100));
        assert!(round_trip_of(&doc).ends_with("\n0,64,100,16\n"));
    }

    #[test]
    fn removing_a_cut_renumbers_the_ones_after_it_and_updates_the_count() {
        let mut doc = Imgcut::parse(SAMPLE.as_bytes()).expect("the sample parses");
        let moved = doc.remove_cut(0).expect("the cut exists");

        assert_eq!(moved, vec![None, Some(0), Some(1)]);
        assert_eq!(doc.count(), 2);

        let written = round_trip_of(&doc);

        assert!(written.starts_with("[imgcut]\n2\n044_f.png\n2\n64,0,32,48"), "{}", written);
    }

    #[test]
    fn the_last_cut_cannot_be_removed() {
        let mut doc = Imgcut::parse("[imgcut]\n2\nx.png\n1\n0,0,4,4\n".as_bytes()).expect("it parses");

        assert!(doc.remove_cut(0).is_none());
    }

    fn round_trip_of(doc: &Imgcut) -> String {
        String::from_utf8(doc.write()).expect("the output is text")
    }
}
