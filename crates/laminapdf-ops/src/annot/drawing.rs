//! The drawing marks (E4 plan §2.1): ink, squares and circles, lines with their endings and
//! leader lines, polygons and polylines, each with its border (solid, dashed or cloudy) and
//! interior colour. Their keys are ISO 32000-2's (§12.5.6.8–§12.5.6.13), plus `/LMNA_Widths`,
//! the per-point widths of a pressure-varying ink stroke, which PDF has no key for (Annex E's
//! second-class name, prefix `LMNA`), and `/LMNA_Smoothing`, written only when a stroke is drawn
//! without smoothing.

use laminapdf_cos::{Dict, Object};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use serde::{Deserialize, Serialize};

use super::geom::{self, AnnotColor, numbers_of, rect_of};
use crate::wire;

/// The private key holding an ink annotation's per-point widths: one array per stroke of
/// `/InkList`, empty for a stroke of constant width.
pub(crate) const WIDTHS: &str = "LMNA_Widths";
/// The private key saying an ink annotation's strokes are drawn as straight segments.
pub(crate) const SMOOTHING: &str = "LMNA_Smoothing";
/// Ink points closer than this to the point kept before them are dropped, in points (E4 plan
/// §11).
pub(crate) const THIN: f64 = 0.25;
/// The widest border or stroke an op takes, in points.
const MAX_WIDTH: f64 = 144.0;

/// Freehand ink: strokes of points as captured, drawn smoothed through them unless `smoothing`
/// is off (ISO 32000-2 §12.5.6.13).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Ink {
    /// The strokes, at least one.
    pub strokes: Vec<InkStroke>,
    /// The pen's width in points, for strokes without widths of their own (`/BS /W`).
    #[serde(default = "one")]
    pub width: f64,
    /// Whether the appearance draws curves through the points (the default) or straight
    /// segments between them.
    #[serde(default = "yes")]
    pub smoothing: bool,
}

impl Ink {
    /// Smoothed `strokes` of a 1-point pen.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(strokes: Vec<InkStroke>) -> Self {
        Self {
            strokes,
            width: 1.0,
            smoothing: true,
        }
    }

    /// The same ink with a pen `width` points wide.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    /// The same ink, drawn smoothed or as straight segments.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_smoothing(mut self, smoothing: bool) -> Self {
        self.smoothing = smoothing;
        self
    }

    /// The ink with each stroke's points thinned to [`THIN`].
    fn thinned(&self) -> Self {
        Self {
            strokes: self.strokes.iter().map(InkStroke::thinned).collect(),
            width: self.width,
            smoothing: self.smoothing,
        }
    }

    fn is_valid(&self) -> bool {
        !self.strokes.is_empty()
            && width_ok(self.width)
            && self.width > 0.0
            && self.strokes.iter().all(InkStroke::is_valid)
    }
}

/// One stroke of an ink mark: its points in user space, and optionally a width per point, from
/// a pen's pressure.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct InkStroke {
    /// The points, at least one.
    #[serde(with = "wire::points")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }[]"))]
    pub points: Vec<UserSpacePoint>,
    /// A width in points for each point, as many as there are points; absent for the pen's
    /// width all along.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub widths: Option<Vec<f32>>,
}

