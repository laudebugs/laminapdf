//! A review's comments as threads (ISO 32000-2 §12.5.6.2, §12.5.6.4), in page space (ADR 0027):
//! each comment with its replies in order, the annotations grouped with it, the review states set
//! in the thread and the latest of each model, and, for a highlight or another text markup, the
//! text it marks. A comments panel filters and searches this list; it changes nothing.
//!
//! A reply is [`AddReply`](crate::AddReply), a status [`SetReviewState`](crate::SetReviewState),
//! and deleting a selection is [`Document::delete_annotations`].

use laminapdf_model::{AnnotGeometry, AnnotId, BrokenThread, CommentThread, PageFrame, StateModel, ThreadMember};

use crate::annotations::{PageAnnotation, record};
use crate::document::Document;
use crate::error::{Error, Result};

/// One comment and what answers it, each annotation in its own page's space.
///
/// Added in 0.4.1.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PageComment {
    /// The page the comment is on, from 0.
    pub page: u32,
    /// The comment.
    pub comment: PageAnnotation,
    /// The replies, replies to replies included, in document order; each one's
    /// [`PageAnnotation::in_reply_to`] names what it answers.
    pub replies: Vec<CommentPart>,
    /// The annotations grouped with it (a text correction's caret with its strike-out).
    pub group: Vec<CommentPart>,
    /// The review states set in the thread, in document order: its history.
    pub states: Vec<CommentPart>,
    /// The comment's latest `Review` status (`Accepted`, `Rejected`, `Cancelled`, `Completed`,
    /// `None`), by date, then by position.
    pub review: Option<String>,
    /// The comment's latest `Marked` state (`Marked` or `Unmarked`).
    pub marked: Option<String>,
    /// For text markup, the text under its areas: every glyph whose centre is in one of them, in
    /// the order the page shows them, lines joined by a space. Needs the `extract` feature;
    /// `None` without it and for other comments.
    pub quoted: Option<String>,
    /// Set when the thread doesn't start at a comment that answers nothing: a reply to a missing
    /// annotation, or a ring of replies.
    pub broken: Option<BrokenThread>,
}

/// An annotation of a thread, with its page.
///
/// Added in 0.4.1.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CommentPart {
    /// Its page, from 0.
    pub page: u32,
    /// The annotation.
    pub annotation: PageAnnotation,
}

impl Document {
    /// The comments on `pages` (every page when `None`), as threads in document order. A reply on
    /// another page than its comment belongs to the comment's thread, so a thread is listed with
    /// the page its comment is on.
    ///
    /// ```
    /// # #[cfg(feature = "text")] {
    /// use laminapdf::{AddReply, BuildOptions, Builder, NoteIcon, Op, PageSize, PointPt, Pt};
    ///
    /// let mut b = Builder::new(BuildOptions::new().with_id(*b"comment-threads1"));
    /// b.page(PageSize::A4).finish()?;
    /// let mut doc = b.finish()?;
    /// let note = doc.note_at(0, PointPt::new(Pt::new(56.0), Pt::new(56.0)), "Why?", NoteIcon::Comment)?;
    /// let id = doc.apply(vec![Op::from(note)])?.created_annotations.remove(0);
    /// doc.apply(vec![Op::from(AddReply::new(id, "Because"))])?;
    ///
    /// let threads = doc.comment_threads(None)?;
    /// assert_eq!(threads[0].replies[0].annotation.contents.as_deref(), Some("Because"));
    /// # }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range` for a page past the end; reading errors.
    ///
    /// Added in 0.4.1.
    pub fn comment_threads(&self, pages: Option<&[u32]>) -> Result<Vec<PageComment>> {
        let all = self.ops().pages()?;
        let count = u32::try_from(all.len()).unwrap_or(u32::MAX);
        if let Some(&index) = pages.into_iter().flatten().find(|&&p| p >= count) {
            return Err(Error::PageIndexOutOfRange { index, count });
        }
        let mut frames: Vec<Option<PageFrame>> = vec![None; all.len()];
        for (i, page) in all.iter().enumerate() {
            if let Some(slot) = frames.get_mut(i) {
                *slot = Some(page.frame()?);
            }
        }
        drop(all);
        let index = |i: usize| u32::try_from(i).unwrap_or(u32::MAX);
        let mut out = Vec::new();
        for thread in self.ops().comment_threads()? {
            let page = index(thread.page_index);
            if pages.is_some_and(|wanted| !wanted.contains(&page)) {
                continue;
            }
            let Some(frame) = frames.get(thread.page_index).copied().flatten() else {
                continue;
            };
            out.push(self.page_comment(thread, page, &frame, &frames)?);
        }
        Ok(out)
    }

