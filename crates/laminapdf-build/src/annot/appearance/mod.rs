//! Appearance streams for the annotations the editor makes (ISO 32000-2 §12.5.5): each is a
//! Form XObject drawn in user space, so its `/BBox` is the annotation's `/Rect` and it needs no
//! `/Matrix`. Opacity and blending live here, in the form's ExtGState, so no reader applies
//! them twice (E4 plan §2.1).
//!
//! The generators are deterministic: the same input writes the same bytes on every platform.

mod markup;
mod note;

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object, Stream};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use pdf_writer::Content;

use laminapdf_ops::{AnnotColor, AnnotSpec, IntentSpace};
use markup::markup;
use note::{caret, note_icon};

use crate::page::rect_object;
use crate::surface::n;

/// Draws the appearances of the annotations `laminapdf-ops` makes: text markup, notes and
/// carets. Give it to a document with
/// [`Document::with_appearances`](laminapdf_ops::Document::with_appearances); the `laminapdf`
/// façade does so for every document it opens.
///
/// ```
/// use laminapdf_build::AnnotationAppearances;
///
/// # fn open(bytes: Vec<u8>) -> Result<(), laminapdf_ops::Error> {
/// let doc = laminapdf_ops::Document::open(bytes, Default::default())?.with_appearances(AnnotationAppearances);
/// # drop(doc);
/// # Ok(())
/// # }
/// ```
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AnnotationAppearances;

impl laminapdf_ops::Appearances for AnnotationAppearances {
    fn normal(
        &self,
        cos: &mut CosMut<'_>,
        request: &laminapdf_ops::AppearanceRequest<'_>,
    ) -> laminapdf_cos::Result<laminapdf_ops::Appearance> {
        let paint = Paint {
            color: request.color.and_then(DeviceColor::of),
            opacity: request.opacity.get(),
            intent: request.intent,
        };
        let drawn = draw(request.spec, &paint);
        let (form, rect) = drawn.write(cos)?;
        Ok(laminapdf_ops::Appearance::new(form, rect))
    }
}

/// The drawing for `spec`. A markup's quads are never empty and a spec is never one this
/// version doesn't know (the op checks both, and is built against this version's specs), but
/// either still gets an empty form at the origin rather than none.
fn draw(spec: &AnnotSpec, paint: &Paint) -> Drawn {
    let empty = || Drawn::new(UserSpaceRect::from_corners(0.0, 0.0, 0.0, 0.0), 1.0, false);
    match spec {
        AnnotSpec::TextMarkup(m) => markup(m.kind, &m.quads, paint).unwrap_or_else(empty),
        AnnotSpec::Note(n) => note_icon(n.icon, n.at.x, n.at.y - laminapdf_ops::Note::ICON, paint),
        AnnotSpec::Caret(c) => caret(c.symbol, c.rect, paint),
        _ => empty(),
    }
}

/// A device colour as `/C` stores it: one component is gray, three RGB, four CMYK.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DeviceColor {
    Gray(f64),
    Rgb(f64, f64, f64),
    Cmyk(f64, f64, f64, f64),
}

impl DeviceColor {
    /// The device colour of an annotation's `/C`.
    fn of(c: AnnotColor) -> Option<Self> {
        match c {
            AnnotColor::Gray(g) => Some(Self::Gray(g)),
            AnnotColor::Rgb(r, g, b) => Some(Self::Rgb(r, g, b)),
            AnnotColor::Cmyk(c, m, y, k) => Some(Self::Cmyk(c, m, y, k)),
            // A colour space this version doesn't know draws in the mark's own colour.
            _ => None,
        }
    }

    /// The same colour in CMYK, for a PDF/A file whose output intent is CMYK: the simple
    /// conversion (black from the darkest channel), since a device colour has no profile.
    pub(crate) fn to_cmyk(self) -> Self {
        match self {
            Self::Gray(g) => Self::Cmyk(0.0, 0.0, 0.0, 1.0 - g),
            Self::Rgb(r, g, b) => {
                let k = 1.0 - r.max(g).max(b);
                if k >= 1.0 {
                    return Self::Cmyk(0.0, 0.0, 0.0, 1.0);
                }
                let of = |c: f64| (1.0 - c - k) / (1.0 - k);
                Self::Cmyk(of(r), of(g), of(b), k)
            }
            cmyk @ Self::Cmyk(..) => cmyk,
        }
    }

    /// The same colour in RGB, for a PDF/A file whose output intent is RGB.
    pub(crate) fn to_rgb(self) -> Self {
        match self {
            Self::Gray(g) => Self::Rgb(g, g, g),
            Self::Cmyk(c, m, y, k) => {
                let of = |v: f64| (1.0 - v) * (1.0 - k);
                Self::Rgb(of(c), of(m), of(y))
            }
            rgb @ Self::Rgb(..) => rgb,
        }
    }

    fn fill(self, content: &mut Content) {
        match self {
            Self::Gray(g) => content.set_fill_gray(n(g)),
            Self::Rgb(r, g, b) => content.set_fill_rgb(n(r), n(g), n(b)),
            Self::Cmyk(c, m, y, k) => content.set_fill_cmyk(n(c), n(m), n(y), n(k)),
        };
    }