impl InkStroke {
    /// A stroke through `points` at the pen's width.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(points: Vec<UserSpacePoint>) -> Self {
        Self { points, widths: None }
    }

    /// The same stroke with a width for each point.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_widths(mut self, widths: Vec<f32>) -> Self {
        self.widths = Some(widths);
        self
    }

    /// The stroke without points closer than [`THIN`] to the point kept before them. The first
    /// and last points are always kept, so the stroke starts and ends where it did; the last
    /// replaces the kept points it is too close to. Thinning a thinned stroke changes nothing.
    fn thinned(&self) -> Self {
        let near = |p: &UserSpacePoint, q: &UserSpacePoint| (p.x - q.x).hypot(p.y - q.y) < THIN;
        let mut points: Vec<UserSpacePoint> = Vec::with_capacity(self.points.len());
        let mut widths = self.widths.as_ref().map(|_| Vec::with_capacity(self.points.len()));
        let last = self.points.len().saturating_sub(1);
        for (i, p) in self.points.iter().enumerate() {
            if i != last && points.last().is_some_and(|q| near(p, q)) {
                continue;
            }
            if i == last {
                while points.len() > 1 && points.last().is_some_and(|q| near(p, q)) {
                    points.pop();
                    if let Some(w) = widths.as_mut() {
                        w.pop();
                    }
                }
            }
            points.push(*p);
            if let (Some(w), Some(all)) = (widths.as_mut(), self.widths.as_ref()) {
                w.push(all.get(i).copied().unwrap_or(1.0));
            }
        }
        Self { points, widths }
    }

    fn is_valid(&self) -> bool {
        !self.points.is_empty()
            && geom::finite(self.points.iter().copied())
            && self.widths.as_ref().is_none_or(|w| {
                w.len() == self.points.len() && w.iter().all(|&w| w.is_finite() && w > 0.0 && f64::from(w) <= MAX_WIDTH)
            })
    }
}

/// A square or circle filling a box (§12.5.6.8): a rectangle, or the ellipse inside the box.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Shape {
    /// Which shape.
    pub kind: ShapeKind,
    /// The box the shape's outline runs along, in user space.
    #[serde(with = "wire::user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: UserSpaceRect,
    /// The colour it is filled with (`/IC`); absent for none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub interior: Option<AnnotColor>,
    /// Its outline.
    #[serde(default)]
    pub border: Border,
}

impl Shape {
    /// An unfilled `kind` along `rect`, with a 1-point solid outline.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn new(kind: ShapeKind, rect: UserSpaceRect) -> Self {
        Self {
            kind,
            rect,
            interior: None,
            border: Border::default(),
        }
    }

    /// The same shape filled with `color` (`/IC`), or with none.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_interior(mut self, color: Option<AnnotColor>) -> Self {
        self.interior = color;
        self
    }

    /// The same shape with `border` as its outline.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    fn is_valid(&self) -> bool {
        geom::finite(geom::corners(&self.rect)) && self.rect.has_area() && self.border.is_valid(true)
    }
}

/// Which shape.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum ShapeKind {
    /// A rectangle (`/Square`).
    Square,
    /// An ellipse (`/Circle`).
    Circle,
}

impl ShapeKind {
    /// Its `/Subtype`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn subtype(self) -> &'static str {
        match self {
            Self::Square => "Square",
            Self::Circle => "Circle",
        }
    }
}

/// A straight line, with an ending at each end and optionally leader lines (§12.5.6.7).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct LineMark {
    /// Where it starts, in user space.
    #[serde(with = "wire::user_space_point")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }"))]
    pub from: UserSpacePoint,
    /// Where it ends.
    #[serde(with = "wire::user_space_point")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }"))]
    pub to: UserSpacePoint,
    /// The ending at `from` and at `to` (`/LE`).
    #[serde(default)]
    pub ends: [LineEnding; 2],
    /// The colour closed endings are filled with (`/IC`); absent for none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub interior: Option<AnnotColor>,
    /// Its stroke: solid or dashed.
    #[serde(default)]
    pub border: Border,
    /// Leader lines from the ends, which set the line off from what it measures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub leader: Option<Leader>,
}

impl LineMark {
    /// A 1-point solid line from `from` to `to`, with no endings.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn new(from: UserSpacePoint, to: UserSpacePoint) -> Self {
        Self {
            from,
            to,
            ends: [LineEnding::None; 2],
            interior: None,
            border: Border::default(),
            leader: None,
        }
    }

    /// The same line with `start` at `from` and `end` at `to`; an arrow is
    /// `with_ends(LineEnding::None, LineEnding::ClosedArrow)`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_ends(mut self, start: LineEnding, end: LineEnding) -> Self {
        self.ends = [start, end];
        self
    }

    /// The same line filled with `color` (`/IC`), or with none.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_interior(mut self, color: Option<AnnotColor>) -> Self {
        self.interior = color;
        self
    }

    /// The same line with `border` as its outline.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    /// The same line with `leader` lines, or none.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_leader(mut self, leader: Option<Leader>) -> Self {
        self.leader = leader;
        self
    }

    fn is_valid(&self) -> bool {
        geom::finite([self.from, self.to])
            && self.border.is_valid(false)
            && self.leader.as_ref().is_none_or(Leader::is_valid)
    }
}

