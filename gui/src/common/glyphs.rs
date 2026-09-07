use std::sync::OnceLock;

use iced::advanced::graphics::text::{cosmic_text, font_system};
use iced::widget::text::Shaping;
use iced::Font;

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
        assert!(matches!(mono(), Font { family: iced::font::Family::Name(_), .. }));
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
    fn punctuation_around_the_ranges_stays_narrow() {
        assert!(!wide('·'));
        assert!(!wide('#'));
        assert!(wide('\u{ff21}'));
    }
}
