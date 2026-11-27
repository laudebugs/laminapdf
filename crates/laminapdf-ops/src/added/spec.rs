//! What the content ops add: the object's kind and geometry ([`ContentObjectSpec`]), its name
//! ([`ContentId`]), its alternate text ([`Alt`]) and where it goes in the page's stacking order
//! ([`ContentLayer`]).

use std::fmt;

use laminapdf_model::{PointPt, RectPt};
use serde::{Deserialize, Serialize};

use crate::annot::{AnnotColor, AnnotFont, TextAlign, TextSpan};
use crate::blob::BlobId;
use crate::wire;

/// The name of an object Lamina PDF added to a page, chosen by the caller: 1 to 64 ASCII letters,
/// digits, `-`, `_` or `.`, unique on its page. It is stored in the object's form
/// (`/PieceInfo /LaminaPDF /Private /Id`, ISO 32000-2 §14.5), so it names the object again after
/// the file is saved and reopened.
///
/// ```
/// use laminapdf_ops::ContentId;
///
/// assert!(ContentId::new("logo-1").is_some());
/// assert!(ContentId::new("").is_none());
/// assert!(ContentId::new("no spaces").is_none());
/// ```
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(type = "string"))]
pub struct ContentId(String);

impl ContentId {
    /// The longest ID, in bytes.
    ///
    /// Added in 0.7.0.
    pub const MAX_LEN: usize = 64;

    /// The ID `id`; `None` when it is empty, longer than [`MAX_LEN`](Self::MAX_LEN) or holds
    /// anything but ASCII letters, digits, `-`, `_` and `.`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Option<Self> {
        let id = id.into();
        let valid = !id.is_empty()
            && id.len() <= Self::MAX_LEN
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        valid.then_some(Self(id))
    }

    /// The ID as a string.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ContentId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(d)?)
            .ok_or_else(|| serde::de::Error::custom("a content ID is 1 to 64 ASCII letters, digits, '-', '_' or '.'"))
    }
}

/// What a screen reader gets for an added object in a tagged file (ISO 32000-2 §14.9.3).
///
/// On the wire: `{ "text": "The Northwind logo" }` or `"decorative"`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum Alt {
    /// The object is a `Figure` with this alternate text (`/Alt`).
    Text(String),
    /// The object is decoration: an artifact, which screen readers skip (§14.8.2.2).
    Decorative,
}

/// Where an added object goes in the page's stacking order.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum ContentLayer {
    /// Over the page's own content.
    #[default]
    Front,
    /// Under the page's own content, as a background is.
    Behind,
}

impl ContentLayer {
    #[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
    pub(crate) fn is_front(&self) -> bool {
        *self == Self::Front
    }
}

/// An object to add to a page as page content.
///
/// Each kind has its own space, which the op's `matrix` places on the page:
///
/// - [`Image`](Self::Image) and [`Form`](Self::Form) fill the unit square, `(0, 0)` its top-left
///   corner and `(1, 1)` its bottom-right, as page space runs (y down). A matrix
///   `[w, 0, 0, h, x, y]` draws one `w` by `h` points with its top-left corner at `(x, y)`.
/// - [`Shape`](Self::Shape)'s path is in points, y down, so the identity matrix draws it where its
///   coordinates are on the page.
///
/// - [`Text`](Self::Text) is laid out in a box `width` by `height` points, `(0, 0)` its bottom-left
///   corner and y up as text is set; `AddText` places it so it shows where its `rect` is, and an
///   update's `matrix` moves it afterwards.
///
/// On the wire the kind is the only key: `{ "image": { "blob": "…" } }`,
/// `{ "shape": { "path": { "rect": { "rect": [72, 72, 216, 144] } }, "fill": [1, 0, 0] } }`,
/// `{ "form": { "blob": "…", "page": 0 } }` or `{ "text": { "width": 200, "height": 40, "runs":
/// [{ "text": "Hello" }] } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum ContentObjectSpec {
    /// A JPEG or PNG image from the blob store: a JPEG keeps its bytes, a PNG with transparency
    /// gets a soft mask (`/SMask`, ISO 32000-2 §11.6.5.3), and an EXIF orientation shows the image
    /// upright.
    Image {
        /// The image's bytes.
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        blob: BlobId,
        /// The part of the unit square that shows, the rest clipped away: `[0, 0, 1, 1]` (or
        /// none) is the whole image. Cropping keeps the part that shows where it was.
        #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::rect_pt::opt")]
        #[cfg_attr(feature = "tsify", tsify(optional, type = "[number, number, number, number]"))]
        crop: Option<RectPt>,
    },
    /// A line, rectangle, ellipse or polygon, filled, stroked or both.
    Shape {
        /// The path.
        path: PathSpec,
        /// The fill colour; none leaves the inside empty. A line and an open polygon have no inside,
        /// so a fill for one is `invalid-op`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "tsify", tsify(optional))]
        fill: Option<AnnotColor>,
        /// The stroke; none draws no outline.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "tsify", tsify(optional))]
        stroke: Option<Stroke>,
    },
    /// A page of a PDF from the blob store, drawn as a form: a logo or a stamp kept as vectors and
    /// text. It is drawn as the page shows (its `/Rotate` applied), without its annotations and
    /// its marked content.
    Form {
        /// The PDF's bytes.
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        blob: BlobId,
        /// The page, counted from 0.
        page: u32,
    },
    /// Text laid out in a box ([`AddText`](crate::AddText) adds it): see [`TextBlock`].
    Text(TextBlock),
}

