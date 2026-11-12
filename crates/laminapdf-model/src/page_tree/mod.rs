//! Editing the page tree (ISO 32000-2 §7.7.3): inserting, removing and moving pages through
//! the overlay, with every `/Count` (§7.7.3.2), `/Parent` and inherited attribute
//! (§7.7.3.4) kept right.

mod load;
mod write;

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object};

use crate::document::Document;
use crate::id::PageId;
use load::{Kid, Slot, Tree, overlay};

/// Where pages go in the page list. Positions name pages, never indices, since indices shift
/// while an edit runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PagePosition {
    /// Before the first page.
    Start,
    /// After the last page.
    End,
    /// Just before this page.
    Before(PageId),
    /// Just after this page.
    After(PageId),
}

/// Why a page-tree edit failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PageTreeError {
    /// Reading or writing the document failed (as [`laminapdf_cos::Error`]). The page tree
    /// root must be an indirect `/Pages` dictionary.
    #[error(transparent)]
    Cos(#[from] laminapdf_cos::Error),
    /// No page with this ID is in the page tree.
    #[error("no page {0} in the page tree")]
    UnknownPage(PageId),
    /// The page to insert is already in the page tree.
    #[error("page {0} is already in the page tree")]
    AlreadyInTree(PageId),
    /// The object to insert is not a page dictionary.
    #[error("object {0} is not a page dictionary")]
    NotAPage(ObjRef),
}

/// An editor of the page tree, over the document's overlay ([`Document::edit_page_tree`]).
///
/// It reads the tree once and edits a copy in memory; [`PageTreeEditor::finish`] writes the
/// nodes and pages that changed. Dropping it without `finish` changes nothing.
///
/// * **Shape.** Small edits keep the tree's shape: a page goes into the node that holds its
///   neighbour, and nodes left empty are removed. The tree is rebuilt balanced
///   ([`PageTreeEditor::FAN_OUT`] kids per node) only when, after the edits, it is deeper than
///   [`PageTreeEditor::MAX_DEPTH`] or a node the edits touched holds more than
///   [`PageTreeEditor::MAX_KIDS`] kids, and only when every kid is an indirect page or node
///   (a direct kid's inheritance couldn't be kept).
/// * **Inherited attributes.** A page that moves, and every page when the tree is rebuilt,
///   keeps what it showed: an inherited `/Resources`, `/MediaBox`, `/CropBox` or `/Rotate`
///   that its new ancestors don't give is written on the page itself, and one they give but it
///   didn't have is overridden with the neutral value (`/Rotate 0`, empty `/Resources`, a
///   `/CropBox` equal to its media box). An inserted page shows what its own dictionary says
///   in the same way; a `/MediaBox` it lacks is inherited.
/// * **Counts and parents.** Every touched node's `/Count` and `/Kids`, and the `/Parent` of
///   every page and node that moved, are written.
/// * **Pages listed twice.** A page dictionary listed in two places can't have two parents:
///   `finish` copies it to a new object for every listing after the first, raising
///   [`Warning::InvalidEntry`](laminapdf_cos::Warning::InvalidEntry) (key `Kids`) on the
///   node that lists the copy. An ID names the first listing in page order.
///
/// ```
/// use laminapdf_model::{Document, OpenOptions, PagePosition};
/// # use laminapdf_cos::{Dict, Object};
/// # use laminapdf_save::test_support::Writer;
/// # let mut w = Writer::new(1, 7);
/// # let pages = w.reserve();
/// # let mut kids = Vec::new();
/// # for _ in 0..3 {
/// #     kids.push(Object::Ref(w.add([("Type", Object::name("Page")), ("Parent", pages.into())].into_iter().collect::<Dict>())));
/// # }
/// # let media_box = Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]);
/// # w.set(pages, [("Type", Object::name("Pages")), ("Kids", kids.into()), ("Count", 3.into()), ("MediaBox", media_box)].into_iter().collect::<Dict>());
/// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
/// let mut doc = Document::open(bytes, OpenOptions::default())?;
/// let ids: Vec<_> = doc.pages()?.iter().map(|p| p.id().clone()).collect();
/// let mut editor = doc.edit_page_tree()?;
/// editor.move_pages(&ids[2..], &PagePosition::Start)?;
/// editor.finish()?;
/// let order: Vec<_> = doc.pages()?.iter().map(|p| p.id().to_string()).collect();
/// assert_eq!(order, ["p4.0", "p2.0", "p3.0"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
#[must_use = "the changes are written by `finish`"]
pub struct PageTreeEditor<'d> {
    doc: &'d mut Document,
    tree: Tree,
}

