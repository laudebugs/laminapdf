//! The one writer every metadata op goes through: the Info dictionary and the XMP packet
//! changed together from the same values ([`MAPPINGS`](laminapdf_model::xmp::MAPPINGS)).

use std::sync::Arc;

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString, Stream};
use laminapdf_model::xmp::{MAPPINGS, MappedField, PDFA_ID, PDFUA_ID, Packet, PacketState, RdfValue, Value};
use laminapdf_syntax::text::encode_text_string;

use crate::error::{Error, Result};
use crate::op::Effect;

/// How Info's `/Author` joins the authors that XMP's `dc:creator` lists one by one.
pub(crate) const AUTHOR_SEPARATOR: &str = "; ";

/// What to do with a packet that can't be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Malformed {
    /// Replace it with a new packet (the op said so).
    Replace,
    /// Refuse the op: `malformed-xmp`.
    Fail,
    /// Leave it, changing Info alone.
    Skip,
}

/// How an op writes: whether a file without a packet gets one, what happens to a packet that
/// can't be read, and whether PDF 2.0's rule for Info applies.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Policy {
    create_packet: bool,
    /// Make a packet when there is none in a PDF 2.0 file, where the fields other than the
    /// dates have nowhere else to go.
    create_in_pdf2: bool,
    malformed: Malformed,
    /// In a PDF 2.0 file, set only `/CreationDate` and `/ModDate` in Info.
    pdf2_info: bool,
}

impl Policy {
    /// `SetMetadata`: a packet is made when there is none.
    pub(crate) const fn edit(replace_malformed: bool) -> Self {
        Self {
            create_packet: true,
            create_in_pdf2: true,
            malformed: if replace_malformed {
                Malformed::Replace
            } else {
                Malformed::Fail
            },
            pdf2_info: true,
        }
    }

    /// `RemoveMetadata`: nothing to remove from a packet that isn't there.
    pub(crate) const fn remove(replace_malformed: bool) -> Self {
        Self {
            create_packet: false,
            create_in_pdf2: false,
            ..Self::edit(replace_malformed)
        }
    }

    /// `UpdateSaveMetadata`: only what exists, and never a refusal.
    pub(crate) const fn stamp() -> Self {
        Self {
            create_packet: false,
            create_in_pdf2: false,
            malformed: Malformed::Skip,
            pdf2_info: true,
        }
    }

    /// `SetInfo` (v1): an existing packet kept in step, and none added, so its output on a
    /// file without one is what it always was; except in PDF 2.0, where Info takes only the
    /// dates and the other fields go to a packet, made if need be, as `SetMetadata` does.
    pub(crate) const fn info() -> Self {
        Self {
            create_packet: false,
            create_in_pdf2: true,
            malformed: Malformed::Skip,
            pdf2_info: true,
        }
    }
}

/// Writes `changes` (each field's new value, `None` to remove it) to Info and XMP.
pub(crate) fn write(
    doc: &mut laminapdf_model::Document,
    changes: &[(MappedField, Option<Value>)],
    policy: Policy,
) -> Result<Effect> {
    if changes.is_empty() {
        return Ok(Effect::default());
    }
    // Both sides get the value without the characters XML can't hold, so they agree.
    let changes: Vec<(MappedField, Option<Value>)> = changes
        .iter()
        .map(|(field, value)| (*field, value.as_ref().map(xml_value)))
        .collect();
    let pdf2 = policy.pdf2_info && doc.is_pdf2()?;
    let create = policy.create_packet || (pdf2 && policy.create_in_pdf2);
    let sets_a_value = changes.iter().any(|(_, v)| v.is_some());
    let packet = packet_to_edit(doc, sets_a_value, create, policy.malformed)?;
    // A new packet starts from what Info holds, so the two agree on the fields the edit leaves.
    let seed = match &packet {
        Some((_, true)) => super::read::info_values(doc)?,
        _ => Vec::new(),
    };
    let mut cos = doc.cos_mut().edit();
    let mut removed = write_info(&mut cos, &changes, pdf2)?;
    let mut packet_rewritten = false;
    if let Some((mut packet, fresh)) = packet {
        for (field, value) in &seed {
            let _ = packet.set(*field, Some(xml_value(value)));
        }
        for (field, value) in changes {
            removed |= value.is_none() && packet.get(field).is_some();
            // The values are built for their field's kind, so this can't fail.
            let _ = packet.set(field, value);
        }
        if packet.is_changed() {
            let written = packet.write();
            packet_rewritten = written.canonical && !fresh;
            write_packet(&mut cos, written.bytes)?;
        }
    }
    Ok(Effect {
        packet_rewritten,
        removed,
        ..Effect::default()
    })
}

