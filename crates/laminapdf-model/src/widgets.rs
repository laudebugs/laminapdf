//! The form-field widgets a page shows (ISO 32000-2 §12.5.6.19, §12.7.4): the widget
//! annotations in its `/Annots`, each with the field it belongs to; and fields' fully qualified
//! names (§12.7.4.2), with the `<partial>#<n>` rule that keeps a copied field's name unique.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object, Result, tolerant};
use laminapdf_syntax::text::decode_text_string;

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

/// Every form field's fully qualified name (ISO 32000-2 §12.7.4.2), and the rule that picks a
/// name no field has: a copy of field `<partial>` is named `<partial>#<n>`, `n` the smallest
/// number from 2 that makes its fully qualified name unique (E2 plan §11). `#` may appear in a
/// partial name; only a period may not.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldNames {
    names: BTreeSet<String>,
}

impl FieldNames {
    /// Whether some field has the fully qualified name `name`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    /// The partial name for a copy of field `partial` under the field whose fully qualified
    /// name is `parent` (`None` for a top-level field): `<partial>#<n>`, `n` the smallest number
    /// from 2 that no field uses. The new name is recorded, so the next call picks another.
    ///
    /// Added in 0.2.0.
    pub fn unique_partial(&mut self, parent: Option<&str>, partial: &str) -> String {
        let full = |p: &str| parent.map_or_else(|| p.to_owned(), |parent| format!("{parent}.{p}"));
        let mut n: u64 = 2;
        loop {
            let candidate = format!("{partial}#{n}");
            if self.names.insert(full(&candidate)) {
                return candidate;
            }
            n += 1;
        }
    }
}

impl Document {
    /// `field`'s fully qualified name: the partial names (`/T`) from the root field down,
    /// joined by periods (ISO 32000-2 §12.7.4.2); a field without a `/T` adds nothing. `None`
    /// when the `/Parent` chain can't be followed to its end: deeper than
    /// `Limits::max_tree_depth`, a loop, or an object that can't be read.
    ///
    /// # Errors
    /// Unavailable input and hard limits.
    ///
    /// Added in 0.2.0.
    pub fn field_name(&self, field: ObjRef) -> Result<Option<String>> {
        let max_depth = self.limits().max_tree_depth();
        let mut parts = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = Some(field);
        while let Some(r) = next {
            if seen.len() >= max_depth || !seen.insert(r.num) {
                return Ok(None);
            }
            let Some(d) = dict(self, r)? else { return Ok(None) };
            if let Some(t) = d.get("T") {
                match tolerant(self.deref(t))?.as_deref() {
                    Some(Object::String(t)) => parts.push(decode_text_string(&t.bytes)),
                    _ => return Ok(None),
                }
            }
            next = d.get("Parent").and_then(Object::as_ref);
        }
        parts.reverse();
        Ok(Some(parts.join(".")))
    }

    /// The fully qualified names of every field the form lists: `/AcroForm /Fields` and their
    /// `/Kids`, down to `Limits::max_tree_depth`, each node once. A node that can't be read
    /// is skipped (ADR 0003).
    ///
    /// # Errors
    /// Unavailable input and hard limits.
    ///
    /// Added in 0.2.0.
    pub fn field_names(&self) -> Result<FieldNames> {
        let mut names = FieldNames::default();
        let Some(form) = self.catalog()?.get("AcroForm").cloned() else {
            return Ok(names);
        };
        let Some(form) = tolerant(self.deref(&form))?.and_then(|o| o.as_dict().cloned()) else {
            return Ok(names);
        };
        let max_depth = self.limits().max_tree_depth();
        let mut seen = BTreeSet::new();
        let mut stack: Vec<(Object, Option<String>, usize)> = kids(self, form.get("Fields"))?
            .into_iter()
            .map(|k| (k, None, 0))
            .collect();
        while let Some((node, prefix, depth)) = stack.pop() {
            let Some(r) = node.as_ref() else { continue };
            if depth >= max_depth || !seen.insert(r.num) {
                continue;
            }
            let Some(d) = dict(self, r)? else { continue };
            let name = match d
                .get("T")
                .map(|t| tolerant(self.deref(t)))
                .transpose()?
                .flatten()
                .as_deref()
            {
                Some(Object::String(t)) => {
                    let partial = decode_text_string(&t.bytes);
                    let full = prefix.map_or_else(|| partial.clone(), |p| format!("{p}.{partial}"));
                    names.names.insert(full.clone());
                    Some(full)
                }
                _ => prefix,
            };
            for kid in kids(self, d.get("Kids"))? {
                stack.push((kid, name.clone(), depth + 1));
            }
        }
        Ok(names)
    }
}

/// The items of the array `value` (or the array it refers to); none when it isn't one.
fn kids(doc: &Document, value: Option<&Object>) -> Result<Vec<Object>> {
    let Some(value) = value else { return Ok(Vec::new()) };
    Ok(tolerant(doc.deref(value))?
        .and_then(|o| o.as_array().map(<[Object]>::to_vec))
        .unwrap_or_default())
}
