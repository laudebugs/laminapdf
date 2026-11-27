//! Where a page mark's text and pictures sit on the page (E6 plan §2.3), and the fields of a header
//! or footer: pure functions of the page's size, so the same op writes the same bytes everywhere.
//!
//! Everything is in page space (ADR 0027): points from the page's top-left corner as shown, y
//! down. A text box is laid out in its own space, `(0, 0)` its bottom-left corner and y up, so its
//! matrix is `[1, 0, 0, -1, x, y]` with `(x, y)` the box's bottom-left corner in page space; a
//! picture fills the unit square, `(0, 0)` its top-left corner, so its matrix is `[w, 0, 0, h, x, y]`.

use std::borrow::Cow;

use laminapdf_model::Matrix;

use super::mark_types::{MarkMargins, Slot};
use super::spec::{ContentObjectSpec, TextBlock};
use crate::annot::{AnnotColor, AnnotFont, TextAlign, TextSpan};
use crate::blob::BlobId;

/// How tall a line of text is, as a multiple of its size: the line, its descent and a little
/// room, so a box one line tall clips nothing.
const LINE: f64 = 1.5;

/// The watermark's box is this much of the page's diagonal wide, so the text turned to any angle
/// still fits the page when it is shorter than that.
const DIAGONAL: f64 = 0.9;

/// A watermark picture fills this much of the page, at most.
const WATERMARK_FILL: f64 = 0.8;

/// One thing a mark draws: an object and the matrix that places it on the page.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Child {
    pub(super) spec: ContentObjectSpec,
    pub(super) matrix: Matrix,
    /// The edge of the page a slot's text hangs from; none for a mark drawn over the page.
    pub(super) band: Option<Band>,
}

/// A running header's edge of the page, or a running footer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Band {
    Header,
    Footer,
}

/// A page's displayed size in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Sheet {
    pub(super) width: f64,
    pub(super) height: f64,
}

/// A mark's text: its fonts, size and colour.
#[derive(Debug, Clone, Copy)]
pub(super) struct Face<'a> {
    pub(super) fonts: &'a [AnnotFont],
    pub(super) size: f64,
    pub(super) color: Option<AnnotColor>,
}

impl Face<'_> {
    fn block(&self, text: &str, width: f64, height: f64, align: TextAlign) -> ContentObjectSpec {
        let mut block = TextBlock::new(width, height, vec![TextSpan::plain(text)])
            .with_fonts(self.fonts.to_vec(), self.size)
            .with_align(align);
        block.color = self.color;
        ContentObjectSpec::Text(block)
    }
}

/// The text of the slot `slot`, as one line of a box as wide as the text area, aligned by the
/// slot's column and at the margin the slot's edge says; `None` for no text. The boxes of the
/// three slots on an edge overlap, so a slot's text may run past its third of the page.
pub(super) fn slot_child(text: &str, slot: Slot, margins: MarkMargins, sheet: Sheet, face: Face<'_>) -> Option<Child> {
    if text.trim().is_empty() {
        return None;
    }
    let width = (sheet.width - margins.left - margins.right).max(1.0);
    let height = face.size * LINE * f64::from(u8::try_from(text.matches('\n').count() + 1).unwrap_or(u8::MAX));
    let x = margins.left;
    // The box's bottom edge in page space: a top slot hangs from the margin, a bottom slot
    // stands on it.
    let bottom = if slot.is_top() {
        margins.top + height
    } else {
        sheet.height - margins.bottom
    };
    let align = match slot.column() {
        0 => TextAlign::Left,
        1 => TextAlign::Center,
        _ => TextAlign::Right,
    };
    Some(Child {
        spec: face.block(text, width, height, align),
        matrix: Matrix::new([1.0, 0.0, 0.0, -1.0, x, bottom]),
        band: Some(if slot.is_top() { Band::Header } else { Band::Footer }),
    })
}

/// A watermark's or a background's text, centred on the page and turned `angle` degrees
/// counterclockwise as shown.
pub(super) fn centred_text(text: &str, angle: f64, sheet: Sheet, face: Face<'_>) -> Child {
    let lines = u8::try_from(text.matches('\n').count() + 1).unwrap_or(u8::MAX);
    let width = (sheet.width.hypot(sheet.height) * DIAGONAL).max(1.0);
    let height = face.size * LINE * f64::from(lines);
    // The box's centre on the origin, y down, then turned and moved to the page's centre.
    let matrix = Matrix::new([1.0, 0.0, 0.0, -1.0, -width / 2.0, height / 2.0]);
    Child {
        spec: face.block(text, width, height, TextAlign::Center),
        matrix: matrix.then(&about_centre(angle, sheet)),
        band: None,
    }
}

/// An image `pixels` wide and high, centred on the page and turned `angle` degrees
/// counterclockwise: shown whole inside most of the page for a watermark, or covering the page for
/// a background.
pub(super) fn centred_image(blob: BlobId, pixels: (u32, u32), cover: bool, angle: f64, sheet: Sheet) -> Child {
    let (iw, ih) = (f64::from(pixels.0).max(1.0), f64::from(pixels.1).max(1.0));
    let (fit_w, fit_h) = (sheet.width / iw, sheet.height / ih);
    let scale = if cover {
        // Turned, the page's corners reach past the picture: grow it to the diagonal's worth.
        let turned = angle.to_radians();
        let (s, c) = (turned.sin().abs(), turned.cos().abs());
        let (w, h) = (sheet.width * c + sheet.height * s, sheet.width * s + sheet.height * c);
        (w / iw).max(h / ih)
    } else {
        fit_w.min(fit_h) * WATERMARK_FILL
    };
    let (w, h) = (iw * scale, ih * scale);
    Child {
        spec: ContentObjectSpec::Image { blob, crop: None },
        matrix: Matrix::new([w, 0.0, 0.0, h, -w / 2.0, -h / 2.0]).then(&about_centre(angle, sheet)),
        band: None,
    }
}

