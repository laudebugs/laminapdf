//! Comments to and from other apps (E5d, C06): XFDF and FDF both ways, Markdown out. An export
//! changes nothing; an import is one transaction of ordinary annotation ops, so one undo step.

use laminapdf_interchange as ix;
pub use laminapdf_interchange::{
    Format as CommentFormat, ImportMode, ImportOptions, ImportSummary, ImportWarning, MarkdownOptions, SkipReason,
    Skipped,
};

use crate::{Document, Edited, Error, Result};

/// Which comments an export writes (C06's Comments).
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExportComments {
    /// Every comment, with its replies and statuses.
    #[default]
    All,
    /// Highlights and the other text markup, and notes, with their replies: what a notes app
    /// wants.
    HighlightsAndNotes,
}

/// How to export comments.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExportOptions {
    /// The format.
    pub format: CommentFormat,
    /// The pages, counted from 0; every page when `None`.
    pub pages: Option<Vec<u32>>,
    /// Which comments.
    pub comments: ExportComments,
    /// The document's file name, which XFDF (`<f href>`) and FDF (`/F`) record so an import can
    /// tell when it is given to another PDF.
    pub file: Option<String>,
    /// What Markdown includes.
    pub markdown: MarkdownOptions,
}

impl ExportOptions {
    /// Every comment on every page, in `format`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(format: CommentFormat) -> Self {
        Self {
            format,
            pages: None,
            comments: ExportComments::All,
            file: None,
            markdown: MarkdownOptions::default(),
        }
    }
}

/// The subtypes of [`ExportComments::HighlightsAndNotes`].
const HIGHLIGHTS_AND_NOTES: [&str; 5] = ["Highlight", "Underline", "StrikeOut", "Squiggly", "Text"];

impl Document {
    /// The document's comments as an XFDF (ISO 19444-1), FDF (ISO 32000-2 §12.7.8) or Markdown
    /// file. Nothing in the document changes. XFDF and FDF carry the file's `/ID` and the
    /// [`file`](ExportOptions::file) name, keep each reply with its comment by `/NM`, and carry
    /// attached files; Markdown lists each page's comments with the text they mark, as C06's
    /// Copy and preview show.
    ///
    /// ```
    /// # #[cfg(feature = "text")] {
    /// use laminapdf::{BuildOptions, Builder, CommentFormat, ExportOptions, NoteIcon, PageSize, PointPt, Pt};
    ///
    /// let mut b = Builder::new(BuildOptions::new().with_id(*b"export-comments1"));
    /// b.page(PageSize::A4).finish()?;
    /// let mut doc = b.finish()?;
    /// let note = doc.note_at(0, PointPt::new(Pt::new(56.0), Pt::new(56.0)), "Check the total", NoteIcon::Comment)?;
    /// doc.apply(vec![note.into()])?;
    /// let xfdf = doc.export_annotations(&ExportOptions::new(CommentFormat::Xfdf))?;
    /// assert!(String::from_utf8_lossy(&xfdf).contains("<contents>Check the total</contents>"));
    /// # }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range` for a page past the end; reading errors.
    ///
    /// Added in 0.7.0.
    pub fn export_annotations(&self, options: &ExportOptions) -> Result<Vec<u8>> {
        let count = u32::try_from(self.ops().pages()?.len()).unwrap_or(u32::MAX);
        if let Some(&index) = options.pages.iter().flatten().find(|&&p| p >= count) {
            return Err(Error::PageIndexOutOfRange { index, count });
        }
        let pages = options.pages.as_deref();
        match options.format {
            CommentFormat::Markdown => Ok(self.markdown(pages, options)?.into_bytes()),
            format => {
                let mut exchange = ix::exchange_of(self.ops(), pages, options.file.as_deref())?;
                if options.comments == ExportComments::HighlightsAndNotes {
                    keep_highlights_and_notes(&mut exchange);
                }
                Ok(if format == CommentFormat::Fdf {
                    ix::write_fdf(&exchange)
                } else {
                    ix::write_xfdf(&exchange)
                })
            }
        }
    }

    /// What importing an XFDF or FDF file would do, without changing the document: how many
    /// comments it adds and on which pages, how many it finds already here, and what it skips or
    /// warns about (C06's summary).
    ///
    /// # Errors
    /// As [`Document::import_annotations`], except that nothing is applied.
    ///
    /// Added in 0.7.0.
    pub fn preview_import(&self, bytes: &[u8], options: &ImportOptions) -> Result<ImportSummary> {
        let exchange = ix::read(bytes, self.ops().cos().limits())?;
        Ok(ix::preview(self.ops(), &exchange, options)?)
    }

