//! Lettering drawn as paths: the outlines `glyphs.rs` holds, set along a baseline at a size. A
//! standard stamp's name needs no font this way.

use pdf_writer::Content;

use super::glyphs::{CAP_HEIGHT, GLYPHS, UNITS_PER_EM};
use crate::surface::n;

/// One segment of an outline, in font units, y up.
#[derive(Debug, Clone, Copy)]
pub(super) enum Seg {
    /// Starts a contour.
    M(i16, i16),
    /// A straight line.
    L(i16, i16),
    /// A cubic Bézier curve: two control points, then its end.
    C(i16, i16, i16, i16, i16, i16),
    /// Closes the contour.
    Z,
}

/// A glyph's advance and outline.
#[derive(Debug, Clone, Copy)]
pub(super) struct Glyph {
    pub(super) advance: u16,
    pub(super) path: &'static [Seg],
}

/// The glyph for `ch`; a character the table lacks is left out (a stamp's names use only
/// capitals and spaces, which a test checks).
fn glyph(ch: char) -> Option<&'static Glyph> {
    GLYPHS.iter().find(|(c, _)| *c == ch).map(|(_, g)| g)
}

/// The width of `text` at `size` points.
pub(super) fn width(text: &str, size: f64) -> f64 {
    let units: f64 = text.chars().filter_map(glyph).map(|g| f64::from(g.advance)).sum();
    units * size / UNITS_PER_EM
}

/// The height of a capital at `size` points.
pub(super) fn cap_height(size: f64) -> f64 {
    CAP_HEIGHT * size / UNITS_PER_EM
}

/// Adds `text`'s outlines to the current path, its baseline starting at `(x, y)`. The caller
/// fills it.
pub(super) fn trace(content: &mut Content, text: &str, size: f64, left: f64, baseline: f64) {
    let scale = size / UNITS_PER_EM;
    let mut pen = left;
    for g in text.chars().filter_map(glyph) {
        let at = |gx: i16, gy: i16| (n(pen + f64::from(gx) * scale), n(baseline + f64::from(gy) * scale));
        for seg in g.path {
            match *seg {
                Seg::M(px, py) => {
                    let (to_x, to_y) = at(px, py);
                    content.move_to(to_x, to_y);
                }
                Seg::L(px, py) => {
                    let (to_x, to_y) = at(px, py);
                    content.line_to(to_x, to_y);
                }
                Seg::C(x1, y1, x2, y2, x3, y3) => {
                    let (one, two, end) = (at(x1, y1), at(x2, y2), at(x3, y3));
                    content.cubic_to(one.0, one.1, two.0, two.1, end.0, end.1);
                }
                Seg::Z => {
                    content.close_path();
                }
            }
        }
        pen += f64::from(g.advance) * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_standard_name_has_its_glyphs() {
        for name in laminapdf_ops::StampName::ALL {
            let missing: Vec<char> = name.lettering().chars().filter(|c| glyph(*c).is_none()).collect();
            assert!(missing.is_empty(), "{name:?} lacks {missing:?}");
        }
    }

    #[test]
    fn a_space_is_as_wide_as_its_advance_and_draws_nothing() {
        let mut c = Content::new();
        trace(&mut c, " ", 10.0, 0.0, 0.0);
        assert_eq!(c.finish().into_vec(), Vec::<u8>::new());
        assert!(width(" ", 10.0) > 0.0);
    }
}