impl Document {
    /// An editor of the page tree ([`PageTreeEditor`]), reading the tree as it is now.
    ///
    /// # Errors
    /// [`PageTreeError::Cos`] when the catalog's `/Pages` is not an indirect dictionary, when
    /// a node is reached twice ([`laminapdf_cos::Error::ReferenceCycle`]) or reading fails.
    pub fn edit_page_tree(&mut self) -> Result<PageTreeEditor<'_>, PageTreeError> {
        let tree = Tree::load(self)?;
        Ok(PageTreeEditor { doc: self, tree })
    }
}

impl PageTreeEditor<'_> {
    /// Past this depth the tree is rebuilt. Writers' trees are a few levels deep (a balanced
    /// tree of [`PageTreeEditor::FAN_OUT`] kids holds a million pages in four); a deeper one
    /// is a chain left by appends, which costs a reader a step per level for every page.
    pub const MAX_DEPTH: usize = 32;
    /// Past this many kids, a node the edits touched gets the tree rebuilt. Writers that put
    /// every page under the root give files of a thousand pages one flat node, which readers
    /// take well, so only nodes larger than that count as too wide.
    pub const MAX_KIDS: usize = 1024;
    /// The kids per node of a rebuilt tree: shallow, and small enough that rewriting one
    /// node's `/Kids` stays cheap.
    pub const FAN_OUT: usize = 32;

    /// The pages in order, as their IDs (a page listed twice appears twice).
    #[must_use]
    pub fn page_ids(&self) -> Vec<PageId> {
        self.tree
            .order()
            .into_iter()
            .filter_map(|s| self.tree.slots.get(s))
            .map(|s| PageId::object(s.r))
            .collect()
    }

    /// The document being edited, for reading.
    #[must_use]
    pub fn document(&self) -> &Document {
        self.doc
    }

    /// Inserts the page object `page` (a page dictionary already in the document, not in the
    /// page tree) at `at`, and returns its ID. Its `/Parent` is written by `finish`.
    ///
    /// # Errors
    /// [`PageTreeError::NotAPage`] when `page` is not a page dictionary,
    /// [`PageTreeError::AlreadyInTree`], [`PageTreeError::UnknownPage`] for an `at` that names
    /// no page, and reading errors.
    pub fn insert(&mut self, page: ObjRef, at: &PagePosition) -> Result<PageId, PageTreeError> {
        let id = PageId::object(page);
        if self.find(&id).is_some() {
            return Err(PageTreeError::AlreadyInTree(id));
        }
        let dict = self
            .doc
            .read(&Object::Ref(page))?
            .and_then(|o| o.as_dict().cloned())
            .filter(|d| !load::is_tree_node(d))
            .ok_or(PageTreeError::NotAPage(page))?;
        let (node, index) = self.place(at, &BTreeSet::new())?;
        // A page taken out by this editor comes back showing what it showed before.
        let removed = self.tree.slots.iter().position(|s| s.r == page && s.parent.is_none());
        let slot = if let Some(slot) = removed {
            slot
        } else {
            let desired = overlay(self.doc, &load::Attrs::default(), &dict)?;
            self.tree.slots.push(Slot {
                r: page,
                dict,
                desired,
                parent: None,
                moved: true,
            });
            self.tree.slots.len() - 1
        };
        if let Some(s) = self.tree.slots.get_mut(slot) {
            s.parent = Some(node);
            s.moved = true;
        }
        self.attach(node, index, &[slot]);
        Ok(id)
    }

    /// Creates a page object from `dict` (given `/Type /Page`) and inserts it at `at`, as
    /// [`PageTreeEditor::insert`] does.
    ///
    /// # Errors
    /// As [`PageTreeEditor::insert`].
    pub fn insert_new(&mut self, mut dict: Dict, at: &PagePosition) -> Result<PageId, PageTreeError> {
        // Placed first, so a bad position creates nothing.
        self.place(at, &BTreeSet::new())?;
        dict.insert("Type", Object::name("Page"));
        let page = self.doc.cos_mut().edit().create(Object::Dict(dict))?;
        self.insert(page, at)
    }

    /// Takes `page` out of the page tree and returns its object, which stays in the document
    /// (delete it, or keep it for undo). Nodes left empty are removed by `finish`.
    ///
    /// # Errors
    /// [`PageTreeError::UnknownPage`].
    pub fn remove(&mut self, page: &PageId) -> Result<ObjRef, PageTreeError> {
        let slot = self
            .find(page)
            .ok_or_else(|| PageTreeError::UnknownPage(page.clone()))?;
        self.detach(slot);
        self.tree
            .slots
            .get(slot)
            .map(|s| s.r)
            .ok_or_else(|| PageTreeError::UnknownPage(page.clone()))
    }

    /// Moves `pages` to `to`, keeping their order among themselves (the order given) and their
    /// IDs. An anchor that is itself moved is replaced by the nearest page that stays: the next
    /// one for [`PagePosition::Before`], the previous one for [`PagePosition::After`].
    ///
    /// # Errors
    /// [`PageTreeError::UnknownPage`] for a page or anchor not in the tree.
    pub fn move_pages(&mut self, pages: &[PageId], to: &PagePosition) -> Result<(), PageTreeError> {
        let mut slots = Vec::with_capacity(pages.len());
        for id in pages {
            let slot = self.find(id).ok_or_else(|| PageTreeError::UnknownPage(id.clone()))?;
            if !slots.contains(&slot) {
                slots.push(slot);
            }
        }
        let moving: BTreeSet<usize> = slots.iter().copied().collect();
        let (node, index) = self.place(to, &moving)?;
        // The anchor stays, so its node and index hold once the moved pages are out of the way;
        // only pages before it in the same node shift the index.
        let shift = self.tree.nodes.get(node).map_or(0, |n| {
            n.kids
                .iter()
                .take(index)
                .filter(|k| matches!(k, Kid::Page(s) if moving.contains(s)))
                .count()
        });
        for &slot in &slots {
            self.detach(slot);
        }
        for &slot in &slots {
            if let Some(s) = self.tree.slots.get_mut(slot) {
                s.parent = Some(node);
                s.moved = true;
            }
        }
        self.attach(node, index.saturating_sub(shift), &slots);
        Ok(())
    }

    /// Writes the changes: the touched nodes, the pages that moved or were inserted, copies
    /// of pages listed twice, and the tree rebuilt when it passes a threshold.
    ///
    /// # Errors
    /// Errors of the overlay; in strict mode, the warning for a page listed twice.
    pub fn finish(self) -> Result<(), PageTreeError> {
        Ok(write::finish(self.doc, self.tree)?)
    }

    /// The slot of `id`: its first listing in page order.
    fn find(&self, id: &PageId) -> Option<usize> {
        let r = id.as_lamina_id().obj_ref()?;
        self.tree
            .order()
            .into_iter()
            .find(|&s| self.tree.slots.get(s).is_some_and(|slot| slot.r == r))
    }

    /// The node and kid index where `at` puts pages, skipping anchors in `moving`.
    fn place(&self, at: &PagePosition, moving: &BTreeSet<usize>) -> Result<(usize, usize), PageTreeError> {
        let order: Vec<usize> = self.tree.order().into_iter().filter(|s| !moving.contains(s)).collect();
        let anchor = |id: &PageId| -> Result<usize, PageTreeError> {
            self.find(id).ok_or_else(|| PageTreeError::UnknownPage(id.clone()))
        };
        let all = self.tree.order();
        // An anchor being moved gives way to the nearest page that stays.
        let (slot, after) = match at {
            PagePosition::Start => (order.first().copied(), false),
            PagePosition::End => (order.last().copied(), true),
            PagePosition::Before(id) => {
                let a = anchor(id)?;
                let at = all.iter().position(|&s| s == a).unwrap_or(0);
                match all.iter().skip(at).find(|s| !moving.contains(s)) {
                    Some(&s) => (Some(s), false),
                    None => (order.last().copied(), true),
                }
            }
            PagePosition::After(id) => {
                let a = anchor(id)?;
                let at = all.iter().position(|&s| s == a).unwrap_or(0);
                match all.iter().take(at + 1).rev().find(|s| !moving.contains(s)) {
                    Some(&s) => (Some(s), true),
                    None => (order.first().copied(), false),
                }
            }
        };
        let Some(slot) = slot else {
            // No page stays: the root's kids, at the start or the end.
            let len = self.tree.nodes.first().map_or(0, |n| n.kids.len());
            return Ok((0, if matches!(at, PagePosition::Start) { 0 } else { len }));
        };
        let node = self.tree.slots.get(slot).and_then(|s| s.parent).unwrap_or(0);
        let index = self
            .tree
            .nodes
            .get(node)
            .and_then(|n| n.kids.iter().position(|k| *k == Kid::Page(slot)))
            .unwrap_or(0);
        Ok((node, index + usize::from(after)))
    }

    /// Takes slot `slot` out of its node's `/Kids`.
    fn detach(&mut self, slot: usize) {
        let Some(parent) = self.tree.slots.get_mut(slot).and_then(|s| s.parent.take()) else {
            return;
        };
        if let Some(node) = self.tree.nodes.get_mut(parent) {
            node.kids.retain(|k| *k != Kid::Page(slot));
            node.touched = true;
        }
    }

    /// Puts `slots` into `node`'s `/Kids` at `index`, in order.
    fn attach(&mut self, node: usize, index: usize, slots: &[usize]) {
        if let Some(n) = self.tree.nodes.get_mut(node) {
            let index = index.min(n.kids.len());
            n.kids.splice(index..index, slots.iter().map(|&s| Kid::Page(s)));
            n.touched = true;
        }
    }
}
