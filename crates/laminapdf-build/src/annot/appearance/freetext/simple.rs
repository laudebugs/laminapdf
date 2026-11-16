//! Text boxes in the standard 14 fonts: each character encoded in `WinAnsiEncoding` and measured
//! with its AFM width, lines broken greedily after spaces and hyphens, a word wider than the box
//! broken between characters. It needs no shaping and no Unicode tables, so the WebAssembly
//! extract module can draw boxes too, and every build draws the same bytes.

use laminapdf_ops::{RichText, StandardFamily};
use laminapdf_text::{Encoding, Standard14};

/// One run of a line in one face: its codes and width in points.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Piece {
    pub(super) face: Standard14,
    pub(super) bytes: Vec<u8>,
    pub(super) width: f64,
}

/// A laid-out line: its pieces left to right and its width in points, trailing spaces left out.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Line {
    pub(super) pieces: Vec<Piece>,
    pub(super) width: f64,
}

/// One character as drawn: its face, its code and its width in points.
#[derive(Debug, Clone, Copy)]
struct Glyph {
    face: Standard14,
    code: u8,
    width: f64,
    space: bool,
    /// A line may break after it: a space, or a hyphen after a non-space.
    breaks: bool,
}

/// The face of `family` for a span's style.
pub(super) fn face(family: StandardFamily, bold: bool, italic: bool) -> Standard14 {
    Standard14::from_base_font(family.face(bold, italic)).unwrap_or(Standard14::Helvetica)
}

/// `WinAnsiEncoding`'s codes from 0x80 to 0x9F, by the character each draws (ISO 32000-2
/// Annex D.2); the codes below are ASCII's and those above Latin-1's.
const WIN_ANSI_HIGH: [(char, u8); 27] = [
    ('\u{20ac}', 0x80),
    ('\u{201a}', 0x82),
    ('\u{0192}', 0x83),
    ('\u{201e}', 0x84),
    ('\u{2026}', 0x85),
    ('\u{2020}', 0x86),
    ('\u{2021}', 0x87),
    ('\u{02c6}', 0x88),
    ('\u{2030}', 0x89),
    ('\u{0160}', 0x8a),
    ('\u{2039}', 0x8b),
    ('\u{0152}', 0x8c),
    ('\u{017d}', 0x8e),
    ('\u{2018}', 0x91),
    ('\u{2019}', 0x92),
    ('\u{201c}', 0x93),
    ('\u{201d}', 0x94),
    ('\u{2022}', 0x95),
    ('\u{2013}', 0x96),
    ('\u{2014}', 0x97),
    ('\u{02dc}', 0x98),
    ('\u{2122}', 0x99),
    ('\u{0161}', 0x9a),
    ('\u{203a}', 0x9b),
    ('\u{0153}', 0x9c),
    ('\u{017e}', 0x9e),
    ('\u{0178}', 0x9f),
];

/// The `WinAnsiEncoding` code that draws `ch`, if any.
pub(super) fn win_ansi(ch: char) -> Option<u8> {
    match ch {
        ' ' | '\u{a0}' => Some(b' '),
        // The soft hyphen's code draws a hyphen, which 0x2D already does.
        '\u{ad}' => None,
        '\u{21}'..='\u{7e}' | '\u{a1}'..='\u{ff}' => u8::try_from(u32::from(ch)).ok(),
        _ => WIN_ANSI_HIGH.iter().find(|(c, _)| *c == ch).map(|(_, code)| *code),
    }
}

/// The first character of `text` that `WinAnsiEncoding` can't draw, characters a standard 14
/// font draws as nothing aside.
pub(super) fn first_missing(text: &RichText) -> Option<char> {
    text.paragraphs
        .iter()
        .flatten()
        .flat_map(|s| s.text.chars())
        .find(|&ch| !Standard14::draws_nothing(ch) && win_ansi(ch).is_none())
}

/// Lays `text` out in `family` at `size` points into lines at most `limit` points wide.
///
/// # Errors
/// The first character no face can draw.
pub(super) fn lay_out(text: &RichText, family: StandardFamily, size: f64, limit: f64) -> Result<Vec<Line>, char> {
    let mut lines = Vec::new();
    for paragraph in &text.paragraphs {
        let mut glyphs = Vec::new();
        for span in paragraph {
            let face = face(family, span.bold, span.italic);
            let mut previous = ' ';
            for ch in span.text.chars() {
                // A line break inside a span breaks the line, as the shaped layout's UAX #14
                // rules do; `\r\n` is one break.
                if matches!(ch, '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}') {
                    if !(ch == '\n' && previous == '\r') {
                        break_paragraph(&glyphs, limit, &mut lines);
                        glyphs.clear();
                    }
                    previous = ch;
                    continue;
                }
                if Standard14::draws_nothing(ch) {
                    continue;
                }
                let code = win_ansi(ch).ok_or(ch)?;
                let name = Encoding::WinAnsi.glyph_name(code).unwrap_or("space");
                let width = f64::from(face.width(name).unwrap_or(0)) * size / 1000.0;
                let space = ch == ' ';
                glyphs.push(Glyph {
                    face,
                    code,
                    width,
                    space,
                    breaks: space || (ch == '-' && previous != ' '),
                });
                previous = ch;
            }
        }
        break_paragraph(&glyphs, limit, &mut lines);
    }
    Ok(lines)
}

