//! Article threads (ISO 32000-2 §12.4.3): the catalog's `/Threads`, each a chain of beads,
//! the parts of one article spread over pages.
//!
//! A thread's beads form a circular list through `/N`: the walk starts at the thread's `/F`,
//! stops when it comes back to a bead it has seen, and stops at
//! [`Limits::max_tree_depth`](laminapdf_cos::Limits::max_tree_depth) beads per thread with
//! [`Warning::InvalidEntry`] for the `/N` it doesn't follow.

use std::collections::BTreeSet;

use laminapdf_cos::{ObjRef, Object, Result, Warning};

use crate::document::Document;
use crate::geometry::UserSpaceRect;
use crate::id::PageId;
use crate::page::{Pages, read_rect};

/// One part of an article: a rectangle on a page (`/Type /Bead`).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Bead {
    /// The page it is on; `None` when `/P` doesn't name one of the document's pages.
    pub page: Option<PageId>,
    /// That page's index.
    pub page_index: Option<usize>,
    /// The part's rectangle (`/R`), in the page's user space.
    pub rect: Option<UserSpaceRect>,
}

/// An article thread: its title and its beads in reading order (`/Type /Thread`).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Thread {
    /// The thread information dictionary's `/Title` (`/I`), if any.
    pub title: Option<String>,
    /// The beads, from the first (`/F`) along `/N`.
    pub beads: Vec<Bead>,
}

impl Document {
    /// The document's article threads (the catalog's `/Threads`, ISO 32000-2 §12.4.3), in the
    /// catalog's order. A thread or bead that isn't a dictionary is skipped with
    /// [`Warning::InvalidEntry`].
    ///
    /// # Errors
    /// As [`Document::outline`]: strict mode, when a value raises a warning; reading errors
    /// of the catalog and the page tree.
    ///
    /// Added in 0.3.0.
    pub fn threads(&self) -> Result<Vec<Thread>> {
        let catalog = self.catalog()?;
        let Some(value) = catalog.get("Threads") else {
            return Ok(Vec::new());
        };
        let resolved = self.read(value)?;
        let Some(threads) = resolved.as_deref().and_then(Object::as_array) else {
            if !matches!(resolved.as_deref(), None | Some(Object::Null)) {
                self.warn(Warning::InvalidEntry {
                    object: self.trailer().get("Root").and_then(Object::as_ref).map(|r| r.num),
                    key: "Threads",
                })?;
            }
            return Ok(Vec::new());
        };
        let pages = crate::document::tolerant(self.pages())?;
        let mut out = Vec::with_capacity(threads.len());
        for thread in threads {
            if let Some(thread) = self.thread(thread, pages.as_ref())? {
                out.push(thread);
            }
        }
        Ok(out)
    }

    fn thread(&self, value: &Object, pages: Option<&Pages<'_>>) -> Result<Option<Thread>> {
        let resolved = self.read(value)?;
        let Some(thread) = resolved.as_deref().and_then(Object::as_dict) else {
            self.warn(Warning::InvalidEntry {
                object: value.as_ref().map(|r| r.num),
                key: "Threads",
            })?;
            return Ok(None);
        };
        let title = match thread.get("I") {
            Some(info) => match self.read(info)?.as_deref().and_then(Object::as_dict) {
                Some(info) => self.text_entry(info, "Title")?,
                None => None,
            },
            None => None,
        };
        let mut beads = Vec::new();
        let mut seen: BTreeSet<ObjRef> = BTreeSet::new();
        let mut next = thread.get("F").cloned();
        let max = self.limits().max_tree_depth();
        while let Some(value) = next.take() {
            if let Some(r) = value.as_ref()
                && !seen.insert(r)
            {
                break;
            }
            if beads.len() >= max {
                self.warn(Warning::InvalidEntry {
                    object: value.as_ref().map(|r| r.num),
                    key: "N",
                })?;
                break;
            }
            let bead = self.read(&value)?;
            let Some(bead) = bead.as_deref().and_then(Object::as_dict) else {
                if !matches!(bead.as_deref(), None | Some(Object::Null)) {
                    self.warn(Warning::InvalidEntry {
                        object: value.as_ref().map(|r| r.num),
                        key: "N",
                    })?;
                }
                break;
            };
            let page_index = match (bead.get("P").and_then(Object::as_ref), pages) {
                (Some(r), Some(pages)) => pages.index_of_object(r),
                _ => None,
            };
            beads.push(Bead {
                page: page_index
                    .and_then(|i| pages.and_then(|p| p.get(i)))
                    .map(|p| p.id().clone()),
                page_index,
                rect: match bead.get("R") {
                    Some(rect) => read_rect(self, rect)?,
                    None => None,
                },
            });
            // A direct bead can't be reached again, so it can't close the circle: it ends the walk.
            if value.as_ref().is_none() {
                break;
            }
            next = bead.get("N").cloned();
        }
        Ok(Some(Thread { title, beads }))
    }
}
