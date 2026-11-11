//! The one writer every metadata op goes through: the Info dictionary and the XMP packet
//! changed together from the same values ([`MAPPINGS`](laminapdf_model::xmp::MAPPINGS)).

use std::sync::Arc;

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString, Stream};
use laminapdf_model::xmp::{MappedField, Packet, PacketState, Value};
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
    malformed: Malformed,
    /// In a PDF 2.0 file, set only `/CreationDate` and `/ModDate` in Info.
    pdf2_info: bool,
}

impl Policy {
    /// `SetMetadata`: a packet is made when there is none.
    pub(crate) const fn edit(replace_malformed: bool) -> Self {
        Self {
            create_packet: true,
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
            ..Self::edit(replace_malformed)
        }
    }

    /// `UpdateSaveMetadata`: only what exists, and never a refusal.
    pub(crate) const fn stamp() -> Self {
        Self {
            create_packet: false,
            malformed: Malformed::Skip,
            pdf2_info: true,
        }
    }

    /// `SetInfo` (v1): the Info entries it names, whatever the version, and an existing packet
    /// kept in step; its output on a file without a packet is what it always was.
    pub(crate) const fn info() -> Self {
        Self {
            create_packet: false,
            malformed: Malformed::Skip,
            pdf2_info: false,
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
    let pdf2 = policy.pdf2_info && doc.is_pdf2()?;
    let packet = packet_to_edit(doc, changes, policy)?;
    // A new packet starts from what Info holds, so the two agree on the fields the edit leaves.
    let seed = match &packet {
        Some((_, true)) => super::read::info_values(doc)?,
        _ => Vec::new(),
    };
    let mut cos = doc.cos_mut().edit();
    let mut removed = write_info(&mut cos, changes, pdf2)?;
    let mut packet_rewritten = false;
    if let Some((mut packet, fresh)) = packet {
        for (field, value) in &seed {
            let _ = packet.set(*field, Some(xml_value(value)));
        }
        for (field, value) in changes {
            removed |= value.is_none() && packet.get(*field).is_some();
            // The values are built for their field's kind, so this can't fail.
            let _ = packet.set(*field, value.as_ref().map(xml_value));
        }
        if packet.is_changed() {
            let written = packet.write();
            packet_rewritten = written.canonical && !fresh;
            write_packet(&mut cos, written.bytes)?;
        }
    }
    Ok(Effect {
        pages: Vec::new(),
        packet_rewritten,
        removed,
    })
}

/// The packet the op edits, and whether it is a new one; `None` to leave XMP alone.
fn packet_to_edit(
    doc: &laminapdf_model::Document,
    changes: &[(MappedField, Option<Value>)],
    policy: Policy,
) -> Result<Option<(Packet, bool)>> {
    let sets_a_value = changes.iter().any(|(_, v)| v.is_some());
    Ok(match doc.xmp()? {
        PacketState::Parsed(packet) => Some((packet, false)),
        PacketState::Absent if policy.create_packet && sets_a_value => Some((Packet::new_minimal(), true)),
        PacketState::Malformed { .. } => match policy.malformed {
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
                dict.insert(key, info_string(value));
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
/// authors joined with `; `, a date in the form the builder writes.
fn info_string(value: &Value) -> PdfString {
    match value {
        Value::Text(text) => PdfString::literal(encode_text_string(text)),
        Value::List(items) => PdfString::literal(encode_text_string(&items.join(AUTHOR_SEPARATOR))),
        Value::Date(date) => PdfString::literal(date.to_string()),
        // `Value` is non-exhaustive; a new shape is written as nothing rather than guessed.
        _ => PdfString::literal(Vec::new()),
    }
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
