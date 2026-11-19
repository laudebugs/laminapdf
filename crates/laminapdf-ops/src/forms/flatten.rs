//! [`FlattenForm`] (E8 plan §2.1): each listed field's widgets drawn into their pages, then the
//! widgets and the fields removed, so the answers stay on the page and can no longer change.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{AnnotId, FieldId, FieldKind, FieldValue, PageId, TerminalField};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::appearance::{Redraw, acro_form};
use crate::annot::{
    Appearances, FlattenIntent, draw_widgets, every_annotation, object_dict, parent_tree, undrawable, unlink, untag,
};
use crate::blob::BlobStore;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;
use crate::wire::json::{entries, kind_of};

/// Draws form fields into their pages and removes them (ISO 32000-2 §12.5.5, §12.7).
///
/// For each field, in the order given, each widget's normal appearance (the entry `/AS` names) is
/// drawn where the widget shows it, as [`FlattenAnnots`](crate::FlattenAnnots) draws an
/// annotation; a widget hidden for `intent` is removed without being drawn. A widget without an
/// appearance, or every widget while the form's `/NeedAppearances` is true, is drawn afresh first
/// for the field's value, through the document's [`Appearances`]. A field with a widget that still
/// has nothing to show (a push button or radio button without appearances) stays in place, and
/// [`Applied::not_flattened`](crate::Applied::not_flattened) lists that widget
/// (`flatten-no-appearance`). Then the widgets leave their
/// pages and the structure tree, and the fields leave the form: from their parents' `/Kids` (a
/// parent left without kids goes too) or `/AcroForm /Fields`, and from `/CO`.
///
/// A signature field is refused (`field-is-signature`): signing is E10's, and flattening one
/// would break what it signs. When only signature fields are left, the form's `/NeedAppearances`,
/// `/CO` and `/XFA` go too; when other fields are left, `/XFA` still goes, since Acrobat would
/// show an XFA form whose fields no longer exist (the façade reports that as `xfa-removed`).
///
/// In a tagged document the drawing is marked as an artifact and each widget's place in the
/// structure tree goes, as for annotations. The op is class `Content`: a certification refuses
/// it at every level, and a lock on any field it names is a violation. The fields stay in the
/// file's earlier revisions until a full rewrite
/// ([`Applied::still_in_earlier_revisions`](crate::Applied::still_in_earlier_revisions)).
///
/// On the wire: `{ "op": "flattenForm", "args": { "fields": ["f10.0", "f12.0"], "intent": "view" } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct FlattenForm {
    /// The fields: terminal fields of the form, by their IDs.
    #[serde(serialize_with = "wire::field_ids::serialize")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub fields: Vec<FieldId>,
    /// Which appearance of the page is kept: on screen, or printed.
    pub intent: FlattenIntent,
}