// Sound: an op holding a number that isn't finite is refused before it writes (`invalid-op`),
// and JSON can't hold one; ops are compared only after they parse.
impl Eq for ContentObjectSpec {}

impl ContentObjectSpec {
    /// The unit square, `[0, 0, 1, 1]`: the whole space of an image or a page.
    ///
    /// Added in 0.7.0.
    pub const UNIT: RectPt = RectPt {
        x0: laminapdf_model::Pt::ZERO,
        y0: laminapdf_model::Pt::ZERO,
        x1: laminapdf_model::Pt::new(1.0),
        y1: laminapdf_model::Pt::new(1.0),
    };

    /// The kind's name, as `/PieceInfo` stores it.
    pub(crate) const fn kind(&self) -> &'static str {
        match self {
            Self::Image { .. } => "Image",
            Self::Shape { .. } => "Shape",
            Self::Form { .. } => "Form",
            Self::Text(_) => "Text",
        }
    }

    /// Whether every number is finite and in range: a crop inside the unit square and not empty,
    /// a shape that draws something.
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::Image { crop, .. } => crop.is_none_or(|c| {
                c.is_finite()
                    && c.x0.get() >= 0.0
                    && c.y0.get() >= 0.0
                    && c.x1.get() <= 1.0
                    && c.y1.get() <= 1.0
                    && c.x1.get() > c.x0.get()
                    && c.y1.get() > c.y0.get()
            }),
            Self::Shape { path, fill, stroke } => {
                // A line and an open polygon have no inside to fill.
                let fillable = !matches!(path, PathSpec::Line { .. } | PathSpec::Polygon { closed: false, .. });
                path.is_valid()
                    && stroke.as_ref().is_none_or(Stroke::is_valid)
                    && (fill.is_none() || fillable)
                    && (fill.is_some() || stroke.is_some())
            }
            Self::Form { .. } => true,
            Self::Text(block) => block.is_valid(),
        }
    }
}

/// The most characters added text holds: far more than a page shows, few enough to lay out at
/// once.
const MAX_CHARS: usize = 100_000;
/// The most fonts one block names: the font and its fallbacks.
const MAX_FONTS: usize = 8;

/// Text laid out in a box, as [`ContentObjectSpec::Text`] holds it and [`AddText`](crate::AddText)
/// adds it.
///
/// The box is `width` by `height` points; text wraps at its width, starts at its top and is
/// clipped to it. `runs` are spans of one style each, and a line feed in a run starts a
/// paragraph. `fonts` lists the font, then the fallbacks tried, in order, for a character the
/// font lacks: the first is one of `helvetica`, `times`, `courier` or a font's bytes by their
/// ID, the others are bytes (a fallback is always embedded). A character in none of them is
/// `content-glyph-missing`.
///
/// On the wire: `{ "width": 200, "height": 40, "runs": [{ "text": "Q2 is " }, { "text": "late",
/// "bold": true }], "fonts": ["helvetica"], "size": 12 }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct TextBlock {
    /// The box's width in points.
    pub width: f64,
    /// The box's height in points.
    pub height: f64,
    /// The text, in spans of one style each.
    pub runs: Vec<TextSpan>,
    /// How lines sit in the box; left when absent.
    #[serde(default, skip_serializing_if = "TextAlign::is_left")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub align: TextAlign,
    /// The font and its fallbacks; Helvetica when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
    pub fonts: Vec<AnnotFont>,
    /// The size in points, from 1 to 500; 12 when absent.
    #[serde(default = "twelve", skip_serializing_if = "is_twelve")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub size: f64,
    /// The text's colour; black when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub color: Option<AnnotColor>,
}

