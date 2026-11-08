//! The lines a paragraph is broken into, and the runs they are drawn with.

use std::ops::Range;

use crate::shape::ShapedRun;

/// One shaped run of a line.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LineRun {
    /// The glyphs (clusters are byte offsets into the paragraph's text).
    pub run: ShapedRun,
    /// The font: an index into the list given to [`break_lines`](super::break_lines) or
    /// [`break_spans`](super::break_spans).
    pub font: usize,
    /// The bidi level the run was shaped at (odd: right to left).
    pub level: u8,
    /// The run's left edge, in points from the line's left edge.
    pub x: f64,
    /// The spans the run's text comes from: indices into the list given to
    /// [`break_spans`](super::break_spans) (`0..1` for [`break_lines`](super::break_lines)).
    /// Consecutive spans of one style are shaped as one run, so a run can hold several (a
    /// link inside a sentence); it lists only those with text on its line.
    pub spans: Range<usize>,
    /// The font size the run was shaped and measured at, in points.
    pub size: f32,
    /// The run's baseline rise in points ([`StyledSpan::rise`](super::StyledSpan::rise)): draw it with
    /// [`pdf_line_run`](crate::pdf_line_run).
    pub rise: f32,
}

/// One line of a paragraph.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Line {
    /// The line's bytes in the text, trailing spaces and the hard break included.
    pub range: Range<usize>,
    /// The bytes drawn: [`Line::range`] without trailing spaces and hard breaks, which hang
    /// past the line's end (UAX #14) and are neither drawn nor measured.
    pub content: Range<usize>,
    /// The runs in visual order, left to right. Draw them in [`Line::drawing_order`] (each
    /// at its own `x`) so extractors read the line in order.
    pub runs: Vec<LineRun>,
    /// The width of the drawn content in points.
    pub width: f64,
    /// Where the line starts, in points from the box's left edge (alignment).
    pub x: f64,
    /// The line ended at a hard break (a line feed, U+2028, …) rather than for lack of room
    /// or at the end of the text.
    pub hard_break: bool,
    /// How far the line reaches above its baseline, in points: the highest of its runs'
    /// font ascents at their sizes, raised by their rises. Set lines at least
    /// `ascent − descent` apart so mixed sizes never overlap.
    pub ascent: f64,
    /// How far it reaches below (negative: below the baseline), as for [`Line::ascent`] with
    /// the fonts' descents.
    pub descent: f64,
}

impl Line {
    /// The order to draw [`Line::runs`] in for text extraction: their indices in
    /// logical (reading) order, by where each run's text starts. Each run is still drawn at
    /// its own `x`, so the page looks the same in any order, but extractors that keep the
    /// order of the content stream across a font change (pypdf flushes its right-to-left
    /// reversal at every `Tf`, pdf.js orders text items by appearance) read a right-to-left
    /// line made of several runs (Hebrew words around a comma from another font) in the
    /// order it was written.
    ///
    /// ```
    /// # fn hebrew() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSansHebrew-Regular.ttf")).to_vec() }
    /// # fn latin() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
    /// use laminapdf_text::{Font, HarfRust, LayoutOptions, break_lines};
    ///
    /// let fonts = [Font::from_bytes(hebrew())?, Font::from_bytes(latin())?];
    /// let text = "שלום, עולם";
    /// let line = &break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(400.0, 12.0))?[0];
    /// let order = line.drawing_order();
    /// // Visual order is left to right; the first run drawn holds the text's start.
    /// assert_eq!(line.runs[order[0]].run.text_range.start, 0);
    /// # Ok::<(), laminapdf_text::Error>(())
    /// ```
    #[must_use]
    pub fn drawing_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.runs.len()).collect();
        order.sort_by_key(|&i| self.runs.get(i).map(|r| r.run.text_range.start));
        order
    }

    /// Moves the line's byte offsets by `by` (from its paragraph's text to the whole text).
    pub(super) fn shift(&mut self, by: usize) {
        self.range = self.range.start + by..self.range.end + by;
        self.content = self.content.start + by..self.content.end + by;
        for run in &mut self.runs {
            run.run.text_range = run.run.text_range.start + by..run.run.text_range.end + by;
            for glyph in &mut run.run.glyphs {
                glyph.cluster += by;
            }
        }
    }
}
