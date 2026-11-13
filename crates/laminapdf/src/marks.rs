//! Marking up a page (ISO 32000-2 §12.5.6): highlights, underlines, strike-outs and squiggly
//! lines over text, sticky notes and carets, built from what a viewer has in hand (a selection's
//! line boxes, a click, a drawn box) in page space (ADR 0027). Each method gives the edit
//! ([`AddAnnot`]), which [`Document::apply`] applies; [`Document::replace_text`] applies its two
//! edits itself, as one undo step.
//!
//! Every mark gets an appearance stream drawn by `laminapdf-build`, which this crate gives every
//! document it opens.

use laminapdf_model::{PageFrame, PointPt, RectPt, UserSpaceRect};

use crate::links::user_rect;
use laminapdf_ops::{AddAnnot, AnnotCommon, AnnotSpec, Caret, MarkupKind, Note, NoteIcon, Origin, TextMarkup};

use crate::document::{Document, Edited};
use crate::error::Result;

impl Document {
    /// The edit that marks text of page `page` (0-based) with `kind`: `lines` are the boxes of
    /// the text, one per line, in page space, as a selection gives them, and each becomes one of
    /// the mark's quads. In a tagged document the mark is placed in the structure tree after the
    /// text it marks, and its description (`/Contents`, what a screen reader reads) is that text,
    /// or the kind of mark when the boxes hold none ("Highlighted text").
    ///
    /// # Errors
    /// `invalid-op` for no lines; `page-index-out-of-range`; the reading errors of
    /// [`Document::page_text`].
    ///
    /// Added in 0.3.0.
    pub fn mark_text(&self, page: u32, kind: MarkupKind, lines: &[RectPt]) -> Result<AddAnnot> {
        if lines.is_empty() {
            return Err(laminapdf_ops::Error::InvalidOp("a mark on text needs one line or more").into());
        }
        let (id, frame) = self.page_frame(page)?;
        let quads = lines.iter().map(|l| quad(&frame, l)).collect();
        #[cfg_attr(
            not(feature = "extract"),
            expect(unused_mut, reason = "only extraction fills in the marked text")
        )]
        let mut op = AddAnnot::new(id, AnnotSpec::TextMarkup(TextMarkup::new(kind, quads)));
        #[cfg(feature = "extract")]
        {
            let under = self.text_under(page, lines)?;
            op.marked = under.marked;
            if self.is_tagged()? {
                op.common.contents = Some(if under.text.is_empty() {
                    described(kind).to_owned()
                } else {
                    under.text
                });
            }
        }
        Ok(op)
    }

    /// The edit that puts a sticky note on page `page` (0-based): its `icon` with its upper-left
    /// corner at `at` in page space, and `comment` in its Popup, closed. The Popup opens beside
    /// the icon as the page is shown, on the page.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.3.0.
    pub fn note_at(&self, page: u32, at: PointPt, comment: &str, icon: NoteIcon) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        let mut op = AddAnnot::new(id, AnnotSpec::Note(Note::new(frame.to_user_space(at), icon)));
        op.common.contents = Some(comment.to_owned());
        Ok(op)
    }

    /// The edit that puts a caret on page `page` (0-based), filling `area` in page space: where
    /// text goes.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.3.0.
    pub fn caret_at(&self, page: u32, area: RectPt) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        Ok(AddAnnot::new(
            id,
            AnnotSpec::Caret(Caret::new(user_rect(&frame, &[area]))),
        ))
    }

    /// Proposes replacing text of page `page` (0-based) with `replacement`: a strike-out over
    /// `lines` (the text's line boxes in page space) and a caret after its last line, grouped
    /// (`/RT /Group`), so they show, move and delete as one. The caret is the group's primary
    /// annotation and holds `replacement` as its `/Contents`, where Acrobat keeps an inserted
    /// text; the strike-out names the caret (`/IRT`). `common` goes to both, but its
    /// `contents` and `group_with`, which this method sets.
    ///
    /// Both edits are applied as one transaction, one undo step. The caret's ID is
    /// [`Edited::created_annotations`]' first, the strike-out's its second.
    ///
    /// # Errors
    /// As [`Document::mark_text`]; the edits' errors (`appearance-unavailable` in a build
    /// without the `build` feature); `saved-not-rebased`.
    ///
    /// Added in 0.3.0.
    pub fn replace_text(
        &mut self,
        page: u32,
        lines: &[RectPt],
        replacement: &str,
        common: AnnotCommon,
    ) -> Result<Edited> {
        let mut strike = self.mark_text(page, MarkupKind::StrikeOut, lines)?;
        let (id, frame) = self.page_frame(page)?;
        let mut caret = AddAnnot::new(id, AnnotSpec::Caret(Caret::new(caret_after(&frame, lines))));
        caret.common = common.clone();
        caret.common.contents = Some(replacement.to_owned());
        caret.common.group_with = None;
        let quoted = strike.common.contents.take();
        strike.common = common;
        strike.common.contents = quoted;
        strike.common.group_with = None;
        let ((), applied) = self.ops_mut().transact(Origin::Local, |tx| {
            let caret = tx.apply(caret)?.created_annotations;
            strike.common.group_with = caret.first().cloned();
            tx.apply(strike)?;
            Ok(())
        })?;
        Ok(self.edited(applied))
    }
}

