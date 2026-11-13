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

/// An [`AnnotId`](laminapdf_model::AnnotId) as its string form (`"a12.0"`).
pub(crate) mod annot_id {
    use laminapdf_model::AnnotId;

    use super::{Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(id: &AnnotId, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(id)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<AnnotId, D::Error> {
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

/// An optional [`LabelStyle`](laminapdf_model::LabelStyle) by name: `decimal`, `upperRoman`,
/// `lowerRoman`, `upperLetters` or `lowerLetters` (`null` or no value: no style). The names are
/// listed once, in [`label_style::NAMES`]; `PageLabelRange::style`'s TypeScript type spells them
/// out for tsify, and a test checks it against this list.
pub(crate) mod label_style {
    use laminapdf_model::LabelStyle;
    use serde::de::Error as _;

    use super::{Deserialize, Deserializer, Serializer};

    /// Each style and its name on the wire.
    pub(crate) const NAMES: [(LabelStyle, &str); 5] = [
        (LabelStyle::Decimal, "decimal"),
        (LabelStyle::UpperRoman, "upperRoman"),
        (LabelStyle::LowerRoman, "lowerRoman"),
        (LabelStyle::UpperLetters, "upperLetters"),
        (LabelStyle::LowerLetters, "lowerLetters"),
    ];

    const EXPECTED: [&str; 5] = [NAMES[0].1, NAMES[1].1, NAMES[2].1, NAMES[3].1, NAMES[4].1];

    #[expect(
        clippy::ref_option,
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` passes the field by reference"
    )]
    pub(crate) fn serialize<S: Serializer>(style: &Option<LabelStyle>, s: S) -> Result<S::Ok, S::Error> {
        match style.and_then(|style| NAMES.iter().find(|(st, _)| *st == style)) {
            Some((_, name)) => s.serialize_str(name),
            None => s.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<LabelStyle>, D::Error> {
        let Some(name) = Option::<String>::deserialize(d)? else {
            return Ok(None);
        };
        NAMES
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(style, _)| Some(*style))
            .ok_or_else(|| D::Error::unknown_variant(&name, &EXPECTED))
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

/// A [`PagePosition`](laminapdf_model::PagePosition) on the wire: `"start"`, `"end"`,
/// `{ "before": "p12.0" }` or `{ "after": "p12.0" }` (E2 plan §2.1: positions name pages,
/// never indices).
pub(crate) mod position {
    use laminapdf_model::{PageId, PagePosition};
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer, de};

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    enum Wire {
        Start,
        End,
        Before(#[serde(with = "super::page_id")] PageId),
        After(#[serde(with = "super::page_id")] PageId),
    }

    pub(crate) fn serialize<S: Serializer>(at: &PagePosition, s: S) -> Result<S::Ok, S::Error> {
        let wire = match at {
            PagePosition::Start => Wire::Start,
            PagePosition::End => Wire::End,
            PagePosition::Before(id) => Wire::Before(id.clone()),
            PagePosition::After(id) => Wire::After(id.clone()),
            _ => return Err(serde::ser::Error::custom("a page position this version can't write")),
        };
        wire.serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PagePosition, D::Error> {
        Ok(match Wire::deserialize(d).map_err(de::Error::custom)? {
            Wire::Start => PagePosition::Start,
            Wire::End => PagePosition::End,
            Wire::Before(id) => PagePosition::Before(id),
            Wire::After(id) => PagePosition::After(id),
        })
    }
}

/// A list of [`PageId`](laminapdf_model::PageId)s as their string forms.
pub(crate) mod page_ids {
    use laminapdf_model::PageId;
    use serde::{Deserialize, Serializer, ser::SerializeSeq};

    use super::{Deserializer, de};

    pub(crate) fn serialize<S: Serializer>(ids: &[PageId], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(ids.len()))?;
        for id in ids {
            seq.serialize_element(&id.to_string())?;
        }
        seq.end()
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<PageId>, D::Error> {
        Vec::<String>::deserialize(d)?
            .into_iter()
            .map(|s| s.parse().map_err(de::Error::custom))
            .collect()
    }
}

/// A [`PageSize`](laminapdf_model::PageSize) as `{ "width": …, "height": … }` in points.
pub(crate) mod page_size {
    use laminapdf_model::{PageSize, Pt};
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        width: f64,
        height: f64,
    }

    pub(crate) fn serialize<S: Serializer>(size: &PageSize, s: S) -> Result<S::Ok, S::Error> {
        Wire {
            width: size.width.get(),
            height: size.height.get(),
        }
        .serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PageSize, D::Error> {
        let w = Wire::deserialize(d)?;
        Ok(PageSize::new(Pt::new(w.width), Pt::new(w.height)))
    }
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

    fn to_wire(value: &Value) -> Result<XmpValue, &'static str> {
        Ok(match value {
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
            Value::Seq(items) => XmpValue::Seq(items.iter().map(to_wire).collect::<Result<_, _>>()?),
            Value::Bag(items) => XmpValue::Bag(items.iter().map(to_wire).collect::<Result<_, _>>()?),
            // A later kind of value has no wire form yet: refuse rather than write another value.
            _ => return Err("this kind of XMP value has no wire form"),
        })
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
        to_wire(value).map_err(serde::ser::Error::custom)?.serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Value, D::Error> {
        from_wire(XmpValue::deserialize(d)?).map_err(de::Error::custom)
    }
}
