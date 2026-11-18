//! Flattening comments into the page (ISO 32000-2 §12.5.5): [`Document::flatten_annotations`]
//! picks the annotations of a scope and applies [`FlattenAnnots`] to them, so each one's
//! appearance becomes part of its page and the annotation goes.

use laminapdf_model::{AnnotId, PageId};
use laminapdf_ops::{FlattenAnnots, FlattenIntent};

use crate::document::Document;
use crate::error::{Error, Result};

/// Which annotations [`Document::flatten_annotations`] flattens.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FlattenScope {
    /// Every comment and mark on every page.
    All,
    /// Every comment and mark on these pages, from 0.
    Pages(Vec<u32>),
    /// These annotations, in this order.
    Annotations(Vec<AnnotId>),
}

impl Document {
    /// The op that flattens the annotations of `scope` for `intent`: for [`FlattenScope::All`]
    /// and [`FlattenScope::Pages`], every annotation on those pages but form fields' widgets,
    /// links, Popups, replies (a reply goes with the comment it answers) and those whose removal
    /// would take a `Locked` annotation, which may not be removed
    /// ([`flatten_candidates`](laminapdf_ops::flatten_candidates)).
    ///
    /// # Errors
    /// `page-index-out-of-range` for a page past the end; reading errors.
    ///
    /// Added in 0.6.0.
    pub fn flatten_op(&self, scope: &FlattenScope, intent: FlattenIntent) -> Result<FlattenAnnots> {
        let pages = self.ops().pages()?;
        let count = u32::try_from(pages.len()).unwrap_or(u32::MAX);
        let wanted: Option<&[u32]> = match scope {
            FlattenScope::Annotations(ids) => return Ok(FlattenAnnots::new(ids.clone(), intent)),
            FlattenScope::All => None,
            FlattenScope::Pages(pages) => Some(pages),
        };
        if let Some(&index) = wanted.into_iter().flatten().find(|&&p| p >= count) {
            return Err(Error::PageIndexOutOfRange { index, count });
        }
        let ids: Option<Vec<PageId>> = wanted.map(|w| {
            pages
                .iter()
                .enumerate()
                .filter(|(i, _)| u32::try_from(*i).is_ok_and(|i| w.contains(&i)))
                .map(|(_, p)| p.id().clone())
                .collect()
        });
        drop(pages);
        let ids = laminapdf_ops::flatten_candidates(self.ops(), ids.as_deref())?;
        Ok(FlattenAnnots::new(ids, intent))
    }

    /// Flattens the annotations of `scope` for `intent` in one edit, one undo step: each one's
    /// appearance is drawn into its page and the annotation is removed with its Popup and its
    /// replies ([`FlattenAnnots`]). Annotations hidden for `intent` stay as they are, and so does
    /// one that has no appearance and is of a kind Lamina PDF can't draw, such as a sound,
    /// which the report lists (`flatten-no-appearance`).
    ///
    /// ```
    /// # #[cfg(feature = "text")] {
    /// use laminapdf::{BuildOptions, Builder, FlattenIntent, FlattenScope, NoteIcon, Op, PageSize, PointPt, Pt};
    ///
    /// let mut b = Builder::new(BuildOptions::new().with_id(*b"flatten-comments"));
    /// b.page(PageSize::A4).finish()?;
    /// let mut doc = b.finish()?;
    /// let note = doc.note_at(0, PointPt::new(Pt::new(56.0), Pt::new(56.0)), "Approved", NoteIcon::Comment)?;
    /// doc.apply(vec![Op::from(note)])?;
    ///
    /// let edited = doc.flatten_annotations(&FlattenScope::All, FlattenIntent::Print)?;
    /// assert!(edited.still_in_earlier_revisions);
    /// assert!(doc.page_annotations(0)?.is_empty());
    /// # }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; `unknown-annotation`, `flatten-widget`, `annotation-locked` (for
    /// annotations named in [`FlattenScope::Annotations`]),
    /// `annotation-not-editable`; as [`Document::apply`].
    ///
    /// Added in 0.6.0.
    pub fn flatten_annotations(&mut self, scope: &FlattenScope, intent: FlattenIntent) -> Result<crate::Edited> {
        let op = self.flatten_op(scope, intent)?;
        self.apply(vec![op.into()])
    }
}
