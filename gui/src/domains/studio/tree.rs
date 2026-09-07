use nyanko::graphics::rig::AnimModification;

use super::*;
use iced::widget::{container, row, space, text};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Held {
    pub(super) part: i32,
    pub(super) kind: i32,
    pub(super) ordinal: usize,
}

#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) picked: Option<usize>,
    pub(super) by_part: bool,
    pub(super) width: f32,
    pub(super) font: Font,
}

pub(super) struct TreeRow {
    pub(super) label: String,
    pub(super) depth: u16,
    pub(super) mark: &'static str,
    pub(super) part: Option<usize>,
    pub(super) owner: Option<usize>,
    pub(super) track: Option<usize>,
    pub(super) warn: bool,
    pub(super) bucket: bool,
    pub(super) alarm: Option<Alarm>,
    pub(super) guide: Guide,
}

impl TreeRow {
    pub(super) fn cargo(&self) -> Option<Cargo> {
        match (self.part, self.track) {
            (Some(part), _) => Some(Cargo::Part(part)),
            (None, Some(track)) => Some(Cargo::Track(track)),
            _ => None,
        }
    }

    pub(super) fn span(&self) -> f32 {
        ROW_PADDING * 2.0
            + MARKER_WIDTH
            + INDENT * f32::from(self.depth)
            + CHAR_WIDTH * glyphs::columns(&self.label)
    }

    pub(super) fn view(
        &self,
        index: usize,
        frame: Frame,
        carried: bool,
        onto: Option<Mark>,
    ) -> Element<'_, Message> {
        let Frame { picked, by_part, width, font } = frame;
        let alarm = self.alarm;
        let label = text(self.label.as_str())
            .font(font)
            .size(TREE_TEXT_SIZE)
            .shaping(glyphs::shaping(&self.label))
            .wrapping(text::Wrapping::None);

        let label = if self.warn { label.style(text::danger) } else { label };

        let stem = branches(self.guide, self.depth, INDENT, ROW_HEIGHT, MARKER_SIZE);

        let (stem, body) = match self.mark {
            "" => (stem.reach(MARKER_WIDTH), row![label]),
            FOLDER_OPEN => (stem.opened(), row![marker(self.mark), label]),
            _ => (stem, row![marker(self.mark), label]),
        };

        let span = stem.span();

        let landed = container(body.align_y(Vertical::Center))
            .width(Length::Fill)
            .height(Length::Fixed(ROW_HEIGHT))
            .align_y(Vertical::Center)
            .style(move |theme: &Theme| seat(theme, carried, onto, alarm));

        let content = container(landed)
            .height(Length::Fixed(ROW_HEIGHT))
            .align_y(Vertical::Center)
            .padding(Padding::default().left(stem.inset()).right(ROW_PADDING));

        let held = if by_part { self.part } else { self.track };
        let selected = held.is_some() && held == picked;
        let rest = Length::Fixed((width - ROW_PADDING - span).max(0.0));

        let seated = match self.cargo().is_some() {
            true => {
                let grip = mouse_area(content).on_press(Message::Press(index));

                list_row(grip, selected, false, rest, Message::DragEnd)
            }
            false => list_row(content, selected, false, rest, Message::Row(index)),
        };

        let seat = row![space().width(Length::Fixed(ROW_PADDING)), stem, seated].align_y(Vertical::Center);

        let row: Element<'_, Message> = container(seat)
            .width(Length::Fixed(width))
            .height(Length::Fixed(ROW_HEIGHT))
            .into();

        match (self.track, self.owner) {
            (Some(track), _) => editor::target(row, Target::AnimCurve(track)),
            (_, Some(part)) => editor::target(row, Target::AnimPart(part)),
            _ => row,
        }
    }
}

fn marker<'a>(mark: &'static str) -> Element<'a, Message> {
    let glyph = text(mark)
        .font(fonts::MISC_SYMBOLS)
        .size(MARKER_SIZE)
        .line_height(MARKER_LINE_HEIGHT)
        .width(Length::Fixed(MARKER_WIDTH));

    match mark == FOLDER_OPEN {
        true => glyph.style(open_mark).into(),
        false => glyph.into(),
    }
}

