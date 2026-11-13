//! Which form fields have widgets on which pages, for the planner's field-level checks of page
//! ops ([E2 plan §6]): each page's `/Annots` widgets (ISO 32000-2 §12.5.6.19), the terminal
//! field each belongs to (the widget itself when it has a `/T`, else its `/Parent`, §12.7.4),
//! and that field's fully qualified name (§12.7.4.2), which a `FieldMDP` lock's `/Fields` names.
//!
//! Reading is tolerant (ADR 0003): an annotation or field object that doesn't parse is skipped,
//! and a name that can't be read whole is unknown, which every lock covers. Unavailable input
//! and hard limits still fail.
//!
//! [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object, tolerant};
use laminapdf_model::{FieldId, PageId};
use laminapdf_syntax::text::decode_text_string;

use crate::error::Result;

/// The widgets of every page, by terminal field, with each field's fully qualified name.
#[derive(Debug, Clone, Default)]
pub(crate) struct WidgetIndex {
    pages: BTreeMap<PageId, BTreeSet<FieldId>>,
    /// `None` for a name that couldn't be read whole.
    names: BTreeMap<FieldId, Option<String>>,
}

impl WidgetIndex {
    /// Reads every page's widgets in `doc`.
    ///
    /// # Errors
    /// Reading the page tree; unavailable input and hard limits reading an `/Annots` array, a
    /// widget or its field chain.
    pub(crate) fn new(doc: &laminapdf_model::Document) -> Result<Self> {
        let mut index = Self::default();
        for page in doc.pages()?.iter() {
            for w in doc.page_widgets(page.id())? {
                let id = FieldId::object(w.field);
                if !index.names.contains_key(&id) {
                    let name = qualified_name(doc, w.field)?;
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

    /// `field`'s fully qualified name; `None` when it couldn't be read whole.
    pub(crate) fn name(&self, field: &FieldId) -> Option<&str> {
        self.names.get(field).and_then(Option::as_deref)
    }
}

/// The dictionary `r` resolves to (§7.3.10: a reference to a free or replaced object is
/// `null`), if it is one and can be read.
fn dict(doc: &laminapdf_model::Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(tolerant(doc.deref(&Object::Ref(r)))?.and_then(|o| o.as_dict().cloned()))
}

/// `field`'s fully qualified name: the partial names (`/T`) from the root field down, joined
/// by periods (§12.7.4.2); a field without a `/T` adds nothing. `None` when the `/Parent`
/// chain can't be followed to its end: deeper than `Limits::max_tree_depth`, a loop, or an
/// object that can't be read.
///
/// # Errors
/// Unavailable input and hard limits.
pub(crate) fn qualified_name(doc: &laminapdf_model::Document, field: ObjRef) -> Result<Option<String>> {
    let max_depth = doc.limits().max_tree_depth();
    let mut parts = Vec::new();
    let mut seen = BTreeSet::new();
    let mut next = Some(field);
    while let Some(r) = next {
        if seen.len() >= max_depth || !seen.insert(r.num) {
            return Ok(None);
        }
        let Some(d) = dict(doc, r)? else { return Ok(None) };
        if let Some(t) = d.get("T") {
            match tolerant(doc.deref(t))?.as_deref() {
                Some(Object::String(t)) => parts.push(decode_text_string(&t.bytes)),
                _ => return Ok(None),
            }
        }
        next = d.get("Parent").and_then(Object::as_ref);
    }
    parts.reverse();
    Ok(Some(parts.join(".")))
}
