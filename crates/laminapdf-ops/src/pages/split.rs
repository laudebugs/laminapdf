//! Splitting a document into new ones (E2 plan §2.1): by a number of pages, by lists of pages,
//! or where its bookmarks at one level begin. Each part is made as
//! [`Document::extract_pages`] makes a new document, so it carries only what its pages reach,
//! with the bookmarks that go to its pages.

use std::collections::BTreeSet;
use std::num::NonZeroUsize;

use laminapdf_model::{OutlineItem, PageId};

use super::page_ids;
use crate::document::Document;
use crate::error::Result;

/// Where [`Document::split`] cuts a document.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SplitBy {
    /// Parts of this many pages, in page order; the last holds what is left.
    Count(NonZeroUsize),
    /// One part per list of pages (each a list of page IDs, kept in the order given).
    Ranges(Vec<Vec<PageId>>),
    /// A part from each bookmark at this depth (1: the top level) to the next one's page, in
    /// page order; the pages before the first such bookmark make a part of their own. A
    /// document with no bookmark at that depth stays whole.
    Bookmarks {
        /// The depth of the bookmarks that start parts, from 1.
        level: NonZeroUsize,
    },
}

impl Document {
    /// This document cut into new documents as `by` says, each made as
    /// [`Document::extract_pages`] makes one: with its pages' annotations, form fields, links
    /// between them and named destinations, page labels, layers and the bookmarks that go to
    /// its pages (so a part split at a chapter's bookmark has that chapter's bookmarks).
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    /// use laminapdf_ops::{Document, SplitBy};
    /// # use laminapdf_cos::{Dict, Object};
    /// # use laminapdf_save::test_support::Writer;
    /// # let mut w = Writer::new(1, 7);
    /// # let node = w.reserve();
    /// # let page = |w: &mut Writer| w.add([("Type", Object::name("Page")), ("Parent", node.into())].into_iter().collect::<Dict>());
    /// # let kids: Vec<Object> = (0..5).map(|_| page(&mut w).into()).collect();
    /// # w.set(node, [("Type", Object::name("Pages")), ("Kids", kids.into()), ("Count", 5.into()), ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into())].into_iter().collect::<Dict>());
    /// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", node.into())].into_iter().collect::<Dict>());
    /// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
    /// let doc = Document::open(bytes, Default::default())?;
    /// let two = NonZeroUsize::new(2).unwrap();
    /// let parts = doc.split(&SplitBy::Count(two))?;
    /// assert_eq!(parts.len(), 3); // pages 1–2, 3–4 and 5
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// As [`Document::extract_pages`], for the first part that fails (an empty list of pages in
    /// `Ranges` is [`Error::NoPagesLeft`](crate::Error::NoPagesLeft)).
    ///
    /// Added in 0.3.0.
    pub fn split(&self, by: &SplitBy) -> Result<Vec<Self>> {
        self.split_ranges(by)?
            .iter()
            .map(|pages| self.extract_pages(pages))
            .collect()
    }

    /// The pages of each part [`Document::split`] makes, in order.
    ///
    /// # Errors
    /// Errors reading the page tree or the outline.
    ///
    /// Added in 0.3.0.
    pub fn split_ranges(&self, by: &SplitBy) -> Result<Vec<Vec<PageId>>> {
        let pages = page_ids(self)?;
        Ok(match by {
            SplitBy::Count(n) => pages.chunks(n.get()).map(<[PageId]>::to_vec).collect(),
            SplitBy::Ranges(ranges) => ranges.clone(),
            SplitBy::Bookmarks { level } => {
                let outline = self.outline()?;
                let mut starts = BTreeSet::new();
                for item in outline.roots() {
                    starts_at(item, level.get(), &mut starts)?;
                }
                cut(&pages, &starts)
            }
        })
    }
}

/// Adds to `starts` the page index of each bookmark at depth `level` under `item` (`item`'s own
/// when it is at that depth).
fn starts_at(item: OutlineItem<'_, '_>, level: usize, starts: &mut BTreeSet<usize>) -> Result<()> {
    if item.depth() == level {
        if let Some(index) = item.target()?.and_then(|t| t.page_index) {
            starts.insert(index);
        }
        return Ok(());
    }
    for child in item.children() {
        starts_at(child, level, starts)?;
    }
    Ok(())
}

/// `pages` cut before each index in `starts`; the whole list when there are none.
fn cut(pages: &[PageId], starts: &BTreeSet<usize>) -> Vec<Vec<PageId>> {
    let mut bounds: Vec<usize> = starts.iter().copied().filter(|&s| s > 0 && s < pages.len()).collect();
    bounds.push(pages.len());
    let mut parts = Vec::with_capacity(bounds.len());
    let mut from = 0;
    for to in bounds {
        if let Some(part) = pages.get(from..to)
            && !part.is_empty()
        {
            parts.push(part.to_vec());
        }
        from = to;
    }
    parts
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::ObjRef;

    use super::*;

    fn ids(n: u32) -> Vec<PageId> {
        (1..=n).map(|i| PageId::object(ObjRef::new(i, 0))).collect()
    }

    fn sizes(parts: &[Vec<PageId>]) -> Vec<usize> {
        parts.iter().map(Vec::len).collect()
    }

    #[test]
    fn cutting_at_bookmarks_keeps_the_pages_before_the_first_as_a_part() {
        let starts: BTreeSet<usize> = [2, 5].into_iter().collect();
        assert_eq!(sizes(&cut(&ids(8), &starts)), [2, 3, 3]);
    }

    #[test]
    fn a_bookmark_on_the_first_page_starts_the_first_part() {
        let starts: BTreeSet<usize> = [0, 4].into_iter().collect();
        assert_eq!(sizes(&cut(&ids(6), &starts)), [4, 2]);
    }

    #[test]
    fn no_bookmarks_leave_the_document_whole() {
        assert_eq!(sizes(&cut(&ids(3), &BTreeSet::new())), [3]);
    }
}