fn seat(theme: &Theme, carried: bool, onto: Option<Mark>, alarm: Option<Alarm>) -> container::Style {
    let palette = theme.palette();

    let background = match (carried, alarm) {
        (true, _) => Color { a: CARRIED_TINT, ..palette.primary },
        (_, Some(Alarm::Faulted)) => Color { a: ALARM_TINT, ..palette.danger },
        (_, Some(Alarm::Tainted)) => Color { a: ALARM_TINT, ..palette.warning },
        _ => Color::TRANSPARENT,
    };

    let border = match onto {
        Some(Mark::Nest) => Border::default().rounded(4.0).width(NEST_BORDER).color(palette.success),
        _ => Border::default().rounded(4.0).width(0.0).color(palette.primary),
    };

    let seam = |lift: f32| iced::Shadow {
        color: palette.success,
        offset: iced::Vector::new(0.0, lift),
        blur_radius: 0.0,
    };

    container::Style {
        background: Some(background.into()),
        border,
        shadow: match onto {
            Some(Mark::Above) => seam(-SEAM_HEIGHT),
            Some(Mark::Below) => seam(SEAM_HEIGHT),
            _ => iced::Shadow::default(),
        },
        ..container::Style::default()
    }
}

fn owning(part: i32, count: usize) -> Option<usize> {
    usize::try_from(part).ok().filter(|held| *held < count)
}

fn shadowing(tracks: &[AnimModification], count: usize) -> Vec<bool> {
    let mut flags = vec![false; tracks.len()];
    let mut seen = vec![0u32; count];
    let mut stray: Vec<(i32, i32)> = Vec::new();

    for (at, track) in tracks.iter().enumerate().rev() {
        let bit = u32::try_from(track.kind).ok().filter(|kind| *kind < u32::BITS).map(|kind| 1 << kind);

        match (owning(track.part, count), bit) {
            (Some(part), Some(bit)) => {
                flags[at] = seen[part] & bit != 0;
                seen[part] |= bit;
            }
            _ => {
                let pair = (track.part, track.kind);
                flags[at] = stray.contains(&pair);

                if !flags[at] {
                    stray.push(pair);
                }
            }
        }
    }

    flags
}

fn bucketed(tracks: &[AnimModification], count: usize) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut heads = vec![0usize; count + 1];

    for track in tracks {
        if let Some(part) = owning(track.part, count) {
            heads[part + 1] += 1;
        }
    }

    for at in 0..count {
        heads[at + 1] += heads[at];
    }

    let mut owned = vec![0usize; heads[count]];
    let mut fill = heads.clone();
    let mut loose = Vec::new();

    for (at, track) in tracks.iter().enumerate() {
        match owning(track.part, count) {
            Some(part) => {
                owned[fill[part]] = at;
                fill[part] += 1;
            }
            None => loose.push(at),
        }
    }

    (heads, owned, loose)
}

fn curve_label(track: &AnimModification, shadowed: bool) -> String {
    let mut label = String::with_capacity(LABEL_ROOM);

    label.push_str(kind_label(track.kind));
    label.push_str(SEPARATOR);
    label.push_str(&key_label(track.keyframes.len()));

    if track.loop_count != 1 {
        label.push_str(SEPARATOR);
        label.push_str(loop_label(track.loop_count));
    }

    if shadowed {
        label.push_str(SEPARATOR);
        label.push_str(SHADOWED_MARK);
    }

    label
}

fn part_label(model: &Model, at: usize) -> String {
    let Some(declared) = model.parts.get(at) else {
        return format!("Part {}", at);
    };

    let mut label = match declared.name.trim() {
        "" => format!("Part {}", at),
        name => format!("Part {} \u{00b7} {}", at, name),
    };

    if let Some(mark) = hidden(declared) {
        label.push_str(&format!(" \u{00b7} {}", mark));
    }

    label
}

fn leaf(label: String, depth: u16, track: Option<usize>, warn: bool, alarm: Option<Alarm>) -> TreeRow {
    TreeRow { label, depth, mark: "", part: None, owner: None, track, warn, bucket: false, alarm, guide: Guide::default() }
}