/// A turn of `angle` degrees counterclockwise as shown, about the origin, then a move of the
/// origin to the page's centre.
fn about_centre(angle: f64, sheet: Sheet) -> Matrix {
    // `Matrix::rotate` turns clockwise in y-down page space.
    Matrix::rotate(-angle).then(&Matrix::translate(sheet.width / 2.0, sheet.height / 2.0))
}

/// What the fields of a header or footer stand for on one page.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fields<'a> {
    /// The 1-based page number.
    pub(super) page: usize,
    /// The number of pages.
    pub(super) pages: usize,
    /// The page's label, or its number when the document has no labels.
    pub(super) label: &'a str,
    /// The text of `{date}`, given by the op.
    pub(super) date: &'a str,
    /// The text of `{file}`, given by the op.
    pub(super) file: &'a str,
}

/// `text` with its fields filled in: `{page}`, `{pages}`, `{label}`, `{date}` and `{file}`, and
/// `{{` and `}}` for braces. An unknown field, or a brace that opens none, stays as written. The
/// scanner is the flowing layout's ([`fill_template`](crate::fill_template), ADR 0032 §3), which
/// knows other names.
pub(super) fn fill_fields(text: &str, fields: &Fields<'_>) -> String {
    crate::fill_template(text, |name| match name {
        "page" => Some(Cow::Owned(fields.page.to_string())),
        "pages" => Some(Cow::Owned(fields.pages.to_string())),
        "label" => Some(Cow::Borrowed(fields.label)),
        "date" => Some(Cow::Borrowed(fields.date)),
        "file" => Some(Cow::Borrowed(fields.file)),
        _ => None,
    })
}

/// A Bates number: `prefix`, `number` padded with zeros to `digits`, then `suffix`.
pub(super) fn bates(prefix: &str, number: u64, digits: u8, suffix: &str) -> String {
    format!("{prefix}{number:0width$}{suffix}", width = usize::from(digits))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIELDS: Fields<'static> = Fields {
        page: 3,
        pages: 24,
        label: "iii",
        date: "27 Nov 2026",
        file: "report.pdf",
    };

    #[test]
    fn fields_are_filled_and_braces_escaped() {
        assert_eq!(fill_fields("Page {page} of {pages}", &FIELDS), "Page 3 of 24");
        assert_eq!(
            fill_fields("{label} · {date} · {file}", &FIELDS),
            "iii · 27 Nov 2026 · report.pdf"
        );
        assert_eq!(fill_fields("{{page}} = {page}", &FIELDS), "{page} = 3");
    }

    #[test]
    fn an_unknown_field_and_a_lone_brace_stay_as_written() {
        assert_eq!(fill_fields("{nope} { } }", &FIELDS), "{nope} { } }");
    }

    #[test]
    fn bates_numbers_are_padded_and_grow_past_their_digits() {
        assert_eq!(bates("ACME-", 7, 6, ""), "ACME-000007");
        assert_eq!(bates("", 1234, 2, " CONF"), "1234 CONF");
        assert_eq!(bates("", 5, 0, ""), "5");
    }

    #[test]
    fn a_top_slot_hangs_from_the_margin_and_a_bottom_slot_stands_on_it() {
        let sheet = Sheet {
            width: 612.0,
            height: 792.0,
        };
        let face = Face {
            fonts: &[],
            size: 10.0,
            color: None,
        };
        let margins = MarkMargins::default();
        let top = slot_child("x", Slot::TopLeft, margins, sheet, face).unwrap();
        let bottom = slot_child("x", Slot::BottomRight, margins, sheet, face).unwrap();
        // The box is 15 high: its top edge is the margin, its bottom edge the margin plus that.
        assert_eq!(top.matrix.to_array(), [1.0, 0.0, 0.0, -1.0, 36.0, 51.0]);
        // The same left edge in every column, standing on the bottom margin.
        assert_eq!(bottom.matrix.to_array(), [1.0, 0.0, 0.0, -1.0, 36.0, 756.0]);
        assert!(slot_child("  ", Slot::TopLeft, margins, sheet, face).is_none());
    }

    #[test]
    fn a_watermark_turns_about_the_pages_centre() {
        let sheet = Sheet {
            width: 200.0,
            height: 100.0,
        };
        let face = Face {
            fonts: &[],
            size: 10.0,
            color: None,
        };
        let straight = centred_text("DRAFT", 0.0, sheet, face);
        // The box's middle, (width / 2, height / 2) in its own space, lands on the page's.
        let ContentObjectSpec::Text(block) = &straight.spec else {
            panic!("text");
        };
        let (x, y) = straight.matrix.apply(block.width / 2.0, block.height / 2.0);
        assert!((x - 100.0).abs() < 1e-9 && (y - 50.0).abs() < 1e-9, "{x} {y}");
        let turned = centred_text("DRAFT", 45.0, sheet, face);
        let (x, y) = turned.matrix.apply(block.width / 2.0, block.height / 2.0);
        assert!((x - 100.0).abs() < 1e-9 && (y - 50.0).abs() < 1e-9, "{x} {y}");
        // Counterclockwise as shown: the box's right end rises (y shrinks, down being larger).
        let (_, right) = turned.matrix.apply(block.width, block.height / 2.0);
        let (_, left) = turned.matrix.apply(0.0, block.height / 2.0);
        assert!(right < left);
    }
}
