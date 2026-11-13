//! The values the outline and named-destination ops carry.

use laminapdf_cos::{Dict, Object};
use laminapdf_model::PageId;
use serde::{Deserialize, Serialize};

use crate::wire;

/// A colour of 8 bits a channel, `"#rrggbb"` on the wire. A bookmark's `/C` (§12.3.3,
/// Table 151) holds it as three numbers from 0 to 1.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(type = "string"))]
pub struct Rgb {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

impl Rgb {
    /// The colour from its channels.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// The colour of a `/C` array, each number clamped to 0–1 and rounded to 8 bits; `None`
    /// unless it holds three numbers.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_pdf(values: &[Object]) -> Option<Self> {
        let channel = |o: &Object| {
            let v = o.as_f64()?.clamp(0.0, 1.0);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0–255 first"
            )]
            Some((v * 255.0).round() as u8)
        };
        match values {
            [r, g, b] => Some(Self::new(channel(r)?, channel(g)?, channel(b)?)),
            _ => None,
        }
    }

    fn to_pdf(self) -> Object {
        let c = |v: u8| Object::rounded(f64::from(v) / 255.0, 4);
        Object::Array(vec![c(self.r), c(self.g), c(self.b)])
    }
}

impl std::fmt::Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl std::str::FromStr for Rgb {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "a colour is \"#rrggbb\"";
        let hex = s
            .strip_prefix('#')
            .filter(|h| h.len() == 6 && h.is_ascii())
            .ok_or(ERR)?;
        let channel = |i: usize| {
            hex.get(i..i + 2)
                .and_then(|c| u8::from_str_radix(c, 16).ok())
                .ok_or(ERR)
        };
        Ok(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }
}

impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        wire::parse(d)
    }
}

/// A bookmark's look: its colour (`/C`; `None` is the viewer's default, black) and type style
/// (`/F`: bit 1 italic, bit 2 bold).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct OutlineStyle {
    /// The colour.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(type = "string | null", optional))]
    pub color: Option<Rgb>,
    /// Bold.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub bold: bool,
    /// Italic.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub italic: bool,
}

impl OutlineStyle {
    /// The style with this colour, bold and italic.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(color: Option<Rgb>, bold: bool, italic: bool) -> Self {
        Self { color, bold, italic }
    }

    /// Writes `/C` and `/F`, removing each when it is the default. A `/F` with bits this type
    /// doesn't know keeps them.
    pub(crate) fn write(self, dict: &mut Dict) {
        match self.color {
            Some(c) => dict.insert("C", c.to_pdf()),
            None => {
                dict.remove("C");
            }
        }
        let other = dict.get("F").and_then(Object::as_int).unwrap_or(0) & !3;
        let flags = other | i64::from(self.italic) | (i64::from(self.bold) << 1);
        if flags == 0 {
            dict.remove("F");
        } else {
            dict.insert("F", Object::Integer(flags));
        }
    }
}

/// How a destination shows its page (§12.3.2.2, Table 149), in the page's user space. A `None`
/// coordinate keeps the viewer's current one (`null` in the file).
///
/// Every number must be finite: the ops refuse any other with `view-not-finite`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum DestinationView {
    /// `/XYZ`: the point `left`, `top` at the top left, at `zoom` (1 is 100 %; `None` or 0
    /// keeps the current zoom).
    Xyz {
        /// Left edge.
        left: Option<f64>,
        /// Top edge.
        top: Option<f64>,
        /// Zoom factor.
        zoom: Option<f64>,
    },
    /// `/Fit`: the whole page.
    Fit,
    /// `/FitH`: the page's width, `top` at the top.
    FitH {
        /// Top edge.
        top: Option<f64>,
    },
    /// `/FitV`: the page's height, `left` at the left.
    FitV {
        /// Left edge.
        left: Option<f64>,
    },
    /// `/FitR`: the rectangle.
    FitR {
        /// Left edge.
        left: f64,
        /// Bottom edge.
        bottom: f64,
        /// Right edge.
        right: f64,
        /// Top edge.
        top: f64,
    },
    /// `/FitB`: the page's bounding box.
    FitB,
    /// `/FitBH`: the bounding box's width.
    FitBH {
        /// Top edge.
        top: Option<f64>,
    },
    /// `/FitBV`: the bounding box's height.
    FitBV {
        /// Left edge.
        left: Option<f64>,
    },
}