    /// Imports an XFDF or FDF file's comments as one transaction, so one undo step takes them all
    /// back. [`ImportMode::Merge`] keeps the comments already here (matched by `/NM` on the same
    /// page) and adds the rest, [`ImportMode::Replace`] deletes the comments on the pages the
    /// file names first, and [`ImportMode::Add`] adds everything as copies. Each comment gets an
    /// appearance drawn by Lamina PDF, and keeps the Popup the file places.
    ///
    /// ```
    /// # #[cfg(feature = "text")] {
    /// use laminapdf::{
    ///     BuildOptions, Builder, CommentFormat, ExportOptions, ImportMode, ImportOptions, NoteIcon, PageSize, PointPt, Pt,
    /// };
    ///
    /// let blank = || -> Result<laminapdf::Document, laminapdf::Error> {
    ///     let mut b = Builder::new(BuildOptions::new().with_id(*b"import-comments1"));
    ///     b.page(PageSize::A4).finish()?;
    ///     b.finish()
    /// };
    /// let mut reviewed = blank()?;
    /// let note = reviewed.note_at(0, PointPt::new(Pt::new(56.0), Pt::new(56.0)), "Check the total", NoteIcon::Comment)?;
    /// reviewed.apply(vec![note.into()])?;
    /// let xfdf = reviewed.export_annotations(&ExportOptions::new(CommentFormat::Xfdf))?;
    ///
    /// let mut clean = blank()?;
    /// let (summary, _) = clean.import_annotations(&xfdf, &ImportOptions::new(ImportMode::Merge))?;
    /// assert_eq!(summary.added, 1);
    /// clean.undo()?; // the whole import, in one step
    /// # }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `interchange-unknown-format`, `interchange-malformed` (naming the line), `interchange-dtd`
    /// and `interchange-too-large` for a file that can't be read; the ops' errors, such as
    /// `appearance-unavailable` without the `build` feature, after which nothing has changed.
    ///
    /// Added in 0.7.0.
    pub fn import_annotations(&mut self, bytes: &[u8], options: &ImportOptions) -> Result<(ImportSummary, Edited)> {
        let exchange = ix::read(bytes, self.ops().cos().limits())?;
        let (summary, applied) = ix::import(&mut self.inner, &exchange, options)?;
        Ok((summary, self.edited(applied)))
    }

    /// The comments on `pages` as Markdown.
    fn markdown(&self, pages: Option<&[u32]>, options: &ExportOptions) -> Result<String> {
        let wanted = |subtype: &str| options.comments == ExportComments::All || HIGHLIGHTS_AND_NOTES.contains(&subtype);
        let comments: Vec<ix::MarkdownComment> = self
            .comment_threads(pages)?
            .into_iter()
            .filter(|t| wanted(&t.comment.subtype))
            .map(|t| {
                let mut c = ix::MarkdownComment::new(t.page);
                c.quote = t.quoted;
                c.note = t.comment.contents.filter(|n| Some(n) != c.quote.as_ref());
                c.author = t.comment.author;
                c.date = t.comment.created.or(t.comment.modified);
                c.status = t.review.filter(|s| s != "None");
                c.replies = t
                    .replies
                    .into_iter()
                    .filter_map(|r| {
                        let text = r.annotation.contents?;
                        Some(ix::MarkdownReply::new(r.annotation.author, text))
                    })
                    .collect();
                c
            })
            .collect();
        Ok(ix::write_markdown(&comments, &options.markdown))
    }
}

/// Leaves out every comment but text markup and notes, and the replies to what is left out,
/// replies to replies included.
fn keep_highlights_and_notes(exchange: &mut ix::Exchange) {
    let mut kept: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut keep = vec![false; exchange.comments.len()];
    loop {
        let before = kept.len();
        for (c, keep) in exchange.comments.iter().zip(keep.iter_mut()) {
            let wanted = match &c.in_reply_to {
                Some(parent) => kept.contains(parent),
                None => HIGHLIGHTS_AND_NOTES.contains(&c.subtype()),
            };
            if wanted && !*keep {
                *keep = true;
                kept.extend(c.name());
            }
        }
        if kept.len() == before {
            break;
        }
    }
    let mut flags = keep.into_iter();
    exchange.comments.retain(|_| flags.next().unwrap_or(false));
}