// Sound: as for `ContentObjectSpec`.
impl Eq for TextBlock {}

pub(super) const fn twelve() -> f64 {
    12.0
}

#[cfg_attr(
    target_pointer_width = "64",
    expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")
)]
pub(super) fn is_twelve(v: &f64) -> bool {
    (*v - 12.0).abs() < f64::EPSILON
}

impl TextBlock {
    /// A box `width` by `height` points holding `runs`, in 12-point black Helvetica, left-aligned.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(width: f64, height: f64, runs: Vec<TextSpan>) -> Self {
        Self {
            width,
            height,
            runs,
            align: TextAlign::Left,
            fonts: Vec::new(),
            size: 12.0,
            color: None,
        }
    }

    /// The same block in `fonts` (the font, then fallbacks) at `size` points.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_fonts(mut self, fonts: Vec<AnnotFont>, size: f64) -> Self {
        self.fonts = fonts;
        self.size = size;
        self
    }

    /// The same block with its lines aligned by `align`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// The same block in `color`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_color(mut self, color: AnnotColor) -> Self {
        self.color = Some(color);
        self
    }

    /// The font the text starts in: the first of `fonts`, or Helvetica.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn font(&self) -> AnnotFont {
        self.fonts.first().copied().unwrap_or_default()
    }

    /// The fallback fonts' bytes, in order: every font after the first.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn fallbacks(&self) -> Vec<BlobId> {
        self.fonts
            .iter()
            .skip(1)
            .filter_map(|f| match f {
                AnnotFont::Font(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// The text in paragraphs: a line feed (or `\r\n`, or `\r`) in a run ends one. A `\r` ending
    /// a run and a `\n` starting the next are one break, as in a single run.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn paragraphs(&self) -> Vec<Vec<TextSpan>> {
        let mut out: Vec<Vec<TextSpan>> = vec![Vec::new()];
        let mut after_cr = false;
        for run in &self.runs {
            let mut text = String::with_capacity(run.text.len());
            for ch in run.text.chars() {
                match ch {
                    '\n' if after_cr => after_cr = false,
                    '\r' => {
                        text.push('\n');
                        after_cr = true;
                    }
                    other => {
                        text.push(other);
                        after_cr = false;
                    }
                }
            }
            for (i, part) in text.split('\n').enumerate() {
                if i > 0 {
                    out.push(Vec::new());
                }
                if !part.is_empty()
                    && let Some(last) = out.last_mut()
                {
                    let mut span = run.clone();
                    part.clone_into(&mut span.text);
                    last.push(span);
                }
            }
        }
        out
    }

    /// Whether every value is in range: a box with area, a size from 1 to 500, some text, at most
    /// 100,000 characters, at most eight fonts of which only the first may be a standard one.
    pub(crate) fn is_valid(&self) -> bool {
        let side = |v: f64| v.is_finite() && v > 0.0 && v <= MAX_COORD;
        side(self.width)
            && side(self.height)
            && self.size.is_finite()
            && (1.0..=500.0).contains(&self.size)
            && self.runs.iter().any(|r| !r.text.trim().is_empty())
            && self.runs.iter().map(|r| r.text.chars().count()).sum::<usize>() <= MAX_CHARS
            && self.fonts.len() <= MAX_FONTS
            && self.fonts.iter().skip(1).all(|f| matches!(f, AnnotFont::Font(_)))
    }
}

/// A shape's path, in points, y down ([`ContentObjectSpec::Shape`]).
///
/// On the wire the kind is the only key: `{ "line": { "from": { "x": 72, "y": 72 }, "to": { "x": 144,
/// "y": 72 } } }`, `{ "rect": { "rect": [72, 72, 144, 144], "radius": 6 } }`, `{ "ellipse": { "rect":
/// [72, 72, 144, 144] } }` or `{ "polygon": { "points": […] } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum PathSpec {
    /// A straight line.
    Line {
        /// Where it starts.
        #[serde(with = "wire::point_pt")]
        #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }"))]
        from: PointPt,
        /// Where it ends.
        #[serde(with = "wire::point_pt")]
        #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }"))]
        to: PointPt,
    },
    /// A rectangle, its corners rounded by `radius` points.
    Rect {
        /// The rectangle.
        #[serde(with = "wire::rect_pt")]
        #[cfg_attr(feature = "tsify", tsify(type = "[number, number, number, number]"))]
        rect: RectPt,
        /// How far each corner's curve reaches along its sides; at most half the shorter side.
        #[serde(default, skip_serializing_if = "is_zero")]
        #[cfg_attr(feature = "tsify", tsify(optional))]
        radius: f64,
    },
    /// The ellipse that fits `rect`.
    Ellipse {
        /// The rectangle.
        #[serde(with = "wire::rect_pt")]
        #[cfg_attr(feature = "tsify", tsify(type = "[number, number, number, number]"))]
        rect: RectPt,
    },
    /// Straight sides through `points`: closed back to the first, or open (a polyline).
    Polygon {
        /// The vertices: at least three for a closed polygon, two for an open one.
        #[serde(with = "wire::point_pt::many")]
        #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }[]"))]
        points: Vec<PointPt>,
        /// Whether the last side goes back to the first point.
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        #[cfg_attr(feature = "tsify", tsify(optional))]
        closed: bool,
    },
}