    fn page_comment(
        &self,
        thread: CommentThread,
        page: u32,
        frame: &PageFrame,
        frames: &[Option<PageFrame>],
    ) -> Result<PageComment> {
        let state = |model: &StateModel| {
            thread
                .latest_state(model)
                .and_then(|a| a.state.as_ref())
                .map(|s| s.state.clone())
        };
        let (review, marked) = (state(&StateModel::Review), state(&StateModel::Marked));
        let quoted = self.quoted(page, &thread.comment.geometry, frame)?;
        let parts = |members: Vec<ThreadMember>| -> Vec<CommentPart> {
            members
                .into_iter()
                .filter_map(|m| {
                    let frame = frames.get(m.page_index).copied().flatten()?;
                    Some(CommentPart {
                        page: u32::try_from(m.page_index).unwrap_or(u32::MAX),
                        annotation: record(m.annot, &frame),
                    })
                })
                .collect()
        };
        Ok(PageComment {
            page,
            review,
            marked,
            quoted,
            broken: thread.broken,
            replies: parts(thread.replies),
            group: parts(thread.group),
            states: parts(thread.states),
            comment: record(thread.comment, frame),
        })
    }

    #[cfg(feature = "extract")]
    fn quoted(&self, page: u32, geometry: &AnnotGeometry, frame: &PageFrame) -> Result<Option<String>> {
        let AnnotGeometry::Quads(quads) = geometry else {
            return Ok(None);
        };
        let areas: Vec<[f64; 4]> = quads
            .iter()
            .map(|q| {
                let points = q.map(|p| frame.to_page(p));
                let xs = points.map(|p| p.x.get());
                let ys = points.map(|p| p.y.get());
                [
                    xs.iter().copied().fold(f64::INFINITY, f64::min),
                    ys.iter().copied().fold(f64::INFINITY, f64::min),
                    xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                ]
            })
            .collect();
        let text = self.page_text(page, &crate::extract::TextOptions::default().with_skip_invisible(true))?;
        let mut quoted = String::new();
        let mut last_area = None;
        for glyph in &text.glyphs {
            let x = glyph.quad.iter().map(|p| p.x.get()).sum::<f64>() / 4.0;
            let y = glyph.quad.iter().map(|p| p.y.get()).sum::<f64>() / 4.0;
            let Some(area) = areas
                .iter()
                .position(|[x0, y0, x1, y1]| (*x0..=*x1).contains(&x) && (*y0..=*y1).contains(&y))
            else {
                continue;
            };
            if last_area.is_some_and(|last| last != area) && !quoted.ends_with(char::is_whitespace) {
                quoted.push(' ');
            }
            last_area = Some(area);
            quoted.push_str(&glyph.text);
        }
        let quoted = quoted.trim();
        Ok((!quoted.is_empty()).then(|| quoted.to_owned()))
    }

    #[cfg(not(feature = "extract"))]
    #[expect(
        clippy::unused_self,
        clippy::unnecessary_wraps,
        reason = "the extract build's signature"
    )]
    fn quoted(&self, _page: u32, _geometry: &AnnotGeometry, _frame: &PageFrame) -> Result<Option<String>> {
        Ok(None)
    }

    /// Deletes the annotations `ids` in one transaction, one undo step: each with its Popup, its
    /// replies and its group, as [`DeleteAnnot`](crate::DeleteAnnot) does. An ID that another's
    /// deletion already takes (a reply selected with its comment, both members of a group) is
    /// left out, so selecting a comment and its reply deletes both and the second delete never
    /// fails.
    ///
    /// # Errors
    /// `unknown-annotation`, `annotation-not-editable`, `annotation-locked`; as
    /// [`Document::apply`].
    ///
    /// Added in 0.4.1.
    pub fn delete_annotations(&mut self, ids: &[AnnotId]) -> Result<crate::Edited> {
        let ops = laminapdf_ops::delete_annotations(self.ops(), ids)?;
        self.apply(ops)
    }
}
