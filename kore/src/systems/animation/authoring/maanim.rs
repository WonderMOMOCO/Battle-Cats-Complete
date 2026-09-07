use std::sync::Arc;

use nyanko::common::{cell_value, declared_rows};
use nyanko::graphics::rig::{AnimModification, Animation, Keyframe, Model, RigError};
use nyanko::graphics::tools::timeline;

use super::mamodel::{split, Line, DELIMITER};
use super::{blank_curve, Cadence};

const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];
const NAME_FIELD: usize = 5;
const HEADER_LINES: usize = 3;

#[derive(Clone)]
pub struct Maanim {
    bom: bool,
    tag: String,
    ended: bool,
    names: Vec<String>,
    animation: Arc<Animation>,
}

impl Maanim {
    pub fn parse(bytes: &[u8]) -> Result<Self, RigError> {
        let animation = Animation::parse(bytes)?;

        let bom = bytes.starts_with(&BOM);
        let body = String::from_utf8_lossy(if bom { &bytes[BOM.len()..] } else { bytes }).into_owned();

        let lines = split(&body);
        let tag = lines.first().map(|line| line.text.clone()).unwrap_or_default();
        let ended = lines.last().is_none_or(|line| !line.end.is_empty());

        let names = names(&lines, animation.modifications.len());

        Ok(Self { bom, tag, ended, names, animation: Arc::new(animation) })
    }

    pub fn shared(&self) -> Arc<Animation> {
        Arc::clone(&self.animation)
    }

    pub fn tracks(&self) -> &[AnimModification] {
        &self.animation.modifications
    }

    pub fn track(&self, at: usize) -> Option<&AnimModification> {
        self.animation.modifications.get(at)
    }

    pub fn edit(&mut self, at: usize) -> Option<&mut AnimModification> {
        Arc::make_mut(&mut self.animation).modifications.get_mut(at)
    }

    pub fn insert(&mut self, at: usize, track: AnimModification) {
        let at = at.min(self.animation.modifications.len());

        self.names.insert(at, track.name.clone());
        Arc::make_mut(&mut self.animation).modifications.insert(at, track);
    }

    pub fn remove(&mut self, at: usize) -> Option<AnimModification> {
        if at >= self.animation.modifications.len() {
            return None;
        }

        self.names.remove(at);

        Some(Arc::make_mut(&mut self.animation).modifications.remove(at))
    }

    pub fn add_key(&mut self, track: usize) -> Option<usize> {
        let last = self.track(track)?.keyframes.last().copied();
        let key = last.map_or(Keyframe::default(), |key| Keyframe { frame: key.frame + 1, ..key });
        let keyframes = &mut Arc::make_mut(&mut self.animation).modifications.get_mut(track)?.keyframes;

        let at = keyframes.partition_point(|existing| existing.frame <= key.frame);
        keyframes.insert(at, key);

        Some(at)
    }

    pub fn cadence(&self, part: i32) -> Cadence {
        Cadence::of(self.tracks().iter().filter(|track| track.part == part))
    }

    pub fn effective(&self, part: i32, kind: i32) -> Option<usize> {
        self.tracks()
            .iter()
            .enumerate()
            .rev()
            .find(|(_, track)| track.part == part && track.kind == kind)
            .map(|(at, _)| at)
    }

    pub fn ensure_track(&mut self, part: usize, kind: i32, model: Option<&Model>) -> Option<usize> {
        let wanted = i32::try_from(part).ok()?;

        if let Some(at) = self.effective(wanted, kind) {
            return Some(at);
        }

        let at = self.tracks().len();

        self.insert(at, blank_curve(part, kind, model));

        Some(at)
    }

    pub fn reachable(&self, track: usize, frame: i32) -> Option<i32> {
        let held = self.track(track)?;
        let last = held.keyframes.last()?.frame;
        let local = timeline::local_frame(held, frame)?;

        Some(match local == last {
            true => frame,
            false => local,
        })
    }

    pub fn ensure_key(&mut self, track: usize, frame: i32) -> Option<usize> {
        let frame = self.reachable(track, frame).unwrap_or(frame);
        let held = self.track(track)?;

        if let Some(at) = held.keyframes.iter().position(|key| key.frame == frame) {
            return Some(at);
        }

        let seeded = timeline::value(held, frame).unwrap_or(0);
        let ease = held.keyframes.last().map_or(0, |key| key.ease);
        let key = Keyframe { frame, value: seeded, ease, ease_power: 0 };

        let keyframes = &mut Arc::make_mut(&mut self.animation).modifications.get_mut(track)?.keyframes;
        let at = keyframes.partition_point(|existing| existing.frame < frame);

        keyframes.insert(at, key);

        Some(at)
    }

