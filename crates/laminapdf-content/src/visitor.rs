//! The [`Visitor`] trait and the events the interpreter reports to it.

use laminapdf_cos::Warning;
use laminapdf_model::{FillRule, Matrix, UserSpaceRect};
use laminapdf_syntax::{Dict, Name, ObjRef, Object};

use crate::lexer::{InlineImage, Op};
use crate::state::{FontRef, GraphicsState, TextAdvance};

/// Receives what a content stream draws, in content order (ADR 0023).
///
/// Every method has an empty default, so a visitor implements only what it needs. The
/// events borrow the interpreter's state; copy what you keep.
///
/// Added in 0.2.0.
pub trait Visitor {
    /// Every operator as it is read, before it runs (including unknown and skipped ones);
    /// `depth` is the Form XObject nesting (0 for the page's own content).
    ///
    /// Added in 0.2.0.
    fn operator(&mut self, op: &Op<'_>, depth: usize) {
        let _ = (op, depth);
    }

    /// A text-showing operator (`Tj`, `TJ`, `'`, `"`). Return how far the shown glyphs move
    /// the text matrix ([`TextState::glyph_advance`](crate::TextState::glyph_advance) summed
    /// over them, including `TJ` adjustments); the default, no movement, suits visitors that
    /// don't know the glyph widths.
    ///
    /// Added in 0.2.0.
    fn show_text(&mut self, show: &TextShow<'_>) -> TextAdvance {
        let _ = show;
        TextAdvance::default()
    }

    /// A path construction operator (`m`, `l`, `c`, `v`, `y`, `h`, `re`), in user space, with
    /// the CTM in effect when it ran (which maps it to the page; a `cm` in the middle of a path,
    /// which the spec forbids but files contain, changes it for the later segments only).
    ///
    /// Added in 0.2.0.
    fn path_segment(&mut self, segment: &PathSegment, ctm: &Matrix) {
        let _ = (segment, ctm);
    }

    /// A path-painting operator (`S`, `s`, `f`, `F`, `f*`, `B`, `B*`, `b`, `b*`, `n`) ending
    /// the path built by the preceding segments, or [`Paint::Discarded`] for a path its content
    /// stream left unpainted.
    ///
    /// Added in 0.2.0.
    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        let _ = paint;
    }

    /// `gs`: a graphics state parameter dictionary was applied (after the parameters the
    /// [`GraphicsState`] tracks were set from it; soft masks, blend modes, overprint and the
    /// rest are read from the dictionary).
    ///
    /// Added in 0.2.0.
    fn set_graphics_state(&mut self, params: &GraphicsStateParams<'_>) {
        let _ = params;
    }

    /// `sh`: a shading painted over the clip.
    ///
    /// Added in 0.2.0.
    fn paint_shading(&mut self, shading: &ShadingPaint<'_>) {
        let _ = shading;
    }

    /// An image: an image XObject drawn by `Do`, or an inline image.
    ///
    /// Added in 0.2.0.
    fn image(&mut self, image: &ImageDraw<'_>) {
        let _ = image;
    }

    /// `BMC` or `BDC`.
    ///
    /// Added in 0.2.0.
    fn begin_marked(&mut self, marked: &MarkedContent<'_>) {
        let _ = marked;
    }

    /// `EMC` (also called for sequences a content stream left open, ADR 0023).
    ///
    /// Added in 0.2.0.
    fn end_marked(&mut self) {}

    /// `MP` or `DP`.
    ///
    /// Added in 0.2.0.
    fn marked_point(&mut self, marked: &MarkedContent<'_>) {
        let _ = marked;
    }

    /// A Form XObject is about to run (`Do`), its matrix already applied to the CTM.
    ///
    /// Added in 0.2.0.
    fn enter_xobject(&mut self, form: &FormXObject<'_>) {
        let _ = form;
    }

    /// The Form XObject entered last has finished.
    ///
    /// Added in 0.2.0.
    fn leave_xobject(&mut self, form: &FormXObject<'_>) {
        let _ = form;
    }

    /// Something was repaired or skipped. The same warning is recorded on the document
    /// ([`laminapdf_cos::Document::warnings`]); in strict mode the interpretation fails with
    /// [`laminapdf_cos::Error::Strict`] right after this call.
    ///
    /// Added in 0.2.0.
    fn warning(&mut self, warning: &Warning) {
        let _ = warning;
    }
}