/// Leader lines (§12.5.6.7, Table 178): perpendicular lines from the line's ends, the line
/// drawn at their far end.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Leader {
    /// How long the leader lines are (`/LL`), in points: positive to the left of the line
    /// going from its start to its end, negative to the right.
    pub length: f64,
    /// How far they carry on past the line (`/LLE`), 0 or more.
    #[serde(default)]
    pub extension: f64,
    /// The gap between the line's ends and where the leader lines begin (`/LLO`), 0 or more.
    #[serde(default)]
    pub offset: f64,
}

impl Leader {
    /// Leader lines `length` points long, with no extension or offset.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(length: f64) -> Self {
        Self {
            length,
            extension: 0.0,
            offset: 0.0,
        }
    }

    /// The same leader lines carrying on `extension` points past the line.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_extension(mut self, extension: f64) -> Self {
        self.extension = extension;
        self
    }

    /// The same leader lines starting `offset` points off the line's points.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }

    fn is_valid(&self) -> bool {
        [self.length, self.extension, self.offset].iter().all(|v| v.is_finite())
            && self.extension >= 0.0
            && self.offset >= 0.0
    }
}

/// A closed polygon or an open polyline through vertices (§12.5.6.9).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Poly {
    /// Closed or open.
    pub kind: PolyKind,
    /// The vertices in user space: at least three for a polygon, two for a polyline.
    #[serde(with = "wire::points")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }[]"))]
    pub vertices: Vec<UserSpacePoint>,
    /// A polyline's ending at its first and its last vertex (`/LE`); a polygon has none.
    #[serde(default)]
    pub ends: [LineEnding; 2],
    /// A polygon's fill, or the colour a polyline's closed endings are filled with (`/IC`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub interior: Option<AnnotColor>,
    /// Its outline: solid, dashed, or for a polygon cloudy.
    #[serde(default)]
    pub border: Border,
}

impl Poly {
    /// An unfilled `kind` through `vertices`, with a 1-point solid outline and no endings.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn new(kind: PolyKind, vertices: Vec<UserSpacePoint>) -> Self {
        Self {
            kind,
            vertices,
            ends: [LineEnding::None; 2],
            interior: None,
            border: Border::default(),
        }
    }

    /// The same polyline with `start` at its first vertex and `end` at its last; a polygon has
    /// no endings.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_ends(mut self, start: LineEnding, end: LineEnding) -> Self {
        self.ends = [start, end];
        self
    }

    /// The same polygon or polyline filled with `color` (`/IC`), or with none.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_interior(mut self, color: Option<AnnotColor>) -> Self {
        self.interior = color;
        self
    }

    /// The same polygon or polyline with `border` as its outline.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    fn is_valid(&self) -> bool {
        let least = match self.kind {
            PolyKind::Polygon => 3,
            PolyKind::PolyLine => 2,
        };
        self.vertices.len() >= least
            && geom::finite(self.vertices.iter().copied())
            && self.border.is_valid(self.kind == PolyKind::Polygon)
            && (self.kind == PolyKind::PolyLine || self.ends == [LineEnding::None; 2])
    }
}

/// Closed or open.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum PolyKind {
    /// A closed outline (`/Polygon`).
    Polygon,
    /// An open path (`/PolyLine`).
    PolyLine,
}

impl PolyKind {
    /// Its `/Subtype`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn subtype(self) -> &'static str {
        match self {
            Self::Polygon => "Polygon",
            Self::PolyLine => "PolyLine",
        }
    }
}