impl<'de> Deserialize<'de> for FlattenForm {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        Self::from_value(Value::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl FlattenForm {
    /// The op that flattens `fields` for `intent`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(fields: Vec<FieldId>, intent: FlattenIntent) -> Self {
        Self { fields, intent }
    }

    /// The op from its wire arguments.
    fn from_value(value: Value) -> core::result::Result<Self, String> {
        let mut map = entries(value, "the arguments of flattenForm", &["fields", "intent"])?;
        let fields = match map.remove("fields") {
            Some(Value::Array(ids)) => ids
                .into_iter()
                .map(|id| match id {
                    Value::String(id) => id.parse().map_err(|e: laminapdf_model::ParseIdError| e.to_string()),
                    other => Err(format!("invalid type: {}, expected a field ID", kind_of(&other))),
                })
                .collect::<core::result::Result<_, _>>()?,
            Some(other) => {
                return Err(format!(
                    "invalid type: {}, expected an array of field IDs",
                    kind_of(&other)
                ));
            }
            None => return Err("missing field `fields`".to_owned()),
        };
        let intent = match map.remove("intent") {
            Some(intent) => serde_json::from_value(intent).map_err(|e| e.to_string())?,
            None => return Err("missing field `intent`".to_owned()),
        };
        Ok(Self::new(fields, intent))
    }

    /// Applies it.
    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Effect> {
        let form = doc.form()?;
        let mut targets: Vec<TerminalField> = Vec::new();
        for id in &self.fields {
            let field = form.field(id).ok_or_else(|| Error::UnknownField(id.clone()))?;
            if field.kind == FieldKind::Signature {
                return Err(Error::FieldIsSignature(id.clone()));
            }
            if !targets.iter().any(|t| t.id == field.id) {
                targets.push(field.clone());
            }
        }
        for field in &targets {
            draw_afresh(doc, field, form.need_appearances, appearances, blobs)?;
        }
        let mut not_flattened = Vec::new();
        let mut kept = Vec::with_capacity(targets.len());
        for field in targets {
            let ids: Vec<AnnotId> = field.widgets.iter().map(|w| w.id.clone()).collect();
            let missing = undrawable(doc, &ids, self.intent)?;
            if missing.is_empty() {
                kept.push(field);
            } else {
                not_flattened.extend(missing);
            }
        }
        let targets = kept;
        if targets.is_empty() {
            return Ok(Effect {
                not_flattened,
                ..Effect::default()
            });
        }
        let widgets: Vec<AnnotId> = targets
            .iter()
            .flat_map(|f| f.widgets.iter().map(|w| w.id.clone()))
            .collect();
        let gone: BTreeSet<ObjRef> = targets
            .iter()
            .flat_map(|f| std::iter::once(&f.id).chain(&f.shared))
            .filter_map(|id| id.as_lamina_id().obj_ref())
            .collect();
        // Read before the widgets go: a field merged with its widget is deleted with it.
        let mut parents = Vec::with_capacity(gone.len());
        for &field in &gone {
            parents.push((field, parent_of(doc, field)?));
        }
        let drawn = draw_widgets(doc, &widgets, self.intent)?;
        let mut pages = drawn.pages;
        for page in remove_widgets(doc, &widgets)? {
            if !pages.contains(&page) {
                pages.push(page);
            }
        }
        for (field, parent) in parents {
            detach(doc, field, parent, MAX_DEPTH)?;
        }
        tidy_form(doc, &gone)?;
        Ok(Effect {
            pages,
            removed: true,
            flattened: widgets,
            not_flattened,
            flattened_unembedded: drawn.unembedded,
            ..Effect::default()
        })
    }
}

/// How far up `/Parent` an emptied parent is removed: the form view's own depth limit.
const MAX_DEPTH: usize = 64;

/// Draws `field`'s widgets afresh for its value where one has no appearance, or every one while
/// `/NeedAppearances` is true, so flattening draws what the field holds.
fn draw_afresh(
    doc: &mut laminapdf_model::Document,
    field: &TerminalField,
    need_appearances: bool,
    appearances: Option<&dyn Appearances>,
    blobs: &BlobStore,
) -> Result<()> {
    let mut missing = false;
    for w in &field.widgets {
        if let Some(r) = w.id.as_lamina_id().obj_ref()
            && let Some(dict) = object_dict(doc, r)?
        {
            missing |= doc.normal_appearance(&dict)?.is_none();
        }
    }
    if !missing && !need_appearances {
        return Ok(());
    }
    let behaviour = doc.field_behaviour(field)?;
    let value = field.value.clone().unwrap_or(FieldValue::Clear);
    if let Some(redraw) = Redraw::plan(doc, field, &behaviour, &value, &[], appearances, blobs)? {
        redraw.write(doc, &[], appearances, blobs)?;
    }
    Ok(())
}

/// Takes `widgets` off their pages and out of the structure tree, and deletes them; the pages
/// they were on.
fn remove_widgets(doc: &mut laminapdf_model::Document, widgets: &[AnnotId]) -> Result<Vec<PageId>> {
    let all = every_annotation(doc)?;
    let tree = parent_tree(doc)?;
    let mut pages = Vec::new();
    let mut arrays = Vec::new();
    for (page, annots) in &all {
        let gone: Vec<usize> = annots
            .iter()
            .filter(|a| widgets.contains(&a.id))
            .map(|a| a.index)
            .collect();
        if !gone.is_empty() {
            unlink(doc, page, &gone, &mut arrays)?;
            pages.push(page.clone());
        }
    }
    for r in widgets.iter().filter_map(|id| id.as_lamina_id().obj_ref()) {
        untag(doc, tree.as_ref(), r)?;
        doc.cos_mut().edit().delete(r)?;
    }
    Ok(pages)
}

/// Field object `field`'s `/Parent`.
fn parent_of(doc: &laminapdf_model::Document, field: ObjRef) -> Result<Option<ObjRef>> {
    Ok(object_dict(doc, field)?.and_then(|d| d.get("Parent").and_then(Object::as_ref)))
}

/// Takes field object `field` out of `parent`'s `/Kids`, or the form's `/Fields` when it has no
/// parent, and deletes it; a parent left without kids goes the same way, up to `depth` parents.
fn detach(doc: &mut laminapdf_model::Document, field: ObjRef, parent: Option<ObjRef>, depth: usize) -> Result<()> {
    match parent {
        Some(parent) => {
            let Some(mut dict) = object_dict(doc, parent)? else {
                return delete_object(doc, field);
            };
            let left = without(doc, &mut dict, "Kids", field)?;
            doc.cos_mut().edit().set(parent, Object::Dict(dict))?;
            if left == 0 && depth > 0 {
                let grandparent = parent_of(doc, parent)?;
                detach(doc, parent, grandparent, depth - 1)?;
            }
        }
        None => {
            if let Some((at, mut dict)) = form_object(doc)? {
                without(doc, &mut dict, "Fields", field)?;
                at.write(doc, dict)?;
            }
        }
    }
    delete_object(doc, field)
}

/// Deletes object `r`, which a widget's deletion may already have taken.
fn delete_object(doc: &mut laminapdf_model::Document, r: ObjRef) -> Result<()> {
    if object_dict(doc, r)?.is_some() {
        doc.cos_mut().edit().delete(r)?;
    }
    Ok(())
}

/// `dict`'s array `key` without `r`, written back where it lives (in `dict`, or its own object);
/// how many entries are left.
fn without(doc: &mut laminapdf_model::Document, dict: &mut Dict, key: &str, r: ObjRef) -> Result<usize> {
    let Some(entry) = dict.get(key).cloned() else {
        return Ok(0);
    };
    let items = laminapdf_cos::tolerant(doc.deref(&entry))?
        .and_then(|o| o.as_array().map(<[Object]>::to_vec))
        .unwrap_or_default();
    let kept: Vec<Object> = items.into_iter().filter(|o| o.as_ref() != Some(r)).collect();
    let left = kept.len();
    match entry {
        Object::Ref(array) => doc.cos_mut().edit().set(array, Object::Array(kept))?,
        _ => dict.insert(key, Object::Array(kept)),
    }
    Ok(left)
}

/// Where the form dictionary lives: its own object, or directly in the catalog.
enum FormAt {
    Object(ObjRef),
    Catalog(ObjRef),
}

impl FormAt {
    /// Writes the form dictionary back where it lives.
    fn write(&self, doc: &mut laminapdf_model::Document, form: Dict) -> Result<()> {
        match *self {
            Self::Object(r) => doc.cos_mut().edit().set(r, Object::Dict(form))?,
            Self::Catalog(root) => {
                if let Some(mut catalog) = object_dict(doc, root)? {
                    catalog.insert("AcroForm", Object::Dict(form));
                    doc.cos_mut().edit().set(root, Object::Dict(catalog))?;
                }
            }
        }
        Ok(())
    }
}

/// The form dictionary and where it lives; none for a document without a form.
fn form_object(doc: &laminapdf_model::Document) -> Result<Option<(FormAt, Dict)>> {
    let Some(root) = doc.cos().trailer().get("Root").and_then(Object::as_ref) else {
        return Ok(None);
    };
    let Some(form) = acro_form(doc)? else { return Ok(None) };
    let at = match object_dict(doc, root)?.and_then(|c| c.get("AcroForm").and_then(Object::as_ref)) {
        Some(r) => FormAt::Object(r),
        None => FormAt::Catalog(root),
    };
    Ok(Some((at, form)))
}

/// Takes the fields `gone` out of `/CO`; with only signature fields left, `/NeedAppearances`,
/// `/CO` and `/XFA` go too, and with others left `/XFA` goes still.
fn tidy_form(doc: &mut laminapdf_model::Document, gone: &BTreeSet<ObjRef>) -> Result<()> {
    let Some((at, mut form)) = form_object(doc)? else {
        return Ok(());
    };
    for &field in gone {
        without(doc, &mut form, "CO", field)?;
    }
    form.remove("XFA");
    let only_signatures = doc.form()?.fields.iter().all(|f| f.kind == FieldKind::Signature);
    if only_signatures {
        form.remove("NeedAppearances");
        form.remove("CO");
    }
    at.write(doc, form)?;
    Ok(())
}