/// Edits the XMP packet alone with `edit`, which returns whether it removed a property the
/// packet held: properties the Info dictionary doesn't mirror. A missing or unreadable packet
/// is handled as `policy` says; a new packet starts from Info's values, so the two agree.
pub(crate) fn edit_packet(
    doc: &mut laminapdf_model::Document,
    sets_a_value: bool,
    policy: Policy,
    edit: impl FnOnce(&mut Packet) -> Result<bool>,
) -> Result<Effect> {
    let Some((mut packet, fresh)) = packet_to_edit(doc, sets_a_value, policy.create_packet, policy.malformed)? else {
        return Ok(Effect::default());
    };
    if fresh {
        for (field, value) in super::read::info_values(doc)? {
            // Info's values are read in their field's shape, so this can't fail.
            let _ = packet.set(field, Some(xml_value(&value)));
        }
    }
    let removed = edit(&mut packet)?;
    let written = packet.write();
    // An edit that set a value the packet already held writes the same bytes: nothing changes.
    if !packet.is_changed() || (!fresh && written.bytes == packet.original()) {
        return Ok(Effect::default());
    }
    write_packet(&mut doc.cos_mut().edit(), written.bytes)?;
    Ok(Effect {
        packet_rewritten: written.canonical && !fresh,
        removed,
        declared: written.declared,
        ..Effect::default()
    })
}

/// Replaces the XMP packet with `xml` and brings Info's mirrored entries into step with it. A
/// packet that claims PDF/A gets the extension schema entries its properties lack. Refused
/// when `xml` doesn't parse, or changes the PDF/A or PDF/UA identification.
pub(crate) fn replace_packet(doc: &mut laminapdf_model::Document, xml: &str) -> Result<Effect> {
    let mut new = Packet::parse(xml.as_bytes()).map_err(Error::InvalidXmpPacket)?;
    let old = doc.xmp()?;
    let same_claims = match &old {
        // A packet that can't be read can't be parsed for its claims: they are read from its
        // text instead, and the new packet must claim the same parts and conformance.
        PacketState::Malformed { bytes, .. } => {
            let text = String::from_utf8_lossy(bytes);
            [PDFA_ID, PDFUA_ID]
                .into_iter()
                .all(|ns| scanned_claims(&text, ns) == claimed_values(&new, ns))
        }
        _ => claims(old.packet()) == claims(Some(&new)),
    };
    if !same_claims {
        return Err(Error::XmpClaimReadOnly);
    }
    new.declare_extensions();
    let written = new.write();
    if old.packet().is_some_and(|p| p.original() == written.bytes.as_slice()) {
        return Ok(Effect::default());
    }
    let info = super::read::info_values(doc)?;
    let changes: Vec<(MappedField, Option<Value>)> = MAPPINGS
        .iter()
        .filter(|m| m.info_key.is_some())
        .filter_map(|m| {
            let value = new.get(m.field).map(|v| xml_value(&v));
            let held = info.iter().find(|(f, _)| *f == m.field).map(|(_, v)| v);
            let same = match (&value, held) {
                (Some(v), Some(h)) => super::read::same(h, v),
                (None, None) => true,
                _ => false,
            };
            (!same).then_some((m.field, value))
        })
        .collect();
    let pdf2 = doc.is_pdf2()?;
    let mut cos = doc.cos_mut().edit();
    let removed_from_info = write_info(&mut cos, &changes, pdf2)?;
    write_packet(&mut cos, written.bytes)?;
    Ok(Effect {
        removed: !matches!(old, PacketState::Absent) || removed_from_info,
        declared: written.declared,
        ..Effect::default()
    })
}

/// The identification properties a claim is made of.
const CLAIM_NAMES: [&str; 2] = ["part", "conformance"];

