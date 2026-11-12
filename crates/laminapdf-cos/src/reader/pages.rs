//! The page tree: [`Page`]s with their inherited attributes (§7.7.3), the walk and page content.

use std::collections::BTreeSet;
use std::sync::Arc;

use laminapdf_syntax::{Dict, ObjRef, Object};

use super::{Document, Warning};
use crate::error::{Error, Result, absent_if_malformed};

/// A page with its inheritable attributes (§7.7.3.4): `/Resources` resolved, the others as
/// stored. `laminapdf-model`'s page view reads the boxes and `/Rotate` from them.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Page {
    /// The page object's reference (`None` for a non-standard direct page dictionary).
    pub obj_ref: Option<ObjRef>,
    /// The page dictionary itself.
    pub dict: Dict,
    /// `/Resources`, own or inherited, resolved. Always an [`Object::Dict`]; shared (not
    /// copied) between pages that inherit the same resources. See [`Page::resources_dict`].
    pub resources: Option<Arc<Object>>,
    /// The inheritable attributes as stored (unresolved, own or inherited).
    pub inherited: Arc<Inherited>,
    /// Where a direct page dictionary sits (`None` when the page is an indirect object).
    pub direct_path: Option<DirectPath>,
}

/// The location of a direct (inline) object: its nearest indirect ancestor and the keys and
/// array indices that lead from there to it, e.g. `5 0 R` → `/Kids` → `3` (ADR 0005 §5).
/// The same bytes always give the same path.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct DirectPath {
    /// The nearest indirect ancestor (`None` when the chain starts at the trailer, i.e. the
    /// trailer's `/Root` is itself a direct dictionary).
    pub anchor: Option<ObjRef>,
    /// The steps from the anchor (from the trailer when `anchor` is `None`).
    pub segments: Vec<PathSegment>,
}

/// One step of a [`DirectPath`].
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathSegment {
    /// A dictionary key, e.g. `Kids`.
    Key(&'static str),
    /// An array index.
    Index(usize),
}

impl DirectPath {
    /// The path `segments` from `anchor` (`None`: from the trailer).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(anchor: Option<ObjRef>, segments: Vec<PathSegment>) -> Self {
        Self { anchor, segments }
    }

    fn child(&self, step: PathSegment) -> Self {
        let mut segments = Vec::with_capacity(self.segments.len() + 1);
        segments.extend_from_slice(&self.segments);
        segments.push(step);
        Self {
            anchor: self.anchor,
            segments,
        }
    }
}

impl Page {
    /// The resolved `/Resources` dictionary, if any.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn resources_dict(&self) -> Option<&Dict> {
        match self.resources.as_deref() {
            Some(Object::Dict(d)) => Some(d),
            _ => None,
        }
    }
}

/// Inheritable page attributes (§7.7.3.4) as stored, own or inherited and unresolved (a
/// value may be a reference). Carried down the page tree and shared via [`Arc`], so passing
/// them to every kid (and every page) never copies a dictionary.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Inherited {
    /// `/MediaBox`.
    pub media_box: Option<Arc<Object>>,
    /// `/CropBox`.
    pub crop_box: Option<Arc<Object>>,
    /// `/Rotate`.
    pub rotate: Option<Arc<Object>>,
    /// `/Resources`.
    pub resources: Option<Arc<Object>>,
}

const INHERITABLE: [&str; 4] = ["MediaBox", "CropBox", "Rotate", "Resources"];

impl Inherited {
    /// Each inheritable attribute's key with its value: `/MediaBox`, `/CropBox`, `/Rotate`,
    /// `/Resources`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn entries(&self) -> [(&'static str, Option<&Arc<Object>>); 4] {
        let values = [&self.media_box, &self.crop_box, &self.rotate, &self.resources];
        let mut out = [("", None); 4];
        for ((slot, key), value) in out.iter_mut().zip(INHERITABLE).zip(values) {
            *slot = (key, value.as_ref());
        }
        out
    }

    /// `parent` with any attributes `dict` sets itself replacing the inherited ones.
    /// Returns `parent` unchanged (a pointer copy) when `dict` sets none of them.
    ///
    /// A value that is `null`, or a reference that `is_null` says resolves to `null` (a missing
    /// or free object), counts as absent (§7.3.7, §7.3.10): the inherited value is kept.
    ///
    /// # Errors
    /// Whatever `is_null` fails with (it resolves references).
    fn overlay(parent: &Arc<Self>, dict: &Dict, is_null: impl Fn(&Object) -> Result<bool>) -> Result<Arc<Self>> {
        let mut own: [Option<&Object>; 4] = [None; 4];
        for (slot, key) in own.iter_mut().zip(INHERITABLE) {
            if let Some(value) = dict.get(key)
                && !is_null(value)?
            {
                *slot = Some(value);
            }
        }
        if own.iter().all(Option::is_none) {
            return Ok(Arc::clone(parent));
        }
        let [media_box, crop_box, rotate, resources] = own;
        let pick = |own: Option<&Object>, inherited: &Option<Arc<Object>>| {
            own.map(|o| Arc::new(o.clone())).or_else(|| inherited.clone())
        };
        Ok(Arc::new(Self {
            media_box: pick(media_box, &parent.media_box),
            crop_box: pick(crop_box, &parent.crop_box),
            rotate: pick(rotate, &parent.rotate),
            resources: pick(resources, &parent.resources),
        }))
    }
}