    pub fn pose(
        &mut self,
        part: usize,
        kind: i32,
        frame: i32,
        value: i32,
        model: Option<&Model>,
    ) -> bool {
        let Some(track) = self.ensure_track(part, kind, model) else {
            return false;
        };

        let Some(at) = self.ensure_key(track, frame) else {
            return false;
        };

        let Some(key) = self.edit(track).and_then(|track| track.keyframes.get_mut(at)) else {
            return false;
        };

        if key.value == value {
            return false;
        }

        key.value = value;

        true
    }

    pub fn posed(&self, part: usize, kind: i32, frame: i32) -> Option<i32> {
        let wanted = i32::try_from(part).ok()?;
        let track = self.effective(wanted, kind)?;

        timeline::value(self.track(track)?, frame)
    }

    pub fn remove_key(&mut self, track: usize, at: usize) -> bool {
        let Some(keyframes) = Arc::make_mut(&mut self.animation)
            .modifications
            .get_mut(track)
            .map(|track| &mut track.keyframes)
        else {
            return false;
        };

        if at >= keyframes.len() {
            return false;
        }

        keyframes.remove(at);

        true
    }

    pub fn retarget(&mut self, moved: &[Option<usize>]) -> bool {
        let mut doomed = Vec::new();
        let mut shifted = false;

        for (at, track) in self.animation.modifications.iter().enumerate() {
            let Ok(part) = usize::try_from(track.part) else {
                continue;
            };

            match moved.get(part) {
                Some(None) => doomed.push(at),
                Some(Some(landed)) => shifted |= i32::try_from(*landed) != Ok(track.part),
                None => {}
            }
        }

        if doomed.is_empty() && !shifted {
            return false;
        }

        for at in doomed.iter().rev() {
            self.remove(*at);
        }

        for track in Arc::make_mut(&mut self.animation).modifications.iter_mut() {
            if let Ok(part) = usize::try_from(track.part)
                && let Some(Some(landed)) = moved.get(part)
                && let Ok(landed) = i32::try_from(*landed)
            {
                track.part = landed;
            }
        }

        true
    }

    pub fn revalue(&mut self, kind: i32, moved: &[Option<usize>]) -> bool {
        let landed = |value: i32| {
            usize::try_from(value)
                .ok()
                .and_then(|at| moved.get(at))
                .map(|landed| landed.and_then(|at| i32::try_from(at).ok()).unwrap_or(-1))
        };

        let shifted = self.animation.modifications.iter().filter(|track| track.kind == kind).any(|track| {
            track.keyframes.iter().any(|key| landed(key.value).is_some_and(|value| value != key.value))
        });

        if !shifted {
            return false;
        }

        for track in Arc::make_mut(&mut self.animation).modifications.iter_mut() {
            if track.kind != kind {
                continue;
            }

            for key in track.keyframes.iter_mut() {
                if let Some(value) = landed(key.value) {
                    key.value = value;
                }
            }
        }

        true
    }

    pub fn sort_keys(&mut self, track: usize) {
        if let Some(track) = Arc::make_mut(&mut self.animation).modifications.get_mut(track) {
            track.keyframes.sort_by_key(|key| key.frame);
        }
    }

    fn name(&self, at: usize) -> &str {
        let parsed = self.animation.modifications.get(at).map_or("", |track| track.name.as_str());

        self.names
            .get(at)
            .filter(|raw| raw.trim() == parsed)
            .map_or(parsed, String::as_str)
    }

    pub fn write(&self) -> Vec<u8> {
        let delimiter = DELIMITER;
        let mut body = String::new();

        push_line(&mut body, &self.tag);
        push_line(&mut body, &self.animation.version.to_string());
        push_line(&mut body, &self.animation.modifications.len().to_string());

        for (at, track) in self.animation.modifications.iter().enumerate() {
            for value in [track.part, track.kind, track.loop_count, track.min_value, track.max_value] {
                body.push_str(&value.to_string());
                body.push(delimiter);
            }

            push_line(&mut body, self.name(at));
            push_line(&mut body, &track.keyframes.len().to_string());

            for key in &track.keyframes {
                let row = [key.frame, key.value, key.ease, key.ease_power];
                let fields: Vec<String> = row.iter().map(i32::to_string).collect();

                push_line(&mut body, &fields.join(&delimiter.to_string()));
            }
        }

        if !self.ended {
            body.pop();
        }

        let mut bytes = Vec::with_capacity(body.len() + BOM.len());

        if self.bom {
            bytes.extend_from_slice(&BOM);
        }

        bytes.extend_from_slice(body.as_bytes());
        bytes
    }
}

