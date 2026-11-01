//! Named resource lookup (ISO 32000-2 §7.8.3) and the resolved values the interpreter reads
//! from dictionaries, with ADR 0003's tolerance: anything that can't be read counts as missing,
//! except the errors that [must propagate](laminapdf_cos::Error::must_propagate).

use std::sync::Arc;

use laminapdf_cos::{Document, ObjRef, Object, Result, tolerant};

/// A resource found by name.
#[derive(Debug, Clone)]
pub(crate) struct Found {
    /// Its object, when the entry is a reference.
    pub(crate) object: Option<ObjRef>,
    /// The resolved value.
    pub(crate) value: Arc<Object>,
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
    let Some(value) = resolve(doc, entry)? else {
        return Ok(None);
    };
    Ok(Some(Found { object, value }))
}

/// `obj` resolved tolerantly; `None` when unreadable or `null`.
pub(crate) fn resolve(doc: &Document, obj: &Object) -> Result<Option<Arc<Object>>> {
    Ok(tolerant(doc.deref(obj))?
        .map(laminapdf_cos::Resolved::into_arc)
        .filter(|value| !matches!(**value, Object::Null)))
}

/// A finite number, resolved tolerantly.
pub(crate) fn number(doc: &Document, obj: &Object) -> Result<Option<f64>> {
    Ok(resolve(doc, obj)?.and_then(|n| n.as_f64()).filter(|n| n.is_finite()))
}

/// An array of exactly `len` finite numbers (the array and its elements may be indirect).
pub(crate) fn numbers(doc: &Document, obj: &Object, len: usize) -> Result<Option<Vec<f64>>> {
    let Some(array) = resolve(doc, obj)? else {
        return Ok(None);
    };
    let Some(items) = array.as_array().filter(|items| items.len() == len) else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(len);
    for item in items {
        match number(doc, item)? {
            Some(n) => out.push(n),
            None => return Ok(None),
        }
    }
    Ok(Some(out))
}

/// The initial colour of a colour space given as a `/ColorSpace` resource value (ISO 32000-2
/// §8.6.8 Table 73 and §8.6.4–8.6.6): 0 for each component of the device, CIE-based and
/// `Indexed` spaces (clipped to the space's `/Range`, black for `DeviceCMYK`), tint 1.0 for
/// each colourant of `Separation` and `DeviceN`, none for `Pattern`. `None` when the value is
/// not a colour space.
pub(crate) fn initial_components(doc: &Document, space: &Object) -> Result<Option<Vec<f64>>> {
    let Some(space) = resolve(doc, space)? else {
        return Ok(None);
    };
    let (family, params) = match &*space {
        Object::Name(name) => (name.as_bytes().to_vec(), None),
        Object::Array(items) => {
            let Some(family) = items.first() else {
                return Ok(None);
            };
            let Some(family) = resolve(doc, family)? else {
                return Ok(None);
            };
            let Object::Name(family) = &*family else {
                return Ok(None);
            };
            (family.as_bytes().to_vec(), items.get(1))
        }
        _ => return Ok(None),
    };
    Ok(match family.as_slice() {
        b"DeviceGray" | b"CalGray" | b"Indexed" => Some(vec![0.0]),
        b"DeviceRGB" | b"CalRGB" => Some(vec![0.0; 3]),
        b"DeviceCMYK" => Some(vec![0.0, 0.0, 0.0, 1.0]),
        b"Pattern" => Some(Vec::new()),
        b"Separation" => Some(vec![1.0]),
        b"DeviceN" => match params {
            Some(names) => resolve(doc, names)?
                .and_then(|names| names.as_array().map(<[Object]>::len))
                .filter(|&n| n > 0)
                .map(|n| vec![1.0; n]),
            None => None,
        },
        b"Lab" => {
            // L* starts at 0; a* and b* at 0 clipped to /Range (default [-100 100 -100 100]).
            let dict = match params {
                Some(p) => resolve(doc, p)?,
                None => None,
            };
            let range = match dict.as_deref().and_then(Object::as_dict).and_then(|d| d.get("Range")) {
                Some(range) => numbers(doc, range, 4)?,
                None => None,
            }
            .unwrap_or_else(|| vec![-100.0, 100.0, -100.0, 100.0]);
            let clip = |i: usize| {
                let lo = range.get(i).copied().unwrap_or(0.0);
                let hi = range.get(i + 1).copied().unwrap_or(0.0);
                0.0_f64.max(lo).min(hi)
            };
            Some(vec![0.0, clip(0), clip(2)])
        }
        b"ICCBased" => {
            let Some(profile) = (match params {
                Some(p) => resolve(doc, p)?,
                None => None,
            }) else {
                return Ok(None);
            };
            let Some(dict) = profile.as_dict() else {
                return Ok(None);
            };
            let n = match dict.get("N") {
                Some(n) => resolve(doc, n)?.and_then(|n| n.as_int()),
                None => None,
            };
            let Some(n @ (1 | 3 | 4)) = n.and_then(|n| usize::try_from(n).ok()) else {
                return Ok(None);
            };
            let range = match dict.get("Range") {
                Some(range) => numbers(doc, range, 2 * n)?,
                None => None,
            };
            Some(
                (0..n)
                    .map(|i| {
                        range.as_ref().map_or(0.0, |r| {
                            let lo = r.get(2 * i).copied().unwrap_or(0.0);
                            let hi = r.get(2 * i + 1).copied().unwrap_or(0.0);
                            0.0_f64.max(lo).min(hi)
                        })
                    })
                    .collect(),
            )
        }
        _ => None,
    })
}