/// One path construction step (§8.5.2), in user space.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSegment {
    /// `x y m`.
    MoveTo {
        /// x.
        x: f64,
        /// y.
        y: f64,
    },
    /// `x y l`.
    LineTo {
        /// x.
        x: f64,
        /// y.
        y: f64,
    },
    /// `x1 y1 x2 y2 x3 y3 c`; `v` and `y` are expanded to it (§8.5.2.2: `v`'s first control
    /// point is the current point, `y`'s second is the end point).
    CurveTo {
        /// First control point.
        c1: (f64, f64),
        /// Second control point.
        c2: (f64, f64),
        /// End point.
        end: (f64, f64),
    },
    /// `h`.
    ClosePath,
    /// `x y width height re`.
    Rect {
        /// x.
        x: f64,
        /// y.
        y: f64,
        /// Width.
        width: f64,
        /// Height.
        height: f64,
    },
}

/// How a path is painted (§8.5.3, Table 59).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Paint {
    /// `S`; `s` closes the path first (`close`).
    Stroke {
        /// Whether the path was closed first.
        close: bool,
    },
    /// `f`, `F`, `f*`.
    Fill(FillRule),
    /// `B`, `B*`; `b`, `b*` close the path first.
    FillStroke {
        /// The fill rule.
        rule: FillRule,
        /// Whether the path was closed first.
        close: bool,
    },
    /// `n`: no painting (usually after `W` to set a clip).
    None,
    /// The content stream ended (or a form was drawn) with the path unpainted: it is dropped
    /// (`content-path-unpainted`, ADR 0023 §3).
    Discarded,
}

impl Paint {
    /// Whether anything is painted: a stroke, a fill or both, not [`Paint::None`] or
    /// [`Paint::Discarded`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn draws(self) -> bool {
        matches!(self, Self::Stroke { .. } | Self::Fill(_) | Self::FillStroke { .. })
    }
}

/// A path-painting operator.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct PathPaint<'s> {
    /// How the path is painted.
    pub paint: Paint,
    /// The clip rule when `W`/`W*` came before the painting operator: the path also
    /// intersects the clip, effective after painting (§8.5.4).
    pub clip: Option<FillRule>,
    /// The graphics state.
    pub state: &'s GraphicsState,
    /// The path's bounding box (its points and control points) in the page's default user
    /// space; `None` for a path with no points.
    pub bounds: Option<UserSpaceRect>,
}

/// One element of a text-showing operator.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextItem<'o> {
    /// Glyph codes to show (still encoded; decoding them is the font's job).
    Text(&'o [u8]),
    /// A `TJ` number: move by `−n/1000` text space units times `Tfs` (and `Th` horizontally).
    Adjust(f64),
}

/// A graphics state parameter dictionary applied by `gs` (§8.4.5).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct GraphicsStateParams<'s> {
    /// The `/ExtGState` resource name.
    pub name: &'s Name,
    /// The dictionary's object, when indirect.
    pub object: Option<ObjRef>,
    /// The dictionary.
    pub dict: &'s Dict,
    /// The graphics state after the dictionary was applied.
    pub state: &'s GraphicsState,
}

/// A text-showing operator (§9.4.3).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct TextShow<'s> {
    /// The string of `Tj`, `'` and `"`, or the array of `TJ`.
    pub operand: &'s Object,
    /// The graphics state, with the text state and CTM (`"` has already set `Tw` and `Tc`).
    pub state: &'s GraphicsState,
    /// The text matrix before the glyphs (`'` and `"` have already moved to the next line).
    pub tm: Matrix,
    /// The text line matrix.
    pub tlm: Matrix,
    /// The Form XObject nesting (0 for the page's own content).
    pub depth: usize,
}

