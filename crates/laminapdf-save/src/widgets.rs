//! Which form fields have widgets on which pages, for the planner's field-level checks of page
//! ops ([E2 plan §6]): each page's `/Annots` widgets (ISO 32000-2 §12.5.6.19), the terminal
//! field each belongs to (the widget itself when it has a `/T`, else its `/Parent`, §12.7.4),
//! and that field's fully qualified name (§12.7.4.2), which a `FieldMDP` lock's `/Fields` names.
//!
//! [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{FieldId, PageId};
use laminapdf_syntax::text::decode_text_string;

use crate::error::Result;

/// The widgets of every page, by terminal field, with each field's fully qualified name.
#[derive(Debug, Clone, Default)]
pub(crate) struct WidgetIndex {
    pages: BTreeMap<PageId, BTreeSet<FieldId>>,
    names: BTreeMap<FieldId, String>,
}

impl WidgetIndex {
    /// Reads every page's widgets in `doc`.
    ///
    /// # Errors
    /// Reading the page tree, an `/Annots` array or a widget's field chain.
    pub(crate) fn new(doc: &laminapdf_model::Document) -> Result<Self> {
        let mut index = Self::default();
        let max_depth = doc.limits().max_tree_depth();
        for page in doc.pages()?.iter() {
            for annot in page.annots()? {
                let Some(widget) = dict(doc, annot)? else { continue };
                // Some writers leave a widget's `/Subtype` out; a field key marks it then.
                let is_widget = widget.get_name("Subtype").map_or_else(
                    || widget.contains_key("FT") || widget.contains_key("Parent"),
                    |s| s == b"Widget",
                );
                if !is_widget {
                    continue;
                }
                let field = if widget.contains_key("T") {
                    annot
                } else {
                    match widget.get("Parent").and_then(Object::as_ref) {
                        Some(parent) => parent,
                        None => annot,
                    }
                };
                let id = FieldId::object(field);
                if !index.names.contains_key(&id) {
                    let name = qualified_name(doc, field, max_depth)?;
                    index.names.insert(id.clone(), name);
                }
                index.pages.entry(page.id().clone()).or_default().insert(id);
            }
        }
        Ok(index)
    }

    /// The fields with a widget on `page`.
    pub(crate) fn on_page(&self, page: &PageId) -> impl Iterator<Item = &FieldId> + '_ {
        self.pages.get(page).into_iter().flatten()
    }

    /// `field`'s fully qualified name, when one of its widgets is on a page.
    pub(crate) fn name(&self, field: &FieldId) -> Option<&str> {
        self.names.get(field).map(String::as_str)
    }
}

/// The dictionary of `r`, if it is one.
fn dict(doc: &laminapdf_model::Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(doc.object(r.num)?.as_dict().cloned())
}

/// The partial names (`/T`) from the root field down to `field`, joined by periods (§12.7.4.2);
/// a field without a `/T` adds nothing. The chain stops at `max_depth` or a loop.
fn qualified_name(doc: &laminapdf_model::Document, field: ObjRef, max_depth: usize) -> Result<String> {
    let mut parts = Vec::new();
    let mut seen = BTreeSet::new();
    let mut next = Some(field);
    while let Some(r) = next {
        if parts.len() > max_depth || !seen.insert(r.num) {
            break;
        }
        let Some(d) = dict(doc, r)? else { break };
        if let Some(Object::String(t)) = d.get("T") {
            parts.push(decode_text_string(&t.bytes));
        }
        next = d.get("Parent").and_then(Object::as_ref);
    }
    parts.reverse();
    Ok(parts.join("."))
}
