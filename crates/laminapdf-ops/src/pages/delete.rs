//! `DeletePages`: pages out of the page tree (ISO 32000-2 §7.7.3), with what named them
//! cleaned up (E2 plan §11).

use laminapdf_model::PageId;

use super::{checked, cleanup, page_ids, pages_before, relabel, structure, widgets_on};
use crate::class::ModificationClass;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::widgets::WidgetChange;
use crate::wire;

/// What [`DeletePages`] does with a destination that names a deleted page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum ReferencePolicy {
    /// Point it at the nearest surviving page (the next one, else the one before) with the view
    /// `/Fit`, so the outline item, link or named destination keeps working.
    #[default]
    Retarget,
    /// Remove it: an outline item loses its destination, a link annotation leaves its page,
    /// a named destination and `/OpenAction` are removed.
    Remove,
}

/// Deletes `pages` (each once; unknown pages are an error). Outline items, links and `GoTo`
/// actions on surviving pages, named destinations and `/OpenAction` that named a deleted page
/// follow `references` ([`ReferencePolicy`]); structure elements whose content was all on
/// deleted pages are removed with their `/ParentTree` entries. The page objects stay in the
/// file's earlier revisions. Deleting every page is [`Error::NoPagesLeft`].
///
/// On the wire: `{ "op": "deletePages", "args": { "pages": ["p7.0"], "references": "retarget" } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeletePages {
    /// The pages.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// What happens to destinations that name them; `retarget` when left out.
    #[serde(default)]
    pub references: ReferencePolicy,
}

impl DeletePages {
    /// The op that deletes `pages`, retargeting what named them.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(pages: Vec<PageId>) -> Self {
        Self {
            pages,
            references: ReferencePolicy::Retarget,
        }
    }

    /// With `references` deciding what happens to destinations that name the pages.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_references(mut self, references: ReferencePolicy) -> Self {
        self.references = references;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let before = pages_before(doc)?;
        let pages = checked(&before, &self.pages)?;
        if pages.is_empty() {
            return Ok(Effect::default());
        }
        if pages.len() == before.len() {
            return Err(Error::NoPagesLeft);
        }
        let references = doc.page_references(&pages)?.entries().to_vec();
        let widgets = widgets_on(doc, &pages, WidgetChange::Deleted, ModificationClass::PageStructure)?;
        let parent_keys = structure::parent_tree_keys(doc, &pages)?;
        {
            let mut editor = doc.edit_page_tree()?;
            for page in &pages {
                editor.remove(page)?;
            }
            editor.finish()?;
        }
        let mut changes = cleanup::destinations(doc, &references, &before, &pages, self.references)?;
        changes.extend(structure::clean(doc, &references, &pages, &parent_keys)?);
        let after = page_ids(doc)?;
        relabel(doc, &before, &after)?;
        Ok(Effect {
            pages,
            page_order: Some(after),
            references: changes,
            widgets,
            removed: true,
            ..Effect::default()
        })
    }
}
