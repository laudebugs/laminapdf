//! Whether optional content is off as the document opens (ISO 32000-2 §8.11): an optional
//! content group against the default configuration's `/BaseState`, `/ON` and `/OFF`
//! (§8.11.4.3), and a membership dictionary by its `/OCGs` and `/P` policy (§8.11.2.2). A
//! visibility expression (`/VE`) and the usage application dictionaries aren't evaluated: the
//! membership's `/P` decides, as a reader without them does. Anything that can't be read counts
//! as on, so flattening never drops what a viewer shows.

use laminapdf_cos::{Dict, ObjRef, Object};

use crate::error::Result;

/// Whether the content that `/OC` value `oc` marks is hidden as the document opens.
pub(super) fn is_off(doc: &laminapdf_model::Document, oc: &Object) -> Result<bool> {
    let Some(config) = default_config(doc)? else {
        return Ok(false);
    };
    let Some(target) = oc.as_ref() else {
        return Ok(false);
    };
    let Some(dict) = laminapdf_cos::tolerant(doc.cos().resolve(target))?
        .as_deref()
        .and_then(Object::as_dict)
        .cloned()
    else {
        return Ok(false);
    };
    if dict.get_name("Type") != Some(b"OCMD".as_slice()) {
        return Ok(!config.on(target));
    }
    let groups: Vec<ObjRef> = match dict.get("OCGs") {
        Some(Object::Ref(r)) => match laminapdf_cos::tolerant(doc.cos().resolve(*r))?.as_deref() {
            Some(Object::Array(items)) => items.iter().filter_map(Object::as_ref).collect(),
            Some(Object::Dict(_)) => vec![*r],
            _ => Vec::new(),
        },
        Some(Object::Array(items)) => items.iter().filter_map(Object::as_ref).collect(),
        _ => Vec::new(),
    };
    if groups.is_empty() {
        return Ok(false);
    }
    let on = groups.iter().filter(|g| config.on(**g)).count();
    let visible = match dict.get_name("P") {
        Some(b"AllOn") => on == groups.len(),
        Some(b"AnyOff") => on < groups.len(),
        Some(b"AllOff") => on == 0,
        _ => on > 0,
    };
    Ok(!visible)
}

/// The default configuration's states.
struct Config {
    base_off: bool,
    on: Vec<ObjRef>,
    off: Vec<ObjRef>,
}

impl Config {
    fn on(&self, group: ObjRef) -> bool {
        if self.base_off {
            self.on.contains(&group)
        } else {
            !self.off.contains(&group)
        }
    }
}

/// `/OCProperties /D`, read through references; `None` when the document has none.
fn default_config(doc: &laminapdf_model::Document) -> Result<Option<Config>> {
    let catalog = doc.catalog()?;
    let Some(properties) = resolve_dict(doc, catalog.get("OCProperties"))? else {
        return Ok(None);
    };
    let Some(d) = resolve_dict(doc, properties.get("D"))? else {
        return Ok(None);
    };
    let refs = |key: &str| -> Result<Vec<ObjRef>> {
        Ok(match d.get(key) {
            Some(Object::Array(items)) => items.iter().filter_map(Object::as_ref).collect(),
            Some(Object::Ref(r)) => laminapdf_cos::tolerant(doc.cos().resolve(*r))?
                .as_deref()
                .and_then(Object::as_array)
                .map(|items| items.iter().filter_map(Object::as_ref).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
        })
    };
    Ok(Some(Config {
        base_off: d.get_name("BaseState") == Some(b"OFF".as_slice()),
        on: refs("ON")?,
        off: refs("OFF")?,
    }))
}

fn resolve_dict(doc: &laminapdf_model::Document, value: Option<&Object>) -> Result<Option<Dict>> {
    Ok(match value {
        Some(Object::Dict(d)) => Some(d.clone()),
        Some(Object::Ref(r)) => laminapdf_cos::tolerant(doc.cos().resolve(*r))?
            .as_deref()
            .and_then(Object::as_dict)
            .cloned(),
        _ => None,
    })
}
