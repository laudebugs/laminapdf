//! What a page deletion does to the interactive form (ISO 32000-2 §12.7): a widget on a deleted
//! page leaves its field's `/Kids`, and a field left with no widget (or a merged field and
//! widget on a deleted page) leaves its parent's `/Kids` or the form's `/Fields`, so form tools
//! don't list a field that no page shows.

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
    match doc.catalog()?.get("AcroForm").cloned() {
        Some(Object::Ref(form)) => drop_from_list(doc, form, "Fields", field),
        Some(Object::Dict(mut form)) => {
            let fields = form.get("Fields").cloned();
            match fields {
                Some(Object::Ref(list)) => drop_from_array(doc, list, field),
                Some(Object::Array(mut items)) => {
                    items.retain(|o| o.as_ref() != Some(field));
                    form.insert("Fields", Object::Array(items));
                    Ok(set_catalog_entry(
                        &mut doc.cos_mut().edit(),
                        "AcroForm",
                        Some(Object::Dict(form)),
                    )?)
                }
                _ => Ok(()),
            }
        }
        _ => Ok(()),
    }
}

/// Takes `target` out of the array at `key` of the dictionary object `holder` (the array may
/// be indirect).
fn drop_from_list(doc: &mut laminapdf_model::Document, holder: ObjRef, key: &str, target: ObjRef) -> Result<()> {
    match doc.object(holder.num)?.as_dict().and_then(|d| d.get(key)).cloned() {
        Some(Object::Ref(list)) => drop_from_array(doc, list, target),
        Some(Object::Array(_)) => edit_dict(doc, holder, |d| {
            if let Some(Object::Array(items)) = d.get_mut(key) {
                items.retain(|o| o.as_ref() != Some(target));
            }
        }),
        _ => Ok(()),
    }
}

/// Takes `target` out of the array object `list`.
fn drop_from_array(doc: &mut laminapdf_model::Document, list: ObjRef, target: ObjRef) -> Result<()> {
    edit_object(doc, list, |o| {
        if let Object::Array(items) = o {
            items.retain(|o| o.as_ref() != Some(target));
        }
    })
}
