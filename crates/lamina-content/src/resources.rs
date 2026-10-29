//! Named resource lookup (ISO 32000-2 §7.8.3) with ADR 0003's tolerance: anything that can't
//! be read counts as missing, except the errors no reader may swallow.

use std::sync::Arc;

use lamina_cos::{Document, Error, ObjRef, Object, Result};

/// A resource found by name.
#[derive(Debug, Clone)]
pub(crate) struct Found {
    /// Its object, when the entry is a reference.
    pub(crate) object: Option<ObjRef>,
    /// The resolved value.
    pub(crate) value: Arc<Object>,
}

/// Errors no tolerant path may swallow: strict-mode refusals (the caller asked for them) and
/// input that is not available (the call has to be retried).
pub(crate) const fn must_propagate(e: &Error) -> bool {
    matches!(e, Error::Strict { .. } | Error::DataPending { .. } | Error::Source(_))
}

/// `Ok(Some(v))`, `Ok(None)` for an unreadable value, or a [`must_propagate`] error.
pub(crate) fn tolerant<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if must_propagate(&e) => Err(e),
        Err(_) => Ok(None),
    }
}

/// The resource `name` in `category` (e.g. `Font`) of the resource dictionary `resources`;
/// `None` when the dictionary, the category or the entry is missing, unreadable or `null`.
pub(crate) fn lookup(doc: &Document, resources: Option<&Object>, category: &str, name: &[u8]) -> Result<Option<Found>> {
    let Some(dict) = resources.and_then(Object::as_dict) else {
        return Ok(None);
    };
    let Some(category) = dict.get(category) else {
        return Ok(None);
    };
    let Some(category) = tolerant(doc.deref(category))? else {
        return Ok(None);
    };
    let Some(entry) = category
        .as_dict()
        .and_then(|d| d.iter().find(|(key, _)| key.as_bytes() == name))
        .map(|(_, entry)| entry)
    else {
        return Ok(None);
    };
    let object = entry.as_ref();
    let Some(value) = tolerant(doc.deref(entry))? else {
        return Ok(None);
    };
    if matches!(*value, Object::Null) {
        return Ok(None);
    }
    Ok(Some(Found {
        object,
        value: value.into_arc(),
    }))
}

/// `obj` resolved tolerantly (a `/Resources` or `/Font` reference), `None` when unreadable.
pub(crate) fn resolve(doc: &Document, obj: &Object) -> Result<Option<Arc<Object>>> {
    Ok(tolerant(doc.deref(obj))?.map(lamina_cos::Resolved::into_arc))
}
