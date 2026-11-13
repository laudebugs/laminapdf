//! What duplicating a page does to its annotations and form fields (E2 plan §2.2): each
//! annotation becomes a new object on the copy, with a new `/NM` (ISO 32000-2 §12.5.2) and its
//! popup, its parent and the annotation it replies to remapped when they were copied too. A
//! widget never becomes a second widget of its field, which would tie the copy's value to the
//! original's: a widget merged with its field is copied with a new partial name, and the widgets
//! of a field with kids get one new field per copied page, named `<partial>#<n>` (§12.7.4.2,
//! [`FieldNames::unique_partial`]). Signature widgets are not copied.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{FieldId, FieldNames, PageId};
use laminapdf_syntax::text::{decode_text_string, encode_text_string};

use super::forms::{is_signature_field, list_field};
use crate::error::Result;

/// What duplicating a page did to one form field whose widget the page shows.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldChange {
    /// The field on the source page.
    pub field: FieldId,
    /// The copy whose widget it concerns.
    pub page: PageId,
    /// What happened.
    pub outcome: FieldOutcome,
}

/// What happened to a field when the page showing its widget was duplicated.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldOutcome {
    /// The copy's widget belongs to a new field, `to`, with its own value, named `name` (its
    /// fully qualified name, ending in `#<n>`).
    Renamed {
        /// The new field.
        to: FieldId,
        /// Its fully qualified name.
        name: String,
    },
    /// A signature field's widget: not copied, since a signature signs one place in one file.
    NotCopied,
}

/// The annotations a page's copy got: each copied annotation's new object by its source, and
/// what happened to the fields whose widgets the page shows.
#[derive(Debug, Default)]
pub(crate) struct CopiedAnnots {
    pub(crate) map: BTreeMap<ObjRef, ObjRef>,
    pub(crate) fields: Vec<FieldChange>,
}

/// The keys of a field dictionary that belong to the field, not its place in the tree.
const NOT_COPIED_FROM_FIELD: [&str; 3] = ["Kids", "Parent", "T"];

/// Copies `source`'s annotations (`annots`, the references its `/Annots` lists: an annotation
/// dictionary stored directly in the array has no object to copy, and stays off the copy) onto
/// its copy `copy` (`copy_id`), naming new fields from `names`; returns what it copied. The
/// copy's `/Annots` is written as a new direct array.
pub(crate) fn copy_annotations(
    doc: &mut laminapdf_model::Document,
    source: &PageId,
    annots: &[ObjRef],
    copy: ObjRef,
    copy_id: &PageId,
    names: &mut FieldNames,
) -> Result<CopiedAnnots> {
    if annots.is_empty() {
        return Ok(CopiedAnnots::default());
    }
    let mut out = CopiedAnnots::default();
    // Each widget's field, and the fields not copied (signatures), decided first.
    let mut field_of: BTreeMap<ObjRef, ObjRef> = BTreeMap::new();
    let mut skipped: BTreeSet<ObjRef> = BTreeSet::new();
    let mut skipped_widgets: BTreeSet<ObjRef> = BTreeSet::new();
    for w in doc.page_widgets(source)? {
        if is_signature_field(doc, w.field)? {
            skipped_widgets.insert(w.widget);
            if skipped.insert(w.field) {
                out.fields.push(FieldChange {
                    field: FieldId::object(w.field),
                    page: copy_id.clone(),
                    outcome: FieldOutcome::NotCopied,
                });
            }
            continue;
        }
        field_of.insert(w.widget, w.field);
    }
    for &annot in annots {
        if !skipped_widgets.contains(&annot)
            && !out.map.contains_key(&annot)
            && doc.object(annot.num)?.as_dict().is_some()
        {
            out.map.insert(annot, doc.reserve_number()?);
        }
    }
    let new_fields = fields_for(doc, &field_of, &out.map, copy_id, names, &mut out.fields)?;
    let mut list = Vec::with_capacity(annots.len());
    for old in annots {
        let Some(&new) = out.map.get(old) else { continue };
        let Some(mut dict) = doc.object(old.num)?.as_dict().cloned() else {
            continue;
        };
        rewrite(&mut dict, copy, new, &out.map);
        if let Some(&field) = field_of.get(old) {
            match new_fields.get(&field) {
                Some(NewField::Merged { partial }) => {
                    dict.insert("T", text(partial));
                }
                Some(NewField::Kid { field: new_field }) => {
                    dict.insert("Parent", Object::Ref(*new_field));
                }
                None => {}
            }
        }
        doc.cos_mut().edit().set(new, Object::Dict(dict))?;
        list.push(Object::Ref(new));
    }
    super::cleanup::edit_dict(doc, copy, |d| {
        d.insert("Annots", Object::Array(list));
    })?;
    Ok(out)
}

