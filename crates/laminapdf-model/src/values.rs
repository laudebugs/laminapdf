//! Small readers of PDF values that several views share: a name's bytes, a text string (§7.9.2.2)
//! and the file a file specification names (§7.11.3).

use laminapdf_cos::{Dict, ObjRef, Object, Result, tolerant};
use laminapdf_syntax::text::decode_text_string;

use crate::document::Document;

/// A text string, decoded (PDFDocEncoding or UTF-16, §7.9.2.2).
pub(crate) fn text_of(obj: &Object) -> Option<String> {
    match obj {
        Object::String(s) => Some(decode_text_string(&s.bytes)),
        _ => None,
    }
}

/// The bytes of the name `value` holds (resolved); `None` for anything else.
pub(crate) fn read_name(doc: &Document, value: Option<&Object>) -> Result<Option<Vec<u8>>> {
    let Some(value) = value else { return Ok(None) };
    Ok(match doc.read(value)?.as_deref() {
        Some(Object::Name(n)) => Some(n.as_bytes().to_vec()),
        _ => None,
    })
}

/// The file the file specification dictionary `spec` names: `/UF`, else `/F` (§7.11.3), a text
/// string or (as some writers store it) a name.
pub(crate) fn file_spec_name(doc: &Document, spec: &Dict) -> Result<Option<String>> {
    for key in ["UF", "F"] {
        if let Some(v) = spec.get(key)
            && let Some(name) = doc
                .read(v)?
                .as_deref()
                .and_then(|o| text_of(o).or_else(|| name_text(o)))
        {
            return Ok(Some(name));
        }
    }
    Ok(None)
}

/// A name's bytes as text (lossily).
fn name_text(obj: &Object) -> Option<String> {
    match obj {
        Object::Name(n) => Some(name_bytes_text(n.as_bytes())),
        _ => None,
    }
}

/// A name's bytes as text: UTF-8 (§7.3.5), anything else replaced.
pub(crate) fn name_bytes_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The dictionary `r` resolves to (§7.3.10: a reference to a free or replaced object is
/// `null`), if it is one and can be read.
pub(crate) fn dict_at(doc: &Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(tolerant(doc.deref(&Object::Ref(r)))?.and_then(|o| o.as_dict().cloned()))
}

/// The items of the array `value` (or the array it refers to); none when it isn't one.
pub(crate) fn array_items(doc: &Document, value: Option<&Object>) -> Result<Vec<Object>> {
    let Some(value) = value else { return Ok(Vec::new()) };
    Ok(tolerant(doc.deref(value))?
        .and_then(|o| o.as_array().map(<[Object]>::to_vec))
        .unwrap_or_default())
}
