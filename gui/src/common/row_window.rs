use std::ops::Range;

pub(crate) const ROW_HEIGHT: f32 = 50.0;
pub(crate) const ROW_SPACING: f32 = 4.0;
pub(crate) const ROW_PITCH: f32 = ROW_HEIGHT + ROW_SPACING;

const BAND_ROWS: usize = 8;

pub(crate) struct RowWindow {
    pub(crate) range: Range<usize>,
    pub(crate) pad_before: f32,
    pub(crate) pad_after: f32,
}

pub(crate) fn compute(total: usize, viewport_h: f32, offset: f32) -> RowWindow {
    compute_with(total, viewport_h, offset, ROW_HEIGHT, ROW_SPACING)
}

pub(crate) fn compute_with(total: usize, viewport_h: f32, offset: f32, height: f32, spacing: f32) -> RowWindow {
    if total == 0 {
        return RowWindow { range: 0..0, pad_before: 0.0, pad_after: 0.0 };
    }

    let pitch = height + spacing;
    let content_h = pitch * total as f32 - spacing;
    let max_offset = (content_h - viewport_h).max(0.0);
    let offset = offset.clamp(0.0, max_offset).round();

    let anchor = (offset / pitch).floor() as usize;
    let span = ((viewport_h / pitch).ceil() as usize).saturating_add(1);

    let first = anchor - anchor % BAND_ROWS;
    let end = first.saturating_add(BAND_ROWS).saturating_add(span).min(total);

    RowWindow {
        range: first..end,
        pad_before: if first > 0 { pitch * first as f32 - spacing } else { 0.0 },
        pad_after: if end < total { pitch * (total - end) as f32 - spacing } else { 0.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(offset: f32) -> RowWindow {
        compute_with(1000, 460.0, offset, 21.0, 4.0)
    }

    #[test]
    fn scrolling_inside_one_band_keeps_the_very_same_rows() {
        // Every row the window drops has to be shaped again from scratch, so the
        // window only moves once the scroll leaves the band it was built for.
        let settled = window(0.0);

        for step in 1..BAND_ROWS {
            let held = window(step as f32 * 25.0);

            assert_eq!(held.range, settled.range, "step {step} rebuilt the window");
            assert_eq!(held.pad_before, settled.pad_before);
        }

        assert_ne!(window(BAND_ROWS as f32 * 25.0).range, settled.range, "the next band moves");
    }

    #[test]
    fn the_window_still_covers_the_viewport_everywhere_in_its_band() {
        let pitch = 25.0;

        for anchor in 0..64usize {
            let held = window(anchor as f32 * pitch);
            let last = anchor + (460.0 / pitch).ceil() as usize;

            assert!(held.range.start <= anchor, "anchor {anchor} sits above the window");
            assert!(held.range.end > last, "anchor {anchor} leaves the bottom uncovered");
        }
    }

    #[test]
    fn the_pads_account_for_every_row_outside_the_window() {
        let held = window(500.0);
        let pitch = 25.0;
        let content = pitch * 1000.0 - 4.0;
        let rows = pitch * (held.range.end - held.range.start) as f32 - 4.0;

        assert!((held.pad_before + rows + held.pad_after + 4.0 * 2.0 - content).abs() < 0.5);
    }
}
