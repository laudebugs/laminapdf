//! The file identifier `/ID` (§14.4, [ADR 0007] §5).
//!
//! `/ID[0]` is the document's permanent identifier (and, for the standard security handler
//! R2–R4, an input to the file key, §7.6.3.3), so every save keeps it. `/ID[1]` changes with
//! every save: the first 16 bytes of the SHA-256 of the content written. `/ID[0]` is derived
//! the same way (equal to `/ID[1]`) only when the document has none. No clock or random
//! number is involved, so the output is byte-deterministic (ADR 0001).
//!
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use laminapdf_cos::{Dict, Document, Object, OverlayEntry, PdfString};
use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::error::Result;

/// Bytes kept from the digest (an MD5-sized identifier, as most writers produce).
const ID_LEN: usize = 16;

/// The content hash `/ID[1]` comes from.
#[derive(Debug, Clone, Default)]
pub(crate) struct ContentHash(Sha256);

impl ContentHash {
    pub(crate) fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    /// The identifier: the first 16 bytes of the digest.
    fn finish(self) -> Vec<u8> {
        self.0.finalize().iter().take(ID_LEN).copied().collect()
    }
}

/// The document's `/ID[0]`, when its trailer (`trailer`, as it is now) has an `/ID` array
/// whose first element is a non-empty string; the array and its elements may be indirect.
/// A malformed `/ID` (not an array, not a string) gives `None`: the writer derives a new one.
///
/// # Errors
/// Any error resolving an indirect `/ID` or element (unavailable input, strict mode, a
/// limit, an unparsable object): an `/ID[0]` that exists but can't be read is not silently
/// replaced.
pub(crate) fn permanent_id(doc: &Document, trailer: &Dict) -> Result<Option<PdfString>> {
    let Some(id) = trailer.get("ID") else {
        return Ok(None);
    };
    let array = doc.deref(id)?;
    let Some(first) = array.as_array().and_then(<[Object]>::first) else {
        return Ok(None);
    };
    Ok(match &*doc.deref(first)? {
        Object::String(s) if !s.bytes.is_empty() => Some(s.clone()),
        _ => None,
    })
}

/// The `/ID` array to write: `permanent` (kept as it was written) or the content hash, then
/// the content hash.
pub(crate) fn id_array(permanent: Option<PdfString>, hash: ContentHash) -> Object {
    let changing = hash.finish();
    let first = permanent.unwrap_or_else(|| PdfString::hex(changing.clone()));
    Object::Array(vec![Object::String(first), Object::String(PdfString::hex(changing))])
}

/// An `xmpMM:InstanceID` for a save that stamps its metadata without one from the shell: a
/// UUID (version 8, RFC 9562 §5.8) from the SHA-256 of the document's `/ID[0]` and every
/// object and the trailer the save is about to write, so the same edits give the same ID and
/// different edits different ones.
///
/// # Errors
/// As [`permanent_id`].
pub(crate) fn derived_instance_id(doc: &Document) -> Result<[u8; ID_LEN]> {
    let mut hash = Sha256::new();
    hash.update(b"lamina-instance-id\n");
    if let Some(id) = permanent_id(doc, doc.trailer())? {
        hash.update(&id.bytes);
    }
    let mut bytes = Vec::new();
    for (num, entry) in doc.overlay().iter() {
        bytes.clear();
        bytes.extend_from_slice(format!("{num} ").as_bytes());
        match entry {
            OverlayEntry::New(object) | OverlayEntry::Modified(object) => object.write_to(&mut bytes),
            _ => bytes.extend_from_slice(b"free"),
        }
        hash.update(&bytes);
    }
    bytes.clear();
    Object::Dict(doc.trailer().clone()).write_to(&mut bytes);
    hash.update(&bytes);
    let mut id = [0u8; ID_LEN];
    for (slot, byte) in id.iter_mut().zip(hash.finalize()) {
        *slot = byte;
    }
    // Version 8 (custom) and the RFC 9562 variant.
    if let Some(b) = id.get_mut(6) {
        *b = (*b & 0x0f) | 0x80;
    }
    if let Some(b) = id.get_mut(8) {
        *b = (*b & 0x3f) | 0x80;
    }
    Ok(id)
}

/// The ID as XMP writes it: `uuid:` and the 8-4-4-4-12 hexadecimal form.
pub(crate) fn uuid(id: [u8; ID_LEN]) -> String {
    let hex = id.iter().fold(String::with_capacity(2 * ID_LEN), |mut s, b| {
        // Writing to a String can't fail.
        let _ = write!(s, "{b:02x}");
        s
    });
    let part = |range: std::ops::Range<usize>| hex.get(range).unwrap_or_default();
    format!(
        "uuid:{}-{}-{}-{}-{}",
        part(0..8),
        part(8..12),
        part(12..16),
        part(16..20),
        part(20..32)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uuid_has_the_8_4_4_4_12_form() {
        let id: [u8; 16] = core::array::from_fn(|i| u8::try_from(i).unwrap_or(0));
        assert_eq!(uuid(id), "uuid:00010203-0405-0607-0809-0a0b0c0d0e0f");
    }
}
