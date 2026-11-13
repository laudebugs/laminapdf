//! `DuplicatePages`: copies of pages within the document (E2 plan §2.2), sharing their source's
//! content streams and resources, with their own annotations, fields and structure.

use laminapdf_cos::{Dict, Object};
use laminapdf_model::{PageId, PagePosition};

use super::copies::copy_annotations;
use super::tags::{Pair, clone_tags};
use super::{checked, page_ids, pages_before, relabel};
use crate::error::Result;
use crate::op::Effect;
use crate::wire;

/// Copies `pages` (each once, in the order given) to `at`. A copy shares its source's content
/// streams and resources and shows what it shows, inherited attributes included (ISO 32000-2
/// §7.7.3.4). Its annotations are new objects with a new `/NM`; a form field's widget becomes
/// a new field named `<partial>#<n>` with its own value, and a signature's widget is not copied
/// (both in [`Applied::fields`](crate::Applied::fields)). In a tagged document the copy gets
/// its own `/StructParents` key and clones of the elements whose content lies only on its
/// source; content of elements that span pages is left untagged and the copy is listed in
/// [`Applied::tags_incomplete`](crate::Applied::tags_incomplete). Page labels continue the range
/// each copy lands in.
///
/// On the wire: `{ "op": "duplicatePages", "args": { "pages": ["p7.0"], "at": { "after": "p7.0" } } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DuplicatePages {
    /// The pages to copy.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// Where the copies go.
    #[serde(with = "wire::position")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "\"start\" | \"end\" | { before: string } | { after: string }")
    )]
    pub at: PagePosition,
}

impl DuplicatePages {
    /// The op that copies `pages` to `at`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(pages: Vec<PageId>, at: PagePosition) -> Self {
        Self { pages, at }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let before = pages_before(doc)?;
        let pages = checked(&before, &self.pages)?;
        if pages.is_empty() {
            return Ok(Effect::default());
        }
        let dicts = pages.iter().map(|p| copy_dict(doc, p)).collect::<Result<Vec<_>>>()?;
        let mut copies: Vec<PageId> = Vec::with_capacity(pages.len());
        {
            let mut editor = doc.edit_page_tree()?;
            let mut at = self.at.clone();
            for dict in dicts {
                let id = editor.insert_new(dict, &at)?;
                at = PagePosition::After(id.clone());
                copies.push(id);
            }
            editor.finish()?;
        }
        let mut names = doc.field_names()?;
        let mut fields = Vec::new();
        let mut per_page = Vec::with_capacity(pages.len());
        for (source, copy) in pages.iter().zip(&copies) {
            let (Some(s), Some(c)) = (source.as_lamina_id().obj_ref(), copy.as_lamina_id().obj_ref()) else {
                continue;
            };
            let annots = copy_annotations(doc, source, c, copy, &mut names)?;
            fields.extend(annots.fields);
            per_page.push((s, c, annots.map));
        }
        let pairs: Vec<Pair<'_>> = per_page
            .iter()
            .map(|(source, copy, annots)| Pair {
                source: *source,
                copy: *copy,
                annots,
            })
            .collect();
        let incomplete = clone_tags(doc, &pairs)?;
        let tags_incomplete = copies
            .iter()
            .filter(|c| c.as_lamina_id().obj_ref().is_some_and(|r| incomplete.contains(&r)))
            .cloned()
            .collect();
        let after = page_ids(doc)?;
        relabel(doc, &before, &after)?;
        Ok(Effect {
            pages: copies,
            page_order: Some(after),
            fields,
            tags_incomplete,
            ..Effect::default()
        })
    }
}

/// The keys a copy doesn't take from its source: its place in the tree, what the copy gets
/// anew (annotations, structure key), and article beads, which belong to their thread's pages.
const NOT_COPIED: [&str; 5] = ["Parent", "Annots", "StructParents", "B", "Type"];

/// The dictionary `page`'s copy starts from: its own entries but those of [`NOT_COPIED`], and
/// the attributes it inherits (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`) set on it, so
/// the copy shows the same wherever it lands. Values are copied as they are, so references
/// keep sharing the source's objects.
fn copy_dict(doc: &laminapdf_model::Document, page: &PageId) -> Result<Dict> {
    let pages = doc.pages()?;
    let Some(p) = pages.by_id(page) else {
        return Err(crate::Error::UnknownPage(page.clone()));
    };
    let mut dict = Dict::new();
    for (key, value) in p.dict().iter() {
        if !NOT_COPIED.iter().any(|k| key.0 == k.as_bytes()) {
            dict.insert(key.clone(), value.clone());
        }
    }
    for (key, value) in p.cos().inherited.entries() {
        if let Some(value) = value {
            dict.insert(key, Object::clone(value));
        }
    }
    Ok(dict)
}
