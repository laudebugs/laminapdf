//! Editing the outline's tree (ISO 32000-2 §12.3.3): items inserted, moved and removed by
//! [`OutlineItemId`], and the doubly linked lists written back whole.
//!
//! [`OutlineTree::read`] takes the tree as the reader walked it ([`Outline`](crate::Outline):
//! `/First` and `/Next` only, cycles cut, broken items skipped). [`OutlineTree::write`] then
//! gives every item the `/Parent`, `/Prev`, `/Next`, `/First`, `/Last` and `/Count` that tree
//! needs, and writes only the items whose entries differ, so an edit to one branch rewrites
//! that branch's neighbours and ancestors and nothing else. A tree whose stored links already
//! disagreed with the walk (a cycle, a wrong `/Last`, a `/Prev` pointing elsewhere, an item
//! stored as a direct dictionary) is rewritten whole by its first write, which
//! [`OutlineTree::repaired`] reports.

use laminapdf_cos::{CosMut, Dict, ObjRef, Object, Result};

use crate::document::Document;
use crate::edit::{put, set_catalog_entry};
use crate::id::OutlineItemId;

/// Where an item goes among its siblings. Positions name items, never indices, since indices
/// shift while an edit runs.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SiblingPosition {
    /// Before the parent's first child.
    First,
    /// After the parent's last child.
    Last,
    /// Just before this sibling.
    Before(OutlineItemId),
    /// Just after this sibling.
    After(OutlineItemId),
}

/// Why an outline edit failed.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OutlineTreeError {
    /// No item of the outline has this ID.
    #[error("the outline has no item with this ID")]
    UnknownItem(OutlineItemId),
    /// The position names an item that isn't a child of the parent the edit names.
    #[error("the position names an item that is not a child of the given parent")]
    NotASibling(OutlineItemId),
    /// A move would put an item inside itself or one of its descendants.
    #[error("an outline item can't be moved inside itself")]
    IntoItself(OutlineItemId),
}

impl OutlineTreeError {
    /// The item the error names.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn item(&self) -> &OutlineItemId {
        match self {
            Self::UnknownItem(id) | Self::NotASibling(id) | Self::IntoItself(id) => id,
        }
    }
}

/// Where an item is stored: an indirect object, or a direct dictionary that the first write
/// turns into one (a direct item can't be linked to: `/Prev` and `/Parent` need references).
#[derive(Debug, Clone)]
enum Slot {
    Indirect(ObjRef),
    Direct(Dict),
}

#[derive(Debug, Clone)]
struct Node {
    id: OutlineItemId,
    slot: Slot,
    parent: Option<usize>,
    children: Vec<usize>,
    open: bool,
    /// Removed from the tree; kept so indices stay valid.
    gone: bool,
}

/// The outline's tree, edited by item ID and written back by [`OutlineTree::write`].
///
/// ```
/// use laminapdf_cos::{Dict, Object, PdfString};
/// use laminapdf_model::{Document, OpenOptions, OutlineTree, SiblingPosition};
/// # use laminapdf_save::test_support::Writer;
/// # let mut w = Writer::new(1, 7);
/// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::<Object>::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
/// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
/// let mut doc = Document::open(bytes, OpenOptions::default())?;
/// let mut tree = OutlineTree::read(&doc)?;
/// let title: Dict = [("Title", Object::String(PdfString::literal("Chapter 1")))].into_iter().collect();
/// let item = doc.cos_mut().edit().create(Object::Dict(title))?;
/// tree.insert(item, None, &SiblingPosition::Last)?;
/// tree.write(&mut doc.cos_mut().edit())?;
/// assert_eq!(doc.outline()?.len(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
pub struct OutlineTree {
    /// The `/Outlines` dictionary, when the catalog points at one.
    root: Option<Slot>,
    nodes: Vec<Node>,
    roots: Vec<usize>,
    repaired: bool,
}

