//! Sticky-note icons (ISO 32000-2 §12.5.6.4, Table 183's seven names) and the caret
//! (§12.5.6.11). An icon is drawn in a 20-point square: the note's colour fills its shape and a
//! black outline draws it, so it reads on any page.

use laminapdf_model::UserSpaceRect;
use pdf_writer::Content;

use super::{DeviceColor, Drawn, Paint};
use crate::surface::n;

/// The standard note icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoteIconStyle {
    Note,
    Comment,
    Key,
    Help,
    Paragraph,
    NewParagraph,
    Insert,
}

/// A caret alone, or with a paragraph sign over it (`/Sy /P`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaretStyle {
    Plain,
    Paragraph,
}

/// The side of an icon's square, in points.
pub(crate) const ICON: f64 = 20.0;

/// Bézier control-point distance for a quarter circle of radius 1.
const KAPPA: f64 = 0.552_284_75;

/// The appearance of a note icon whose square has its lower-left corner at `(x, y)`.
pub(crate) fn note_icon(icon: NoteIconStyle, x: f64, y: f64, paint: &Paint) -> Drawn {
    let bbox = UserSpaceRect::from_corners(x, y, x + ICON, y + ICON);
    let mut drawn = Drawn::new(bbox, paint.opacity, false);
    let mut pen = Pen {
        c: &mut drawn.content,
        x,
        y,
    };
    paint.color_or(DeviceColor::Rgb(1.0, 0.82, 0.0)).fill(pen.c);
    paint.black().stroke(pen.c);
    pen.c.set_line_width(0.6);
    match icon {
        NoteIconStyle::Note => {
            pen.rect(3.0, 1.0, 14.0, 18.0);
            pen.c.fill_nonzero_and_stroke();
            for line in [14.0, 11.0, 8.0, 5.0] {
                pen.move_to(6.0, line);
                pen.line_to(14.0, line);
            }
            pen.c.stroke();
        }
        NoteIconStyle::Comment => {
            pen.move_to(2.0, 7.0);
            pen.line_to(18.0, 7.0);
            pen.line_to(18.0, 19.0);
            pen.line_to(2.0, 19.0);
            pen.c.close_path();
            pen.move_to(5.0, 7.0);
            pen.line_to(4.0, 1.5);
            pen.line_to(10.0, 7.0);
            pen.c.fill_nonzero_and_stroke();
        }
        NoteIconStyle::Key => {
            pen.circle(6.5, 13.5, 4.5);
            pen.c.fill_nonzero_and_stroke();
            pen.c.set_line_width(1.5);
            pen.move_to(9.5, 10.5);
            pen.line_to(17.5, 2.5);
            pen.move_to(14.5, 5.5);
            pen.line_to(16.5, 7.5);
            pen.move_to(12.0, 8.0);
            pen.line_to(14.0, 10.0);
            pen.c.stroke();
        }
        NoteIconStyle::Help => {
            pen.circle(10.0, 10.0, 9.0);
            pen.c.fill_nonzero_and_stroke();
            pen.c.set_line_width(1.6);
            pen.move_to(7.0, 12.5);
            pen.curve_to(7.0, 16.5, 13.0, 16.5, 13.0, 12.5);
            pen.curve_to(13.0, 10.0, 10.0, 10.0, 10.0, 7.5);
            pen.c.stroke();
            paint.black().fill(pen.c);
            pen.circle(10.0, 4.5, 1.0);
            pen.c.fill_nonzero();
        }
        NoteIconStyle::Paragraph => {
            pen.rect(1.0, 1.0, 18.0, 18.0);
            pen.c.fill_nonzero_and_stroke();
            paint.black().fill(pen.c);
            pen.c.set_line_width(1.2);
            pilcrow(&mut pen, 4.0, 3.0, 14.0);
        }
        NoteIconStyle::NewParagraph => {
            pen.move_to(10.0, 19.0);
            pen.line_to(2.0, 8.0);
            pen.line_to(18.0, 8.0);
            pen.c.close_path();
            pen.c.fill_nonzero_and_stroke();
            pen.c.set_line_width(1.2);
            for line in [5.0, 2.0] {
                pen.move_to(3.0, line);
                pen.line_to(17.0, line);
            }
            pen.c.stroke();
        }
        NoteIconStyle::Insert => {
            pen.move_to(10.0, 18.0);
            pen.line_to(2.0, 2.0);
            pen.line_to(18.0, 2.0);
            pen.c.close_path();
            pen.c.fill_nonzero_and_stroke();
        }
    }
    drawn
}

