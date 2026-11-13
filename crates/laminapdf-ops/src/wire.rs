//! serde helpers for the domain types of `laminapdf-model`, which has no serde dependency: IDs
//! and dates as their string forms, rotations as degrees (ADR 0006 §1).

use std::fmt::Display;
use std::str::FromStr;

use serde::de::{self, Deserialize, Deserializer};
use serde::ser::Serializer;

/// A value from its string form ([`FromStr`]).
pub(crate) fn parse<'de, T, D>(d: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    s.parse().map_err(de::Error::custom)
}

/// A [`PageId`](laminapdf_model::PageId) as its string form (`"p12.0"`).
pub(crate) mod page_id {
    use laminapdf_model::PageId;

    use super::{Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(id: &PageId, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(id)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PageId, D::Error> {
        super::parse(d)
    }
}

/// An optional [`PdfDate`](laminapdf_model::PdfDate) as its PDF string form
/// (`"D:20261029120000Z"`).
pub(crate) mod opt_date {
    use laminapdf_model::PdfDate;

    use super::{Deserialize, Deserializer, Serializer, de};

    #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
    pub(crate) fn serialize<S: Serializer>(date: &Option<PdfDate>, s: S) -> Result<S::Ok, S::Error> {
        match date {
            Some(date) => s.collect_str(date),
            None => s.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<PdfDate>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| s.parse().map_err(de::Error::custom))
            .transpose()
    }
}

/// A [`Rotation`](laminapdf_model::Rotation) as degrees: exactly 0, 90, 180 or 270.
pub(crate) mod rotation {
    use laminapdf_model::Rotation;

    use super::{Deserialize, Deserializer, Serializer, de};

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` passes the field by reference"
    )]
    pub(crate) fn serialize<S: Serializer>(rotation: &Rotation, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u16(rotation.degrees())
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Rotation, D::Error> {
        let degrees = u16::deserialize(d)?;
        Rotation::from_canonical_degrees(degrees)
            .ok_or_else(|| de::Error::invalid_value(de::Unexpected::Unsigned(u64::from(degrees)), &"0, 90, 180 or 270"))
    }
}

/// `Option<Option<T>>` that tells an absent field (`None`: leave it) from `null`
/// (`Some(None)`: clear it). Used with `#[serde(default)]`.
pub(crate) mod double_option {
    use super::{Deserialize, Deserializer};

    #[expect(
        clippy::option_option,
        reason = "the point: absent, null and a value are three cases"
    )]
    pub(crate) fn deserialize<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(d).map(Some)
    }
}

/// `Option<Option<PdfDate>>` as [`double_option`] reads it, the date in its PDF string form:
/// absent leaves the field alone, `null` clears it, `"D:…"` sets it.
pub(crate) mod double_date {
    use laminapdf_model::PdfDate;

    use super::{Deserialize, Deserializer, Serializer, de};

    #[expect(
        clippy::ref_option,
        clippy::option_option,
        reason = "serde's `with` passes the field by reference; absent, null and a date are three cases"
    )]
    pub(crate) fn serialize<S: Serializer>(date: &Option<Option<PdfDate>>, s: S) -> Result<S::Ok, S::Error> {
        match date {
            Some(Some(date)) => s.collect_str(date),
            Some(None) | None => s.serialize_none(),
        }
    }

    #[expect(clippy::option_option, reason = "absent, null and a date are three cases")]
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<PdfDate>>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| s.parse().map_err(de::Error::custom))
            .transpose()
            .map(Some)
    }
}

/// For `skip_serializing_if`: a flag left at its default.
#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
pub(crate) const fn is_false(flag: &bool) -> bool {
    !*flag
}

/// An [`OutlineItemId`](laminapdf_model::OutlineItemId) as its string form (`"o12.0"`).
pub(crate) mod outline_item_id {
    use laminapdf_model::OutlineItemId;

    use super::{Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(id: &OutlineItemId, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(id)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<OutlineItemId, D::Error> {
        super::parse(d)
    }
}

/// An optional [`OutlineItemId`](laminapdf_model::OutlineItemId): `null` (or absent) for the
/// top level.
pub(crate) mod opt_outline_item_id {
    use laminapdf_model::OutlineItemId;

    use super::{Deserialize, Deserializer, Serializer, de};

    #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
    pub(crate) fn serialize<S: Serializer>(id: &Option<OutlineItemId>, s: S) -> Result<S::Ok, S::Error> {
        match id {
            Some(id) => s.collect_str(id),
            None => s.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<OutlineItemId>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| s.parse().map_err(de::Error::custom))
            .transpose()
    }
}

/// A [`SiblingPosition`](laminapdf_model::SiblingPosition): `"first"`, `"last"`,
/// `{ "before": "o12.0" }` or `{ "after": "o12.0" }`.
pub(crate) mod sibling_position {
    use laminapdf_model::{OutlineItemId, SiblingPosition};
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer, de};

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    enum Wire {
        First,
        Last,
        Before(String),
        After(String),
    }

    pub(crate) fn serialize<S: Serializer>(at: &SiblingPosition, s: S) -> Result<S::Ok, S::Error> {
        match at {
            SiblingPosition::First => Wire::First,
            SiblingPosition::Before(id) => Wire::Before(id.to_string()),
            SiblingPosition::After(id) => Wire::After(id.to_string()),
            _ => Wire::Last,
        }
        .serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SiblingPosition, D::Error> {
        let id = |s: String| s.parse::<OutlineItemId>().map_err(de::Error::custom);
        Ok(match Wire::deserialize(d)? {
            Wire::First => SiblingPosition::First,
            Wire::Last => SiblingPosition::Last,
            Wire::Before(s) => SiblingPosition::Before(id(s)?),
            Wire::After(s) => SiblingPosition::After(id(s)?),
        })
    }
}