impl OutlineTree {
    /// The outline of `doc` as the reader walks it.
    ///
    /// # Errors
    /// Reading errors of the outline ([`Document::outline`]).
    ///
    /// Added in 0.2.0.
    pub fn read(doc: &Document) -> Result<Self> {
        let catalog = doc.catalog()?;
        let root = match catalog.get("Outlines") {
            Some(Object::Ref(r)) => doc
                .read(&Object::Ref(*r))?
                .filter(|o| o.as_dict().is_some())
                .map(|_| Slot::Indirect(*r)),
            Some(Object::Dict(d)) => Some(Slot::Direct(d.clone())),
            _ => None,
        };
        let outline = doc.outline()?;
        let mut tree = Self {
            root,
            nodes: Vec::with_capacity(outline.len()),
            roots: Vec::new(),
            repaired: false,
        };
        let mut stack: Vec<(Option<usize>, crate::OutlineItem<'_, '_>)> = outline.roots().map(|i| (None, i)).collect();
        stack.reverse();
        while let Some((parent, item)) = stack.pop() {
            let Some(id) = item.id().cloned() else { continue };
            let slot = match id.as_lamina_id().obj_ref() {
                Some(r) => Slot::Indirect(r),
                None => Slot::Direct(item.dict().cloned().unwrap_or_default()),
            };
            let index = tree.nodes.len();
            tree.nodes.push(Node {
                id,
                slot,
                parent,
                children: Vec::new(),
                open: item.is_open()?,
                gone: false,
            });
            match parent.and_then(|p| tree.nodes.get_mut(p)) {
                Some(p) => p.children.push(index),
                None => tree.roots.push(index),
            }
            let mut children: Vec<_> = item.children().map(|c| (Some(index), c)).collect();
            children.reverse();
            stack.extend(children);
        }
        tree.repaired = tree.nodes.iter().any(|n| matches!(n.slot, Slot::Direct(_)))
            || matches!(tree.root, Some(Slot::Direct(_)))
            || tree.links_differ(doc)?;
        Ok(tree)
    }