impl<'s> TextShow<'s> {
    /// The current font (`None` before any `Tf`; see [`FontRef::dict`] for a missing one).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn font(&self) -> Option<&'s FontRef> {
        self.state.text.font.as_ref()
    }

    /// The strings and `TJ` adjustments in order (other array elements are skipped).
    ///
    /// Added in 0.2.0.
    pub fn items(&self) -> impl Iterator<Item = TextItem<'s>> + 's {
        let items: &'s [Object] = match self.operand {
            Object::Array(items) => items,
            single => std::slice::from_ref(single),
        };
        items.iter().filter_map(|item| match item {
            Object::String(s) => Some(TextItem::Text(&s.bytes)),
            Object::Integer(_) | Object::Real(_) => item.as_f64().map(TextItem::Adjust),
            _ => None,
        })
    }

    /// The text rendering matrix at the start of the show:
    /// `Trm = [Tfs·Th 0 0 Tfs 0 Trise] × Tm × CTM` (§9.4.4).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn rendering_matrix(&self) -> Matrix {
        self.state.text.text_space_matrix().then(&self.tm).then(&self.state.ctm)
    }
}

/// Where an image comes from.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ImageSource<'s> {
    /// An image XObject drawn by `Do` (not decoded).
    XObject {
        /// The `/XObject` resource name.
        name: &'s Name,
        /// The image's object, when indirect.
        object: Option<ObjRef>,
        /// The image stream (an [`Object::Stream`]).
        stream: &'s Object,
    },
    /// An inline image.
    Inline(&'s InlineImage<'s>),
}

/// An image painted into the unit square of user space (§8.9.4).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ImageDraw<'s> {
    /// The image.
    pub source: ImageSource<'s>,
    /// The graphics state (its CTM maps the unit square to the page).
    pub state: &'s GraphicsState,
    /// The resource dictionary the image is drawn with: where an inline image's named colour
    /// space (`/CS /CS0`) is defined.
    ///
    /// Added in 0.4.0.
    pub resources: Option<&'s Object>,
}

/// `sh`: a shading painted over the clip (§8.7.4.2).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ShadingPaint<'s> {
    /// The `/Shading` resource name.
    pub name: &'s Name,
    /// The shading's object, when indirect.
    pub object: Option<ObjRef>,
    /// The shading dictionary or stream.
    pub shading: &'s Object,
    /// The graphics state.
    pub state: &'s GraphicsState,
}

/// A marked-content sequence or point (§14.6).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MarkedContent<'s> {
    /// The tag, e.g. `P`, `Span`, `Artifact`, `OC`.
    pub tag: &'s Name,
    /// The property list: inline, or the `/Properties` resource it names (`None` for
    /// `BMC`/`MP`, or a missing resource).
    pub properties: Option<&'s Dict>,
    /// The `/Properties` resource name, when the property list was given by name (e.g. an
    /// optional content group for `/OC`).
    pub properties_name: Option<&'s Name>,
    /// The property list's `/MCID`, which links the content to the structure tree (§14.7.5).
    pub mcid: Option<i64>,
    /// The Form XObject nesting (0 for the page's own content).
    pub depth: usize,
}

/// A Form XObject drawn by `Do` (§8.10).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct FormXObject<'s> {
    /// The `/XObject` resource name.
    pub name: &'s Name,
    /// The form's object, when indirect.
    pub object: Option<ObjRef>,
    /// The form dictionary (`/Group`, `/OC`, `/Ref` and the rest are left to the visitor).
    pub dict: &'s Dict,
    /// `/Matrix` (identity when absent or malformed).
    pub matrix: Matrix,
    /// `/BBox` in form space, normalised (§7.9.5): the clip the form's content is drawn in
    /// (noted, not applied: the interpreter does not track clipping). `None` when missing or
    /// malformed (`invalid-entry`).
    pub bbox: Option<UserSpaceRect>,
    /// The nesting depth of this form (1 for a form drawn by the page's own content).
    pub depth: usize,
    /// The graphics state the form starts with (its matrix applied to the CTM).
    pub state: &'s GraphicsState,
}