// On 64-bit targets only: clippy's size limit there is 8 bytes, and on wasm32 it is 4.
#[cfg_attr(
    target_pointer_width = "64",
    expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")
)]
fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

const fn yes() -> bool {
    true
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
const fn is_true(v: &bool) -> bool {
    *v
}

/// The largest coordinate a shape may have, in points: ISO 32000-2's limit on page size
/// (Annex C) is 14,400 units, and a shape may reach well off its page.
const MAX_COORD: f64 = 1.0e6;

impl PathSpec {
    fn is_valid(&self) -> bool {
        let point = |p: &PointPt| p.x.get().abs() <= MAX_COORD && p.y.get().abs() <= MAX_COORD;
        let rect = |r: &RectPt| {
            r.is_finite()
                && point(&PointPt::new(r.x0, r.y0))
                && point(&PointPt::new(r.x1, r.y1))
                && r.x1.get() > r.x0.get()
                && r.y1.get() > r.y0.get()
        };
        match self {
            Self::Line { from, to } => point(from) && point(to) && from != to,
            Self::Rect { rect: r, radius } => {
                let half = (r.x1.get() - r.x0.get()).min(r.y1.get() - r.y0.get()) / 2.0;
                rect(r) && radius.is_finite() && *radius >= 0.0 && *radius <= half
            }
            Self::Ellipse { rect: r } => rect(r),
            Self::Polygon { points, closed } => {
                points.len() >= if *closed { 3 } else { 2 } && points.len() <= 10_000 && points.iter().all(point)
            }
        }
    }
}

/// How a shape's outline is drawn.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Stroke {
    /// The colour.
    pub color: AnnotColor,
    /// The width in points, more than 0 and at most 144.
    pub width: f64,
    /// Solid, dashed or dotted.
    #[serde(default, skip_serializing_if = "StrokeDash::is_solid")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub dash: StrokeDash,
}

// Sound: as for `ContentObjectSpec`.
impl Eq for Stroke {}

impl Stroke {
    /// A solid stroke `width` points wide in `color`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(color: AnnotColor, width: f64) -> Self {
        Self {
            color,
            width,
            dash: StrokeDash::Solid,
        }
    }

    /// The same stroke, dashed or dotted.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_dash(mut self, dash: StrokeDash) -> Self {
        self.dash = dash;
        self
    }

    fn is_valid(&self) -> bool {
        self.width.is_finite() && self.width > 0.0 && self.width <= 144.0
    }
}

/// A stroke's pattern.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum StrokeDash {
    /// An unbroken line.
    #[default]
    Solid,
    /// Dashes three widths long with gaps as long.
    Dashed,
    /// Round dots a width apart.
    Dotted,
}

impl StrokeDash {
    #[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
    fn is_solid(&self) -> bool {
        *self == Self::Solid
    }
}