/// `packet`'s `part` and `conformance` in `namespace`, trimmed, in [`CLAIM_NAMES`]' order.
fn claimed_values(packet: &Packet, namespace: &str) -> Vec<(&'static str, String)> {
    CLAIM_NAMES
        .into_iter()
        .filter_map(|name| match packet.property(namespace, name)? {
            RdfValue::Text(t) => Some((name, t.trim().to_owned())),
            _ => None,
        })
        .collect()
}

/// The `part` and `conformance` in `namespace` that the text of a packet that can't be parsed
/// shows: `<p:name>value</p:name>` or `p:name="value"` for each prefix `p` it binds to the
/// namespace (the first value found for each), in [`CLAIM_NAMES`]' order.
fn scanned_claims(text: &str, namespace: &str) -> Vec<(&'static str, String)> {
    let prefixes: Vec<&str> = text
        .match_indices(namespace)
        .filter_map(|(at, _)| {
            let before = text
                .get(..at)?
                .trim_end_matches(['"', '\'', '=', ' ', '\t', '\r', '\n']);
            let start = before.rfind("xmlns:")?;
            before.get(start + "xmlns:".len()..).filter(|p| !p.is_empty())
        })
        .collect();
    CLAIM_NAMES
        .into_iter()
        .filter_map(|name| {
            prefixes.iter().find_map(|prefix| {
                let element = format!("<{prefix}:{name}>");
                let attribute = format!("{prefix}:{name}=");
                if let Some(at) = text.find(&element) {
                    let rest = text.get(at + element.len()..)?;
                    return Some((name, rest.get(..rest.find('<')?)?.trim().to_owned()));
                }
                let rest = text.get(text.find(&attribute)? + attribute.len()..)?;
                let quote = rest.chars().next().filter(|q| matches!(q, '"' | '\''))?;
                let value = rest.get(1..)?;
                Some((name, value.get(..value.find(quote)?)?.trim().to_owned()))
            })
        })
        .collect()
}

/// A packet's PDF/A and PDF/UA identification: its `pdfaid:` and `pdfuaid:` properties, text
/// trimmed, in order.
fn claims(packet: Option<&Packet>) -> Vec<(String, String, RdfValue)> {
    let mut out: Vec<(String, String, RdfValue)> = packet
        .into_iter()
        .flat_map(Packet::properties)
        .filter(|p| p.namespace == PDFA_ID || p.namespace == PDFUA_ID)
        .map(|p| {
            let value = match &p.value {
                RdfValue::Text(t) => RdfValue::Text(t.trim().to_owned()),
                other => other.clone(),
            };
            (p.namespace.clone(), p.name.clone(), value)
        })
        .collect();
    out.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    out
}

/// The packet the op edits, and whether it is a new one; `None` to leave XMP alone.
fn packet_to_edit(
    doc: &laminapdf_model::Document,
    sets_a_value: bool,
    create: bool,
    malformed: Malformed,
) -> Result<Option<(Packet, bool)>> {
    Ok(match doc.xmp()? {
        PacketState::Parsed(packet) => Some((packet, false)),
        PacketState::Absent if create && sets_a_value => Some((Packet::new_minimal(), true)),
        PacketState::Malformed { .. } => match malformed {
            Malformed::Replace => Some((Packet::new_minimal(), true)),
            Malformed::Fail => return Err(Error::MalformedXmp),
            Malformed::Skip => None,
        },
        _ => None,
    })
}

/// `value` without the characters XML can't hold, which the packet can't carry.
fn xml_value(value: &Value) -> Value {
    match value {
        Value::Text(s) => Value::Text(super::xml_text(s)),
        Value::List(items) => Value::List(items.iter().map(|s| super::xml_text(s)).collect()),
        other => other.clone(),
    }
}

