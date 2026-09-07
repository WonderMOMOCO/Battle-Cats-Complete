use std::collections::HashMap;
use std::sync::OnceLock;

use iced::advanced::graphics::text::{cosmic_text, font_system, Paragraph};
use iced::advanced::text::{Alignment, Paragraph as _, Text as Shaped};
use iced::alignment::Vertical;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::{Font, Pixels, Size};

const WIDE: [(u32, u32); 12] = [
    (0x1100, 0x115f),
    (0x2e80, 0x303e),
    (0x3041, 0x33ff),
    (0x3400, 0x4dbf),
    (0x4e00, 0x9fff),
    (0xa000, 0xa4cf),
    (0xac00, 0xd7a3),
    (0xf900, 0xfaff),
    (0xfe30, 0xfe6f),
    (0xff00, 0xff60),
    (0xffe0, 0xffe6),
    (0x20000, 0x3fffd),
];

const MIDDLE_DOT: char = '\u{00b7}';

static MONO: OnceLock<String> = OnceLock::new();

pub(crate) fn mono() -> Font {
    if let Some(named) = MONO.get() {
        return Font::with_name(named.as_str());
    }

    let Ok(mut system) = font_system().try_write() else {
        return Font::MONOSPACE;
    };

    let named = system.raw().db().family_name(&cosmic_text::fontdb::Family::Monospace).to_owned();

    if named.is_empty() {
        return Font::MONOSPACE;
    }

    Font::with_name(MONO.get_or_init(|| named).as_str())
}

pub(crate) fn wide(glyph: char) -> bool {
    let code = glyph as u32;

    WIDE.iter().any(|(first, last)| code >= *first && code <= *last)
}

pub(crate) fn shaping(label: &str) -> Shaping {
    match label.chars().all(|glyph| glyph.is_ascii() || glyph == MIDDLE_DOT) {
        true => Shaping::Basic,
        false => Shaping::Auto,
    }
}

pub(crate) struct Ruler {
    font: Font,
    size: f32,
    advances: HashMap<char, f32>,
}

impl Ruler {
    pub(crate) fn new(font: Font, size: f32) -> Self {
        Self { font, size, advances: HashMap::new() }
    }

    pub(crate) fn width(&mut self, content: &str) -> f32 {
        if !content.is_ascii() {
            return self.shape(content);
        }

        let mut total = 0.0;

        for glyph in content.chars() {
            if let Some(held) = self.advances.get(&glyph) {
                total += *held;
                continue;
            }

            let advance = self.shape(glyph.encode_utf8(&mut [0; 4]));

            self.advances.insert(glyph, advance);
            total += advance;
        }

        total
    }

    fn shape(&self, content: &str) -> f32 {
        Paragraph::with_text(Shaped {
            content,
            bounds: Size::INFINITE,
            size: Pixels(self.size),
            line_height: LineHeight::default(),
            font: self.font,
            align_x: Alignment::Default,
            align_y: Vertical::Top,
            shaping: Shaping::default(),
            wrapping: Wrapping::None,
        })
        .min_bounds()
        .width
    }
}

pub(crate) fn columns(text: &str) -> f32 {
    text.chars().map(|glyph| if wide(glyph) { 2.0 } else { 1.0 }).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kana_and_kanji_take_two_columns() {
        // Part names in the game data are Japanese, and measuring them as one
        // column each is what clipped the labels.
        assert_eq!(columns("リンゴ"), 6.0);
        assert_eq!(columns("土"), 2.0);
        assert_eq!(columns("abc"), 3.0);
    }

    #[test]
    fn a_mixed_label_counts_each_half_separately() {
        assert_eq!(columns("067土1.png"), 10.0);
    }

    #[test]
    fn the_monospace_family_resolves_to_a_font_with_a_name() {
        // Asking cosmic-text for the generic monospace family makes it rescan for a
        // font covering the script on every single shape, and never cache the miss.
        // Resolving the name needs the font system lock every shape also takes, so it
        // gives up and retries on a later frame rather than blocking on one. Here the
        // sibling tests hold that lock while the font database loads, so wait them out.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);

        let resolved = std::iter::repeat_with(mono)
            .inspect(|_| std::thread::yield_now())
            .take_while(|_| std::time::Instant::now() < deadline)
            .find(|named| matches!(named, Font { family: iced::font::Family::Name(_), .. }));

        assert!(resolved.is_some());
    }

    #[test]
    fn only_our_own_separator_earns_the_cheap_shaper() {
        // Advanced shaping costs about five times basic, and a lone middle dot was
        // dragging every tree row onto it. A real name still needs font fallback.
        assert_eq!(shaping("Angle \u{00b7} 6 keys"), Shaping::Basic);
        assert_eq!(shaping("Part 3"), Shaping::Basic);
        assert_eq!(shaping("Part 3 \u{00b7} \u{571f}"), Shaping::Auto);
        assert_eq!(shaping("\u{25b8}"), Shaping::Auto);
    }

    #[test]
    fn summed_advances_match_shaping_the_whole_name() {
        // The mining Files tab measures tens of thousands of names, and shaping each one
        // whole cost about a tenth of a second. Per-character advances have to add up to
        // the same width or the caption line count moves; only the order the floats are
        // summed in differs, which is worth a hundredth of a pixel at most.
        let mut ruler = Ruler::new(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }, 13.0);

        for name in ["game/cats/000/f/uni000_f00.png", "Unit_Explanation301_en.csv", "M_+#.tsv", ""] {
            let summed = ruler.width(name);
            let shaped = ruler.shape(name);

            assert!((summed - shaped).abs() < 0.01, "{name}: summed {summed} shaped {shaped}");
        }
    }

    #[test]
    fn a_name_outside_ascii_is_shaped_whole() {
        // Contextual scripts do not add up character by character, so a modder's name
        // has to fall back to shaping the string in one piece.
        let mut ruler = Ruler::new(Font::DEFAULT, 13.0);
        let name = "\u{571f}\u{4e0a}.png";

        assert_eq!(ruler.width(name), ruler.shape(name));
        assert!(ruler.width(name) > 0.0);
    }

    #[test]
    fn punctuation_around_the_ranges_stays_narrow() {
        assert!(!wide('·'));
        assert!(!wide('#'));
        assert!(wide('\u{ff21}'));
    }
}