pub(super) fn listing(
    doc: Option<&Maanim>,
    model: Option<&Model>,
    expanded: &HashSet<usize>,
    loose_open: bool,
    blame: &Blame,
) -> Vec<TreeRow> {
    let mut listed = listed(doc, model, expanded, loose_open, blame);
    let mut tracer = Tracer::default();

    for index in (0..listed.len()).rev() {
        let depth = listed[index].depth;
        let above = index.checked_sub(1).map(|prev| listed[prev].depth);

        listed[index].guide = tracer.back(depth, above);
    }

    listed
}

fn listed(
    doc: Option<&Maanim>,
    model: Option<&Model>,
    expanded: &HashSet<usize>,
    loose_open: bool,
    blame: &Blame,
) -> Vec<TreeRow> {
    let tracks: &[AnimModification] = doc.map_or(&[], Maanim::tracks);
    let shadowed = shadowing(tracks, model.map_or(0, |model| model.parts.len()));

    let Some(model) = model else {
        return tracks
            .iter()
            .enumerate()
            .map(|(at, track)| {
                let warn = shadowed[at];

                leaf(curve_label(track, warn), 0, Some(at), warn, blame.track(at))
            })
            .collect();
    };

    let count = model.parts.len();
    let brood = lineage(model);
    let (heads, owned, loose) = bucketed(tracks, count);

    let walked = walk(&brood, count, expanded);
    let mut listed = Vec::with_capacity(walked.len() + tracks.len().min(count) + 1);

    for (part, depth) in walked {
        let curves = &owned[heads[part]..heads[part + 1]];
        let open = expanded.contains(&part);
        let depth = depth as u16;
        let barest = curves.is_empty() && brood.barren(part);

        listed.push(TreeRow {
            label: part_label(model, part),
            depth,
            mark: match (barest, open) {
                (true, _) => "",
                (_, true) => FOLDER_OPEN,
                (_, false) => FOLDER_SHUT,
            },
            part: Some(part),
            owner: Some(part),
            track: None,
            warn: false,
            bucket: false,
            alarm: blame.part(part),
            guide: Guide::default(),
        });

        if !open {
            continue;
        }

        for at in curves.iter().copied() {
            let warn = shadowed[at];

            listed.push(leaf(curve_label(&tracks[at], warn), depth + 1, Some(at), warn, blame.track(at)));
        }
    }

    if loose.is_empty() {
        return listed;
    }

    let mut label = String::with_capacity(LABEL_ROOM);
    label.push_str(LOOSE_LABEL);
    label.push_str(SEPARATOR);
    label.push_str(&key_label(loose.len()));

    listed.push(TreeRow {
        label,
        depth: 0,
        mark: if loose_open { FOLDER_OPEN } else { FOLDER_SHUT },
        part: None,
        owner: None,
        track: None,
        warn: true,
        bucket: true,
        alarm: blame.bucket(),
        guide: Guide::default(),
    });

    if !loose_open {
        return listed;
    }

    for at in loose {
        let warn = shadowed[at];

        listed.push(leaf(curve_label(&tracks[at], warn), 1, Some(at), warn, blame.track(at)));
    }

    listed
}

struct Brood {
    heads: Vec<usize>,
    kids: Vec<usize>,
    roots: Vec<usize>,
}

impl Brood {
    fn of(&self, at: usize) -> &[usize] {
        &self.kids[self.heads[at]..self.heads[at + 1]]
    }

    fn barren(&self, at: usize) -> bool {
        self.heads[at] == self.heads[at + 1]
    }
}

fn lineage(model: &Model) -> Brood {
    let count = model.parts.len();
    let mut heads = vec![0usize; count + 1];
    let mut roots = Vec::new();

    let parent = |at: usize, part: &ModelPart| owning(part.parent, count).filter(|held| *held != at);

    for (at, part) in model.parts.iter().enumerate() {
        match parent(at, part) {
            Some(held) => heads[held + 1] += 1,
            None => roots.push(at),
        }
    }

    for at in 0..count {
        heads[at + 1] += heads[at];
    }

    let mut kids = vec![0usize; heads[count]];
    let mut fill = heads.clone();

    for (at, part) in model.parts.iter().enumerate() {
        if let Some(held) = parent(at, part) {
            kids[fill[held]] = at;
            fill[held] += 1;
        }
    }

    Brood { heads, kids, roots }
}

