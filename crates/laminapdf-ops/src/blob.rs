//! Content-addressed references to large payloads (ADR 0006 §1).

use std::fmt;
use std::str::FromStr;

/// A large binary payload (an image, a font) referenced by its SHA-256 content hash, so ops
/// stay small; the bytes live in a blob store held by the document (ADR 0006 §1). Declared
/// for the ops that need it (images, fonts, attachments); no B3 op uses it yet.
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