    /// Whether the stored tree's links disagreed with the walk, so the next
    /// [`OutlineTree::write`] rewrites them whole. A wrong `/Count` alone doesn't count: the
    /// write corrects it without a report, since many writers get it wrong harmlessly.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn repaired(&self) -> bool {
        self.repaired
    }

    /// Whether the tree has this item.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn contains(&self, id: &OutlineItemId) -> bool {
        self.find(id).is_some()
    }

    /// The item's parent (`None` at the top level), or `None` for an unknown item.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn parent(&self, id: &OutlineItemId) -> Option<Option<&OutlineItemId>> {
        let node = self.nodes.get(self.find(id)?)?;
        Some(node.parent.and_then(|p| self.nodes.get(p)).map(|p| &p.id))
    }

    /// The item and every item under it, the item first.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn subtree(&self, id: &OutlineItemId) -> Vec<OutlineItemId> {
        let Some(index) = self.find(id) else { return Vec::new() };
        self.descendants(index)
            .into_iter()
            .filter_map(|i| self.nodes.get(i).map(|n| n.id.clone()))
            .collect()
    }

    /// Every item, depth-first.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn items(&self) -> Vec<OutlineItemId> {
        let mut out = Vec::new();
        for &root in &self.roots {
            for i in self.descendants(root) {
                if let Some(n) = self.nodes.get(i) {
                    out.push(n.id.clone());
                }
            }
        }
        out
    }

    /// Adds the item dictionary at `item` (already written, without links) under `parent`
    /// (`None`: the top level), at `at`. It starts closed.
    ///
    /// # Errors
    /// [`OutlineTreeError::UnknownItem`] for a parent the tree hasn't; `NotASibling` for a
    /// position naming an item outside the parent's children.
    ///
    /// Added in 0.2.0.
    pub fn insert(
        &mut self,
        item: ObjRef,
        parent: Option<&OutlineItemId>,
        at: &SiblingPosition,
    ) -> core::result::Result<OutlineItemId, OutlineTreeError> {
        let parent = parent.map(|p| self.require(p)).transpose()?;
        let slot = self.slot_index(parent, at)?;
        let id = OutlineItemId::object(item);
        let index = self.nodes.len();
        self.nodes.push(Node {
            id: id.clone(),
            slot: Slot::Indirect(item),
            parent,
            children: Vec::new(),
            open: false,
            gone: false,
        });
        self.siblings_mut(parent).insert(slot, index);
        Ok(id)
    }

    /// Moves `item`, with its children, under `parent` at `at`.
    ///
    /// # Errors
    /// `UnknownItem`, `NotASibling`; `IntoItself` when `parent` is the item or under it, or
    /// the position names the item itself.
    ///
    /// Added in 0.2.0.
    pub fn move_item(
        &mut self,
        item: &OutlineItemId,
        parent: Option<&OutlineItemId>,
        at: &SiblingPosition,
    ) -> core::result::Result<(), OutlineTreeError> {
        let index = self.require(item)?;
        let parent = parent.map(|p| self.require(p)).transpose()?;
        if parent.is_some_and(|p| self.descendants(index).contains(&p))
            || matches!(at, SiblingPosition::Before(s) | SiblingPosition::After(s) if s == item)
        {
            return Err(OutlineTreeError::IntoItself(item.clone()));
        }
        // Check the position before taking the item out, so a refused move changes nothing.
        self.slot_index(parent, at)?;
        let old = self.nodes.get(index).and_then(|n| n.parent);
        self.siblings_mut(old).retain(|&i| i != index);
        let slot = self.slot_index(parent, at)?;
        self.siblings_mut(parent).insert(slot, index);
        if let Some(node) = self.nodes.get_mut(index) {
            node.parent = parent;
        }
        Ok(())
    }

    /// Removes `item`. With `promote`, its children take its place, in order; without, they
    /// go with it. Returns the removed items, `item` first.
    ///
    /// # Errors
    /// `UnknownItem`.
    ///
    /// Added in 0.2.0.
    pub fn remove(
        &mut self,
        item: &OutlineItemId,
        promote: bool,
    ) -> core::result::Result<Vec<OutlineItemId>, OutlineTreeError> {
        let index = self.require(item)?;
        let (parent, children) = match self.nodes.get(index) {
            Some(n) => (n.parent, n.children.clone()),
            None => return Err(OutlineTreeError::UnknownItem(item.clone())),
        };
        let gone = if promote { vec![index] } else { self.descendants(index) };
        let siblings = self.siblings_mut(parent);
        let at = siblings.iter().position(|&i| i == index).unwrap_or(siblings.len());
        siblings.retain(|&i| i != index);
        if promote {
            for (k, &child) in children.iter().enumerate() {
                siblings.insert(at + k, child);
            }
            for &child in &children {
                if let Some(c) = self.nodes.get_mut(child) {
                    c.parent = parent;
                }
            }
            if let Some(n) = self.nodes.get_mut(index) {
                n.children.clear();
            }
        }
        let mut removed = Vec::with_capacity(gone.len());
        for i in gone {
            if let Some(n) = self.nodes.get_mut(i) {
                n.gone = true;
                removed.push(n.id.clone());
            }
        }
        Ok(removed)
    }

    /// Shows `item`'s children (`open`) or hides them. An item without children has no
    /// `/Count` to hold the state (§12.3.3), so for one the call writes nothing: the state holds
    /// only once the item has children in the same tree.
    ///
    /// # Errors
    /// `UnknownItem`.
    ///
    /// Added in 0.2.0.
    pub fn set_open(&mut self, item: &OutlineItemId, open: bool) -> core::result::Result<(), OutlineTreeError> {
        let index = self.require(item)?;
        if let Some(n) = self.nodes.get_mut(index) {
            n.open = open;
        }
        Ok(())
    }

    /// Writes the links and counts of every item, and of `/Outlines`, that differ from the
    /// tree's; creates `/Outlines` for a first item. Items stored as direct dictionaries become
    /// indirect objects (and so get new IDs).
    ///
    /// # Errors
    /// Errors of the overlay.
    ///
    /// Added in 0.2.0.
    pub fn write(&mut self, cos: &mut CosMut<'_>) -> Result<()> {
        for node in &mut self.nodes {
            if let Slot::Direct(dict) = &node.slot {
                let r = cos.create(Object::Dict(dict.clone()))?;
                node.slot = Slot::Indirect(r);
                node.id = OutlineItemId::object(r);
            }
        }
        let root = match self.root.take() {
            Some(Slot::Indirect(r)) => r,
            Some(Slot::Direct(dict)) => {
                let r = cos.create(Object::Dict(dict))?;
                set_catalog_entry(cos, "Outlines", Some(Object::Ref(r)))?;
                r
            }
            None if self.roots.is_empty() => return Ok(()),
            None => {
                let dict: Dict = [("Type", Object::name("Outlines"))].into_iter().collect();
                let r = cos.create(Object::Dict(dict))?;
                set_catalog_entry(cos, "Outlines", Some(Object::Ref(r)))?;
                r
            }
        };
        self.root = Some(Slot::Indirect(root));
        for (r, links) in self.wanted(root) {
            let Some(current) = cos.get(r)? else { continue };
            let Some(dict) = current.as_dict() else { continue };
            let mut next = dict.clone();
            links.apply(&mut next);
            if next != *dict {
                cos.set(r, Object::Dict(next))?;
            }
        }
        self.repaired = false;
        Ok(())
    }

    /// Whether any stored link (not counts, which writers often get wrong harmlessly)
    /// differs from what [`OutlineTree::write`] would write.
    fn links_differ(&self, doc: &Document) -> Result<bool> {
        let Some(Slot::Indirect(root)) = self.root else {
            return Ok(!self.roots.is_empty());
        };
        for (r, links) in self.wanted(root) {
            let Some(current) = doc.read(&Object::Ref(r))? else {
                return Ok(true);
            };
            let Some(dict) = current.as_dict() else { return Ok(true) };
            let mut want = dict.clone();
            links.apply_links(&mut want);
            if want != *dict {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Each live object's wanted entries: `/Outlines` first, then the items.
    fn wanted(&self, root: ObjRef) -> Vec<(ObjRef, Links)> {
        let r = |i: usize| match self.nodes.get(i).map(|n| &n.slot) {
            Some(Slot::Indirect(r)) => Some(*r),
            _ => None,
        };
        let ends = |children: &[usize]| {
            (
                children.first().and_then(|&c| r(c)),
                children.last().and_then(|&c| r(c)),
            )
        };
        let (first, last) = ends(&self.roots);
        let visible: i64 = self.roots.iter().map(|&c| 1 + self.visible(c)).sum();
        let mut out = vec![(
            root,
            Links {
                parent: None,
                prev: None,
                next: None,
                first,
                last,
                count: (visible > 0).then_some(visible),
                is_root: true,
                has_children: !self.roots.is_empty(),
            },
        )];
        let mut walk: Vec<(Option<usize>, &[usize])> = vec![(None, &self.roots)];
        while let Some((parent, children)) = walk.pop() {
            for (k, &c) in children.iter().enumerate() {
                let (Some(own), Some(node)) = (r(c), self.nodes.get(c)) else {
                    continue;
                };
                let (first, last) = ends(&node.children);
                let shown: i64 = node.children.iter().map(|&g| 1 + self.visible(g)).sum();
                out.push((
                    own,
                    Links {
                        parent: Some(parent.and_then(r).unwrap_or(root)),
                        prev: k.checked_sub(1).and_then(|p| children.get(p)).and_then(|&p| r(p)),
                        next: children.get(k + 1).and_then(|&n| r(n)),
                        first,
                        last,
                        count: (!node.children.is_empty()).then_some(if node.open { shown } else { -shown }),
                        is_root: false,
                        has_children: !node.children.is_empty(),
                    },
                ));
                walk.push((Some(c), &node.children));
            }
        }
        out
    }

    /// The items shown under `index` (its descendants, as far as open items reach).
    fn visible(&self, index: usize) -> i64 {
        let mut total = 0;
        let mut stack = vec![index];
        while let Some(i) = stack.pop() {
            let Some(n) = self.nodes.get(i) else { continue };
            if n.open {
                total += i64::try_from(n.children.len()).unwrap_or(i64::MAX);
                stack.extend(&n.children);
            }
        }
        total
    }

    fn descendants(&self, index: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![index];
        while let Some(i) = stack.pop() {
            out.push(i);
            if let Some(n) = self.nodes.get(i) {
                stack.extend(n.children.iter().rev());
            }
        }
        out
    }

    fn find(&self, id: &OutlineItemId) -> Option<usize> {
        self.nodes.iter().position(|n| !n.gone && n.id == *id)
    }

    fn require(&self, id: &OutlineItemId) -> core::result::Result<usize, OutlineTreeError> {
        self.find(id).ok_or_else(|| OutlineTreeError::UnknownItem(id.clone()))
    }

    fn siblings_mut(&mut self, parent: Option<usize>) -> &mut Vec<usize> {
        match parent {
            Some(p) if p < self.nodes.len() => match self.nodes.get_mut(p) {
                Some(n) => &mut n.children,
                None => &mut self.roots,
            },
            _ => &mut self.roots,
        }
    }

    /// The index in the parent's children where `at` puts an item.
    fn slot_index(
        &mut self,
        parent: Option<usize>,
        at: &SiblingPosition,
    ) -> core::result::Result<usize, OutlineTreeError> {
        let sibling = match at {
            SiblingPosition::Before(s) | SiblingPosition::After(s) => Some((s.clone(), self.require(s)?)),
            _ => None,
        };
        let siblings = self.siblings_mut(parent);
        Ok(match (at, sibling) {
            (SiblingPosition::First, _) => 0,
            (SiblingPosition::Before(_) | SiblingPosition::After(_), Some((id, s))) => {
                let k = siblings
                    .iter()
                    .position(|&i| i == s)
                    .ok_or(OutlineTreeError::NotASibling(id))?;
                if matches!(at, SiblingPosition::After(_)) {
                    k + 1
                } else {
                    k
                }
            }
            _ => siblings.len(),
        })
    }
}

/// The entries one item (or `/Outlines`) should have.
#[derive(Debug, Clone, Copy)]
struct Links {
    parent: Option<ObjRef>,
    prev: Option<ObjRef>,
    next: Option<ObjRef>,
    first: Option<ObjRef>,
    last: Option<ObjRef>,
    count: Option<i64>,
    is_root: bool,
    has_children: bool,
}

impl Links {
    fn apply_links(self, dict: &mut Dict) {
        if !self.is_root {
            put(dict, "Parent", self.parent.map(Object::Ref));
            put(dict, "Prev", self.prev.map(Object::Ref));
            put(dict, "Next", self.next.map(Object::Ref));
        }
        put(dict, "First", self.first.map(Object::Ref));
        put(dict, "Last", self.last.map(Object::Ref));
    }

    /// Links and `/Count`. A leaf's stored `/Count` of 0 is left alone: it means the same as
    /// none, and many writers put it there.
    fn apply(self, dict: &mut Dict) {
        self.apply_links(dict);
        match self.count {
            Some(count) => dict.insert("Count", Object::Integer(count)),
            None if !self.has_children && dict.get("Count").and_then(Object::as_int) == Some(0) => {}
            None => {
                dict.remove("Count");
            }
        }
    }
}