pub(super) fn roots(model: &Model) -> Vec<usize> {
    lineage(model).roots
}

fn walk(brood: &Brood, count: usize, expanded: &HashSet<usize>) -> Vec<(usize, usize)> {
    let mut listed = Vec::with_capacity(count);
    let mut seen = vec![false; count];

    for root in &brood.roots {
        descend(*root, 0, brood, expanded, &mut seen, &mut listed, true);
    }

    for at in 0..count {
        descend(at, 0, brood, expanded, &mut seen, &mut listed, true);
    }

    listed
}

fn descend(
    at: usize,
    depth: usize,
    brood: &Brood,
    expanded: &HashSet<usize>,
    seen: &mut [bool],
    listed: &mut Vec<(usize, usize)>,
    visible: bool,
) {
    if seen.get(at).copied().unwrap_or(true) {
        return;
    }

    seen[at] = true;

    if visible {
        listed.push((at, depth));
    }

    let open = visible && expanded.contains(&at);

    for child in brood.of(at) {
        descend(*child, depth + 1, brood, expanded, seen, listed, open);
    }
}

pub(super) fn hidden(part: &ModelPart) -> Option<&'static str> {
    if part.id < 0 {
        return Some("not drawn");
    }

    if part.sprite < 0 {
        return Some("no sprite");
    }

    if part.opacity == 0 {
        return Some("transparent");
    }

    if part.scale_x == 0 || part.scale_y == 0 {
        return Some("no scale");
    }

    None
}

pub(super) fn held_curve(doc: &Maanim, at: usize) -> Option<Held> {
    let track = doc.track(at)?;
    let ordinal = doc
        .tracks()
        .iter()
        .take(at)
        .filter(|other| other.part == track.part && other.kind == track.kind)
        .count();

    Some(Held { part: track.part, kind: track.kind, ordinal })
}

pub(super) fn locate_curve(doc: &Maanim, held: &Held) -> Option<usize> {
    doc.tracks()
        .iter()
        .enumerate()
        .filter(|(_, track)| track.part == held.part && track.kind == held.kind)
        .nth(held.ordinal)
        .map(|(at, _)| at)
}

