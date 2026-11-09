//! Small readers of PDF values that several views share: a name's bytes, a text string (§7.9.2.2)
//! and the file a file specification names (§7.11.3).

use laminapdf_cos::{Dict, Object, Result};
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
        Object::Name(n) => Some(String::from_utf8_lossy(n.as_bytes()).into_owned()),
        _ => None,
    }
}
