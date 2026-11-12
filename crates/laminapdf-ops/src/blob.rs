//! Content-addressed references to large payloads (ADR 0006 §1).

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use sha2::{Digest as _, Sha256};

/// A large binary payload (an image, a font, another PDF) referenced by its SHA-256 content
/// hash, so ops stay small; the bytes live in the document's [`BlobStore`] (ADR 0006 §1).
///
/// On the wire it is 64 lowercase hex digits.
///
/// ```
/// use laminapdf_ops::BlobId;
///
/// let id = BlobId::from_bytes([0xAB; 32]);
/// assert_eq!(id.to_string().parse::<BlobId>(), Ok(id));
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlobId([u8; 32]);

impl BlobId {
    /// The ID with this SHA-256 digest.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn from_bytes(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// The SHA-256 digest.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The ID of `bytes`: their SHA-256 digest, the same on every platform.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }
}

/// A source document ([`Document::add_source`](crate::Document::add_source)): the [`BlobId`]
/// of its bytes, so the same file has the same ID on every device and in every session. Ops
/// name source documents only by this ID; their passwords never enter an op.
///
/// Added in 0.2.0.
pub type SourceId = BlobId;

/// The bytes that ops name by [`BlobId`], kept for the whole session (ADR 0006 §1): undo,
/// redo and a restarted document worker replaying the log find them here. Nothing is removed while
/// the session lasts, since an op on the undo or redo stack may still name a blob.
///
/// ```
/// use laminapdf_ops::{BlobId, BlobStore};
///
/// let mut store = BlobStore::default();
/// let id = store.insert(b"font bytes".to_vec());
/// assert_eq!(id, BlobId::of(b"font bytes"));
/// assert_eq!(store.get(&id).map(|b| &b[..]), Some(&b"font bytes"[..]));
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default)]
pub struct BlobStore {
    blobs: BTreeMap<BlobId, Arc<[u8]>>,
}

impl BlobStore {
    /// Keeps `bytes` and returns their ID. Bytes already held are kept once.
    ///
    /// Added in 0.2.0.
    pub fn insert(&mut self, bytes: impl Into<Arc<[u8]>>) -> BlobId {
        let bytes = bytes.into();
        let id = BlobId::of(&bytes);
        self.blobs.entry(id).or_insert(bytes);
        id
    }

    /// The bytes with this ID, if the store holds them.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn get(&self, id: &BlobId) -> Option<&Arc<[u8]>> {
        self.blobs.get(id)
    }

    /// How many blobs the store holds.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    /// Whether the store holds no blob.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }
}

impl fmt::Display for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

/// The string is not 64 lowercase hex digits.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a blob id (64 lowercase hex digits)")]
#[non_exhaustive]
pub struct ParseBlobIdError;

impl FromStr for BlobId {
    type Err = ParseBlobIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let digit = |c: u8| match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        };
        let (pairs, rest) = s.as_bytes().as_chunks::<2>();
        if pairs.len() != 32 || !rest.is_empty() {
            return Err(ParseBlobIdError);
        }
        let mut out = [0; 32];
        for (slot, &[hi, lo]) in out.iter_mut().zip(pairs) {
            *slot = digit(hi)
                .zip(digit(lo))
                .map(|(h, l)| h << 4 | l)
                .ok_or(ParseBlobIdError)?;
        }
        Ok(Self(out))
    }
}

impl serde::Serialize for BlobId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for BlobId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        crate::wire::parse(d)
    }
}
