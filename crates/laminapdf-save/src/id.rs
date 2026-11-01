//! The file identifier `/ID` (§14.4, [ADR 0007] §5).
//!
//! `/ID[0]` is the document's permanent identifier (and, for the standard security handler
//! R2–R4, an input to the file key, §7.6.3.3), so every save keeps it. `/ID[1]` changes with
//! every save: the first 16 bytes of the SHA-256 of the content written. `/ID[0]` is derived
//! the same way (equal to `/ID[1]`) only when the document has none. No clock or random
//! number is involved, so the output is byte-deterministic (ADR 0001).
//!
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use laminapdf_cos::{Dict, Document, Object, PdfString};
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
