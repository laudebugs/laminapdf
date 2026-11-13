//! Small write helpers the model's writers share: the catalog entry setter.

use laminapdf_cos::{CosMut, Error, Object, Result};

/// Sets (`Some`) or removes (`None`) the catalog's `key`, in the catalog object or, when the
/// trailer holds the catalog directly, in the trailer.
///
/// # Errors
/// Errors of the overlay; [`Error::MissingRoot`] without a catalog, and [`Error::Type`] when
/// `/Root` is not a dictionary.
///
/// Added in 0.2.0.
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

fn put(dict: &mut laminapdf_cos::Dict, key: &str, value: Option<Object>) {
    match value {
        Some(value) => dict.insert(key, value),
        None => {
            dict.remove(key);
        }
    }
}
