//! What a page deletion does to the interactive form (ISO 32000-2 §12.7): a widget on a deleted
//! page leaves its field's `/Kids`, and a field left with no widget (or a merged field and
//! widget on a deleted page) leaves its parent's `/Kids` or the form's `/Fields`, so form tools
//! don't list a field that no page shows. Signature fields stay, so the signatures a save
//! would invalidate stay visible.

use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{PageReference, ReferenceKind, set_catalog_entry};

use super::cleanup::{ReferenceChange, ReferenceOutcome, edit_dict, edit_object};
use crate::error::Result;

/// Takes the widgets in `references` (all on deleted pages) out of the form.
pub(crate) fn drop_widgets(
    doc: &mut laminapdf_model::Document,
    references: &[PageReference],
) -> Result<Vec<ReferenceChange>> {
    let mut changes = Vec::new();
    for reference in references {
        let ReferenceKind::Widget { widget, field, .. } = &reference.kind else {
            continue;
        };
        if is_signature_field(doc, *field)? {
            // A signature field stays in the form, signed or not: the save planner, validators
            // and viewers find signatures through `/AcroForm /Fields`, and taking a signed one
            // out would hide the signature its save invalidates.
            continue;
        }
        if widget == field {
            unlist_field(doc, *field)?;
        } else {
            drop_from_list(doc, *field, "Kids", *widget)?;
            if !has_kids(doc, *field)? {
                unlist_field(doc, *field)?;
            }
        }
        changes.push(ReferenceChange::new(reference.clone(), ReferenceOutcome::Removed));
    }
    Ok(changes)
}

/// Whether `field` is a signature field: `/FT /Sig` on it or inherited from a parent
/// (ISO 32000-2 §12.7.4.1, §12.7.5.5).
pub(crate) fn is_signature_field(doc: &laminapdf_model::Document, field: ObjRef) -> Result<bool> {
    let mut next = Some(field);
    let mut seen = std::collections::BTreeSet::new();
    while let Some(r) = next {
        if !seen.insert(r.num) || seen.len() > doc.limits().max_tree_depth() {
            break;
        }
        let object = doc.object(r.num)?;
        let Some(dict) = object.as_dict() else { break };
        if let Some(ft) = dict.get_name("FT") {
            return Ok(ft == b"Sig");
        }
        next = dict.get("Parent").and_then(Object::as_ref);
    }
    Ok(false)
}

/// Whether `field` still lists a kid, following an indirect `/Kids` array.
fn has_kids(doc: &laminapdf_model::Document, field: ObjRef) -> Result<bool> {
    let kids = doc.object(field.num)?.as_dict().and_then(|d| d.get("Kids")).cloned();
    Ok(match kids {
        Some(Object::Ref(list)) => doc.object(list.num)?.as_array().is_some_and(|items| !items.is_empty()),
        Some(Object::Array(items)) => !items.is_empty(),
        _ => false,
    })
}

/// Takes `field` out of its parent's `/Kids`, and a parent left with no kids out of its own,
/// up the tree; a field with no parent leaves the form's `/Fields`.
fn unlist_field(doc: &mut laminapdf_model::Document, field: ObjRef) -> Result<()> {
    let mut field = field;
    let mut seen = std::collections::BTreeSet::new();
    while let Some(parent) = doc
        .object(field.num)?
        .as_dict()
        .and_then(|d| d.get("Parent"))
        .and_then(Object::as_ref)
    {
        drop_from_list(doc, parent, "Kids", field)?;
        if has_kids(doc, parent)? || !seen.insert(parent.num) || seen.len() > doc.limits().max_tree_depth() {
            return Ok(());
        }
        field = parent;
    }
    edit_form_fields(doc, false, |items| items.retain(|o| o.as_ref() != Some(field)))
}

/// Puts `field` in its parent's `/Kids`, or in `/AcroForm /Fields` when it has no `parent`:
/// how a new field joins the form (ISO 32000-2 §12.7.4).
pub(crate) fn list_field(doc: &mut laminapdf_model::Document, parent: Option<ObjRef>, field: ObjRef) -> Result<()> {
    match parent {
        Some(parent) => edit_list(doc, parent, "Kids", |items| items.push(Object::Ref(field))),
        None => edit_form_fields(doc, true, |items| items.push(Object::Ref(field))),
    }
}

/// Edits `/AcroForm /Fields` with `f`, wherever the form and the array are stored (direct or
/// indirect). A document without a form is left alone; a form without `/Fields` gets one only
/// when `create` (a field being added), and is left alone otherwise.
fn edit_form_fields(doc: &mut laminapdf_model::Document, create: bool, f: impl FnOnce(&mut Vec<Object>)) -> Result<()> {
    match doc.catalog()?.get("AcroForm").cloned() {
        Some(Object::Ref(form)) => {
            if create
                || doc
                    .object(form.num)?
                    .as_dict()
                    .is_some_and(|d| d.contains_key("Fields"))
            {
                edit_list(doc, form, "Fields", f)
            } else {
                Ok(())
            }
        }
        Some(Object::Dict(mut form)) => match form.get("Fields").cloned() {
            Some(Object::Ref(list)) => edit_array(doc, list, f),
            fields @ (Some(Object::Array(_)) | None) if create || fields.is_some() => {
                let mut items = match fields {
                    Some(Object::Array(items)) => items,
                    _ => Vec::new(),
                };
                f(&mut items);
                form.insert("Fields", Object::Array(items));
                Ok(set_catalog_entry(
                    &mut doc.cos_mut().edit(),
                    "AcroForm",
                    Some(Object::Dict(form)),
                )?)
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

/// Takes `target` out of the array at `key` of the dictionary object `holder`.
fn drop_from_list(doc: &mut laminapdf_model::Document, holder: ObjRef, key: &str, target: ObjRef) -> Result<()> {
    if !doc.object(holder.num)?.as_dict().is_some_and(|d| d.contains_key(key)) {
        return Ok(());
    }
    edit_list(doc, holder, key, |items| items.retain(|o| o.as_ref() != Some(target)))
}

/// Edits with `f` the array at `key` of the dictionary object `holder` (the array may be
/// indirect); a holder without one gets a new array.
fn edit_list(
    doc: &mut laminapdf_model::Document,
    holder: ObjRef,
    key: &str,
    f: impl FnOnce(&mut Vec<Object>),
) -> Result<()> {
    match doc.object(holder.num)?.as_dict().and_then(|d| d.get(key)).cloned() {
        Some(Object::Ref(list)) => edit_array(doc, list, f),
        _ => edit_dict(doc, holder, |d| {
            let mut items = match d.get(key) {
                Some(Object::Array(items)) => items.clone(),
                _ => Vec::new(),
            };
            f(&mut items);
            d.insert(key, Object::Array(items));
        }),
    }
}

/// Edits the array object `list` with `f`.
fn edit_array(doc: &mut laminapdf_model::Document, list: ObjRef, f: impl FnOnce(&mut Vec<Object>)) -> Result<()> {
    edit_object(doc, list, |o| {
        if let Object::Array(items) = o {
            f(items);
        }
    })
}