/// [`DestinationView`] as read: every kind a struct, so `deny_unknown_fields` refuses extra
/// fields on `fit` and `fitB` too (serde ignores them on an internally tagged unit variant).
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum ViewWire {
    Xyz {
        #[serde(default)]
        left: Option<f64>,
        #[serde(default)]
        top: Option<f64>,
        #[serde(default)]
        zoom: Option<f64>,
    },
    Fit {},
    FitH {
        #[serde(default)]
        top: Option<f64>,
    },
    FitV {
        #[serde(default)]
        left: Option<f64>,
    },
    FitR {
        left: f64,
        bottom: f64,
        right: f64,
        top: f64,
    },
    FitB {},
    FitBH {
        #[serde(default)]
        top: Option<f64>,
    },
    FitBV {
        #[serde(default)]
        left: Option<f64>,
    },
}

impl<'de> Deserialize<'de> for DestinationView {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match ViewWire::deserialize(d)? {
            ViewWire::Xyz { left, top, zoom } => Self::Xyz { left, top, zoom },
            ViewWire::Fit {} => Self::Fit,
            ViewWire::FitH { top } => Self::FitH { top },
            ViewWire::FitV { left } => Self::FitV { left },
            ViewWire::FitR {
                left,
                bottom,
                right,
                top,
            } => Self::FitR {
                left,
                bottom,
                right,
                top,
            },
            ViewWire::FitB {} => Self::FitB,
            ViewWire::FitBH { top } => Self::FitBH { top },
            ViewWire::FitBV { left } => Self::FitBV { left },
        })
    }
}

// Sound for every op that applies: the ops refuse a non-finite number (`view-not-finite`)
// before writing, and the JSON form can't carry NaN.
impl Eq for DestinationView {}

impl DestinationView {
    /// The array after the page (`/XYZ 0 792 null`), or `None` when a number isn't finite.
    pub(crate) fn to_pdf(self) -> Option<Vec<Object>> {
        let n = |v: f64| v.is_finite().then(|| Object::rounded(v, 4));
        let opt = |v: Option<f64>| match v {
            Some(v) => n(v),
            None => Some(Object::Null),
        };
        let name = |s: &str| Object::name(s);
        Some(match self {
            Self::Xyz { left, top, zoom } => vec![name("XYZ"), opt(left)?, opt(top)?, opt(zoom)?],
            Self::Fit => vec![name("Fit")],
            Self::FitH { top } => vec![name("FitH"), opt(top)?],
            Self::FitV { left } => vec![name("FitV"), opt(left)?],
            Self::FitR {
                left,
                bottom,
                right,
                top,
            } => vec![name("FitR"), n(left)?, n(bottom)?, n(right)?, n(top)?],
            Self::FitB => vec![name("FitB")],
            Self::FitBH { top } => vec![name("FitBH"), opt(top)?],
            Self::FitBV { left } => vec![name("FitBV"), opt(left)?],
        })
    }
}

/// A page of this document and how to show it: what `/Dest [page /XYZ …]` holds.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct ExplicitDestination {
    /// The page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// How to show it.
    pub view: DestinationView,
}

impl ExplicitDestination {
    /// `page`, shown as `view`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(page: PageId, view: DestinationView) -> Self {
        Self { page, view }
    }
}

/// Where a bookmark goes.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum OutlineTarget {
    /// A page of this document (`/Dest [page /XYZ …]`).
    Destination(ExplicitDestination),
    /// A named destination (`/Dest (name)`, or `/Dest /name` for a name the catalog's
    /// `/Dests` holds).
    Named(String),
    /// A web address (`/A << /S /URI /URI (…) >>`).
    Uri(String),
}

/// What [`DeleteOutlineItem`](super::DeleteOutlineItem) does with the item's children.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum ChildItems {
    /// They go with it.
    Delete,
    /// They move up into its place, in order.
    Promote,
}

/// Whether [`RenameNamedDestination`](super::RenameNamedDestination) renames the uses of the
/// name in this file too.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum RenameLinks {
    /// Bookmarks, links and `GoTo` actions that name it keep the old name (and so break).
    Keep,
    /// They are renamed too.
    #[default]
    Follow,
}
