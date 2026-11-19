//! [`FlattenForm`] (E8 plan §2.1): each listed field's widgets drawn into their pages, then the
//! widgets and the fields removed, so the answers stay on the page and can no longer change.

use std::collections::BTreeSet;

use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{AnnotId, FieldId, FieldKind, FieldValue, TerminalField};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::appearance::{Redraw, draws};
use crate::annot::{Appearances, FlattenIntent, draw_widgets, every_annotation, remove_annotations, undrawable};
use crate::blob::BlobStore;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::pages::{edit_form, edit_form_list, unlist_field};
use crate::wire::json::{entries, kind_of};

/// Draws form fields into their pages and removes them (ISO 32000-2 §12.5.5, §12.7).
///
/// For each field, in the order given, each widget's normal appearance (the entry `/AS` names) is
/// drawn where the widget shows it, as [`FlattenAnnots`](crate::FlattenAnnots) draws an
/// annotation; a widget hidden for `intent` is removed without being drawn. A field with a shown
/// widget that has no appearance at all, of a kind no generator draws (a push button, a radio
/// button), stays in place, and [`Applied::not_flattened`](crate::Applied::not_flattened) lists
/// that widget (`flatten-no-appearance`). For the others, a shown widget without an appearance,
/// or every widget while the form's `/NeedAppearances` is true, is drawn afresh first for the
/// field's value, through the document's [`Appearances`]. Then the fields leave the form: from
/// their parents' `/Kids` (a parent left without kids leaves its own parent's, as page deletion
/// does) or `/AcroForm /Fields`, and from `/CO`; their widgets leave their pages, the structure
/// tree and the parent tree.
///
/// A signature field is refused (`field-is-signature`): signing is E10's, and flattening one
/// would break what it signs. When only signature fields are left, the form's `/NeedAppearances`,
/// `/CO` and `/XFA` go too; when other fields are left, `/XFA` still goes, since Acrobat would
/// show an XFA form whose fields no longer exist, and
/// [`Applied::xfa_removed`](crate::Applied::xfa_removed) says so (`xfa-removed`).
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
    #[serde(serialize_with = "field_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub fields: Vec<FieldId>,
    /// Which appearance of the page is kept: on screen, or printed.
    pub intent: FlattenIntent,
}

/// The field IDs written as their string forms (`"f12.0"`); [`FlattenForm`]'s own reader reads
/// them.
fn field_ids<S: serde::Serializer>(ids: &[FieldId], s: S) -> core::result::Result<S::Ok, S::Error> {
    s.collect_seq(ids.iter().map(ToString::to_string))
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
        let mut named: BTreeSet<&FieldId> = BTreeSet::new();
        let mut targets: Vec<&TerminalField> = Vec::new();
        for id in &self.fields {
            let field = form.field(id).ok_or_else(|| Error::UnknownField(id.clone()))?;
            if field.kind == FieldKind::Signature {
                return Err(Error::FieldIsSignature(id.clone()));
            }
            if named.insert(&field.id) {
                targets.push(field);
            }
        }
        // A field whose shown widget has no appearance, and of a kind no generator draws (a
        // push button, a radio button), stays; it is decided before anything is drawn.
        let all = every_annotation(doc)?;
        let mut not_flattened = Vec::new();
        // Each field kept, with whether a shown widget of it has no appearance yet.
        let mut kept: Vec<(&TerminalField, bool)> = Vec::with_capacity(targets.len());
        for field in targets {
            let missing = undrawable(doc, &all, &widget_ids(field), self.intent)?;
            if missing.is_empty() || draws(field.kind) {
                kept.push((field, !missing.is_empty()));
            } else {
                not_flattened.extend(missing);
            }
        }
        if kept.is_empty() {
            return Ok(Effect {
                not_flattened,
                ..Effect::default()
            });
        }
        for &(field, missing) in &kept {
            if missing || form.need_appearances {
                draw_afresh(doc, field, appearances, blobs)?;
            }
        }
        let kept: Vec<&TerminalField> = kept.into_iter().map(|(f, _)| f).collect();
        let widgets: Vec<AnnotId> = kept.iter().flat_map(|f| widget_ids(f)).collect();
        let objects: BTreeSet<ObjRef> = kept
            .iter()
            .flat_map(|f| std::iter::once(&f.id).chain(&f.shared))
            .filter_map(|id| id.as_lamina_id().obj_ref())
            .collect();
        let drawn = draw_widgets(doc, &widgets, self.intent)?;
        // Unlisted while every object still holds its `/Parent`: a field merged with its widget
        // goes with the widget.
        for &field in &objects {
            unlist_field(doc, field)?;
        }
        let all = every_annotation(doc)?;
        let mut pages = drawn.pages;
        for page in remove_annotations(doc, &all, &widgets)? {
            if !pages.contains(&page) {
                pages.push(page);
            }
        }
        for &field in &objects {
            if doc.cos().deref(&Object::Ref(field))?.as_dict().is_some() {
                doc.cos_mut().edit().delete(field)?;
            }
        }
        let xfa_removed = tidy_form(doc, &objects)?;
        Ok(Effect {
            pages,
            removed: true,
            flattened: widgets,
            not_flattened,
            flattened_unembedded: drawn.unembedded,
            flattened_fields: kept.iter().map(|f| (f.id.clone(), f.name.clone())).collect(),
            xfa_removed,
            ..Effect::default()
        })
    }
}

fn widget_ids(field: &TerminalField) -> Vec<AnnotId> {
    field.widgets.iter().map(|w| w.id.clone()).collect()
}

/// Draws `field`'s widgets afresh for its value, so flattening draws what the field holds: when
/// a widget shown for the intent has no appearance, or while `/NeedAppearances` is true. A
/// widget hidden for the intent is removed undrawn, so it asks for nothing.
fn draw_afresh(
    doc: &mut laminapdf_model::Document,
    field: &TerminalField,
    appearances: Option<&dyn Appearances>,
    blobs: &BlobStore,
) -> Result<()> {
    let behaviour = doc.field_behaviour(field)?;
    let value = field.value.clone().unwrap_or(FieldValue::Clear);
    if let Some(redraw) = Redraw::plan(doc, field, &behaviour, &value, &[], appearances, blobs)? {
        redraw.write(doc, &[], appearances, blobs)?;
    }
    Ok(())
}

/// Takes the field objects `gone` out of the form's `/CO`, and removes `/XFA`; with only
/// signature fields left, `/NeedAppearances` and `/CO` too. Whether `/XFA` went while other
/// fields were left.
fn tidy_form(doc: &mut laminapdf_model::Document, gone: &BTreeSet<ObjRef>) -> Result<bool> {
    edit_form_list(doc, "CO", false, |items| {
        items.retain(|o| o.as_ref().is_none_or(|r| !gone.contains(&r)));
    })?;
    let left = doc.form()?;
    let had_xfa = left.has_xfa;
    let only_signatures = left.fields.iter().all(|f| f.kind == FieldKind::Signature);
    let catalog = doc.catalog()?;
    let form = catalog
        .get("AcroForm")
        .and_then(|f| doc.deref(f).ok()?.as_dict().cloned());
    // The form is written again only when it holds something to remove.
    let to_remove = |key: &str| form.as_ref().is_some_and(|f| f.contains_key(key));
    if to_remove("XFA") || (only_signatures && (to_remove("NeedAppearances") || to_remove("CO"))) {
        edit_form(doc, |form| {
            form.remove("XFA");
            if only_signatures {
                form.remove("NeedAppearances");
                form.remove("CO");
            }
        })?;
    }
    Ok(had_xfa && !only_signatures)
}
