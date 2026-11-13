//! The document outline (bookmarks, ISO 32000-2 §12.3.3) and destinations (§12.3.2).
//!
//! The walk is iterative with a visited set: an item reached twice raises
//! [`Warning::OutlineCycle`] and ends that branch; an item that is missing, not a dictionary
//! or nested deeper than [`Limits::max_tree_depth`](laminapdf_cos::Limits::max_tree_depth) is
//! skipped with its subtree ([`Warning::OutlineItemSkipped`]).

use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use laminapdf_cos::{DirectPath, ObjRef, Object, PathSegment, Result, Warning};
use laminapdf_syntax::text::decode_text_string;

use crate::document::{Document, object_number};
use crate::geometry::UserSpaceRect;
use crate::id::OutlineItemId;
use crate::page::{Pages, number, read_rect};

/// How a destination shows its page (§12.3.2.2, Table 149). Coordinates that the file
/// leaves `null` ("unchanged") are `None`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum View {
    /// `/XYZ left top zoom`.
    Xyz {
        /// Left edge.
        left: Option<f64>,
        /// Top edge.
        top: Option<f64>,
        /// Zoom factor (`None` or 0: unchanged).
        zoom: Option<f64>,
    },
    /// `/Fit`: the whole page.
    Fit,
    /// `/FitH top`: the page's width.
    FitH {
        /// Top edge.
        top: Option<f64>,
    },
    /// `/FitV left`: the page's height.
    FitV {
        /// Left edge.
        left: Option<f64>,
    },
    /// `/FitR left bottom right top`: a rectangle.
    FitR(UserSpaceRect),
    /// `/FitB`: the page's bounding box.
    FitB,
    /// `/FitBH top`.
    FitBH {
        /// Top edge.
        top: Option<f64>,
    },
    /// `/FitBV left`.
    FitBV {
        /// Left edge.
        left: Option<f64>,
    },
    /// A missing or unknown view: show the page as is.
    Unknown,
}

/// Where a destination points (§12.3.2).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Destination {
    /// The page, when the destination names one of this document's pages.
    pub page: Option<crate::id::PageId>,
    /// The page's index, when resolved.
    pub page_index: Option<usize>,
    /// How to show it.
    pub view: View,
    /// The destination's name, for a named destination (§12.3.2.4).
    pub name: Option<Vec<u8>>,
    /// The explicit destination array as stored (references unresolved), so unresolved
    /// targets (another file's page number, a broken reference) keep their data.
    pub raw: Object,
}

/// The type of an outline item's action (`/A /S`, §12.6.4, Table 201).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ActionKind {
    /// `GoTo`: a destination in this document (its target is [`OutlineItem::target`]).
    GoTo,
    /// `GoToR`: a destination in another file.
    GoToR,
    /// `GoToE`: a destination in an embedded file.
    GoToE,
    /// `Launch`.
    Launch,
    /// `Thread`.
    Thread,
    /// `URI`.
    Uri,
    /// `Named`.
    Named,
    /// `JavaScript`.
    JavaScript,
    /// `SubmitForm`.
    SubmitForm,
    /// `ResetForm`.
    ResetForm,
    /// `ImportData`.
    ImportData,
    /// Any other `/S`, as stored.
    Other(Vec<u8>),
}

impl ActionKind {
    /// The `/S` name the file uses for this type (`Other`'s as stored, lossily decoded).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn as_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(match self {
            Self::GoTo => "GoTo",
            Self::GoToR => "GoToR",
            Self::GoToE => "GoToE",
            Self::Launch => "Launch",
            Self::Thread => "Thread",
            Self::Uri => "URI",
            Self::Named => "Named",
            Self::JavaScript => "JavaScript",
            Self::SubmitForm => "SubmitForm",
            Self::ResetForm => "ResetForm",
            Self::ImportData => "ImportData",
            Self::Other(name) => return String::from_utf8_lossy(name),
        })
    }

    pub(crate) fn from_name(name: &[u8]) -> Self {
        match name {
            b"GoTo" => Self::GoTo,
            b"GoToR" => Self::GoToR,
            b"GoToE" => Self::GoToE,
            b"Launch" => Self::Launch,
            b"Thread" => Self::Thread,
            b"URI" => Self::Uri,
            b"Named" => Self::Named,
            b"JavaScript" => Self::JavaScript,
            b"SubmitForm" => Self::SubmitForm,
            b"ResetForm" => Self::ResetForm,
            b"ImportData" => Self::ImportData,
            other => Self::Other(other.to_vec()),
        }
    }
}

