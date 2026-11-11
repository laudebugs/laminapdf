//! What reader mode reads from the structure tree itself, besides the blocks extraction makes of
//! it: each element's place in the tree's (reading) order, the elements shown as crops
//! (`Figure`, `Formula`, `Table`: ISO 32000-2 §14.8.4.7–8, whose content reflowed as text would
//! lose its meaning) with their `/Alt` and their marked content, and how much of the tree's
//! content sits in no block-level element at all.

use std::collections::BTreeMap;

use laminapdf_cos::ObjRef;
use laminapdf_model::{PageId, StructElem, StructKid, StructTree};

/// What a crop shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CropKind {
    Figure,
    Formula,
    Table,
}

impl CropKind {
    fn of(kind: &[u8]) -> Option<Self> {
        match kind {
            b"Figure" => Some(Self::Figure),
            b"Formula" => Some(Self::Formula),
            b"Table" => Some(Self::Table),
            _ => None,
        }
    }
}

/// An element shown as a crop of its page.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Crop {
    pub(crate) kind: CropKind,
    /// Its place in the tree's order.
    pub(crate) order: usize,
    pub(crate) alt: Option<String>,
    /// The marked content it holds, its descendants' included: (page index, MCID).
    pub(crate) content: Vec<(usize, i64)>,
}

/// The structure tree, read for reader mode.
#[derive(Debug, Clone, Default)]
pub(crate) struct TreeFacts {
    /// Each element's place in the tree's order (pre-order), by object.
    pub(crate) order: BTreeMap<ObjRef, usize>,
    /// The crop each element inside a crop element belongs to, the crop's own included.
    pub(crate) in_crop: BTreeMap<ObjRef, usize>,
    pub(crate) crops: Vec<Crop>,
    /// Marked-content sequences in the tree, and those with no block-level element above them.
    pub(crate) content: usize,
    pub(crate) generic: usize,
}

/// Standard types that hold content without saying what it is (§14.8.4.2–4, §14.8.4.6):
/// content under only these is "generic", text with no paragraph, heading or list around it.
const GENERIC: [&[u8]; 12] = [
    b"Document",
    b"DocumentFragment",
    b"Part",
    b"Art",
    b"Sect",
    b"Div",
    b"NonStruct",
    b"Private",
    b"Span",
    b"Aside",
    b"Index",
    b"TOC",
];

struct Walk<'a> {
    tree: &'a StructTree,
    pages: &'a BTreeMap<PageId, usize>,
    facts: TreeFacts,
    next: usize,
}

impl Walk<'_> {
    /// Walks `elem`, whose page is `page` unless it names its own, inside the crop `crop`, and
    /// under a block-level element when `block`.
    fn walk(&mut self, elem: &StructElem, page: Option<usize>, crop: Option<usize>, block: bool) {
        let order = self.next;
        self.next += 1;
        let kind = self.tree.standard_kind(&elem.kind);
        let page = elem.page.as_ref().and_then(|p| self.pages.get(p).copied()).or(page);
        let crop = match (crop, CropKind::of(kind)) {
            (Some(crop), _) => Some(crop),
            (None, Some(kind)) => {
                self.facts.crops.push(Crop {
                    kind,
                    order,
                    alt: elem.alt.clone().filter(|a| !a.trim().is_empty()),
                    content: Vec::new(),
                });
                Some(self.facts.crops.len() - 1)
            }
            (None, None) => None,
        };
        if let Some(object) = elem.object {
            self.facts.order.insert(object, order);
            if let Some(crop) = crop {
                self.facts.in_crop.insert(object, crop);
            }
        }
        let block = block || !GENERIC.contains(&kind);
        for kid in &elem.kids {
            match kid {
                StructKid::Elem(child) => self.walk(child, page, crop, block),
                StructKid::Mcid { page: own, mcid } => {
                    self.facts.content += 1;
                    if !block {
                        self.facts.generic += 1;
                    }
                    let page = own.as_ref().and_then(|p| self.pages.get(p).copied()).or(page);
                    if let (Some(crop), Some(page)) = (crop, page)
                        && let Some(c) = self.facts.crops.get_mut(crop)
                    {
                        c.content.push((page, *mcid));
                    }
                }
                // An annotation or XObject: no marked content of its own to read.
                _ => {}
            }
        }
    }
}

/// Reads `tree`, whose pages are numbered by `pages`.
pub(crate) fn facts(tree: &StructTree, pages: &BTreeMap<PageId, usize>) -> TreeFacts {
    let mut walk = Walk {
        tree,
        pages,
        facts: TreeFacts::default(),
        next: 0,
    };
    for root in &tree.roots {
        walk.walk(root, None, None, false);
    }
    walk.facts
}