#[cfg(test)]
fn walk_of(model: &Model, expanded: &HashSet<usize>) -> Vec<(usize, usize)> {
    walk(&lineage(model), model.parts.len(), expanded)
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::ModelPart;

    use super::*;

    const SAMPLE: &str = "[modelanim:animation]\n1\n2\n0,11,-1,0,0,\n1\n0,0,0,0\n1,4,-1,0,0,\n1\n0,0,0,0\n";
    const DOUBLED: &str = "[modelanim:animation]\n1\n3\n5,11,-1,0,0,\n1\n0,0,0,0\n2,4,-1,0,0,\n1\n0,0,0,0\n5,11,-1,0,0,\n1\n0,0,0,0\n";

    fn model(parents: &[i32]) -> Model {
        Model {
            parts: parents.iter().map(|parent| ModelPart { parent: *parent, ..ModelPart::default() }).collect(),
            ..Model::default()
        }
    }

    fn doc() -> Maanim {
        Maanim::parse(SAMPLE.as_bytes()).expect("the sample parses")
    }

    #[test]
    fn a_remembered_channel_survives_another_being_inserted_before_it() {
        // Two channels share part 5 and kind 11, so an index alone cannot name one.
        let mut doc = Maanim::parse(DOUBLED.as_bytes()).expect("the sample parses");
        let held = held_curve(&doc, 2).expect("the track exists");

        assert_eq!(held, Held { part: 5, kind: 11, ordinal: 1 });

        doc.insert(0, authoring::blank_curve(9, 12, None));

        assert_eq!(locate_curve(&doc, &held), Some(3));
    }

    #[test]
    fn a_remembered_channel_is_gone_once_its_occurrence_is() {
        let mut doc = Maanim::parse(DOUBLED.as_bytes()).expect("the sample parses");
        let held = held_curve(&doc, 2).expect("the track exists");

        doc.remove(2);

        assert_eq!(locate_curve(&doc, &held), None);
        assert_eq!(locate_curve(&doc, &Held { part: 5, kind: 11, ordinal: 0 }), Some(0));
    }

    #[test]
    fn a_part_owns_its_channels_and_its_children_sit_beside_them() {
        // Part 0 is the root and part 1 hangs off it, each driven by one channel.
        let listed = listing(Some(&doc()), Some(&model(&[-1, 0])), &HashSet::from([0, 1]), false, &Blame::default());

        let shape: Vec<(u16, bool)> =
            listed.iter().map(|row| (row.depth, row.track.is_some())).collect();

        assert_eq!(shape, vec![(0, false), (1, true), (1, false), (2, true)]);
    }

    #[test]
    fn a_leaf_says_nothing_and_shows_no_folder_mark() {
        // The tree has to end somewhere, so a part with nothing under it is not an
        // empty state to announce, it simply does not open.
        let listed = listing(None, Some(&model(&[-1, 0])), &HashSet::from([0, 1]), false, &Blame::default());

        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|row| row.track.is_none()));
        assert_eq!(listed[1].mark, "", "a childless part drops its caret");
        assert_eq!(listed[0].mark, FOLDER_OPEN, "one that bears a child keeps it");
    }

    #[test]
    fn a_lone_root_is_worth_opening_but_several_are_not() {
        // Most units hang everything off part 0, so opening it saves a click every time.
        assert_eq!(roots(&model(&[-1, 0, 1])), vec![0]);
        assert_eq!(roots(&model(&[-1, -1, 0])), vec![0, 1]);
    }

    #[test]
    fn a_tree_starts_fully_collapsed() {
        let listed = listing(Some(&doc()), Some(&model(&[-1, 0])), &HashSet::new(), false, &Blame::default());

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].mark, FOLDER_SHUT);
    }

    #[test]
    fn a_channel_naming_a_part_the_model_lacks_folds_away_but_stays_reachable() {
        // The engine does not bound check the part index, so the channel has to stay
        // reachable, but a broken file can hold hundreds, so the bucket starts shut.
        let model = model(&[-1]);
        let shut = listing(Some(&doc()), Some(&model), &HashSet::from([0]), false, &Blame::default());

        let bucket = shut.iter().find(|row| row.bucket).expect("the bucket is listed");
        assert!(bucket.warn && bucket.mark == FOLDER_SHUT);
        assert_eq!(shut.iter().filter(|row| row.track.is_some()).count(), 1, "only part 0's own");

        let open = listing(Some(&doc()), Some(&model), &HashSet::from([0]), true, &Blame::default());
        assert_eq!(open.iter().filter(|row| row.track.is_some()).count(), 2);
    }

    #[test]
    fn without_a_model_every_channel_still_lists_flat() {
        let listed = listing(Some(&doc()), None, &HashSet::new(), false, &Blame::default());

        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|row| row.depth == 0 && row.track.is_some()));
    }

    #[test]
    fn a_hierarchy_lists_parents_before_their_children() {
        // 0 is the root, 1 and 3 hang off it, 2 hangs off 1.
        let listed = walk_of(&model(&[-1, 0, 1, 0]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 1), (2, 2), (3, 1)]);
    }

    #[test]
    fn a_folded_part_hides_its_descendants_but_not_its_siblings() {
        let listed = walk_of(&model(&[-1, 0, 1, 0]), &HashSet::from([0]));

        assert_eq!(listed, vec![(0, 0), (1, 1), (3, 1)]);
    }

    #[test]
    fn a_parent_cycle_still_lists_every_part() {
        // The file is not bound checked, so 1 and 2 pointing at each other has to
        // stay visible rather than dropping out of the tree entirely.
        let listed = walk_of(&model(&[-1, 2, 1]), &HashSet::from([0, 1, 2]));

        assert_eq!(listed.len(), 3);
        assert!(listed.iter().any(|(part, _)| *part == 1));
        assert!(listed.iter().any(|(part, _)| *part == 2));
    }

    #[test]
    fn a_parent_past_the_end_is_treated_as_a_root() {
        let listed = walk_of(&model(&[-1, 9]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn a_part_parented_to_itself_does_not_recurse() {
        let listed = walk_of(&model(&[-1, 1]), &HashSet::from([0, 1]));

        assert_eq!(listed, vec![(0, 0), (1, 0)]);
    }
}