struct Item {
    id: OutlineItemId,
    /// The item dictionary, shared with the object cache for indirect items.
    node: Arc<Object>,
    depth: usize,
    children: Vec<usize>,
}

/// The document outline: its items in depth-first order, as a tree.
///
/// Walking it is done once, when the outline is read; each item's fields are read lazily.
///
/// Added in 0.2.0.
pub struct Outline<'d> {
    doc: &'d Document,
    items: Vec<Item>,
    roots: Vec<usize>,
    /// Item index by ID.
    by_id: BTreeMap<OutlineItemId, usize>,
    pages: PageSource<'d>,
    names: OnceCell<BTreeMap<Vec<u8>, Object>>,
}

/// The pages destinations resolve against: borrowed from [`Pages::outline`], or walked once
/// on first need for [`Document::outline`].
enum PageSource<'d> {
    Borrowed(&'d Pages<'d>),
    Lazy(OnceCell<Option<Pages<'d>>>),
}

impl std::fmt::Debug for Outline<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outline")
            .field("items", &self.items.len())
            .finish_non_exhaustive()
    }
}

/// An item still to visit: the stored value, its parent's index, its depth and its path
/// from the nearest indirect object.
struct Pending {
    obj: Object,
    parent: Option<usize>,
    depth: usize,
    path: DirectPath,
}

impl<'d> Outline<'d> {
    pub(crate) fn new(doc: &'d Document) -> Result<Self> {
        Self::build(doc, PageSource::Lazy(OnceCell::new()))
    }

    pub(crate) fn with_pages(doc: &'d Document, pages: &'d Pages<'d>) -> Result<Self> {
        Self::build(doc, PageSource::Borrowed(pages))
    }

    /// An outline without its items, for resolving destinations alone (a page's links)
    /// without walking a bookmark tree that can hold thousands of items.
    pub(crate) const fn without_items(doc: &'d Document, pages: &'d Pages<'d>) -> Self {
        Self {
            doc,
            items: Vec::new(),
            roots: Vec::new(),
            by_id: BTreeMap::new(),
            pages: PageSource::Borrowed(pages),
            names: OnceCell::new(),
        }
    }

    /// An outline without its items that walks the page tree only when a destination needs
    /// it: for resolving one destination (the catalog's `/OpenAction`).
    pub(crate) const fn destinations_only(doc: &'d Document) -> Self {
        Self {
            doc,
            items: Vec::new(),
            roots: Vec::new(),
            by_id: BTreeMap::new(),
            pages: PageSource::Lazy(OnceCell::new()),
            names: OnceCell::new(),
        }
    }