/// An outline's width and style (`/BS`, and `/BE` for a cloudy one).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Border {
    /// The width in points; 0 draws no outline.
    #[serde(default = "one")]
    pub width: f64,
    /// Solid, dashed or cloudy.
    #[serde(default)]
    pub style: BorderStyle,
}

impl Border {
    /// A solid outline `width` points wide.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn solid(width: f64) -> Self {
        Self {
            width,
            style: BorderStyle::Solid,
        }
    }

    /// A dashed outline `width` points wide: `pattern` alternates dash and gap lengths in
    /// points.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn dashed(width: f64, pattern: Vec<f64>) -> Self {
        Self {
            width,
            style: BorderStyle::Dashed { pattern },
        }
    }

    /// A cloudy outline `width` points wide, its bumps `intensity` from 0 (none) to 2 (the
    /// biggest).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn cloudy(width: f64, intensity: f64) -> Self {
        Self {
            width,
            style: BorderStyle::Cloudy { intensity },
        }
    }

    /// Whether the values are in range; a cloudy border only where `clouds` allows one
    /// (§12.5.4, `/BE` on squares, circles and polygons).
    fn is_valid(&self, clouds: bool) -> bool {
        width_ok(self.width)
            && match &self.style {
                BorderStyle::Solid => true,
                BorderStyle::Dashed { pattern } => {
                    !pattern.is_empty()
                        && pattern.len() <= 8
                        && pattern.iter().all(|v| v.is_finite() && *v >= 0.0)
                        && pattern.iter().any(|v| *v > 0.0)
                }
                BorderStyle::Cloudy { intensity } => clouds && (0.0..=2.0).contains(intensity),
            }
    }
}

impl Default for Border {
    fn default() -> Self {
        Self::solid(1.0)
    }
}

/// An outline's style.
///
/// On the wire: `{ "type": "solid" }`, `{ "type": "dashed", "pattern": [3, 2] }` or
/// `{ "type": "cloudy", "intensity": 1 }`.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum BorderStyle {
    /// A solid line.
    #[default]
    Solid,
    /// Dashes: dash and gap lengths in points, alternating, up to eight.
    Dashed {
        /// The pattern.
        pattern: Vec<f64>,
    },
    /// Bumps like a cloud's edge, for squares, circles and polygons.
    Cloudy {
        /// From 0 to 2: how big the bumps are.
        intensity: f64,
    },
}

/// A line ending (`/LE`, ISO 32000-2 Table 179).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum LineEnding {
    /// None.
    #[default]
    None,
    /// A square around the end.
    Square,
    /// A circle around the end.
    Circle,
    /// A diamond around the end.
    Diamond,
    /// Two short lines meeting at the end: an open arrowhead.
    OpenArrow,
    /// A filled triangle at the end: a closed arrowhead.
    ClosedArrow,
    /// A short line across the end.
    Butt,
    /// An open arrowhead pointing back along the line.
    ROpenArrow,
    /// A closed arrowhead pointing back along the line.
    RClosedArrow,
    /// A short line across the end at 30° from square.
    Slash,
}

impl LineEnding {
    /// Every ending, in Table 179's order.
    ///
    /// Added in 0.5.0.
    pub const ALL: [Self; 10] = [
        Self::None,
        Self::Square,
        Self::Circle,
        Self::Diamond,
        Self::OpenArrow,
        Self::ClosedArrow,
        Self::Butt,
        Self::ROpenArrow,
        Self::RClosedArrow,
        Self::Slash,
    ];

    /// Its name in `/LE`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Square => "Square",
            Self::Circle => "Circle",
            Self::Diamond => "Diamond",
            Self::OpenArrow => "OpenArrow",
            Self::ClosedArrow => "ClosedArrow",
            Self::Butt => "Butt",
            Self::ROpenArrow => "ROpenArrow",
            Self::RClosedArrow => "RClosedArrow",
            Self::Slash => "Slash",
        }
    }

    /// The ending a name in `/LE` names; any other name is none, as readers draw it.
    pub(crate) fn from_name(name: &[u8]) -> Self {
        Self::ALL
            .into_iter()
            .find(|e| e.name().as_bytes() == name)
            .unwrap_or_default()
    }
}