impl Document {
    /// Walks the page tree in document order, applying inherited attributes.
    ///
    /// Tolerant: a node that is missing, unparsable, not a dictionary, deeper than
    /// [`Limits::max_page_tree_depth`] or otherwise over a limit (including a missing `/Pages`
    /// root) is skipped, as is a `/Kids` that is not an array — each with
    /// [`Warning::PageTreeNodeSkipped`]. `/Count` is not trusted: the walk is authoritative,
    /// and a root `/Count` that differs raises [`Warning::PageCountMismatch`].
    ///
    /// Page attributes are inherited as stored (the page's own reference is resolved only to
    /// tell an explicit `null` from a value); `laminapdf-model` reads the boxes and `/Rotate`. A
    /// malformed `/Resources` or `/Count` reads as absent.
    ///
    /// # Errors
    /// [`Error::ReferenceCycle`] when a page-tree node is reachable twice (e.g. a
    /// self-referencing `/Kids`) or a reference chain loops; [`Error::Strict`] in strict mode
    /// for any skip or any warning raised while reading attributes; [`Error::LimitExceeded`]
    /// when reading a page attribute exceeds a hard limit; errors from [`Document::catalog`].
    ///
    /// [`Limits::max_page_tree_depth`]: laminapdf_syntax::Limits::max_page_tree_depth
    ///
    /// Added in 0.2.0.
    pub fn pages(&self) -> Result<Vec<Page>> {
        let _call = self.call();
        let catalog = self.catalog()?;
        let root = catalog.get("Pages").cloned().unwrap_or(Object::Null);
        // A direct `/Pages` sits in the catalog: its path starts there (ADR 0005 §5).
        let root_path = (!matches!(root, Object::Ref(_))).then(|| {
            let catalog_ref = self.trailer().get("Root").and_then(Object::as_ref);
            let segments = match catalog_ref {
                Some(_) => vec![PathSegment::Key("Pages")],
                None => vec![PathSegment::Key("Root"), PathSegment::Key("Pages")],
            };
            DirectPath::new(catalog_ref, segments)
        });
        let mut visited = BTreeSet::new();
        let mut stack = vec![(root.clone(), Arc::new(Inherited::default()), 0usize, root_path)];
        let mut pages = Vec::new();
        while let Some((node_obj, inherited, depth, path)) = stack.pop() {
            let obj_ref = node_obj.as_ref();
            let skip = || {
                self.warn(Warning::PageTreeNodeSkipped {
                    object: obj_ref.map(|r| r.num),
                })
            };
            // ADR 0003 §3: `max_page_tree_depth` is a hard limit hit inside the walk, one of
            // the tolerant paths, so the subtree is skipped with `page-tree-node-skipped`
            // (strict mode: `Error::Strict`) instead of failing the call.
            if depth > self.limits.max_page_tree_depth() {
                skip()?;
                continue;
            }
            if let Some(r) = obj_ref
                && !visited.insert(r.num)
            {
                return Err(Error::ReferenceCycle(r.num));
            }
            let resolved = match self.deref(&node_obj) {
                Ok(obj) => obj,
                Err(e) if e.ends_page_walk() => return Err(e),
                Err(_) => {
                    skip()?;
                    continue;
                }
            };
            let Object::Dict(node) = &*resolved else {
                skip()?;
                continue;
            };
            let attrs = Inherited::overlay(&inherited, node, |o| match o {
                Object::Null => Ok(true),
                // A malformed object counts as present (it was before): the page keeps the
                // attribute and reading it later yields `None`.
                Object::Ref(_) => Ok(absent_if_malformed(self.deref(o))?.is_some_and(|r| matches!(*r, Object::Null))),
                _ => Ok(false),
            })?;
            let is_tree_node = node.get_name("Type") == Some(b"Pages")
                || (node.get_name("Type") != Some(b"Page") && node.contains_key("Kids"));
            if is_tree_node {
                let kids_obj = node.get("Kids").unwrap_or(&Object::Null);
                let kids = match self.deref(kids_obj) {
                    Ok(obj) => obj,
                    Err(e) if e.ends_page_walk() => return Err(e),
                    Err(_) => {
                        skip()?;
                        continue;
                    }
                };
                if let Object::Array(kids) = &*kids {
                    // A direct kid is named from its nearest indirect ancestor: the `/Kids`
                    // array when that is an indirect object, else the node, else the node's
                    // own path. Indirect kids need no path.
                    let base = match (kids_obj.as_ref(), obj_ref) {
                        (Some(array), _) => DirectPath::new(Some(array), Vec::new()),
                        (None, Some(r)) => DirectPath::new(Some(r), vec![PathSegment::Key("Kids")]),
                        (None, None) => {
                            let mut base = path.unwrap_or_else(|| DirectPath::new(None, Vec::new()));
                            base.segments.push(PathSegment::Key("Kids"));
                            base
                        }
                    };
                    stack.extend(kids.iter().enumerate().rev().map(|(i, kid)| {
                        let kid_path = (!matches!(kid, Object::Ref(_))).then(|| base.child(PathSegment::Index(i)));
                        (kid.clone(), Arc::clone(&attrs), depth + 1, kid_path)
                    }));
                } else {
                    skip()?;
                }
            } else {
                let direct_path = if obj_ref.is_none() { path } else { None };
                pages.push(self.make_page(obj_ref, node.clone(), attrs, direct_path)?);
            }
        }
        self.check_page_count(&root, pages.len())?;
        Ok(pages)
    }