/// The appearance of a caret filling `rect`: two curved sides meeting at the top, Acrobat's
/// shape. With a paragraph sign, the caret takes the lower half and the sign the upper.
pub(crate) fn caret(style: CaretStyle, rect: UserSpaceRect, paint: &Paint) -> Drawn {
    let mut drawn = Drawn::new(rect, paint.opacity, false);
    let (w, h) = (rect.width(), rect.height());
    let caret_h = if style == CaretStyle::Paragraph { h / 2.0 } else { h };
    let mut pen = Pen {
        c: &mut drawn.content,
        x: rect.x0,
        y: rect.y0,
    };
    paint.color_or(DeviceColor::Rgb(0.0, 0.0, 1.0)).fill(pen.c);
    let mid = w / 2.0;
    pen.move_to(0.0, 0.0);
    pen.curve_to(mid * 0.9, caret_h * 0.1, mid, caret_h * 0.5, mid, caret_h);
    pen.curve_to(mid, caret_h * 0.5, mid * 1.1, caret_h * 0.1, w, 0.0);
    pen.c.close_path();
    pen.c.fill_nonzero();
    if style == CaretStyle::Paragraph {
        let blue = paint.color_or(DeviceColor::Rgb(0.0, 0.0, 1.0));
        blue.stroke(pen.c);
        pen.c.set_line_width(n((w / 12.0).max(0.3)));
        let side = (h / 2.0).min(w);
        pilcrow(&mut pen, mid - side / 2.0, h / 2.0, side);
    }
    drawn
}

/// A paragraph sign in the `size`-point square at `(x, y)`: a filled bowl and two stems.
fn pilcrow(pen: &mut Pen<'_>, x: f64, y: f64, size: f64) {
    let at = |fx: f64, fy: f64| (x + fx * size, y + fy * size);
    let r = size * 0.22;
    let (cx, cy) = at(0.42, 0.75);
    pen.circle(cx, cy, r);
    pen.c.fill_nonzero();
    let (sx, top) = at(0.6, 0.97);
    let (tx, bottom) = at(0.82, 0.0);
    let (bx, _) = at(0.42, 0.0);
    pen.move_to(bx, top);
    pen.line_to(tx, top);
    pen.move_to(sx, top);
    pen.line_to(sx, bottom);
    pen.move_to(tx, top);
    pen.line_to(tx, bottom);
    pen.c.stroke();
}

/// Draws in an icon's own coordinates, offset to its corner.
struct Pen<'c> {
    c: &'c mut Content,
    x: f64,
    y: f64,
}

impl Pen<'_> {
    fn move_to(&mut self, x: f64, y: f64) {
        self.c.move_to(n(self.x + x), n(self.y + y));
    }

    fn line_to(&mut self, x: f64, y: f64) {
        self.c.line_to(n(self.x + x), n(self.y + y));
    }

    fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        self.c.cubic_to(
            n(self.x + x1),
            n(self.y + y1),
            n(self.x + x2),
            n(self.y + y2),
            n(self.x + x3),
            n(self.y + y3),
        );
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.c.rect(n(self.x + x), n(self.y + y), n(w), n(h));
    }

    /// A circle as four quarter Béziers.
    fn circle(&mut self, cx: f64, cy: f64, r: f64) {
        let k = r * KAPPA;
        self.move_to(cx + r, cy);
        self.curve_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r);
        self.curve_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy);
        self.curve_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r);
        self.curve_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy);
        self.c.close_path();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAINT: Paint = Paint {
        color: None,
        opacity: 1.0,
        intent: None,
    };

    fn text(drawn: Drawn) -> String {
        String::from_utf8(drawn.content()).unwrap()
    }

    #[test]
    fn every_icon_fills_and_outlines_its_square() {
        for icon in [
            NoteIconStyle::Note,
            NoteIconStyle::Comment,
            NoteIconStyle::Key,
            NoteIconStyle::Help,
            NoteIconStyle::Paragraph,
            NoteIconStyle::NewParagraph,
            NoteIconStyle::Insert,
        ] {
            let drawn = text(note_icon(icon, 100.0, 200.0, &PAINT));
            assert!(drawn.contains("\nB"), "{icon:?} draws no outlined shape");
        }
    }

    #[test]
    fn an_icon_is_drawn_at_its_corner() {
        assert!(text(note_icon(NoteIconStyle::Insert, 100.0, 200.0, &PAINT)).contains("110 218 m"));
    }

    #[test]
    fn an_icon_in_a_cmyk_file_paints_in_cmyk() {
        let paint = Paint {
            intent: Some(super::super::IntentSpace::Cmyk),
            ..PAINT
        };
        let drawn = text(note_icon(NoteIconStyle::Note, 0.0, 0.0, &paint));
        assert!(drawn.contains(" k\n") && drawn.contains(" K\n") && !drawn.contains("rg"));
    }

    #[test]
    fn a_caret_meets_at_the_top_middle_of_its_rect() {
        let rect = UserSpaceRect::from_corners(10.0, 100.0, 20.0, 110.0);
        assert!(text(caret(CaretStyle::Plain, rect, &PAINT)).contains(" 15 110 c"));
    }

    #[test]
    fn a_paragraph_caret_adds_the_sign_above() {
        let rect = UserSpaceRect::from_corners(10.0, 100.0, 20.0, 110.0);
        let drawn = text(caret(CaretStyle::Paragraph, rect, &PAINT));
        assert!(drawn.contains(" 15 105 c") && drawn.ends_with("S"));
    }
}