/// Greedy lines: as many words as fit, a word ending after a break opportunity; a word wider
/// than the box alone is broken between characters, at least one per line. An empty paragraph is
/// one empty line.
fn break_paragraph(glyphs: &[Glyph], limit: f64, lines: &mut Vec<Line>) {
    if glyphs.is_empty() {
        lines.push(Line::default());
        return;
    }
    let mut start = 0;
    while start < glyphs.len() {
        let mut end = start;
        let mut fit = None;
        let mut width = 0.0;
        for (i, g) in glyphs.iter().enumerate().skip(start) {
            width += g.width;
            let drawn = width - glyphs.get(start..=i).map_or(0.0, trailing_space);
            if drawn > limit && i > start {
                break;
            }
            end = i + 1;
            if g.breaks {
                fit = Some(end);
            }
        }
        // Break after the last opportunity that fits, else as many characters as fit.
        let stop = if end == glyphs.len() { end } else { fit.unwrap_or(end) };
        lines.push(line(glyphs.get(start..stop).unwrap_or_default()));
        start = stop;
    }
}

/// The width of the spaces `glyphs` ends with, which hang past the line's end.
fn trailing_space(glyphs: &[Glyph]) -> f64 {
    glyphs.iter().rev().take_while(|g| g.space).map(|g| g.width).sum()
}

/// A line of `glyphs`, trailing spaces dropped, runs of one face joined.
fn line(glyphs: &[Glyph]) -> Line {
    let drawn = glyphs.len() - glyphs.iter().rev().take_while(|g| g.space).count();
    let mut out = Line::default();
    for g in glyphs.iter().take(drawn) {
        match out.pieces.last_mut() {
            Some(piece) if piece.face == g.face => {
                piece.bytes.push(g.code);
                piece.width += g.width;
            }
            _ => out.pieces.push(Piece {
                face: g.face,
                bytes: vec![g.code],
                width: g.width,
            }),
        }
        out.width += g.width;
    }
    out
}

#[cfg(test)]
mod tests {
    use laminapdf_ops::TextSpan;

    use super::*;

    fn widths(lines: &[Line]) -> Vec<Vec<u8>> {
        lines
            .iter()
            .map(|l| l.pieces.iter().flat_map(|p| p.bytes.clone()).collect())
            .collect()
    }

    #[test]
    fn lines_break_after_the_last_space_that_fits() {
        // "aaa " is 556 × 3 + 278 thousandths at 10 pt: about 19.5 pt per word.
        let lines = lay_out(&RichText::plain("aaa bbb ccc"), StandardFamily::Helvetica, 10.0, 40.0).unwrap();
        assert_eq!(widths(&lines), [b"aaa bbb".to_vec(), b"ccc".to_vec()]);
    }

    #[test]
    fn a_word_wider_than_the_box_breaks_between_characters() {
        let lines = lay_out(&RichText::plain("abcdef"), StandardFamily::Courier, 10.0, 18.0).unwrap();
        // Courier is 6 pt a character at 10 pt: three fit in 18.
        assert_eq!(widths(&lines), [b"abc".to_vec(), b"def".to_vec()]);
    }

    #[test]
    fn a_line_may_break_after_a_hyphen() {
        let lines = lay_out(&RichText::plain("well-known"), StandardFamily::Courier, 10.0, 36.0).unwrap();
        assert_eq!(widths(&lines), [b"well-".to_vec(), b"known".to_vec()]);
    }

    #[test]
    fn bold_spans_take_the_bold_face() {
        let text = RichText {
            paragraphs: vec![vec![TextSpan::plain("a "), TextSpan::plain("b").with_bold(true)]],
        };
        let lines = lay_out(&text, StandardFamily::Times, 12.0, 500.0).unwrap();
        let faces: Vec<Standard14> = lines[0].pieces.iter().map(|p| p.face).collect();
        assert_eq!(faces, [Standard14::TimesRoman, Standard14::TimesBold]);
    }

    #[test]
    fn the_lookup_finds_the_first_code_of_every_character_win_ansi_draws() {
        // The scan the lookup replaces: the lowest code whose glyph is the character.
        let scan = |ch: char| {
            (0x20..=0xff_u8).find(|&code| {
                let text = Encoding::WinAnsi.to_unicode(code).unwrap_or_default();
                let mut chars = text.chars();
                chars.next() == Some(ch) && chars.next().is_none()
            })
        };
        let differ: Vec<char> = ('\u{21}'..='\u{3000}')
            .filter(|&ch| ch != '\u{a0}' && win_ansi(ch) != scan(ch))
            .collect();
        assert_eq!(differ, Vec::<char>::new());
    }

    #[test]
    fn a_line_break_inside_a_span_breaks_the_line() {
        let text = RichText {
            paragraphs: vec![vec![TextSpan::plain("one\r\ntwo\nthree")]],
        };
        let lines = lay_out(&text, StandardFamily::Courier, 10.0, 500.0).unwrap();
        assert_eq!(widths(&lines), [b"one".to_vec(), b"two".to_vec(), b"three".to_vec()]);
    }

    #[test]
    fn accented_letters_and_the_euro_sign_encode() {
        assert_eq!((win_ansi('é'), win_ansi('€')), (Some(0xe9), Some(0x80)));
    }

    #[test]
    fn a_character_outside_win_ansi_is_missing() {
        assert_eq!(first_missing(&RichText::plain("ok 日本")), Some('日'));
    }

    #[test]
    fn an_empty_paragraph_is_an_empty_line() {
        let lines = lay_out(&RichText::plain("a\n\nb"), StandardFamily::Helvetica, 10.0, 100.0).unwrap();
        assert_eq!(lines.len(), 3);
    }
}
