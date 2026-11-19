//! The wire form of an `EditContent` object's hash: 16 hex digits. Matrices and page-space
//! rectangles are in `crate::wire`, which the content ops share.

use serde::de::{self, Deserialize, Deserializer};
use serde::ser::Serializer;

/// A [`ContentObjectRef`](laminapdf_content::ContentObjectRef)'s hash as 16 lowercase hex digits.
pub(crate) mod hash {
    use super::{Deserialize, Deserializer, Serializer, de};

    // On 64-bit targets only: clippy's size limit there is 8 bytes, and on wasm32 it is 4.
    #[cfg_attr(
        target_pointer_width = "64",
        expect(
            clippy::trivially_copy_pass_by_ref,
            reason = "serde's `with` passes the field by reference"
        )
    )]
    pub(crate) fn serialize<S: Serializer>(hash: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&format_args!("{hash:016x}"))
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        if s.len() != 16 || !s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(de::Error::custom("an object's hash is 16 lowercase hex digits"));
        }
        u64::from_str_radix(&s, 16).map_err(de::Error::custom)
    }
}
