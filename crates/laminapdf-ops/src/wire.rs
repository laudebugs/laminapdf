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

/// An [`XmpValue`](laminapdf_model::xmp::XmpValue) as `{ "type": …, "value": … }`: `text`,
/// `uri`, `integer`, `real` (finite), `boolean`, `date` (a PDF date string), `langAlt` (a list
/// of `{ "lang", "text" }`), `seq` and `bag` (lists of values).
pub(crate) mod xmp_value {
    use laminapdf_model::xmp::{LangText, XmpReal, XmpValue as Value};
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer, de};

    /// The wire form, under the name TypeScript gives it.
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "type", content = "value", rename_all = "camelCase")]
    #[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
    pub(crate) enum XmpValue {
        Text(String),
        Uri(String),
        Integer(i64),
        Real(f64),
        Boolean(bool),
        Date(String),
        LangAlt(Vec<XmpLangText>),
        Seq(Vec<XmpValue>),
        Bag(Vec<XmpValue>),
    }

    /// One entry of a language alternative, under the name TypeScript gives it.
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    #[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
    pub(crate) struct XmpLangText {
        lang: String,
        text: String,
    }

    fn to_wire(value: &Value) -> XmpValue {
        match value {
            Value::Text(s) => XmpValue::Text(s.clone()),
            Value::Uri(s) => XmpValue::Uri(s.clone()),
            Value::Integer(n) => XmpValue::Integer(*n),
            Value::Real(r) => XmpValue::Real(r.get()),
            Value::Boolean(b) => XmpValue::Boolean(*b),
            Value::Date(d) => XmpValue::Date(d.to_string()),
            Value::LangAlt(entries) => XmpValue::LangAlt(
                entries
                    .iter()
                    .map(|e| XmpLangText {
                        lang: e.lang.clone(),
                        text: e.text.clone(),
                    })
                    .collect(),
            ),
            Value::Seq(items) => XmpValue::Seq(items.iter().map(to_wire).collect()),
            Value::Bag(items) => XmpValue::Bag(items.iter().map(to_wire).collect()),
            // A later kind of value has no wire form yet; it can't come from an op log.
            _ => XmpValue::Text(String::new()),
        }
    }

    fn from_wire(wire: XmpValue) -> Result<Value, String> {
        let list = |items: Vec<XmpValue>| items.into_iter().map(from_wire).collect::<Result<Vec<_>, _>>();
        Ok(match wire {
            XmpValue::Text(s) => Value::Text(s),
            XmpValue::Uri(s) => Value::Uri(s),
            XmpValue::Integer(n) => Value::Integer(n),
            XmpValue::Real(r) => Value::Real(XmpReal::new(r).ok_or("a real must be finite")?),
            XmpValue::Boolean(b) => Value::Boolean(b),
            XmpValue::Date(s) => Value::Date(s.parse().map_err(|_| "a date must be a PDF date string")?),
            XmpValue::LangAlt(entries) => {
                Value::LangAlt(entries.into_iter().map(|e| LangText::new(e.lang, e.text)).collect())
            }
            XmpValue::Seq(items) => Value::Seq(list(items)?),
            XmpValue::Bag(items) => Value::Bag(list(items)?),
        })
    }

    pub(crate) fn serialize<S: Serializer>(value: &Value, s: S) -> Result<S::Ok, S::Error> {
        to_wire(value).serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Value, D::Error> {
        from_wire(XmpValue::deserialize(d)?).map_err(de::Error::custom)
    }
}