/// Applies the Info side of `changes`; returns whether it removed an entry that was there.
/// An Info dictionary written directly in the trailer, or missing, becomes a new indirect
/// object that the trailer's `/Info` points to (the first time something changes).
fn write_info(cos: &mut CosMut<'_>, changes: &[(MappedField, Option<Value>)], pdf2: bool) -> Result<bool> {
    // The current Info dictionary: shared with the overlay or base when indirect (a corrupt or
    // missing object reads as absent, like a missing /Info), else the trailer's direct one.
    let (target, current) = match cos.trailer().get("Info") {
        Some(Object::Ref(r)) => {
            let r = *r;
            match cos.get(r)? {
                Some(info) if matches!(*info, Object::Dict(_)) => (Some(r), info),
                _ => (None, Arc::new(Object::Dict(Dict::new()))),
            }
        }
        Some(direct @ Object::Dict(_)) => (None, Arc::new(direct.clone())),
        _ => (None, Arc::new(Object::Dict(Dict::new()))),
    };
    let empty = Dict::new();
    let current = current.as_dict().unwrap_or(&empty);
    let mut dict = current.clone();
    let mut removed = false;
    for (field, value) in changes {
        let Some(key) = field.mapping().info_key else {
            continue;
        };
        match value {
            None => removed |= dict.remove(key).is_some(),
            // ISO 32000-2 §14.3.3: in PDF 2.0 only the dates stay in Info; an old entry is left
            // as it was for older readers.
            Some(_) if pdf2 && !matches!(field, MappedField::Created | MappedField::Modified) => {}
            Some(value) => {
                if let Some(string) = info_string(value) {
                    dict.insert(key, string);
                }
            }
        }
    }
    if dict == *current {
        return Ok(removed);
    }
    if let Some(r) = target {
        cos.set(r, Object::Dict(dict))?;
    } else {
        let r = cos.create(Object::Dict(dict))?;
        let mut trailer = cos.trailer().clone();
        trailer.insert("Info", r);
        cos.set_trailer(trailer);
    }
    Ok(removed)
}

/// A value as Info stores it: a text string (PDFDocEncoding when it can be, else UTF-16BE),
/// authors joined with `; `, a date in the form the builder writes. `None` for a shape a later
/// `Value` adds, which Info has no form for: the entry is then left as it was.
fn info_string(value: &Value) -> Option<PdfString> {
    let bytes = match value {
        Value::Text(text) => encode_text_string(text),
        Value::List(items) => encode_text_string(&items.join(AUTHOR_SEPARATOR)),
        Value::Date(date) => date.to_string().into_bytes(),
        _ => return None,
    };
    Some(PdfString::literal(bytes))
}

/// Writes `bytes` as the catalog's `/Metadata` stream: over the existing stream object, or a
/// new one the catalog points to. The stream is unfiltered (ISO 19005-2 §6.6.2.1 forbids a
/// filter on it in PDF/A).
fn write_packet(cos: &mut CosMut<'_>, bytes: Vec<u8>) -> Result<()> {
    let root = cos.trailer().get("Root").cloned();
    let (catalog_ref, catalog) = match &root {
        Some(Object::Ref(r)) => match cos.get(*r)? {
            Some(object) => match &*object {
                Object::Dict(d) => (Some(*r), d.clone()),
                _ => return Ok(()),
            },
            None => return Ok(()),
        },
        Some(Object::Dict(d)) => (None, d.clone()),
        _ => return Ok(()),
    };
    let existing: Option<ObjRef> = catalog.get("Metadata").and_then(Object::as_ref);
    let old = match existing {
        Some(r) => cos.get(r)?,
        None => None,
    };
    let mut dict = match old.as_deref() {
        Some(Object::Stream(s)) => s.dict.clone(),
        _ => Dict::new(),
    };
    for key in ["Filter", "DecodeParms", "DL", "Length"] {
        dict.remove(key);
    }
    dict.insert("Type", Name::from("Metadata"));
    dict.insert("Subtype", Name::from("XML"));
    let stream = Object::Stream(Stream::new(dict, bytes));
    if let (Some(r), Some(Object::Stream(_))) = (existing, old.as_deref()) {
        return Ok(cos.set(r, stream)?);
    }
    let r = cos.create(stream)?;
    let mut catalog = catalog;
    catalog.insert("Metadata", r);
    if let Some(c) = catalog_ref {
        cos.set(c, Object::Dict(catalog))?;
    } else {
        let mut trailer = cos.trailer().clone();
        trailer.insert("Root", Object::Dict(catalog));
        cos.set_trailer(trailer);
    }
    Ok(())
}