    /// The document it reads.
    pub(crate) const fn doc(&self) -> &'d Document {
        self.doc
    }

    fn build(doc: &'d Document, pages: PageSource<'d>) -> Result<Self> {
        let mut outline = Self {
            doc,
            items: Vec::new(),
            roots: Vec::new(),
            by_id: BTreeMap::new(),
            pages,
            names: OnceCell::new(),
        };
        let catalog = doc.catalog()?;
        let Some(root_obj) = catalog.get("Outlines") else {
            return Ok(outline);
        };
        let catalog_ref = doc.trailer().get("Root").and_then(Object::as_ref);
        let root = doc.read(root_obj)?;
        let Some(root) = root.as_deref().and_then(Object::as_dict) else {
            if !matches!(root.as_deref(), Some(Object::Null)) {
                doc.warn(Warning::InvalidEntry {
                    object: catalog_ref.map(|r| r.num),
                    key: "Outlines",
                })?;
            }
            return Ok(outline);
        };
        let Some(first) = root.get("First") else {
            return Ok(outline);
        };
        // A direct `/Outlines` sits in the catalog (or, when `/Root` is direct too, in the
        // trailer): its items' paths start there, as the page tree's do.
        let root_path = match (root_obj.as_ref(), catalog_ref) {
            (Some(r), _) => DirectPath::new(Some(r), Vec::new()),
            (None, Some(catalog)) => DirectPath::new(Some(catalog), vec![PathSegment::Key("Outlines")]),
            (None, None) => DirectPath::new(None, vec![PathSegment::Key("Root"), PathSegment::Key("Outlines")]),
        };
        outline.walk(first.clone(), &root_path, root_obj.as_ref())?;
        outline.by_id = outline
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| (item.id.clone(), i))
            .collect();
        Ok(outline)
    }

    /// Visits every item depth-first. Pushing an item's `/Next` before its `/First` makes
    /// the arena order pre-order (children before later siblings).
    fn walk(&mut self, first: Object, root_path: &DirectPath, root_ref: Option<ObjRef>) -> Result<()> {
        let doc = self.doc;
        let max_depth = doc.limits().max_tree_depth();
        let mut visited: BTreeSet<u32> = root_ref.map(|r| r.num).into_iter().collect();
        let mut stack = vec![Pending {
            path: child_path(root_path, None, "First"),
            obj: first,
            parent: None,
            depth: 1,
        }];
        while let Some(Pending {
            obj,
            parent,
            depth,
            path,
        }) = stack.pop()
        {
            let skip = || {
                doc.warn(Warning::OutlineItemSkipped {
                    object: object_number(&obj),
                })
            };
            if depth > max_depth {
                skip()?;
                continue;
            }
            if let Some(r) = obj.as_ref()
                && !visited.insert(r.num)
            {
                doc.warn(Warning::OutlineCycle { object: r.num })?;
                continue;
            }
            let Some(node) = doc.read(&obj)? else {
                skip()?;
                continue;
            };
            let Some(dict) = node.as_dict() else {
                // `null` (a missing object) simply ends the chain.
                if !matches!(*node, Object::Null) {
                    skip()?;
                }
                continue;
            };
            let id = match obj.as_ref() {
                Some(r) => OutlineItemId::object(r),
                None => OutlineItemId::path(&path),
            };
            let index = self.items.len();
            if let Some(next) = dict.get("Next") {
                stack.push(Pending {
                    path: child_path(&path, obj.as_ref(), "Next"),
                    obj: next.clone(),
                    parent,
                    depth,
                });
            }
            if let Some(first) = dict.get("First") {
                stack.push(Pending {
                    path: child_path(&path, obj.as_ref(), "First"),
                    obj: first.clone(),
                    parent: Some(index),
                    depth: depth + 1,
                });
            }
            self.items.push(Item {
                id,
                node: Arc::clone(&node),
                depth,
                children: Vec::new(),
            });
            match parent.and_then(|p| self.items.get_mut(p)) {
                Some(p) => p.children.push(index),
                None => self.roots.push(index),
            }
        }
        Ok(())
    }

    /// The number of items (at every level).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when the document has no outline items.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The top-level items.
    ///
    /// Added in 0.2.0.
    pub fn roots(&self) -> impl Iterator<Item = OutlineItem<'_, 'd>> + '_ {
        self.roots.iter().map(|&index| OutlineItem { outline: self, index })
    }

    /// Every item, depth-first (each item before its children, children before later
    /// siblings).
    ///
    /// Added in 0.2.0.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = OutlineItem<'_, 'd>> + '_ {
        (0..self.items.len()).map(|index| OutlineItem { outline: self, index })
    }

    /// The item with ID `id`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn by_id(&self, id: &OutlineItemId) -> Option<OutlineItem<'_, 'd>> {
        self.by_id.get(id).map(|&index| OutlineItem { outline: self, index })
    }

    fn pages(&self) -> Result<Option<&Pages<'d>>> {
        let cell = match &self.pages {
            PageSource::Borrowed(pages) => return Ok(Some(pages)),
            PageSource::Lazy(cell) => cell,
        };
        if let Some(pages) = cell.get() {
            return Ok(pages.as_ref());
        }
        let pages = crate::document::tolerant(self.doc.pages())?;
        Ok(cell.get_or_init(|| pages).as_ref())
    }

    /// The named destination `name` (ISO 32000-2 §12.3.2.4), from the `/Names /Dests` tree or
    /// the catalog's `/Dests` dictionary: `None` when the document names no such destination,
    /// a [`Destination`] without a page when the name is there but its target can't be read.
    ///
    /// # Errors
    /// As [`Outline::roots`]: strict mode, when reading the names or the page tree raises a
    /// warning.
    ///
    /// Added in 0.2.0.
    pub fn named_destination(&self, name: &[u8]) -> Result<Option<Destination>> {
        if !self.named()?.contains_key(name) {
            return Ok(None);
        }
        self.destination(&Object::Name(laminapdf_syntax::Name::new(name)))
    }

    /// The value the document stores for the named destination `name`: a destination array,
    /// or a dictionary holding one in `/D` (ISO 32000-2 §12.3.2.4), as written, with its
    /// references unresolved; `None` when the document names no such destination.
    ///
    /// # Errors
    /// As [`Outline::named_destination`].
    ///
    /// Added in 0.2.0.
    pub fn named_destination_value(&self, name: &[u8]) -> Result<Option<Object>> {
        let Some(value) = self.named()?.get(name).cloned() else {
            return Ok(None);
        };
        let resolved = match self.doc.read(&value)? {
            Some(value) => (*value).clone(),
            None => return Ok(None),
        };
        Ok(Some(match resolved {
            Object::Dict(d) => match d.get("D") {
                Some(dest) => match self.doc.read(dest)? {
                    Some(dest) => (*dest).clone(),
                    None => return Ok(None),
                },
                None => return Ok(None),
            },
            other => other,
        }))
    }

    /// Named destinations: the `/Names /Dests` tree first, then the catalog's `/Dests`
    /// dictionary (PDF 1.1), keyed by name bytes.
    /// The names of every named destination, from the `/Names /Dests` tree and the catalog's
    /// `/Dests` (a name in both listed once), sorted by their bytes.
    ///
    /// # Errors
    /// Reading errors of the name tree and the catalog.
    ///
    /// Added in 0.2.0.
    pub fn destination_names(&self) -> Result<Vec<Vec<u8>>> {
        Ok(self.named()?.keys().cloned().collect())
    }

    fn named(&self) -> Result<&BTreeMap<Vec<u8>, Object>> {
        if let Some(names) = self.names.get() {
            return Ok(names);
        }
        let mut names = BTreeMap::new();
        if let Some(tree) = self.doc.name_tree("Dests")? {
            for (key, value) in tree.entries()? {
                names.entry(key).or_insert(value);
            }
        }
        let catalog = self.doc.catalog()?;
        if let Some(dests) = catalog.get("Dests") {
            let resolved = self.doc.read(dests)?;
            if let Some(dests) = resolved.as_deref().and_then(Object::as_dict) {
                for (key, value) in dests.iter() {
                    names.entry(key.as_bytes().to_vec()).or_insert_with(|| value.clone());
                }
            } else if !matches!(resolved.as_deref(), Some(Object::Null)) {
                self.doc.warn(Warning::InvalidEntry {
                    object: self.doc.trailer().get("Root").and_then(Object::as_ref).map(|r| r.num),
                    key: "Dests",
                })?;
            }
        }
        Ok(self.names.get_or_init(|| names))
    }

    /// Resolves a destination value (`/Dest` or a `GoTo` action's `/D`).
    pub(crate) fn destination(&self, value: &Object) -> Result<Option<Destination>> {
        let Some(resolved) = self.doc.read(value)? else {
            return Ok(None);
        };
        let (name, explicit) = match &*resolved {
            Object::Name(n) => (Some(n.as_bytes().to_vec()), self.named()?.get(n.as_bytes()).cloned()),
            Object::String(s) => (Some(s.bytes.clone()), self.named()?.get(&s.bytes).cloned()),
            other => (None, Some(other.clone())),
        };
        // An unresolvable target keeps what is known: the name, and the value as stored.
        let unresolved = |raw: Object| {
            name.clone().map(|name| Destination {
                page: None,
                page_index: None,
                view: View::Unknown,
                name: Some(name),
                raw,
            })
        };
        let Some(explicit) = explicit else {
            return Ok(unresolved(Object::Null));
        };
        // A named destination's value is an array or a dictionary with `/D` (§12.3.2.4).
        let Some(mut array) = self.doc.read(&explicit)? else {
            return Ok(unresolved(explicit));
        };
        if let Some(d) = array.as_dict().and_then(|d| d.get("D")).cloned() {
            match self.doc.read(&d)? {
                Some(inner) => array = inner,
                None => return Ok(unresolved(explicit)),
            }
        }
        let Some(items) = array.as_array() else {
            return Ok(unresolved(explicit));
        };
        let (page, page_index) = self.target_page(items.first())?;
        Ok(Some(Destination {
            page,
            page_index,
            view: self.view(items)?,
            name,
            raw: (*array).clone(),
        }))
    }

    /// The page of an explicit destination: a page reference, or (as some writers do) a
    /// page index.
    fn target_page(&self, first: Option<&Object>) -> Result<(Option<crate::id::PageId>, Option<usize>)> {
        let Some(pages) = self.pages()? else {
            return Ok((None, None));
        };
        let index = match first {
            Some(Object::Ref(r)) => pages.index_of_object(*r),
            Some(Object::Integer(i)) => usize::try_from(*i).ok().filter(|i| *i < pages.len()),
            _ => None,
        };
        Ok((index.and_then(|i| pages.get(i)).map(|p| p.id().clone()), index))
    }

    fn view(&self, items: &[Object]) -> Result<View> {
        let doc = self.doc;
        let num = |i: usize| -> Result<Option<f64>> {
            match items.get(i) {
                Some(obj) => number(doc, obj),
                None => Ok(None),
            }
        };
        let kind = match items.get(1) {
            Some(obj) => doc.read(obj)?,
            None => None,
        };
        let Some(Object::Name(kind)) = kind.as_deref() else {
            return Ok(View::Unknown);
        };
        Ok(match kind.as_bytes() {
            b"XYZ" => View::Xyz {
                left: num(2)?,
                top: num(3)?,
                zoom: num(4)?,
            },
            b"Fit" => View::Fit,
            b"FitH" => View::FitH { top: num(2)? },
            b"FitV" => View::FitV { left: num(2)? },
            b"FitR" => {
                let rect = Object::Array(items.get(2..6).unwrap_or_default().to_vec());
                match read_rect(doc, &rect)? {
                    Some(r) => View::FitR(r),
                    None => View::Unknown,
                }
            }
            b"FitB" => View::FitB,
            b"FitBH" => View::FitBH { top: num(2)? },
            b"FitBV" => View::FitBV { left: num(2)? },
            _ => View::Unknown,
        })
    }
}