    /// Raises [`Warning::PageCountMismatch`] when the root node's `/Count` differs from `walked`.
    fn check_page_count(&self, root: &Object, walked: usize) -> Result<()> {
        let declared = match absent_if_malformed(self.deref(root))? {
            Some(node) => match node.as_dict().and_then(|d| d.get("Count")) {
                Some(count) => absent_if_malformed(self.deref(count))?.and_then(|c| c.as_int()),
                None => None,
            },
            None => None,
        };
        let walked = u64::try_from(walked).unwrap_or(u64::MAX);
        match declared {
            Some(declared) if u64::try_from(declared).ok() != Some(walked) => self.warn(Warning::PageCountMismatch {
                object: root.as_ref().map(|r| r.num),
                declared,
                walked,
            }),
            _ => Ok(()),
        }
    }

    /// A page from its dictionary and inherited attributes. Malformed `/Resources` is `None`;
    /// cycles, hard limits and strict-mode refusals fail the call.
    fn make_page(
        &self,
        obj_ref: Option<ObjRef>,
        dict: Dict,
        attrs: Arc<Inherited>,
        direct_path: Option<DirectPath>,
    ) -> Result<Page> {
        // A direct dictionary is already shared through `attrs`; a reference resolves to
        // the cached object. Either way no dictionary is copied per page.
        let resources = match attrs.resources.as_ref() {
            Some(o) => match **o {
                Object::Ref(r) => absent_if_malformed(self.resolve(r))?,
                _ => Some(Arc::clone(o)),
            },
            None => None,
        }
        .filter(|shared| matches!(**shared, Object::Dict(_)));
        Ok(Page {
            obj_ref,
            resources,
            dict,
            inherited: attrs,
            direct_path,
        })
    }

    /// Number of leaf pages in the page tree.
    ///
    /// # Errors
    /// As [`Document::pages`].
    ///
    /// Added in 0.2.0.
    pub fn page_count(&self) -> Result<usize> {
        self.pages().map(|p| p.len())
    }

    /// The decoded content of a page: all `/Contents` streams joined with `\n`. `/Contents`
    /// (or an array element) that is `null`, missing or free contributes nothing. All the
    /// page's streams share one [`Limits::max_decoded_total`] budget.
    ///
    /// # Errors
    /// Decode errors, [`Error::LimitExceeded`], or [`Error::Type`] if `/Contents` is not a
    /// stream or array of streams.
    ///
    /// [`Limits::max_decoded_total`]: laminapdf_syntax::Limits::max_decoded_total
    ///
    /// Added in 0.2.0.
    pub fn page_content(&self, page: &Page) -> Result<Vec<u8>> {
        let _call = self.call();
        let Some(contents) = page.dict.get("Contents") else {
            return Ok(Vec::new());
        };
        let resolved = self.deref(contents)?;
        let parts = match &*resolved {
            Object::Array(items) => items.as_slice(),
            single => std::slice::from_ref(single),
        };
        let mut out = Vec::new();
        let mut first = true;
        for part in parts {
            // The object number names the stream in a `flate-truncated` warning.
            let object = match part {
                Object::Ref(r) => Some(r.num),
                _ => contents.as_ref().map(|r| r.num),
            };
            let part = self.deref(part)?;
            let s = match &*part {
                Object::Stream(s) => s,
                // A missing or free object is null (§7.3.10): it contributes no content.
                Object::Null => continue,
                _ => {
                    return Err(Error::Type {
                        expected: "stream",
                        context: "/Contents",
                    });
                }
            };
            if !first {
                out.push(b'\n');
            }
            first = false;
            out.extend_from_slice(&self.decode_stream_of(s, object)?);
        }
        Ok(out)
    }
}
