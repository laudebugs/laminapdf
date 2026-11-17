//! Small write helpers the model's writers share: the catalog entry setter, the `/Names`
//! entry setter and the setter of a value inside an object.

use laminapdf_cos::{CosMut, DirectPath, Error, Object, PathSegment, Result};

/// Sets (`Some`) or removes (`None`) the catalog's `key`, in the catalog object or, when the
/// trailer holds the catalog directly, in the trailer.
///
/// # Errors
/// Errors of the overlay; [`Error::MissingRoot`] without a catalog, and [`Error::Type`] when
/// `/Root` is not a dictionary.
///
/// Added in 0.3.0.
pub fn set_catalog_entry(cos: &mut CosMut<'_>, key: &str, value: Option<Object>) -> Result<()> {
    let root = cos.trailer().get("Root").cloned();
    match root {
        Some(Object::Ref(r)) => {
            let current = cos.get(r)?.ok_or(Error::MissingRoot)?;
            let Object::Dict(mut catalog) = (*current).clone() else {
                return Err(Error::Type {
                    expected: "dictionary",
                    context: "/Root",
                });
            };
            put(&mut catalog, key, value);
            cos.set(r, Object::Dict(catalog))
        }
        Some(Object::Dict(mut catalog)) => {
            put(&mut catalog, key, value);
            let mut trailer = cos.trailer().clone();
            trailer.insert("Root", Object::Dict(catalog));
            cos.set_trailer(trailer);
            Ok(())
        }
        _ => Err(Error::MissingRoot),
    }
}

/// Sets (`Some`) or removes (`None`) `key` of the catalog's `/Names` dictionary (a name tree's
/// root, such as `/Dests` or `/EmbeddedFiles`): in the dictionary's own object, or in the
/// catalog when it is direct there. A `/Names` dictionary is made when there is none.
///
/// # Errors
/// As [`set_catalog_entry`].
///
/// Added in 0.6.0.
pub fn set_names_entry(cos: &mut CosMut<'_>, key: &str, value: Option<Object>) -> Result<()> {
    let catalog = match cos.trailer().get("Root").cloned() {
        Some(Object::Ref(r)) => cos.get(r)?.and_then(|c| c.as_dict().cloned()),
        Some(Object::Dict(d)) => Some(d),
        _ => None,
    }
    .ok_or(Error::MissingRoot)?;
    match catalog.get("Names").cloned() {
        Some(Object::Ref(r)) => {
            let mut names = cos.get(r)?.and_then(|o| o.as_dict().cloned()).unwrap_or_default();
            put(&mut names, key, value);
            cos.set(r, Object::Dict(names))
        }
        Some(Object::Dict(mut names)) => {
            put(&mut names, key, value);
            set_catalog_entry(cos, "Names", Some(Object::Dict(names)))
        }
        _ => match value {
            Some(value) => {
                let names: laminapdf_cos::Dict = [(key, value)].into_iter().collect();
                set_catalog_entry(cos, "Names", Some(Object::Dict(names)))
            }
            None => Ok(()),
        },
    }
}

/// The value at `path` (a direct value inside its anchor object, or inside the trailer when the
/// path has no anchor), as the overlay holds it now; `None` when the path leads nowhere.
///
/// # Errors
/// Errors of the overlay.
///
/// Added in 0.6.0.
pub fn get_at_path(cos: &CosMut<'_>, path: &DirectPath) -> Result<Option<Object>> {
    let mut at = match path.anchor {
        Some(r) => match cos.get(r)? {
            Some(o) => (*o).clone(),
            None => return Ok(None),
        },
        None => Object::Dict(cos.trailer().clone()),
    };
    for segment in &path.segments {
        let next = match (*segment, &at) {
            (PathSegment::Key(k), Object::Dict(dict)) => dict.get(k).cloned(),
            (PathSegment::Key(k), Object::Stream(stream)) => stream.dict.get(k).cloned(),
            (PathSegment::Index(i), Object::Array(items)) => items.get(i).cloned(),
            _ => None,
        };
        let Some(next) = next else { return Ok(None) };
        at = next;
    }
    Ok(Some(at))
}

/// Replaces the value at `path` (a direct value inside its anchor object, or inside the trailer
/// when the path has no anchor) with `value`.
///
/// # Errors
/// Errors of the overlay; [`Error::Type`] when the path doesn't lead to a value.
///
/// Added in 0.6.0.
pub fn set_at_path(cos: &mut CosMut<'_>, path: &DirectPath, value: Object) -> Result<()> {
    let missing = Error::Type {
        expected: "a value at the path",
        context: "set_at_path",
    };
    let mut root = match path.anchor {
        Some(r) => (*cos.get(r)?.ok_or_else(|| missing.clone())?).clone(),
        None => Object::Dict(cos.trailer().clone()),
    };
    let mut at = &mut root;
    for segment in &path.segments {
        at = match (*segment, at) {
            (PathSegment::Key(k), Object::Dict(dict)) => dict.get_mut(k),
            (PathSegment::Key(k), Object::Stream(stream)) => stream.dict.get_mut(k),
            (PathSegment::Index(i), Object::Array(items)) => items.get_mut(i),
            _ => None,
        }
        .ok_or_else(|| missing.clone())?;
    }
    *at = value;
    match (path.anchor, root) {
        (Some(r), root) => cos.set(r, root),
        (None, Object::Dict(trailer)) => {
            cos.set_trailer(trailer);
            Ok(())
        }
        (None, _) => Err(missing),
    }
}

/// Sets (`Some`) or removes (`None`) `dict`'s `key`.
pub(crate) fn put(dict: &mut laminapdf_cos::Dict, key: &str, value: Option<Object>) {
    match value {
        Some(value) => dict.insert(key, value),
        None => {
            dict.remove(key);
        }
    }
}
