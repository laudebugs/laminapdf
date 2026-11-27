//! The optional content group of a hideable page mark (ISO 32000-2 §8.11): the group a mark's
//! forms name by `/OC`, so a reader's layers panel can switch the mark off. One group serves a
//! mark on every page it is on; deleting the mark takes the group out of the catalog's
//! `/OCProperties` once no form names it.
//!
//! Joining the document's `/OCProperties` is the merge's code (`merge_layers`), which keeps a
//! default configuration's name, `/Order` and `/ON`; leaving it is the one function here.

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::set_catalog_entry;
use laminapdf_syntax::text::encode_text_string;

use crate::error::Result;
use crate::pages::{SourceLayers, merge_layers};

/// How the group's usage says the mark sits on the page: a watermark is foreground, a background
/// is background (`/Usage /PageElement /Subtype`, §8.11.4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Element {
    Foreground,
    Background,
}

/// Writes a group named `name` and lists it in the document's `/OCProperties`: in `/OCGs`, in the
/// default configuration's `/Order` so a layers panel shows it, and in its `/ON` list when the
/// configuration's base state is off. The group is on by default.
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
    merge_layers(doc, &SourceLayers::of_group(group))?;
    Ok(group)
}

/// Names `group` `name`, as its mark's text changed.
pub(super) fn rename_group(doc: &mut laminapdf_model::Document, group: ObjRef, name: &str) -> Result<()> {
    let mut cos = doc.cos_mut().edit();
    let Some(current) = cos.get(group)? else {
        return Ok(());
    };
    if let Object::Dict(mut dict) = (*current).clone() {
        dict.insert("Name", PdfString::literal(encode_text_string(name)));
        cos.set(group, Object::Dict(dict))?;
    }
    Ok(())
}

/// Takes `group` out of the document's `/OCProperties`: out of `/OCGs` and out of every
/// configuration's lists (`/Order`, `/ON`, `/OFF`, `/Locked`, `/RBGroups` and the automatic
/// states' `/OCGs`), the default one and those in `/Configs`. The whole entry goes with the last
/// group.
pub(super) fn remove_group(doc: &mut laminapdf_model::Document, group: ObjRef) -> Result<()> {
    let entry = doc.catalog()?.get("OCProperties").cloned();
    let mut props = match &entry {
        Some(e) => doc.deref(e)?.into_owned().as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    let groups: Vec<Object> = resolved_array(doc, props.get("OCGs"))?
        .into_iter()
        .filter(|o| o.as_ref() != Some(group))
        .collect();
    if groups.is_empty() {
        props = Dict::new();
    } else {
        props.insert("OCGs", Object::Array(groups));
        if let Some(d) = props.get("D").cloned() {
            let config = strip_config(doc, &d, group)?;
            props.insert("D", config);
        }
        if let Some(configs) = props.get("Configs").cloned() {
            let mut kept = Vec::new();
            for c in resolved_array(doc, Some(&configs))? {
                kept.push(strip_config(doc, &c, group)?);
            }
            props.insert("Configs", Object::Array(kept));
        }
    }
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

/// The configuration `config` (a dictionary, perhaps by reference) as a direct dictionary without
/// `group` in any of its lists.
fn strip_config(doc: &laminapdf_model::Document, config: &Object, group: ObjRef) -> Result<Object> {
    let Object::Dict(mut dict) = doc.deref(config)?.into_owned() else {
        return Ok(config.clone());
    };
    for key in ["Order", "ON", "OFF", "Locked", "RBGroups"] {
        if let Some(list) = dict.get(key).cloned() {
            dict.insert(key, strip_refs(doc, &list, group)?);
        }
    }
    // Automatic states: each names the groups it switches (`/OCGs`).
    if let Some(states) = dict.get("AS").cloned() {
        let mut kept = Vec::new();
        for state in resolved_array(doc, Some(&states))? {
            let Object::Dict(mut state) = doc.deref(&state)?.into_owned() else {
                continue;
            };
            if let Some(list) = state.get("OCGs").cloned() {
                state.insert("OCGs", strip_refs(doc, &list, group)?);
            }
            kept.push(Object::Dict(state));
        }
        dict.insert("AS", Object::Array(kept));
    }
    Ok(Object::Dict(dict))
}

/// The array `list` (perhaps by reference), nested arrays included, without references to
/// `group`.
fn strip_refs(doc: &laminapdf_model::Document, list: &Object, group: ObjRef) -> Result<Object> {
    let Object::Array(items) = doc.deref(list)?.into_owned() else {
        return Ok(list.clone());
    };
    let mut kept = Vec::new();
    for item in items {
        match &item {
            Object::Ref(r) if *r == group => {}
            Object::Array(_) => kept.push(strip_refs(doc, &item, group)?),
            _ => kept.push(item),
        }
    }
    Ok(Object::Array(kept))
}

/// An array value (perhaps a reference), resolved; empty when absent or not an array.
fn resolved_array(doc: &laminapdf_model::Document, value: Option<&Object>) -> Result<Vec<Object>> {
    let Some(value) = value else { return Ok(Vec::new()) };
    Ok(match doc.deref(value)?.into_owned() {
        Object::Array(items) => items,
        _ => Vec::new(),
    })
}

/// The group `form`'s `/OC` names, if it names one.
pub(super) fn group_of(form: &Dict) -> Option<ObjRef> {
    form.get("OC").and_then(Object::as_ref)
}