/// A width read from `/LMNA_Widths`, if it is one an op could have written: above 0, at most
/// [`MAX_WIDTH`].
#[expect(
    clippy::cast_possible_truncation,
    reason = "a width is at most 144 points, which f32 holds to far below a point"
)]
fn stroke_width(v: f64) -> Option<f32> {
    (v.is_finite() && v > 0.0 && v <= MAX_WIDTH).then_some(v as f32)
}

/// Whether a border or pen width is in range: 0 to [`MAX_WIDTH`].
fn width_ok(width: f64) -> bool {
    width.is_finite() && (0.0..=MAX_WIDTH).contains(&width)
}

const fn one() -> f64 {
    1.0
}

const fn yes() -> bool {
    true
}

/// The drawing marks' share of [`AnnotSpec`](super::AnnotSpec)'s methods.
pub(crate) enum Drawing<'a> {
    Ink(&'a Ink),
    Shape(&'a Shape),
    Line(&'a LineMark),
    Poly(&'a Poly),
}

impl Drawing<'_> {
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::Ink(i) => i.is_valid(),
            Self::Shape(s) => s.is_valid(),
            Self::Line(l) => l.is_valid(),
            Self::Poly(p) => p.is_valid(),
        }
    }
}

/// Ink with its points thinned, as `/InkList` and the appearance hold them.
pub(crate) fn normalised_ink(ink: &Ink) -> Ink {
    ink.thinned()
}

/// The keys of a drawing mark: its geometry, `/BS`, `/BE`, `/IC`, `/LE`, the leader lines and
/// the private ink keys. A shape's `/RD` is the gap between `/Rect`, already in `dict`, and the
/// box the shape runs along.
pub(crate) fn write(dict: &mut Dict, drawing: &Drawing<'_>) {
    match drawing {
        Drawing::Ink(ink) => {
            dict.insert(
                "InkList",
                Object::Array(ink.strokes.iter().map(|s| numbers(&s.points)).collect()),
            );
            write_border(dict, &Border::solid(ink.width));
            if ink.strokes.iter().any(|s| s.widths.is_some()) {
                let widths = ink
                    .strokes
                    .iter()
                    .map(|s| Object::Array(s.widths.iter().flatten().map(|w| Object::Real(f64::from(*w))).collect()))
                    .collect();
                dict.insert(WIDTHS, Object::Array(widths));
            } else {
                dict.remove(WIDTHS);
            }
            if ink.smoothing {
                dict.remove(SMOOTHING);
            } else {
                dict.insert(SMOOTHING, Object::Bool(false));
            }
        }
        Drawing::Shape(shape) => {
            write_border(dict, &shape.border);
            write_interior(dict, shape.interior);
            let rect = dict.get("Rect").and_then(rect_of);
            if let Some(outer) = rect {
                let inner = shape.rect;
                let rd = [
                    inner.x0 - outer.x0,
                    inner.y0 - outer.y0,
                    outer.x1 - inner.x1,
                    outer.y1 - inner.y1,
                ]
                .map(|v| Object::Real(v.max(0.0)));
                dict.insert("RD", Object::Array(rd.to_vec()));
            }
        }
        Drawing::Line(line) => {
            dict.insert("L", numbers(&[line.from, line.to]));
            write_ends(dict, line.ends);
            write_interior(dict, line.interior);
            write_border(dict, &line.border);
            match &line.leader {
                Some(leader) => {
                    dict.insert("LL", Object::Real(leader.length));
                    dict.insert("LLE", Object::Real(leader.extension));
                    dict.insert("LLO", Object::Real(leader.offset));
                }
                None => {
                    for key in ["LL", "LLE", "LLO"] {
                        dict.remove(key);
                    }
                }
            }
        }
        Drawing::Poly(poly) => {
            dict.insert("Vertices", numbers(&poly.vertices));
            if poly.kind == PolyKind::PolyLine {
                write_ends(dict, poly.ends);
            } else {
                dict.remove("LE");
            }
            write_interior(dict, poly.interior);
            write_border(dict, &poly.border);
        }
    }
}

