//! The wire forms of `EditContent`'s values: an object's hash as 16 hex digits, a matrix as
//! `[a, b, c, d, e, f]` and a page-space rectangle as `[x0, y0, x1, y1]`.

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

/// A [`Matrix`](laminapdf_model::Matrix) as `[a, b, c, d, e, f]`.
pub(crate) mod matrix {
    use laminapdf_model::Matrix;

    use super::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(m: &Matrix, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&[m.a, m.b, m.c, m.d, m.e, m.f], s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Matrix, D::Error> {
        Ok(Matrix::new(<[f64; 6]>::deserialize(d)?))
    }
}

/// A [`RectPt`](laminapdf_model::RectPt) as `[x0, y0, x1, y1]`, top-left and bottom-right.
pub(crate) mod rect {
    use laminapdf_model::{Pt, RectPt};

    use super::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(r: &RectPt, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&[r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get()], s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<RectPt, D::Error> {
        let [x0, y0, x1, y1] = <[f64; 4]>::deserialize(d)?;
        Ok(RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1)))
    }
}
