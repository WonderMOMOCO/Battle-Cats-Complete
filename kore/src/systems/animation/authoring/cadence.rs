use std::ops::Range;

use nyanko::graphics::rig::AnimModification;

pub const REACH: i64 = i32::MAX as i64;

const STILL_MARK: &str = "none";
const BEYOND_MARK: &str = "???";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cycle {
    Still,
    Every(i64),
    Beyond,
}

impl Cycle {
    pub fn label(self) -> String {
        match self {
            Cycle::Still => STILL_MARK.to_string(),
            Cycle::Every(period) => period.to_string(),
            Cycle::Beyond => BEYOND_MARK.to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Beat {
    pub first: i32,
    pub last: i32,
    pub span: i32,
    pub passes: Option<i64>,
}

impl Beat {
    pub fn of(curve: &AnimModification) -> Option<Beat> {
        let first = curve.keyframes.first()?.frame;
        let last = curve.keyframes.last()?.frame;
        let span = last.saturating_sub(first);

        let passes = match curve.loop_count {
            _ if span <= 0 => Some(1),
            -1 => None,
            count if count > 1 => Some(i64::from(count)),
            _ => Some(1),
        };

        Some(Beat { first, last, span, passes })
    }

    pub fn settles(self) -> Option<i64> {
        let passes = self.passes?;

        Some(i64::from(self.first).saturating_add(passes.saturating_mul(i64::from(self.span))))
    }

    pub fn steady(self) -> i64 {
        self.settles().unwrap_or_else(|| i64::from(self.first))
    }

    pub fn repeats(self, from: i64, to: i64) -> Range<i64> {
        let span = i64::from(self.span);

        if span <= 0 {
            return 0..1;
        }

        let first = i64::from(self.first);
        let low = (from - first).div_euclid(span).max(0);
        let high = (to - first).div_euclid(span).saturating_add(1);
        let high = self.passes.map_or(high, |passes| high.min(passes));

        low..high.max(low)
    }

    pub fn shifted(self, pass: i64) -> i64 {
        pass.saturating_mul(i64::from(self.span))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cadence {
    pub lead: i64,
    pub settled: i64,
    pub cycle: Cycle,
    pub extent: i64,
}

impl Cadence {
    pub fn of<'a>(curves: impl IntoIterator<Item = &'a AnimModification>) -> Cadence {
        let mut lead = 0;
        let mut settled: Option<i64> = None;
        let mut period: Option<i64> = None;
        let mut beyond = false;

        for curve in curves {
            let Some(beat) = Beat::of(curve) else {
                continue;
            };

            lead = lead.min(i64::from(beat.first));
            settled = Some(settled.map_or_else(|| beat.steady(), |held: i64| held.max(beat.steady())));

            if beat.passes.is_some() || beat.span <= 0 {
                continue;
            }

            let span = i64::from(beat.span);

            match period.map_or(Some(span), |held| lcm(held, span)) {
                Some(found) => period = Some(found),
                None => beyond = true,
            }
        }

        let settled = settled.unwrap_or(0);
        let cycle = match (beyond, period) {
            (true, _) => Cycle::Beyond,
            (_, Some(period)) => Cycle::Every(period),
            (_, None) => Cycle::Still,
        };

        let extent = match cycle {
            Cycle::Still => settled,
            Cycle::Beyond => REACH,
            Cycle::Every(period) => settled.saturating_add(period).min(REACH),
        };

        Cadence { lead, settled, cycle, extent }
    }

    pub fn fold(self, frame: i64) -> i64 {
        match self.cycle {
            Cycle::Still => frame.min(self.settled),
            Cycle::Beyond => frame,
            Cycle::Every(period) if period > 0 && frame >= self.extent => {
                self.settled + (frame - self.settled).rem_euclid(period)
            }
            Cycle::Every(_) => frame,
        }
    }
}

fn gcd(left: i64, right: i64) -> i64 {
    let (mut held, mut rest) = (left.abs(), right.abs());

    while rest != 0 {
        let carried = rest;

        rest = held % rest;
        held = carried;
    }

    held
}

fn lcm(left: i64, right: i64) -> Option<i64> {
    match gcd(left, right) {
        0 => None,
        divisor => (left / divisor).checked_mul(right),
    }
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::Keyframe;
    use nyanko::graphics::tools::timeline;

    use super::*;

    fn curve(kind: i32, loop_count: i32, frames: &[i32]) -> AnimModification {
        AnimModification {
            part: 0,
            kind,
            loop_count,
            keyframes: frames
                .iter()
                .map(|frame| Keyframe { frame: *frame, value: *frame, ease: 0, ease_power: 0 })
                .collect(),
            ..AnimModification::default()
        }
    }

    #[test]
    fn a_part_with_nothing_looping_rests_once_its_last_pass_is_done() {
        // Two thirds of the game's parts look like this. Nothing loops, so the timeline
        // just ends where the animation does.
        let held = [curve(4, 1, &[0, 20, 40]), curve(11, 0, &[0, 12])];
        let cadence = Cadence::of(&held);

        assert_eq!(cadence.cycle, Cycle::Still);
        assert_eq!(cadence.settled, 40);
        assert_eq!(cadence.extent, 40);
        assert_eq!(cadence.fold(9_000), 40);
    }

    #[test]
    fn two_desynced_loops_realign_at_their_least_common_multiple() {
        // Real case: 032_c01 part 51, angle over [30..90] and y over [30..53]. Both start
        // at 30, so everything the part ever does fits in 30 + lcm(60, 23).
        let held = [curve(11, -1, &[30, 60, 90]), curve(5, -1, &[30, 53])];
        let cadence = Cadence::of(&held);

        assert_eq!(cadence.lead, 0);
        assert_eq!(cadence.settled, 30);
        assert_eq!(cadence.cycle, Cycle::Every(1_380));
        assert_eq!(cadence.extent, 1_410);
    }

    #[test]
    fn a_cycle_authored_before_frame_zero_keeps_its_negative_lead() {
        // 044_f00 keys everything at [-30..10] but the playhead can't go below zero, so
        // most of those keys are only ever hit on the way round. Start the axis left of
        // zero or you can't see them at all.
        let held = [curve(11, -1, &[-30, -10, 10]), curve(4, -1, &[-30, 10])];
        let cadence = Cadence::of(&held);

        assert_eq!(cadence.lead, -30);
        assert_eq!(cadence.settled, -30);
        assert_eq!(cadence.cycle, Cycle::Every(40));
        assert_eq!(cadence.extent, 10);
        assert_eq!(cadence.fold(50), -30);
    }

    #[test]
    fn a_finite_repeat_settles_after_its_last_pass_not_its_last_key() {
        let held = [curve(4, 3, &[0, 20])];
        let cadence = Cadence::of(&held);

        assert_eq!(cadence.cycle, Cycle::Still);
        assert_eq!(cadence.settled, 60);
    }

    #[test]
    fn a_period_past_the_engines_own_reach_reports_beyond_rather_than_overflowing() {
        // Nothing stops a modder authoring spans whose lcm blows past i64. Frames are i32
        // in the engine anyway, so there's nothing out there to miss.
        let held = [
            curve(4, -1, &[0, 2_147_483_647]),
            curve(5, -1, &[0, 2_147_483_646]),
            curve(11, -1, &[0, 2_147_483_645]),
        ];
        let cadence = Cadence::of(&held);

        assert_eq!(cadence.cycle, Cycle::Beyond);
        assert_eq!(cadence.extent, REACH);
        assert_eq!(cadence.fold(9_000), 9_000);
    }

    #[test]
    fn folding_a_frame_never_changes_what_any_curve_resolves_to() {
        // The reason we never clamp the viewer. Let the playhead run as far as it likes;
        // folding it back onto the axis reads identically to every curve.
        let held = [
            curve(11, -1, &[30, 60, 90]),
            curve(5, -1, &[30, 53]),
            curve(4, 1, &[0, 40]),
            curve(9, 3, &[0, 14]),
            curve(2, -1, &[5]),
        ];
        let cadence = Cadence::of(&held);

        for frame in 0..4_000 {
            let folded = cadence.fold(i64::from(frame));

            assert!(folded < cadence.extent, "frame {frame} folded outside the drawn range");

            for track in &held {
                assert_eq!(
                    timeline::local_frame(track, frame),
                    timeline::local_frame(track, folded as i32),
                    "kind {} disagrees at frame {frame}",
                    track.kind,
                );
            }
        }
    }

    #[test]
    fn a_repeat_walk_only_visits_the_passes_that_reach_the_window() {
        let beat = Beat::of(&curve(4, -1, &[10, 30])).expect("the curve holds keys");

        assert_eq!(beat.repeats(0, 9), 0..0, "the window closes before the curve starts");
        assert_eq!(beat.repeats(0, 15), 0..1);
        assert_eq!(beat.repeats(70, 110), 3..6);
        assert_eq!(beat.shifted(3), 60);

        let capped = Beat::of(&curve(4, 2, &[10, 30])).expect("the curve holds keys");

        assert_eq!(capped.repeats(0, 400), 0..2);
        assert_eq!(capped.settles(), Some(50));

        let held = Beat::of(&curve(4, -1, &[7])).expect("the curve holds keys");

        assert_eq!(held.repeats(0, 400), 0..1);
        assert_eq!(held.steady(), 7);
    }

    #[test]
    fn a_lead_in_moves_the_restart_off_frame_zero() {
        // Cat 000's walk: a 16 frame sprite cycle alongside one-shot channels that
        // settle at 3. The first pass runs 0..18 and every pass after it loops 3..18,
        // which is the frame the timeline draws its loop line on.
        let held = Cadence::of(&[
            curve(2, -1, &[0, 4, 8, 12, 16]),
            curve(5, 1, &[0, 3]),
            curve(4, 1, &[0, 3]),
        ]);

        assert_eq!(held.settled, 3);
        assert_eq!(held.extent, 19);
        assert_eq!(held.fold(18), 18);
        assert_eq!(held.fold(19), 3, "the pass after the first restarts at the settle");
        assert_eq!(held.fold(35), 3);
    }
}
