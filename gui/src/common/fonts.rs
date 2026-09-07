use iced::Font;

pub(crate) const MISC_SYMBOLS: Font = Font::with_name("Miscellaneous-Symbols");

pub(crate) const MISC_SYMBOLS_LINE_HEIGHT: f32 = 0.805;

pub(crate) const TRIANGLE_LEAD: f32 = 0.107;
pub(crate) const TRIANGLE_MIDDLE: f32 = 0.301;
pub(crate) const TRIANGLE_REACH: f32 = 0.194;
pub(crate) const TRIANGLE_ROW: f32 = 6.0 / 7.0;
pub(crate) const TRIANGLE_GAP: f32 = 0.125;

pub(crate) const UPLOAD: &str = "\u{E000}";
pub(crate) const MOON_OPEN: &str = "\u{263E}";
pub(crate) const MOON_CLOSE: &str = "\u{263D}";

#[cfg(test)]
mod tests {
    use iced::advanced::graphics::text::{cosmic_text, font_system};

    use super::*;

    const MARKERS: [char; 4] = ['\u{25b4}', '\u{25b8}', '\u{25be}', '\u{25c2}'];

    #[test]
    fn the_symbol_font_carries_the_small_triangles() {
        // Windows' default families have no small triangles, so a tree arrow that
        // shapes through them lands on whatever face does cover it, at that face's
        // metrics — bigger, and hugging the top of the row. They ship in
        // Miscellaneous-Symbols instead; a rebuild that drops them fails here first.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);

        while std::time::Instant::now() < deadline {
            let Ok(mut system) = font_system().try_write() else {
                std::thread::yield_now();
                continue;
            };

            system.load_font(std::borrow::Cow::Borrowed(kore::common::assets::FONT_SYMBOLS));

            let raw = system.raw();
            let family = cosmic_text::fontdb::Family::Name("Miscellaneous-Symbols");
            let query = cosmic_text::fontdb::Query { families: &[family], ..Default::default() };

            let Some(id) = raw.db().query(&query) else {
                panic!("Miscellaneous-Symbols did not load");
            };

            let Some(face) = raw.get_font(id, cosmic_text::fontdb::Weight::NORMAL) else {
                panic!("Miscellaneous-Symbols has no readable face");
            };

            let swash = face.as_swash();
            let covered = swash.charmap();
            let metrics = swash.glyph_metrics(&[]);
            let vertical = swash.metrics(&[]);
            let upem = f32::from(metrics.units_per_em());
            let mut scaler = swash::scale::ScaleContext::new();

            for marker in MARKERS {
                let glyph = covered.map(marker);

                assert!(glyph != 0, "{marker} is missing");

                // The tree guides hang off these two: TRIANGLE_MIDDLE puts the trunk
                // down the middle of the arrow, TRIANGLE_LEAD walks the elbow out to
                // where its ink starts. Redraw the glyphs and the guides must follow.
                let lead = metrics.lsb(glyph) / upem;
                let middle = metrics.advance_width(glyph) / upem / 2.0;

                assert!((lead - TRIANGLE_LEAD).abs() < 0.001, "{marker} leads at {lead}");
                assert!((middle - TRIANGLE_MIDDLE).abs() < 0.001, "{marker} centres at {middle}");

                // A tree row centres the whole ascent-to-descent block, not the ink, so
                // ink drawn off that block's middle sits visibly low however the line
                // height is set — the row cancels it out. Every glyph in this font is
                // built around 348.5, and the arrows have to keep to it.
                let Some(outline) = scaler.builder(swash).size(upem).build().scale_outline(glyph) else {
                    panic!("{marker} has no outline");
                };

                let box_middle = (vertical.ascent - vertical.descent) / 2.0;
                let ink = outline.bounds();
                let ink_middle = (ink.min.y + ink.max.y) / 2.0;

                assert!((ink_middle - box_middle).abs() < 1.0, "{marker} inks at {ink_middle}, block at {box_middle}");

                // TRIANGLE_GAP is held off the ink on both axes — sideways from the shut
                // arrow's edge, downwards from the open arrow's apex — so the guides can
                // only keep an even clearance while the ink stays square about its centre.
                let across = (ink.max.x - ink.min.x) / 2.0 / upem;
                let down = (ink.max.y - ink.min.y) / 2.0 / upem;

                assert!((across - TRIANGLE_REACH).abs() < 0.001, "{marker} reaches {across} across");
                assert!((down - TRIANGLE_REACH).abs() < 0.001, "{marker} reaches {down} down");
            }

            assert_eq!(covered.map('A'), 0, "the query resolved some other face");

            return;
        }

        panic!("the font system stayed locked");
    }
}
