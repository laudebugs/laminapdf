//! A tagged document's logical structure (ISO 32000-2 §14.7), as a reader shows it: each element
//! with its standard type (a custom type followed through the `/RoleMap`), its alternate text,
//! language and actual text, and its kids in reading order, down to the marked content they own
//! on each page. A viewer links that content to the page's text by MCID (a text run's
//! `PageTextRun::mcid`) to give assistive technology the document's headings, paragraphs,
//! lists and tables.

use std::collections::BTreeMap;

use laminapdf_model::{PageId, StructElem, StructKid, StructTree};

use crate::document::Document;
use crate::error::Result;

/// One element of the structure tree.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StructureNode {
    /// Its standard structure type (`H1`, `P`, `L`, `LI`, `Table`, `TD`, `Figure` …): the
    /// element's `/S`, followed through the `/RoleMap` (document text).
    pub kind: String,
    /// `/Alt`: alternate text, a figure's description.
    pub alt: Option<String>,
    /// `/Lang`: the language of its content (a BCP 47 tag).
    pub lang: Option<String>,
    /// `/ActualText`: the text its content stands for.
    pub actual_text: Option<String>,
    /// Its kids, in reading order.
    pub kids: Vec<StructureKid>,
}

/// A kid of a [`StructureNode`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StructureKid {
    /// A child element.
    Node(StructureNode),
    /// Marked content `mcid` on page `page` (0-based index).
    Content {
        /// The page whose content stream holds it.
        page: u32,
        /// Its marked-content ID.
        mcid: i64,
    },
}

impl Document {
    /// The document's structure tree, its top-level elements in reading order; `None` when it
    /// has no `/StructTreeRoot`. Marked content on a page the document doesn't have, or with
    /// no page, is left out, as are object references (`OBJR`: annotations, XObjects), which
    /// carry no text of the page.
    ///
    /// # Errors
    /// Reading errors (the tree, the page tree).
    pub fn structure_tree(&self) -> Result<Option<Vec<StructureNode>>> {
        let Some(tree) = self.ops().structure()? else {
            return Ok(None);
        };
        // One map of the pages, so each marked-content kid finds its page at once.
        let pages: BTreeMap<PageId, u32> = self.page_infos()?.into_iter().map(|p| (p.id, p.index)).collect();
        let index = |id: &PageId| pages.get(id).copied();
        Ok(Some(tree.roots.iter().map(|elem| node(&tree, elem, &index)).collect()))
    }
}

/// `elem` and its kids, with pages as indices.
fn node(tree: &StructTree, elem: &StructElem, index: &impl Fn(&PageId) -> Option<u32>) -> StructureNode {
    let kids = elem
        .kids
        .iter()
        .filter_map(|kid| match kid {
            StructKid::Elem(child) => Some(StructureKid::Node(node(tree, child, index))),
            StructKid::Mcid { page, mcid } => {
                let page = page.as_ref().or(elem.page.as_ref()).and_then(index)?;
                Some(StructureKid::Content { page, mcid: *mcid })
            }
            _ => None,
        })
        .collect();
    StructureNode {
        kind: String::from_utf8_lossy(tree.standard_kind(&elem.kind)).into_owned(),
        alt: elem.alt.clone(),
        lang: elem.lang.clone(),
        actual_text: elem.actual_text.clone(),
        kids,
    }
}
