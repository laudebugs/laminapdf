//! The form-field widgets a page shows (ISO 32000-2 §12.5.6.19, §12.7.4): the widget
//! annotations in its `/Annots`, each with the field it belongs to.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object, Result, tolerant};

use crate::document::Document;
use crate::id::PageId;

/// A widget annotation on a page and the field it belongs to.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct PageWidget {
    /// The widget annotation.
    pub widget: ObjRef,
    /// The field it belongs to: the widget itself when it has a `/T` or a `/V` (a merged field
    /// and widget), else the first field up its `/Parent` chain that has one, since a field in
    /// between may have no partial name (§12.7.4.2).
    pub field: ObjRef,
}

impl Document {
    /// The widgets `page` lists in its `/Annots`, in that order. Reading is tolerant (ADR
    /// 0003): an annotation or field object that doesn't parse is skipped, and a `/Parent`
    /// chain that ends, loops or passes `Limits::max_tree_depth` before a named field gives
    /// the widget's immediate parent.
    ///
    /// # Errors
    /// Unavailable input and hard limits.
    ///
    /// Added in 0.2.0.
    pub fn page_widgets(&self, page: &PageId) -> Result<Vec<PageWidget>> {
        let Some(r) = page.as_lamina_id().obj_ref() else {
            return Ok(Vec::new());
        };
        let Some(page) = dict(self, r)? else {
            return Ok(Vec::new());
        };
        let annots = match page.get("Annots") {
            Some(value) => tolerant(self.deref(value))?.and_then(|v| v.as_array().map(<[Object]>::to_vec)),
            None => None,
        };
        let mut out = Vec::new();
        for annot in annots.unwrap_or_default().iter().filter_map(Object::as_ref) {
            let Some(widget) = dict(self, annot)? else { continue };
            if is_widget(&widget) {
                out.push(PageWidget {
                    widget: annot,
                    field: field_of(self, annot, &widget)?,
                });
            }
        }
        Ok(out)
    }
}

/// A widget annotation: `/Subtype /Widget`, or, as some writers leave the subtype out, an
/// annotation without one that carries a field key.
fn is_widget(dict: &Dict) -> bool {
    dict.get_name("Subtype").map_or_else(
        || dict.contains_key("FT") || dict.contains_key("Parent"),
        |s| s == b"Widget",
    )
}

/// The field widget `widget` (dictionary `dict`) belongs to ([`PageWidget::field`]).
fn field_of(doc: &Document, widget: ObjRef, dict: &Dict) -> Result<ObjRef> {
    let names = |d: &Dict| d.contains_key("T") || d.contains_key("V");
    if names(dict) {
        return Ok(widget);
    }
    let Some(parent) = dict.get("Parent").and_then(Object::as_ref) else {
        return Ok(widget);
    };
    let mut seen = BTreeSet::from([widget.num]);
    let mut next = Some(parent);
    while let Some(r) = next {
        if seen.len() > doc.limits().max_tree_depth() || !seen.insert(r.num) {
            break;
        }
        let Some(d) = self::dict(doc, r)? else { break };
        if names(&d) {
            return Ok(r);
        }
        next = d.get("Parent").and_then(Object::as_ref);
    }
    Ok(parent)
}

/// The dictionary `r` resolves to (§7.3.10: a reference to a free or replaced object is
/// `null`), if it is one and can be read.
fn dict(doc: &Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(tolerant(doc.deref(&Object::Ref(r)))?.and_then(|o| o.as_dict().cloned()))
}