/// `/BS` (width and style, with `/D` for dashes) and `/BE` for clouds.
fn write_border(dict: &mut Dict, border: &Border) {
    let mut bs = Dict::new();
    bs.insert("Type", Object::name("Border"));
    bs.insert("W", Object::number(border.width));
    match &border.style {
        BorderStyle::Dashed { pattern } => {
            bs.insert("S", Object::name("D"));
            bs.insert("D", Object::Array(pattern.iter().map(|v| Object::number(*v)).collect()));
            dict.remove("BE");
        }
        BorderStyle::Cloudy { intensity } => {
            bs.insert("S", Object::name("S"));
            let mut be = Dict::new();
            be.insert("S", Object::name("C"));
            be.insert("I", Object::number(*intensity));
            dict.insert("BE", be);
        }
        BorderStyle::Solid => {
            bs.insert("S", Object::name("S"));
            dict.remove("BE");
        }
    }
    dict.insert("BS", bs);
    // `/Border` is the older form (§12.5.2) and would conflict with `/BS`.
    dict.remove("Border");
}

fn write_interior(dict: &mut Dict, interior: Option<AnnotColor>) {
    match interior {
        Some(color) => {
            dict.insert("IC", color.to_object());
        }
        None => {
            dict.remove("IC");
        }
    }
}

fn write_ends(dict: &mut Dict, ends: [LineEnding; 2]) {
    dict.insert(
        "LE",
        Object::Array(ends.iter().map(|e| Object::name(e.name())).collect()),
    );
}

/// Points as a flat array of numbers, x then y.
fn numbers(points: &[UserSpacePoint]) -> Object {
    Object::Array(
        points
            .iter()
            .flat_map(|p| [Object::Real(p.x), Object::Real(p.y)])
            .collect(),
    )
}

/// Points from a flat array of numbers; a trailing odd number is dropped.
fn points_of(value: &Object) -> Option<Vec<UserSpacePoint>> {
    let values = numbers_of(value)?;
    let (pairs, _) = values.as_chunks::<2>();
    Some(pairs.iter().map(|[x, y]| UserSpacePoint::new(*x, *y)).collect())
}

/// `/BS` and `/BE` read back: a dashed pattern from `/D` (3 on, 3 off when it has none), clouds
/// from `/BE /S /C`; `/BS /W` else the older `/Border`'s third number else 1.
fn read_border(dict: &Dict, clouds: bool) -> Border {
    let bs = dict.get("BS").and_then(Object::as_dict);
    let width = bs
        .and_then(|bs| bs.get("W").and_then(Object::as_f64))
        .or_else(|| dict.get("Border").and_then(numbers_of).and_then(|b| b.get(2).copied()))
        .filter(|w| width_ok(*w))
        .unwrap_or(1.0);
    let be = dict.get("BE").and_then(Object::as_dict);
    if clouds && be.and_then(|be| be.get_name("S")) == Some(b"C".as_slice()) {
        let intensity = be
            .and_then(|be| be.get("I").and_then(Object::as_f64))
            .unwrap_or(0.0)
            .clamp(0.0, 2.0);
        return Border::cloudy(width, intensity);
    }
    if bs.and_then(|bs| bs.get_name("S")) == Some(b"D".as_slice()) {
        let pattern = bs
            .and_then(|bs| bs.get("D").and_then(numbers_of))
            .filter(|d| Border::dashed(width, d.clone()).is_valid(false))
            .unwrap_or_else(|| vec![3.0]);
        return Border::dashed(width, pattern);
    }
    Border::solid(width)
}

