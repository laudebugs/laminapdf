//! The pass over every object: metadata streams, `/PieceInfo`, `/Thumb`, embedded files and
//! what holds them, the XMP packet, and the objects nothing references.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, Object, Result, tolerant};

use super::{Attachment, AttachmentHolder, Scan, XmpPacket, len, name_of, text_of};
use crate::document::Document;

impl Scan<'_> {
    /// Reads every in-use object once and classifies what it finds.
    pub(super) fn objects(&mut self, catalog: &Dict) -> Result<()> {
        let xmp_object = catalog.get("Metadata").and_then(Object::as_ref).map(|r| r.num);
        let numbers: Vec<u32> = self.doc.object_numbers().collect();
        let mut associated: Vec<Object> = Vec::new();
        for num in numbers {
            let Some(obj) = tolerant(self.doc.object(num))? else {
                continue;
            };
            let Some(dict) = obj.as_dict() else {
                continue;
            };
            if dict.get_name("Type") == Some(b"Metadata") && Some(num) != xmp_object {
                self.out.metadata_streams.push(num);
            }
            if dict.get("PieceInfo").is_some() {
                self.out.piece_info.push(num);
            }
            if dict.get("Thumb").is_some() && dict.get_name("Type") == Some(b"Page") {
                self.out.thumbnails.push(num);
            }
            if let Some(a) = attachment(self.doc, dict, Some(num))? {
                self.attachments.insert(num, a);
            }
            if let Some(af) = dict.get("AF") {
                associated.push(af.clone());
            }
        }
        for af in associated {
            let Some(items) = self.read(&af)? else { continue };
            for item in items.as_array().unwrap_or_default() {
                self.hold(item, AttachmentHolder::AssociatedFiles)?;
            }
        }
        if let Some(tree) = self.doc.name_tree("EmbeddedFiles")? {
            for (_, spec) in tree.entries()? {
                self.hold(&spec, AttachmentHolder::EmbeddedFiles)?;
            }
        }
        if let Some(meta) = catalog.get("Metadata") {
            self.out.xmp = self.xmp(meta)?;
        }
        self.out.unreferenced = unreferenced(self.doc)?;
        Ok(())
    }

    fn xmp(&self, meta: &Object) -> Result<Option<XmpPacket>> {
        let Some(stream) = self.read(meta)? else {
            return Ok(None);
        };
        let Object::Stream(stream) = &*stream else {
            return Ok(None);
        };
        let data = tolerant(self.doc.decode_stream(stream))?;
        let bytes = data.as_ref().map_or(stream.data.len(), Vec::len);
        let namespaces = data.as_deref().map(namespaces).unwrap_or_default();
        Ok(Some(XmpPacket {
            object: meta.as_ref().map(|r| r.num),
            bytes: len(bytes),
            namespaces,
        }))
    }
}

/// The embedded file `dict` describes, when it is a file specification with `/EF`.
pub(super) fn attachment(doc: &Document, dict: &Dict, object: Option<u32>) -> Result<Option<Attachment>> {
    let Some(ef) = dict.get("EF") else {
        return Ok(None);
    };
    let Some(ef) = doc.read(ef)? else {
        return Ok(None);
    };
    let Some(ef) = ef.as_dict() else {
        return Ok(None);
    };
    let mut name = None;
    for key in ["UF", "F"] {
        if name.is_none()
            && let Some(v) = dict.get(key)
        {
            name = doc.read(v)?.as_deref().and_then(text_of);
        }
    }
    let file = match ef.get("UF").or_else(|| ef.get("F")) {
        Some(f) => doc.read(f)?,
        None => None,
    };
    let (mut size, mut mime) = (None, None);
    if let Some(Object::Stream(file)) = file.as_deref() {
        mime = file.dict.get("Subtype").and_then(name_of);
        let params = match file.dict.get("Params") {
            Some(p) => doc.read(p)?,
            None => None,
        };
        size = params
            .as_deref()
            .and_then(Object::as_dict)
            .and_then(|p| p.get_int("Size"))
            .and_then(|s| u64::try_from(s).ok())
            .or(Some(len(file.data.len())));
    }
    Ok(Some(Attachment {
        name,
        size,
        mime,
        object,
        holders: Vec::new(),
    }))
}

/// The namespace URIs `xmlns:prefix="…"` attributes declare in `xml`, once each, in order.
pub(super) fn namespaces(xml: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = xml;
    while let Some(at) = find(rest, b"xmlns") {
        rest = rest.get(at + 5..).unwrap_or_default();
        let Some(eq) = rest.iter().position(|&b| b == b'=') else {
            break;
        };
        let value = rest.get(eq + 1..).unwrap_or_default();
        let value = value.trim_ascii_start();
        let Some((&quote, value)) = value.split_first() else {
            break;
        };
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        let Some(end) = value.iter().position(|&b| b == quote) else {
            break;
        };
        let uri = String::from_utf8_lossy(value.get(..end).unwrap_or_default()).into_owned();
        if !uri.is_empty() && !out.contains(&uri) {
            out.push(uri);
        }
        rest = value.get(end..).unwrap_or_default();
    }
    out
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The in-use objects nothing reachable from the trailer's `/Root`, `/Info` and `/Encrypt`
/// references, object and cross-reference streams aside.
fn unreferenced(doc: &Document) -> Result<Vec<u32>> {
    let mut reached: BTreeSet<u32> = BTreeSet::new();
    let mut stack: Vec<Object> = ["Root", "Info", "Encrypt"]
        .iter()
        .filter_map(|k| doc.trailer().get(k).cloned())
        .collect();
    while let Some(obj) = stack.pop() {
        match obj {
            Object::Ref(r) => {
                if reached.insert(r.num)
                    && let Some(value) = tolerant(doc.object(r.num))?
                {
                    stack.push((*value).clone());
                }
            }
            Object::Array(items) => stack.extend(items),
            Object::Dict(d) => stack.extend(d.iter().map(|(_, v)| v.clone())),
            Object::Stream(s) => stack.extend(s.dict.iter().map(|(_, v)| v.clone())),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for num in doc.object_numbers() {
        if reached.contains(&num) {
            continue;
        }
        let structural = tolerant(doc.object(num))?.is_some_and(
            |o| matches!(&*o, Object::Stream(s) if matches!(s.dict.get_name("Type"), Some(b"ObjStm" | b"XRef"))),
        );
        if !structural {
            out.push(num);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::namespaces;

    #[test]
    fn namespaces_are_listed_once_in_order() {
        let xml =
            br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
            <rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:x="adobe:ns:meta/"/>"#;
        assert_eq!(
            namespaces(xml),
            [
                "adobe:ns:meta/",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
                "http://purl.org/dc/elements/1.1/"
            ]
        );
    }

    #[test]
    fn a_broken_attribute_ends_the_list_without_failing() {
        assert_eq!(namespaces(b"xmlns:a=\"one\" xmlns:b=unquoted xmlns:c=\"two"), ["one"]);
    }
}