/// The path of the item stored at `key` of the item at `path` (`own` when that item is
/// indirect, which re-anchors the path).
fn child_path(path: &DirectPath, own: Option<ObjRef>, key: &'static str) -> DirectPath {
    if let Some(r) = own {
        DirectPath::new(Some(r), vec![PathSegment::Key(key)])
    } else {
        let mut segments = path.segments.clone();
        segments.push(PathSegment::Key(key));
        DirectPath::new(path.anchor, segments)
    }
}

/// One outline item: a view over its dictionary.
///
/// Added in 0.2.0.
#[derive(Clone, Copy)]
pub struct OutlineItem<'o, 'd> {
    outline: &'o Outline<'d>,
    index: usize,
}

impl std::fmt::Debug for OutlineItem<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutlineItem")
            .field("id", &self.id())
            .field("depth", &self.depth())
            .finish_non_exhaustive()
    }
}

impl<'o, 'd> OutlineItem<'o, 'd> {
    fn item(&self) -> Option<&'o Item> {
        self.outline.items.get(self.index)
    }

    fn get(&self, key: &str) -> Option<&'o Object> {
        self.item().and_then(|i| i.node.as_dict()).and_then(|d| d.get(key))
    }

    /// The item dictionary.
    #[cfg(feature = "navigation")]
    pub(crate) fn dict(&self) -> Option<&'o laminapdf_cos::Dict> {
        self.item().and_then(|i| i.node.as_dict())
    }

    /// The item's stable ID.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn id(&self) -> Option<&'o OutlineItemId> {
        self.item().map(|i| &i.id)
    }

    /// The nesting depth: 1 for a top-level item.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.item().map_or(0, |i| i.depth)
    }

    /// The item's children, in order.
    ///
    /// Added in 0.2.0.
    pub fn children(&self) -> impl Iterator<Item = OutlineItem<'o, 'd>> + 'o {
        let outline = self.outline;
        self.item()
            .map(|i| i.children.as_slice())
            .unwrap_or_default()
            .iter()
            .map(move |&index| OutlineItem { outline, index })
    }

    /// `/Title`, decoded (§7.9.2.2); empty when absent or not a string.
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::DataPending`] / [`laminapdf_cos::Error::Source`].
    ///
    /// Added in 0.2.0.
    pub fn title(&self) -> Result<String> {
        let Some(title) = self.get("Title") else {
            return Ok(String::new());
        };
        Ok(match self.outline.doc.read(title)?.as_deref() {
            Some(Object::String(s)) => decode_text_string(&s.bytes),
            _ => String::new(),
        })
    }

    /// `/Count`: positive when the item is open (the number of visible descendants),
    /// negative when closed; `None` when absent.
    ///
    /// # Errors
    /// As [`OutlineItem::title`].
    ///
    /// Added in 0.2.0.
    pub fn count(&self) -> Result<Option<i64>> {
        let Some(count) = self.get("Count") else {
            return Ok(None);
        };
        Ok(self.outline.doc.read(count)?.and_then(|c| c.as_int()))
    }

    /// Whether the item is shown open (`/Count` positive).
    ///
    /// # Errors
    /// As [`OutlineItem::title`].
    ///
    /// Added in 0.2.0.
    pub fn is_open(&self) -> Result<bool> {
        Ok(self.count()?.is_some_and(|c| c > 0))
    }

    /// The action type of `/A`, if the item has an action.
    ///
    /// # Errors
    /// As [`OutlineItem::title`].
    ///
    /// Added in 0.2.0.
    pub fn action(&self) -> Result<Option<ActionKind>> {
        match self.action_dict()?.as_deref().and_then(Object::as_dict) {
            Some(action) => self.action_kind(action),
            None => Ok(None),
        }
    }

    /// Where the item points: `/Dest`, or a `GoTo` action's `/D`. Resolved to a page of
    /// this document and a view when possible; otherwise the raw data is kept
    /// ([`Destination::raw`], [`Destination::name`]).
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::Strict`] in strict mode when reading named destinations or the
    /// page tree raises a warning; as [`OutlineItem::title`].
    ///
    /// Added in 0.2.0.
    pub fn target(&self) -> Result<Option<Destination>> {
        // §12.3.3: `/Dest` and `/A` are exclusive; a `/Dest` that is null or unreadable
        // counts as absent, so a `GoTo` action is still followed.
        if let Some(dest) = self.get("Dest")
            && let Some(dest) = self.outline.destination(dest)?
        {
            return Ok(Some(dest));
        }
        let Some(action) = self.action_dict()? else {
            return Ok(None);
        };
        let Some(action) = action.as_dict() else {
            return Ok(None);
        };
        if self.action_kind(action)? != Some(ActionKind::GoTo) {
            return Ok(None);
        }
        match action.get("D") {
            Some(d) => self.outline.destination(d),
            None => Ok(None),
        }
    }

    /// The web address of a `URI` action (§12.6.4.8), lossily decoded from its ASCII bytes.
    ///
    /// # Errors
    /// As [`OutlineItem::title`].
    ///
    /// Added in 0.2.0.
    pub fn uri(&self) -> Result<Option<String>> {
        let Some(action) = self.action_dict()? else {
            return Ok(None);
        };
        let Some(action) = action.as_dict() else {
            return Ok(None);
        };
        if self.action_kind(action)? != Some(ActionKind::Uri) {
            return Ok(None);
        }
        Ok(
            match action
                .get("URI")
                .map(|u| self.outline.doc.read(u))
                .transpose()?
                .flatten()
                .as_deref()
            {
                Some(Object::String(s)) => Some(String::from_utf8_lossy(&s.bytes).into_owned()),
                _ => None,
            },
        )
    }

    /// The colour (`/C`, §12.3.3 Table 151) as three numbers from 0 to 1, when it holds three
    /// numbers.
    ///
    /// # Errors
    /// As [`OutlineItem::title`].
    ///
    /// Added in 0.2.0.
    pub fn color(&self) -> Result<Option<[f64; 3]>> {
        let Some(c) = self.get("C") else { return Ok(None) };
        let Some(c) = self.outline.doc.read(c)? else {
            return Ok(None);
        };
        Ok(match c.as_array() {
            Some([r, g, b]) => match (r.as_f64(), g.as_f64(), b.as_f64()) {
                (Some(r), Some(g), Some(b)) => Some([r, g, b]),
                _ => None,
            },
            _ => None,
        })
    }

    /// The style flags (`/F`: bit 1 italic, bit 2 bold); 0 without them.
    ///
    /// # Errors
    /// As [`OutlineItem::title`].
    ///
    /// Added in 0.2.0.
    pub fn flags(&self) -> Result<i64> {
        let Some(f) = self.get("F") else { return Ok(0) };
        Ok(self.outline.doc.read(f)?.and_then(|f| f.as_int()).unwrap_or(0))
    }

    /// `/A`, resolved.
    fn action_dict(&self) -> Result<Option<Arc<Object>>> {
        match self.get("A") {
            Some(action) => self.outline.doc.read(action),
            None => Ok(None),
        }
    }

    fn action_kind(&self, action: &laminapdf_cos::Dict) -> Result<Option<ActionKind>> {
        let Some(s) = action.get("S") else {
            return Ok(None);
        };
        Ok(match self.outline.doc.read(s)?.as_deref() {
            Some(Object::Name(n)) => Some(ActionKind::from_name(n.as_bytes())),
            _ => None,
        })
    }
}