fn read_interior(dict: &Dict) -> Option<AnnotColor> {
    dict.get("IC")
        .and_then(numbers_of)
        .and_then(|c| AnnotColor::from_components(&c))
}

fn read_ends(dict: &Dict) -> [LineEnding; 2] {
    let names: Vec<LineEnding> = dict
        .get("LE")
        .and_then(Object::as_array)
        .map(|a| {
            a.iter()
                .map(|e| match e {
                    Object::Name(n) => LineEnding::from_name(n.as_bytes()),
                    _ => LineEnding::None,
                })
                .collect()
        })
        .unwrap_or_default();
    [
        names.first().copied().unwrap_or_default(),
        names.get(1).copied().unwrap_or_default(),
    ]
}

/// The spec a drawing annotation's keys describe, whichever app made it. A shape's box is
/// `/Rect` less `/RD`.
pub(crate) fn read(subtype: &[u8], dict: &Dict, rect: Option<UserSpaceRect>) -> Option<super::AnnotSpec> {
    use super::AnnotSpec;
    match subtype {
        b"Ink" => {
            let list = dict.get("InkList").and_then(Object::as_array)?;
            let widths = dict.get(WIDTHS).and_then(Object::as_array);
            let strokes = list
                .iter()
                .enumerate()
                .filter_map(|(i, stroke)| {
                    let points = points_of(stroke)?;
                    let widths = widths
                        .and_then(|w| w.get(i))
                        .and_then(numbers_of)
                        .filter(|w| !w.is_empty() && w.len() == points.len())
                        .and_then(|w| w.into_iter().map(stroke_width).collect());
                    Some(InkStroke { points, widths })
                })
                .collect();
            let border = read_border(dict, false);
            let smoothing = !matches!(dict.get(SMOOTHING), Some(Object::Bool(false)));
            Some(AnnotSpec::Ink(Ink {
                strokes,
                width: if border.width > 0.0 { border.width } else { 1.0 },
                smoothing,
            }))
        }
        b"Square" | b"Circle" => {
            let outer = rect?;
            let rd = dict.get("RD").and_then(numbers_of).unwrap_or_default();
            let inset = |i: usize| rd.get(i).copied().filter(|v| v.is_finite() && *v >= 0.0).unwrap_or(0.0);
            let inner = UserSpaceRect::from_corners(
                outer.x0 + inset(0),
                outer.y0 + inset(1),
                outer.x1 - inset(2),
                outer.y1 - inset(3),
            );
            let kind = if subtype == b"Square" {
                ShapeKind::Square
            } else {
                ShapeKind::Circle
            };
            Some(AnnotSpec::Shape(Shape {
                kind,
                rect: if inner.has_area() { inner } else { outer },
                interior: read_interior(dict),
                border: read_border(dict, true),
            }))
        }
        b"Line" => {
            let ends = dict.get("L").and_then(points_of)?;
            let (&from, &to) = (ends.first()?, ends.get(1)?);
            let leader = dict.get("LL").and_then(Object::as_f64).map(|length| Leader {
                length,
                extension: dict.get("LLE").and_then(Object::as_f64).unwrap_or(0.0).max(0.0),
                offset: dict.get("LLO").and_then(Object::as_f64).unwrap_or(0.0).max(0.0),
            });
            Some(AnnotSpec::Line(LineMark {
                from,
                to,
                ends: read_ends(dict),
                interior: read_interior(dict),
                border: read_border(dict, false),
                leader,
            }))
        }
        b"Polygon" | b"PolyLine" => {
            let kind = if subtype == b"Polygon" {
                PolyKind::Polygon
            } else {
                PolyKind::PolyLine
            };
            Some(AnnotSpec::Poly(Poly {
                kind,
                vertices: dict.get("Vertices").and_then(points_of)?,
                ends: if kind == PolyKind::PolyLine {
                    read_ends(dict)
                } else {
                    [LineEnding::None; 2]
                },
                interior: read_interior(dict),
                border: read_border(dict, kind == PolyKind::Polygon),
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "drawing_tests.rs"]
mod tests;
