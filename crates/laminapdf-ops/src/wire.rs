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

/// A list of [`AnnotId`](laminapdf_model::AnnotId)s as their string forms.
pub(crate) mod annot_ids {
    use laminapdf_model::AnnotId;
    use serde::{Deserialize, Serializer, ser::SerializeSeq};

    use super::{Deserializer, de};

    pub(crate) fn serialize<S: Serializer>(ids: &[AnnotId], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(ids.len()))?;
        for id in ids {
            seq.serialize_element(&id.to_string())?;
        }
        seq.end()
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<AnnotId>, D::Error> {
        Vec::<String>::deserialize(d)?
            .into_iter()
            .map(|s| s.parse().map_err(de::Error::custom))
            .collect()
    }
}

/// An optional list of page IDs, as [`page_ids`] writes one; absent when `None`.
pub(crate) mod opt_page_ids {
    use laminapdf_model::PageId;
    use serde::{Deserialize, Serializer};

    use super::{Deserializer, de};

    #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
    pub(crate) fn serialize<S: Serializer>(ids: &Option<Vec<PageId>>, s: S) -> Result<S::Ok, S::Error> {
        match ids {
            Some(ids) => super::page_ids::serialize(ids, s),
            None => s.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<PageId>>, D::Error> {
        Option::<Vec<String>>::deserialize(d)?
            .map(|ids| ids.into_iter().map(|s| s.parse().map_err(de::Error::custom)).collect())
            .transpose()
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

/// A [`UserSpaceRect`](laminapdf_model::UserSpaceRect) as `{ "x0", "y0", "x1", "y1" }`.
pub(crate) mod user_space_rect {
    use laminapdf_model::UserSpaceRect;
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Wire {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    }

    impl From<&UserSpaceRect> for Wire {
        fn from(r: &UserSpaceRect) -> Self {
            Self {
                x0: r.x0,
                y0: r.y0,
                x1: r.x1,
                y1: r.y1,
            }
        }
    }

    impl From<Wire> for UserSpaceRect {
        fn from(w: Wire) -> Self {
            Self::from_corners(w.x0, w.y0, w.x1, w.y1)
        }
    }

    pub(crate) fn serialize<S: Serializer>(rect: &UserSpaceRect, s: S) -> Result<S::Ok, S::Error> {
        Wire::from(rect).serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<UserSpaceRect, D::Error> {
        Wire::deserialize(d).map(UserSpaceRect::from)
    }
}

/// An optional [`UserSpaceRect`](laminapdf_model::UserSpaceRect): absent to leave it alone.
pub(crate) mod opt_user_space_rect {
    use laminapdf_model::UserSpaceRect;
    use serde::{Deserialize, Serialize};

    use super::user_space_rect::Wire;
    use super::{Deserializer, Serializer};

    #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
    pub(crate) fn serialize<S: Serializer>(rect: &Option<UserSpaceRect>, s: S) -> Result<S::Ok, S::Error> {
        rect.as_ref().map(Wire::from).serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<UserSpaceRect>, D::Error> {
        Ok(Option::<Wire>::deserialize(d)?.map(UserSpaceRect::from))
    }
}

/// Quadrilaterals, each four [`UserSpacePoint`](laminapdf_model::UserSpacePoint)s as
/// `{ "x", "y" }`, in `/QuadPoints` order.
pub(crate) mod quads {
    use laminapdf_model::UserSpacePoint;
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer};

    #[derive(Clone, Copy, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Point {
        x: f64,
        y: f64,
    }

    pub(crate) fn serialize<S: Serializer>(quads: &[[UserSpacePoint; 4]], s: S) -> Result<S::Ok, S::Error> {
        let wire: Vec<[Point; 4]> = quads.iter().map(|q| q.map(|p| Point { x: p.x, y: p.y })).collect();
        wire.serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<[UserSpacePoint; 4]>, D::Error> {
        let wire = Vec::<[Point; 4]>::deserialize(d)?;
        Ok(wire
            .into_iter()
            .map(|q| q.map(|p| UserSpacePoint::new(p.x, p.y)))
            .collect())
    }

    /// Absent to leave the quads alone; an empty list removes them.
    pub(crate) mod opt {
        use laminapdf_model::UserSpacePoint;
        use serde::Deserialize;

        use super::super::{Deserializer, Serializer};

        #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
        pub(crate) fn serialize<S: Serializer>(
            quads: &Option<Vec<[UserSpacePoint; 4]>>,
            s: S,
        ) -> Result<S::Ok, S::Error> {
            match quads {
                Some(q) => super::serialize(q, s),
                None => s.serialize_none(),
            }
        }

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
            d: D,
        ) -> Result<Option<Vec<[UserSpacePoint; 4]>>, D::Error> {
            #[derive(Deserialize)]
            struct Quads(#[serde(with = "super")] Vec<[UserSpacePoint; 4]>);
            Ok(Option::<Quads>::deserialize(d)?.map(|q| q.0))
        }
    }
}

/// Points, each a [`UserSpacePoint`](laminapdf_model::UserSpacePoint) as `{ "x", "y" }`: an ink
/// stroke's points, a polygon's vertices.
pub(crate) mod points {
    use laminapdf_model::UserSpacePoint;
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer};

    #[derive(Clone, Copy, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Point {
        x: f64,
        y: f64,
    }

    pub(crate) fn serialize<S: Serializer>(points: &[UserSpacePoint], s: S) -> Result<S::Ok, S::Error> {
        let wire: Vec<Point> = points.iter().map(|p| Point { x: p.x, y: p.y }).collect();
        wire.serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<UserSpacePoint>, D::Error> {
        let wire = Vec::<Point>::deserialize(d)?;
        Ok(wire.into_iter().map(|p| UserSpacePoint::new(p.x, p.y)).collect())
    }

    /// Absent to leave the points alone.
    pub(crate) mod opt {
        use laminapdf_model::UserSpacePoint;
        use serde::Deserialize;

        use super::super::{Deserializer, Serializer};

        #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
        pub(crate) fn serialize<S: Serializer>(points: &Option<Vec<UserSpacePoint>>, s: S) -> Result<S::Ok, S::Error> {
            match points {
                Some(p) => super::serialize(p, s),
                None => s.serialize_none(),
            }
        }

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<UserSpacePoint>>, D::Error> {
            #[derive(Deserialize)]
            struct Points(#[serde(with = "super")] Vec<UserSpacePoint>);
            Ok(Option::<Points>::deserialize(d)?.map(|p| p.0))
        }
    }
}

/// An optional [`UserSpacePoint`](laminapdf_model::UserSpacePoint): absent to leave it alone.
pub(crate) mod opt_user_space_point {
    use laminapdf_model::UserSpacePoint;
    use serde::Deserialize;

    use super::{Deserializer, Serializer};

    #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
    pub(crate) fn serialize<S: Serializer>(point: &Option<UserSpacePoint>, s: S) -> Result<S::Ok, S::Error> {
        match point {
            Some(p) => super::user_space_point::serialize(p, s),
            None => s.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<UserSpacePoint>, D::Error> {
        #[derive(Deserialize)]
        struct Point(#[serde(with = "super::user_space_point")] UserSpacePoint);
        Ok(Option::<Point>::deserialize(d)?.map(|p| p.0))
    }
}

/// A [`UserSpacePoint`](laminapdf_model::UserSpacePoint) as `{ "x", "y" }`.
pub(crate) mod user_space_point {
    use laminapdf_model::UserSpacePoint;
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        x: f64,
        y: f64,
    }

    pub(crate) fn serialize<S: Serializer>(p: &UserSpacePoint, s: S) -> Result<S::Ok, S::Error> {
        Wire { x: p.x, y: p.y }.serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<UserSpacePoint, D::Error> {
        Wire::deserialize(d).map(|w| UserSpacePoint::new(w.x, w.y))
    }
}

/// An optional [`AnnotId`](laminapdf_model::AnnotId) as its string.
pub(crate) mod opt_annot_id {
    use laminapdf_model::AnnotId;
    use serde::Deserialize;

    use super::{Deserializer, Serializer};

    #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
    pub(crate) fn serialize<S: Serializer>(id: &Option<AnnotId>, s: S) -> Result<S::Ok, S::Error> {
        match id {
            Some(id) => super::annot_id::serialize(id, s),
            None => s.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<AnnotId>, D::Error> {
        #[derive(Deserialize)]
        struct Id(#[serde(with = "super::annot_id")] AnnotId);
        Ok(Option::<Id>::deserialize(d)?.map(|id| id.0))
    }
}

/// An [`AttachmentId`](laminapdf_model::AttachmentId) as its string form (`"e12.0"`).
pub(crate) mod attachment_id {
    use laminapdf_model::AttachmentId;

    use super::{Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(id: &AttachmentId, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(id)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<AttachmentId, D::Error> {
        super::parse(d)
    }
}

/// An [`AfRelationship`](laminapdf_model::AfRelationship) by its camelCase name
/// ([`AfRelationship::camel_name`](laminapdf_model::AfRelationship::camel_name)): `source`,
/// `data`, `alternative`, `supplement`, `encryptedPayload`, `formData`, `schema` or
/// `unspecified`. The ops' TypeScript types spell the names out for tsify, and a test checks them
/// against [`AfRelationship::ALL`](laminapdf_model::AfRelationship::ALL).
pub(crate) mod af_relationship {
    use laminapdf_model::AfRelationship;

    use super::{Deserialize, Deserializer, Serializer};

    /// Every name, for the error that names what a value may be, in [`AfRelationship::ALL`]'s
    /// order.
    const EXPECTED: [&str; AfRelationship::ALL.len()] = {
        let mut out = [""; AfRelationship::ALL.len()];
        let mut all: &[AfRelationship] = &AfRelationship::ALL;
        let mut slots: &mut [&str] = &mut out;
        while let ([relationship, rest @ ..], [slot, more @ ..]) = (all, slots) {
            *slot = relationship.camel_name();
            all = rest;
            slots = more;
        }
        out
    };

    /// The relationship named `name` on the wire.
    pub(crate) fn from_name<E: serde::de::Error>(name: &str) -> Result<AfRelationship, E> {
        AfRelationship::ALL
            .into_iter()
            .find(|r| r.camel_name() == name)
            .ok_or_else(|| E::unknown_variant(name, &EXPECTED))
    }

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` passes the field by reference"
    )]
    pub(crate) fn serialize<S: Serializer>(relationship: &AfRelationship, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(relationship.camel_name())
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<AfRelationship, D::Error> {
        from_name(&String::deserialize(d)?)
    }
}

/// An optional [`AfRelationship`](laminapdf_model::AfRelationship) by its [`af_relationship`]
/// name.
pub(crate) mod opt_af_relationship {
    use laminapdf_model::AfRelationship;

    use super::{Deserialize, Deserializer, Serializer};

    #[expect(
        clippy::ref_option,
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` passes the field by reference"
    )]
    pub(crate) fn serialize<S: Serializer>(relationship: &Option<AfRelationship>, s: S) -> Result<S::Ok, S::Error> {
        match relationship {
            Some(r) => s.serialize_str(r.camel_name()),
            None => s.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<AfRelationship>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|n| super::af_relationship::from_name::<D::Error>(&n))
            .transpose()
    }
}

/// A [`FieldId`](laminapdf_model::FieldId) written as its string form (`"f12.0"`); it is read
/// by [`SetFieldValue`](crate::SetFieldValue)'s own reader.
pub(crate) mod field_id {
    use laminapdf_model::FieldId;

    use super::Serializer;

    pub(crate) fn serialize<S: Serializer>(id: &FieldId, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(id)
    }
}

/// A [`FieldValue`](laminapdf_model::FieldValue) as `{ "type": "text", "value": "…" }`,
/// `{ "type": "button", "value": "Yes" }` (`null` for off), `{ "type": "choice", "value":
/// ["…"] }` or `{ "type": "clear" }`.
pub(crate) mod field_value {
    use laminapdf_model::FieldValue as Value;
    use serde::Serialize;

    use super::Serializer;

    /// The wire form, under the name TypeScript gives it. It is read by
    /// [`SetFieldValue`](crate::SetFieldValue)'s own reader, through one `serde_json::Value`.
    #[derive(Serialize)]
    #[serde(tag = "type", content = "value", rename_all = "camelCase")]
    #[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
    pub(crate) enum FieldValue {
        Text(String),
        // An off button is `null` on the wire.
        Button(#[cfg_attr(feature = "tsify", tsify(type = "string | null"))] Option<String>),
        Choice(Vec<String>),
        Clear,
    }

    pub(crate) fn serialize<S: Serializer>(value: &Value, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Value::Text(t) => FieldValue::Text(t.clone()),
            Value::Button(b) => FieldValue::Button(b.clone()),
            Value::Choice(c) => FieldValue::Choice(c.clone()),
            Value::Clear => FieldValue::Clear,
            // A later kind of value has no wire form yet: refuse rather than write another value.
            _ => return Err(serde::ser::Error::custom("this kind of field value has no wire form")),
        }
        .serialize(s)
    }
}

/// Reading an op's arguments from one `serde_json::Value`, through non-generic functions, so
/// the WebAssembly modules hold one copy of the parsing code rather than one per serde
/// deserializer that reaches an op (the JSON reader, a `Value`, serde's buffered content).
pub(crate) mod json {
    use serde_json::Value;

    /// An object's entries, refusing any key outside `fields` (ADR 0006 §1: unknown fields are
    /// rejected), as serde's `deny_unknown_fields` words it.
    pub(crate) fn entries(value: Value, what: &str, fields: &[&str]) -> Result<serde_json::Map<String, Value>, String> {
        let Value::Object(map) = value else {
            return Err(format!("invalid type: {}, expected {what}", kind_of(&value)));
        };
        if let Some(key) = map.keys().find(|k| !fields.contains(&k.as_str())) {
            return Err(format!("unknown field `{key}`, expected one of {}", quoted(fields)));
        }
        Ok(map)
    }

    /// A JSON value's kind, in serde's words.
    pub(crate) const fn kind_of(value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "sequence",
            Value::Object(_) => "map",
        }
    }

    /// `names` as serde lists expected names: `` `a`, `b` ``.
    pub(crate) fn quoted(names: &[&str]) -> String {
        names.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")
    }
}

/// A [`PdfDate`](laminapdf_model::PdfDate) as its PDF string form (`"D:20261029120000Z"`).
pub(crate) mod date {
    use laminapdf_model::PdfDate;

    use super::{Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(date: &PdfDate, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(date)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PdfDate, D::Error> {
        super::parse(d)
    }
}

/// A [`Matrix`](laminapdf_model::Matrix) as `[a, b, c, d, e, f]`.
pub(crate) mod matrix {
    use laminapdf_model::Matrix;

    use super::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(m: &Matrix, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&m.to_array(), s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Matrix, D::Error> {
        Ok(Matrix::new(<[f64; 6]>::deserialize(d)?))
    }

    /// Absent to leave the matrix alone.
    pub(crate) mod opt {
        use laminapdf_model::Matrix;
        use serde::Deserialize;

        use super::super::{Deserializer, Serializer};

        #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
        pub(crate) fn serialize<S: Serializer>(m: &Option<Matrix>, s: S) -> Result<S::Ok, S::Error> {
            match m {
                Some(m) => super::serialize(m, s),
                None => s.serialize_none(),
            }
        }

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Matrix>, D::Error> {
            Ok(Option::<[f64; 6]>::deserialize(d)?.map(Matrix::new))
        }
    }
}

/// A [`RectPt`](laminapdf_model::RectPt) as `[x0, y0, x1, y1]`, top-left and bottom-right.
pub(crate) mod rect_pt {
    use laminapdf_model::{Pt, RectPt};

    use super::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(r: &RectPt, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&[r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get()], s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<RectPt, D::Error> {
        let [x0, y0, x1, y1] = <[f64; 4]>::deserialize(d)?;
        Ok(RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1)))
    }

    /// Absent (or `null`) for none.
    pub(crate) mod opt {
        use laminapdf_model::{Pt, RectPt};
        use serde::Deserialize;

        use super::super::{Deserializer, Serializer};

        #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
        pub(crate) fn serialize<S: Serializer>(r: &Option<RectPt>, s: S) -> Result<S::Ok, S::Error> {
            match r {
                Some(r) => super::serialize(r, s),
                None => s.serialize_none(),
            }
        }

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<RectPt>, D::Error> {
            Ok(Option::<[f64; 4]>::deserialize(d)?
                .map(|[x0, y0, x1, y1]| RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))))
        }
    }

    /// `Option<Option<RectPt>>`: absent leaves the rectangle alone, `null` removes it.
    pub(crate) mod double {
        use laminapdf_model::{Pt, RectPt};
        use serde::Deserialize;

        use super::super::{Deserializer, Serializer};

        #[expect(
            clippy::ref_option,
            clippy::option_option,
            reason = "serde's `with` passes the field by reference; absent, null and a value are three cases"
        )]
        pub(crate) fn serialize<S: Serializer>(r: &Option<Option<RectPt>>, s: S) -> Result<S::Ok, S::Error> {
            match r {
                Some(Some(r)) => super::serialize(r, s),
                Some(None) | None => s.serialize_none(),
            }
        }

        #[expect(clippy::option_option, reason = "absent, null and a value are three cases")]
        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<RectPt>>, D::Error> {
            Ok(Some(Option::<[f64; 4]>::deserialize(d)?.map(|[x0, y0, x1, y1]| {
                RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
            })))
        }
    }
}

/// A [`PointPt`](laminapdf_model::PointPt) as `{ "x", "y" }`.
pub(crate) mod point_pt {
    use laminapdf_model::PointPt;
    use serde::{Deserialize, Serialize};

    use super::{Deserializer, Serializer};

    #[derive(Clone, Copy, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Point {
        pub(super) x: f64,
        pub(super) y: f64,
    }

    pub(crate) fn serialize<S: Serializer>(p: &PointPt, s: S) -> Result<S::Ok, S::Error> {
        Point {
            x: p.x.get(),
            y: p.y.get(),
        }
        .serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PointPt, D::Error> {
        let p = Point::deserialize(d)?;
        Ok(PointPt::from_points(p.x, p.y))
    }

    /// Points, each as `{ "x", "y" }`.
    pub(crate) mod many {
        use laminapdf_model::PointPt;
        use serde::{Deserialize, Serialize};

        use super::super::{Deserializer, Serializer};
        use super::Point;

        pub(crate) fn serialize<S: Serializer>(points: &[PointPt], s: S) -> Result<S::Ok, S::Error> {
            let wire: Vec<Point> = points
                .iter()
                .map(|p| Point {
                    x: p.x.get(),
                    y: p.y.get(),
                })
                .collect();
            wire.serialize(s)
        }

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<PointPt>, D::Error> {
            Ok(Vec::<Point>::deserialize(d)?
                .into_iter()
                .map(|p| PointPt::from_points(p.x, p.y))
                .collect())
        }
    }
}