    fn stroke(self, content: &mut Content) {
        match self {
            Self::Gray(g) => content.set_stroke_gray(n(g)),
            Self::Rgb(r, g, b) => content.set_stroke_rgb(n(r), n(g), n(b)),
            Self::Cmyk(c, m, y, k) => content.set_stroke_cmyk(n(c), n(m), n(y), n(k)),
        };
    }
}

/// What every generator is given besides its geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Paint {
    /// `/C`; the generators' own default when absent.
    pub(crate) color: Option<DeviceColor>,
    /// 0 to 1.
    pub(crate) opacity: f64,
    /// The output intent's device space, on a PDF/A file (ISO 19005-2 §6.2.4.3).
    pub(crate) intent: Option<IntentSpace>,
}

impl Paint {
    /// The colour to draw in: `color` or `default`, in the intent's space.
    fn color_or(&self, default: DeviceColor) -> DeviceColor {
        let color = self.color.unwrap_or(default);
        match self.intent {
            Some(IntentSpace::Cmyk) => color.to_cmyk(),
            Some(IntentSpace::Rgb) => color.to_rgb(),
            None => color,
            // An intent space this version doesn't know: the colour as given, the closest it can do.
            Some(_) => color,
        }
    }

    /// Black, or what stands for it in the intent's space.
    fn black(&self) -> DeviceColor {
        Self { color: None, ..*self }.color_or(DeviceColor::Gray(0.0))
    }
}

/// An appearance drawn but not yet written: its content, the box it covers and its
/// ExtGState, if it has one.
pub(crate) struct Drawn {
    content: Content,
    bbox: UserSpaceRect,
    gs: Option<Dict>,
}

impl Drawn {
    /// Starts an appearance over `bbox`, setting the ExtGState `/GS0` first when it has one:
    /// opacity below 1, or `multiply` for a highlight.
    fn new(bbox: UserSpaceRect, opacity: f64, multiply: bool) -> Self {
        let mut content = Content::new();
        let translucent = opacity < 1.0;
        let gs = (translucent || multiply).then(|| {
            let mut gs = Dict::new();
            gs.insert("Type", Name::from("ExtGState"));
            if translucent {
                gs.insert("CA", Object::number(opacity));
                gs.insert("ca", Object::number(opacity));
            }
            if multiply {
                gs.insert("BM", Name::from("Multiply"));
            }
            content.set_parameters(pdf_writer::Name(b"GS0"));
            gs
        });
        Self { content, bbox, gs }
    }

    /// Writes the Form XObject into the overlay; returns it with the `/Rect` it covers.
    pub(crate) fn write(self, cos: &mut CosMut<'_>) -> laminapdf_cos::Result<(ObjRef, UserSpaceRect)> {
        let mut dict = Dict::new();
        dict.insert("Type", Name::from("XObject"));
        dict.insert("Subtype", Name::from("Form"));
        dict.insert("BBox", rect_object(&self.bbox));
        let mut resources = Dict::new();
        if let Some(gs) = self.gs {
            let mut states = Dict::new();
            states.insert("GS0", gs);
            resources.insert("ExtGState", states);
        }
        dict.insert("Resources", resources);
        let bytes = self.content.finish().into_vec();
        let form = cos.create(Object::Stream(Stream::flate(dict, &bytes)))?;
        Ok((form, self.bbox))
    }

    #[cfg(test)]
    pub(crate) fn content(self) -> Vec<u8> {
        self.content.finish().into_vec()
    }
}

/// The box around `points`, grown by `margin` on every side.
fn bounds(points: impl IntoIterator<Item = UserSpacePoint>, margin: f64) -> Option<UserSpaceRect> {
    let mut points = points.into_iter();
    let first = points.next()?;
    let [mut x0, mut y0, mut x1, mut y1] = [first.x, first.y, first.x, first.y];
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    Some(UserSpaceRect::from_corners(
        x0 - margin,
        y0 - margin,
        x1 + margin,
        y1 + margin,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_to_cmyk_takes_black_from_the_darkest_channel() {
        assert_eq!(
            DeviceColor::Rgb(1.0, 1.0, 0.0).to_cmyk(),
            DeviceColor::Cmyk(0.0, 0.0, 1.0, 0.0)
        );
    }

    #[test]
    fn black_rgb_converts_to_pure_k() {
        assert_eq!(
            DeviceColor::Rgb(0.0, 0.0, 0.0).to_cmyk(),
            DeviceColor::Cmyk(0.0, 0.0, 0.0, 1.0)
        );
    }

    #[test]
    fn cmyk_to_rgb_multiplies_out_black() {
        assert_eq!(
            DeviceColor::Cmyk(0.0, 0.0, 1.0, 0.5).to_rgb(),
            DeviceColor::Rgb(0.5, 0.5, 0.0)
        );
    }

    #[test]
    fn an_opaque_mark_has_no_extgstate() {
        let drawn = Drawn::new(UserSpaceRect::from_corners(0.0, 0.0, 1.0, 1.0), 1.0, false);
        assert!(drawn.gs.is_none());
    }

    #[test]
    fn a_highlight_multiplies_through_its_extgstate() {
        let drawn = Drawn::new(UserSpaceRect::from_corners(0.0, 0.0, 1.0, 1.0), 1.0, true);
        assert_eq!(
            drawn.gs.and_then(|gs| gs.get("BM").cloned()),
            Some(Object::from(Name::from("Multiply")))
        );
    }
}
