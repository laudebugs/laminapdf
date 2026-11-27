//! The optional content group of a hideable page mark (ISO 32000-2 §8.11): the group a mark's
//! forms name by `/OC`, so a reader's layers panel can switch the mark off. One group serves a
//! mark on every page it is on; deleting the mark takes the group out of the catalog's
//! `/OCProperties` once no form names it.

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::set_catalog_entry;
use laminapdf_syntax::text::encode_text_string;

use crate::error::Result;

/// How the group's usage says the mark sits on the page: a watermark is foreground, a background
/// is background (`/Usage /PageElement /Subtype`, §8.11.4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Element {
    Foreground,
    Background,
}

/// Writes a group named `name` and lists it in the document's `/OCProperties`: in `/OCGs`, and
/// in the default configuration's `/Order` so a layers panel shows it. The group is on by
/// default.
pub(super) fn add_group(doc: &mut laminapdf_model::Document, name: &str, element: Element) -> Result<ObjRef> {
    let subtype = match element {
        Element::Foreground => "FG",
        Element::Background => "BG",
    };
    let mut page_element = Dict::new();
    page_element.insert("Subtype", Object::name(subtype));
    let mut usage = Dict::new();
    usage.insert("PageElement", page_element);
    let mut group = Dict::new();
    group.insert("Type", Object::name("OCG"));
    group.insert("Name", PdfString::literal(encode_text_string(name)));
    group.insert("Usage", usage);
    let group = doc.cos_mut().edit().create(Object::Dict(group))?;
    edit_properties(doc, |props| {
        let mut groups = array(props.get("OCGs"));
        groups.push(Object::Ref(group));
        props.insert("OCGs", Object::Array(groups));
        let mut config = dict(props.get("D"));
        let mut order = array(config.get("Order"));
        order.push(Object::Ref(group));
        config.insert("Order", Object::Array(order));
        props.insert("D", Object::Dict(config));
    })?;
    Ok(group)
}

/// Takes `group` out of the document's `/OCProperties`, with the whole entry when it was the last
/// group.
pub(super) fn remove_group(doc: &mut laminapdf_model::Document, group: ObjRef) -> Result<()> {
    edit_properties(doc, |props| {
        let drop =
            |items: Vec<Object>| -> Vec<Object> { items.into_iter().filter(|o| o.as_ref() != Some(group)).collect() };
        let groups = drop(array(props.get("OCGs")));
        if groups.is_empty() {
            *props = Dict::new();
            return;
        }
        props.insert("OCGs", Object::Array(groups));
        let mut config = dict(props.get("D"));
        for key in ["Order", "ON", "OFF"] {
            if config.get(key).is_some() {
                config.insert(key, Object::Array(drop(array(config.get(key)))));
            }
        }
        props.insert("D", Object::Dict(config));
    })
}

/// The group `form`'s `/OC` names, if it names one.
pub(super) fn group_of(form: &Dict) -> Option<ObjRef> {
    form.get("OC").and_then(Object::as_ref)
}

/// Changes the catalog's `/OCProperties` through `edit`, which sees it resolved; an empty result
/// removes the entry. A dictionary the catalog holds by reference is changed in place.
fn edit_properties(doc: &mut laminapdf_model::Document, edit: impl FnOnce(&mut Dict)) -> Result<()> {
    let entry = doc.catalog()?.get("OCProperties").cloned();
    let mut props = match &entry {
        Some(e) => doc.deref(e)?.into_owned().as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    // The lists the edit reads may be held by reference: resolve them first.
    for key in ["OCGs", "D"] {
        if let Some(value) = props.get(key).cloned() {
            props.insert(key, doc.deref(&value)?.into_owned());
        }
    }
    if let Some(Object::Dict(config)) = props.get("D").cloned() {
        let mut config = config;
        for key in ["Order", "ON", "OFF"] {
            if let Some(value) = config.get(key).cloned() {
                config.insert(key, doc.deref(&value)?.into_owned());
            }
        }
        props.insert("D", Object::Dict(config));
    }
    edit(&mut props);
    match (entry, props.is_empty()) {
        (Some(Object::Ref(r)), false) => {
            doc.cos_mut().edit().set(r, Object::Dict(props))?;
            Ok(())
        }
        (_, true) => Ok(set_catalog_entry(&mut doc.cos_mut().edit(), "OCProperties", None)?),
        (_, false) => Ok(set_catalog_entry(
            &mut doc.cos_mut().edit(),
            "OCProperties",
            Some(Object::Dict(props)),
        )?),
    }
}

fn array(value: Option<&Object>) -> Vec<Object> {
    match value {
        Some(Object::Array(items)) => items.clone(),
        _ => Vec::new(),
    }
}

fn dict(value: Option<&Object>) -> Dict {
    match value {
        Some(Object::Dict(d)) => d.clone(),
        _ => Dict::new(),
    }
}