/// What a copied widget's field became.
enum NewField {
    /// The widget is its own field: its copy gets this partial name.
    Merged { partial: String },
    /// The widget is a kid of a field: its copy is a kid of this new field.
    Kid { field: ObjRef },
}

/// Makes the new fields for the copied widgets (one per source field, shared by its widgets
/// on the page, so a radio group stays one group), lists them in the form, and reports them.
fn fields_for(
    doc: &mut laminapdf_model::Document,
    field_of: &BTreeMap<ObjRef, ObjRef>,
    copied: &BTreeMap<ObjRef, ObjRef>,
    copy_id: &PageId,
    names: &mut FieldNames,
    report: &mut Vec<FieldChange>,
) -> Result<BTreeMap<ObjRef, NewField>> {
    let mut out: BTreeMap<ObjRef, NewField> = BTreeMap::new();
    let mut kids: BTreeMap<ObjRef, Vec<Object>> = BTreeMap::new();
    for (&widget, &field) in field_of {
        let Some(&new_widget) = copied.get(&widget) else {
            continue;
        };
        if out.contains_key(&field) {
            kids.entry(field).or_default().push(Object::Ref(new_widget));
            continue;
        }
        let Some(dict) = doc.object(field.num)?.as_dict().cloned() else {
            continue;
        };
        let parent = dict.get("Parent").and_then(Object::as_ref);
        let parent_name = match parent {
            Some(p) => doc.field_name(p)?,
            None => None,
        };
        let partial = match dict.get("T").map(|t| doc.deref(t)).transpose()?.as_deref() {
            Some(Object::String(t)) => decode_text_string(&t.bytes),
            _ => String::new(),
        };
        let partial = names.unique_partial(parent_name.as_deref(), &partial);
        let full = parent_name.map_or_else(|| partial.clone(), |p| format!("{p}.{partial}"));
        let new_field = if widget == field {
            out.insert(field, NewField::Merged { partial });
            new_widget
        } else {
            let mut new = Dict::new();
            for (key, value) in dict.iter() {
                if !NOT_COPIED_FROM_FIELD.iter().any(|k| key.0 == k.as_bytes()) {
                    new.insert(key.clone(), value.clone());
                }
            }
            new.insert("T", text(&partial));
            if let Some(p) = parent {
                new.insert("Parent", Object::Ref(p));
            }
            let r = doc.reserve_number()?;
            doc.cos_mut().edit().set(r, Object::Dict(new))?;
            out.insert(field, NewField::Kid { field: r });
            kids.entry(field).or_default().push(Object::Ref(new_widget));
            r
        };
        list_field(doc, parent, new_field)?;
        report.push(FieldChange {
            field: FieldId::object(field),
            page: copy_id.clone(),
            outcome: FieldOutcome::Renamed {
                to: FieldId::object(new_field),
                name: full,
            },
        });
    }
    for (field, widgets) in kids {
        if let Some(NewField::Kid { field: new }) = out.get(&field) {
            super::cleanup::edit_dict(doc, *new, |d| {
                d.insert("Kids", Object::Array(widgets));
            })?;
        }
    }
    Ok(out)
}

/// Points annotation `dict` (copied as `new`) at the copy page `page`, its popup, parent and
/// reply target at their copies when they were copied, and gives it a new `/NM`; its
/// `/StructParent` goes, since the copy gets its own entry or none (the structure step).
fn rewrite(dict: &mut Dict, page: ObjRef, new: ObjRef, copied: &BTreeMap<ObjRef, ObjRef>) {
    dict.insert("P", Object::Ref(page));
    for key in ["Popup", "Parent", "IRT"] {
        if let Some(Object::Ref(r)) = dict.get(key)
            && let Some(&to) = copied.get(r)
        {
            dict.insert(key, Object::Ref(to));
        }
    }
    if let Some(Object::String(nm)) = dict.get("NM") {
        let name = format!("{}-{}", decode_text_string(&nm.bytes), new.num);
        dict.insert("NM", text(&name));
    }
    dict.remove("StructParent");
}

/// `s` as a text string (PDFDocEncoding, else UTF-16BE).
fn text(s: &str) -> Object {
    Object::String(PdfString::literal(encode_text_string(s)))
}