/// What a screen reader says for a mark over no text it could read.
#[cfg(feature = "extract")]
const fn described(kind: MarkupKind) -> &'static str {
    match kind {
        MarkupKind::Highlight => "Highlighted text",
        MarkupKind::Underline => "Underlined text",
        MarkupKind::StrikeOut => "Struck-out text",
        MarkupKind::Squiggly => "Text marked with a wavy line",
    }
}

/// A line box in page space as a quad in user space, in Acrobat's order: upper left, upper right,
/// lower left, lower right of the text as it reads.
fn quad(frame: &PageFrame, l: &RectPt) -> [laminapdf_model::UserSpacePoint; 4] {
    [
        PointPt::new(l.x0, l.y0),
        PointPt::new(l.x1, l.y0),
        PointPt::new(l.x0, l.y1),
        PointPt::new(l.x1, l.y1),
    ]
    .map(|p| frame.to_user_space(p))
}

/// A caret's box after the last line: as tall as the line's lower two thirds, and half as
/// wide as it is tall, centred on the line's end.
fn caret_after(frame: &PageFrame, lines: &[RectPt]) -> UserSpaceRect {
    let last = lines.last().copied().unwrap_or_default();
    let height = (last.y1.get() - last.y0.get()) * 2.0 / 3.0;
    let half = height / 4.0;
    let x = last.x1.get();
    let area = RectPt::from_points(x - half, last.y1.get() - height, half * 2.0, height);
    user_rect(frame, &[area])
}

#[cfg(feature = "extract")]
/// The text under a set of line boxes, and the MCIDs of its glyphs.
pub(crate) struct TextUnder {
    /// The marked-content IDs, in reading order, each once.
    pub(crate) marked: Vec<u32>,
    /// The glyphs' text, in content order, its lines joined by spaces.
    pub(crate) text: String,
}

#[cfg(feature = "extract")]
impl Document {
    /// The glyphs of page `page` whose centres lie in `lines` (page space).
    pub(crate) fn text_under(&self, page: u32, lines: &[RectPt]) -> Result<TextUnder> {
        let text = self.page_text(page, &crate::extract::TextOptions::default())?;
        let mut under = TextUnder {
            marked: Vec::new(),
            text: String::new(),
        };
        let mut last_line = None;
        for glyph in &text.glyphs {
            let [a, _, c, _] = glyph.quad;
            let (x, y) = (f64::midpoint(a.x.get(), c.x.get()), f64::midpoint(a.y.get(), c.y.get()));
            let Some(line) = lines
                .iter()
                .position(|l| (l.x0.get()..=l.x1.get()).contains(&x) && (l.y0.get()..=l.y1.get()).contains(&y))
            else {
                continue;
            };
            if last_line.is_some_and(|l| l != line) && !under.text.ends_with(' ') {
                under.text.push(' ');
            }
            last_line = Some(line);
            under.text.push_str(&glyph.text);
            if let Some(m) = glyph.mcid.and_then(|m| u32::try_from(m).ok())
                && !under.marked.contains(&m)
            {
                under.marked.push(m);
            }
        }
        under.text = under.text.trim().to_owned();
        Ok(under)
    }
}