fn push_line(body: &mut String, text: &str) {
    body.push_str(text);
    body.push('\n');
}

fn names(lines: &[Line], tracks: usize) -> Vec<String> {
    let mut cursor = HEADER_LINES;
    let mut found = Vec::with_capacity(tracks);

    for _ in 0..tracks {
        let header = lines.get(cursor).map_or("", |line| line.text.as_str());
        cursor += 1;

        found.push(header.splitn(NAME_FIELD + 1, DELIMITER).nth(NAME_FIELD).unwrap_or_default().to_owned());

        let keys = lines.get(cursor).map_or(0, |line| cell_value(&line.text));
        cursor += 1 + declared_rows(keys).unwrap_or(0);
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const PADDED: &str = "[modelanim:animation]\n1\n2\n20,11,-1,0,0,\tネスト\t\n2\n-30,50,2,2\n10,50,0,0\n21,4,1,0,0,\n1\n0,0,0,0\n";

    fn round_trip(source: &[u8]) -> Vec<u8> {
        let doc = Maanim::parse(source).expect("the sample parses");

        doc.write()
    }

    const POSED: &str = "[modelanim:animation]\n1\n1\n0,4,1,0,0,\n2\n0,0,0,0\n20,100,0,0\n";

    #[test]
    fn a_pose_lands_on_the_channel_the_engine_actually_applies() {
        // Two channels drive the same part and kind; the later one wins in the engine,
        // so editing the earlier one would look like nothing happened.
        let doubled = "[modelanim:animation]\n1\n2\n0,4,1,0,0,\n1\n0,0,0,0\n0,4,1,0,0,\n1\n0,50,0,0\n";
        let mut doc = Maanim::parse(doubled.as_bytes()).expect("the sample parses");

        assert_eq!(doc.effective(0, 4), Some(1));
        assert!(doc.pose(0, 4, 0, 90, None));
        assert_eq!(doc.track(1).map(|t| t.keyframes[0].value), Some(90));
        assert_eq!(doc.track(0).map(|t| t.keyframes[0].value), Some(0), "the shadowed one is untouched");
    }

    #[test]
    fn a_key_inserted_between_two_others_changes_nothing_on_its_own() {
        // Grabbing a part mid-segment must not make the animation jump, so the new key
        // is seeded with whatever the channel already resolved to there.
        let mut doc = Maanim::parse(POSED.as_bytes()).expect("the sample parses");
        let held = doc.posed(0, 4, 10).expect("the channel resolves");

        let track = doc.effective(0, 4).expect("the channel exists");
        let at = doc.ensure_key(track, 10).expect("a key lands on the frame");

        assert_eq!(doc.track(track).map(|t| t.keyframes[at].value), Some(held));
        assert_eq!(doc.posed(0, 4, 10), Some(held));
        assert_eq!(doc.track(track).map(|t| t.keyframes.len()), Some(3));
    }

    #[test]
    fn posing_a_part_with_no_channel_of_that_kind_appends_one() {
        let mut doc = Maanim::parse(POSED.as_bytes()).expect("the sample parses");

        assert_eq!(doc.effective(0, 11), None);
        assert!(doc.pose(0, 11, 5, 900, None));

        let track = doc.effective(0, 11).expect("the channel now exists");
        assert_eq!(track, 1, "appended, so it wins over anything already there");
        assert_eq!(doc.posed(0, 11, 5), Some(900));
    }

    #[test]
    fn posing_the_same_value_twice_reports_no_change() {
        let mut doc = Maanim::parse(POSED.as_bytes()).expect("the sample parses");

        assert!(doc.pose(0, 4, 0, 42, None));
        assert!(!doc.pose(0, 4, 0, 42, None));
    }

    #[test]
    fn a_padded_name_survives_a_round_trip_byte_for_byte() {
        // nyanko trims the name on parse, so the raw field is what has to be written back.
        assert_eq!(round_trip(PADDED.as_bytes()), PADDED.as_bytes());
    }

    #[test]
    fn the_byte_order_mark_is_kept_only_where_the_file_had_one() {
        let with_bom: Vec<u8> = BOM.iter().copied().chain(PADDED.bytes()).collect();

        assert_eq!(round_trip(&with_bom), with_bom);
        assert_eq!(round_trip(PADDED.as_bytes()), PADDED.as_bytes());
    }

    #[test]
    fn the_tag_line_is_copied_rather_than_spelled() {
        // Both tags ship in the real data; writing one for the other would relabel the file.
        let second = PADDED.replace("animation]", "animation2]");

        assert_eq!(round_trip(second.as_bytes()), second.as_bytes());
    }

    #[test]
    fn editing_a_name_drops_the_padding_it_no_longer_matches() {
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        doc.edit(0).expect("the track exists").name = "walk".to_string();

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("20,11,-1,0,0,walk\n"));
    }

    #[test]
    fn a_keyframe_edit_rewrites_only_its_own_row() {
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        doc.edit(0).expect("the track exists").keyframes[1].value = 75;

        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("-30,50,2,2\n10,75,0,0\n"));
        assert!(written.contains("\tネスト\t\n"));
    }

    #[test]
    fn inserting_and_removing_tracks_keeps_the_names_aligned() {
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        let added = AnimModification {
            part: 3,
            kind: 4,
            loop_count: 1,
            keyframes: vec![Keyframe { frame: 0, value: 9, ease: 0, ease_power: 0 }],
            name: "added".to_string(),
            ..AnimModification::default()
        };

        doc.insert(0, added);
        let written = String::from_utf8(doc.write()).expect("the output is text");

        assert!(written.contains("\n3\n3,4,1,0,0,added\n"));
        assert!(written.contains("\tネスト\t\n"));

        doc.remove(0);
        assert_eq!(doc.write(), PADDED.as_bytes());
    }

    #[test]
    fn a_new_keyframe_lands_in_frame_order_and_carries_the_last_value() {
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        let at = doc.add_key(0).expect("the track exists");

        assert_eq!(at, 2);
        assert_eq!(doc.track(0).expect("the track exists").keyframes[2], Keyframe { frame: 11, value: 50, ease: 0, ease_power: 0 });

        assert!(doc.remove_key(0, 2));
        assert_eq!(doc.write(), PADDED.as_bytes());
    }

    #[test]
    fn retargeting_repoints_surviving_curves_and_drops_the_orphaned_ones() {
        // Removing or moving a part renumbers every curve that addresses one.
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");

        assert_eq!(doc.tracks().iter().map(|track| track.part).collect::<Vec<_>>(), vec![20, 21]);

        let mut moved = vec![Some(0); 22];
        moved[20] = None;
        moved[21] = Some(4);

        assert!(doc.retarget(&moved));
        assert_eq!(doc.tracks().len(), 1);
        assert_eq!(doc.track(0).expect("the survivor is kept").part, 4);
        assert!(!doc.retarget(&[]), "an empty map addresses nothing and changes nothing");
    }

    #[test]
    fn sharing_the_animation_does_not_block_a_later_edit() {
        // The viewer holds an Arc every frame; editing must still land.
        let mut doc = Maanim::parse(PADDED.as_bytes()).expect("the sample parses");
        let held = doc.shared();

        doc.edit(0).expect("the track exists").keyframes[0].value = 900;

        assert_eq!(held.modifications[0].keyframes[0].value, 50);
        assert_eq!(doc.shared().modifications[0].keyframes[0].value, 900);
    }

    #[test]
    fn posing_past_a_looping_curve_edits_the_key_the_engine_actually_reads() {
        // A looping curve folds a playhead past its last key back inside its span, so a
        // key written at the raw frame is one the engine never evaluates: the value reads
        // back unchanged and the part refuses to budge. Found on Anubis part 36, whose
        // idle angle curve loops over frames 0..60.
        let mut doc = Maanim::parse(POSED.as_bytes()).expect("the sample parses");
        let track = doc.effective(0, 4).expect("the sample declares the curve");

        if let Some(curve) = doc.edit(track) {
            curve.loop_count = -1;
        }

        assert_eq!(doc.reachable(track, 4), Some(4));
        assert_eq!(doc.reachable(track, 20), Some(0));
        assert_eq!(doc.reachable(track, 25), Some(5));

        doc.pose(0, 4, 20, 999, None);

        assert_eq!(doc.posed(0, 4, 20), Some(999));
        assert_eq!(doc.posed(0, 4, 0), Some(999));
    }

    #[test]
    fn posing_past_a_resting_curve_extends_it_rather_than_folding_back() {
        // A curve that does not loop holds its last value forever, so the playhead really
        // is past the end and a new key there is the right answer.
        let mut doc = Maanim::parse(POSED.as_bytes()).expect("the sample parses");
        let track = doc.effective(0, 4).expect("the sample declares the curve");

        assert_eq!(doc.reachable(track, 40), Some(40));

        doc.pose(0, 4, 40, 999, None);

        assert_eq!(doc.posed(0, 4, 40), Some(999));
        assert_eq!(doc.posed(0, 4, 20), Some(100));
    }
}
