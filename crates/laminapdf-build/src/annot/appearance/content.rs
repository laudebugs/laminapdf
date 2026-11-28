//! The forms of objects added to pages as content (E6 plan §2.1): each in the object's own space
//! ([`ContentObjectSpec`]), which the op's matrix places. An image and a page of another PDF fill
//! the unit square, y down; a shape's path is in points, y down. Opacity is the op's, set around
//! the drawing on the page, so the form draws everything opaque.

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object, Stream};
use laminapdf_model::{Matrix, PointPt, RectPt};
use laminapdf_ops::{AppearanceRefusal, ContentObjectSpec, ContentRequest, PathSpec, Stroke, StrokeDash, TextBlock};
use pdf_writer::Content;

use super::{DeviceColor, KAPPA, Paint};
use crate::image_writer;
use crate::page_form::copy_page;
use crate::surface::n;

/// Whether `request` can be drawn: an image's bytes must be a JPEG or PNG image this version
/// reads, and a page must come with its PDF.
pub(super) fn check(request: &ContentRequest<'_>) -> Result<(), AppearanceRefusal> {
    match request.spec {
        ContentObjectSpec::Image { blob, .. } => request
            .blobs
            .get(blob)
            .and_then(|bytes| image_writer::decode(bytes))
            .map(|_| ())
            .ok_or(AppearanceRefusal::ImageInvalid),
        // The op opens the PDF before it asks, and refuses one it can't open
        // (`content-object-invalid`), as it does an image it can't read.
        ContentObjectSpec::Form { .. } if request.source.is_none() => Err(AppearanceRefusal::ImageInvalid),
        ContentObjectSpec::Text(block) => check_text(block, request),
        _ => Ok(()),
    }
}

/// The pixel size of the image `bytes` as it is shown, its orientation applied.
pub(super) fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let image = image_writer::decode(bytes)?;
    let (w, h) = (image.width(), image.height());
    Some(if image.orientation().swaps_sides() {
        (h, w)
    } else {
        (w, h)
    })
}

/// Writes `request`'s form into the overlay.
pub(super) fn draw(cos: &mut CosMut<'_>, request: &ContentRequest<'_>) -> laminapdf_cos::Result<ObjRef> {
    match request.spec {
        ContentObjectSpec::Image { blob, crop } => {
            let image = request.blobs.get(blob).and_then(|bytes| image_writer::decode(bytes));
            let Some(image) = image else {
                return Err(invalid("an added image's bytes aren't a JPEG or PNG image"));
            };
            let Some(object) = image_writer::write(cos, &image)? else {
                return Err(invalid("an added image can't be written"));
            };
            // The image's unit square, y up, shown upright in the object's unit square, y down.
            let to_object = image_writer::upright_unit(&image).then(&FLIP);
            unit_form(cos, "Im0", object, &to_object, crop.unwrap_or(ContentObjectSpec::UNIT))
        }
        ContentObjectSpec::Form { page, .. } => {
            let source = request
                .source
                .ok_or_else(|| invalid("an added page comes without its PDF"))?;
            let pages = source.pages()?;
            let index = usize::try_from(*page).unwrap_or(usize::MAX);
            let page = pages
                .get(index)
                .ok_or_else(|| invalid("an added page isn't in its PDF"))?;
            let mut copier = laminapdf_model::ObjectCopier::new(source.cos(), *cos.document().limits());
            let Some(copy) = copy_page(source, &page, cos, &mut copier, true).map_err(build_error)? else {
                return Err(invalid("an added page's content can't be read to its end"));
            };
            let (w, h) = (copy.width(), copy.height());
            // The page as shown, from its top-left corner, stretched to the unit square.
            let to_object = copy.to_page.then(&Matrix::scale(
                1.0 / w.max(f64::MIN_POSITIVE),
                1.0 / h.max(f64::MIN_POSITIVE),
            ));
            unit_form(cos, "P0", copy.object, &to_object, ContentObjectSpec::UNIT)
        }
        ContentObjectSpec::Shape { path, fill, stroke } => {
            let paint = Paint {
                color: None,
                opacity: 1.0,
                intent: request.intent,
            };
            let fill = fill.and_then(DeviceColor::of).map(|c| {
                Paint {
                    color: Some(c),
                    ..paint
                }
                .color_or(c)
            });
            let stroke = stroke.as_ref().and_then(|s| {
                DeviceColor::of(s.color).map(|c| {
                    (
                        Paint {
                            color: Some(c),
                            ..paint
                        }
                        .color_or(c),
                        s,
                    )
                })
            });
            let (content, bbox) = shape(path, fill, stroke);
            let mut dict = Dict::new();
            dict.insert("Type", Name::from("XObject"));
            dict.insert("Subtype", Name::from("Form"));
            dict.insert("BBox", rect(bbox));
            dict.insert("Resources", Dict::new());
            cos.create(Object::Stream(Stream::flate(dict, &content)))
        }
        ContentObjectSpec::Text(block) => draw_text(cos, block, request),
        _ => Err(invalid("an added object of a kind this version can't draw")),
    }
}

/// Added text as the text box the appearance generator draws: the block's own box, `(0, 0)` its
/// bottom-left corner, with no border, fill or callout, and its first font and fallbacks.
#[cfg(feature = "freetext")]
fn as_text_box(block: &TextBlock) -> laminapdf_ops::FreeText {
    use laminapdf_model::UserSpaceRect;
    use laminapdf_ops::{Border, RichText};
    let rect = UserSpaceRect::from_corners(0.0, 0.0, block.width, block.height);
    let text = RichText {
        paragraphs: block.paragraphs(),
    };
    laminapdf_ops::FreeText::new(rect, text)
        .with_font(block.font(), block.size)
        .with_fallbacks(block.fallbacks())
        .with_align(block.align)
        .with_border(Border::solid(0.0))
}

/// Whether this build can draw `block`: the standard 14 fonts need the `freetext` feature, fonts
/// given by their bytes and fallbacks the `text` feature, as for a text box.
#[cfg(feature = "freetext")]
fn check_text(block: &TextBlock, request: &ContentRequest<'_>) -> Result<(), AppearanceRefusal> {
    super::freetext::check(&as_text_box(block), Some(request.blobs))
}

#[cfg(not(feature = "freetext"))]
fn check_text(_: &TextBlock, _: &ContentRequest<'_>) -> Result<(), AppearanceRefusal> {
    Err(AppearanceRefusal::NeedsText)
}

/// Draws added text through the text box generator: shaped, wrapped and clipped to the block's
/// box, each font embedded as a subset with its `ToUnicode` map.
#[cfg(feature = "freetext")]
fn draw_text(cos: &mut CosMut<'_>, block: &TextBlock, request: &ContentRequest<'_>) -> laminapdf_cos::Result<ObjRef> {
    let paint = Paint {
        color: block.color.and_then(DeviceColor::of),
        opacity: 1.0,
        intent: request.intent,
    };
    let (form, _) = super::freetext::draw(cos, &as_text_box(block), &paint, Some(request.blobs))?;
    Ok(form)
}

#[cfg(not(feature = "freetext"))]
fn draw_text(_: &mut CosMut<'_>, _: &TextBlock, _: &ContentRequest<'_>) -> laminapdf_cos::Result<ObjRef> {
    Err(invalid("this build draws no text"))
}

/// The unit square's y flipped: y up to y down.
const FLIP: Matrix = Matrix::new([1.0, 0.0, 0.0, -1.0, 0.0, 1.0]);

/// A form over the unit square, clipped to `bbox`, that draws the XObject `object` as `name`
/// through `matrix`.
fn unit_form(
    cos: &mut CosMut<'_>,
    name: &str,
    object: ObjRef,
    matrix: &Matrix,
    bbox: RectPt,
) -> laminapdf_cos::Result<ObjRef> {
    let mut c = Content::new();
    c.save_state();
    c.transform(matrix.to_array().map(n));
    c.x_object(pdf_writer::Name(name.as_bytes()));
    c.restore_state();
    let mut xobjects = Dict::new();
    xobjects.insert(name, Object::Ref(object));
    let mut resources = Dict::new();
    resources.insert("XObject", xobjects);
    let mut dict = Dict::new();
    dict.insert("Type", Name::from("XObject"));
    dict.insert("Subtype", Name::from("Form"));
    dict.insert("BBox", rect(bbox));
    dict.insert("Resources", resources);
    cos.create(Object::Stream(Stream::flate(dict, &c.finish().into_vec())))
}

/// A shape's drawing and the box it covers, its stroke included.
fn shape(path: &PathSpec, fill: Option<DeviceColor>, stroke: Option<(DeviceColor, &Stroke)>) -> (Vec<u8>, RectPt) {
    let mut c = Content::new();
    if let Some((color, s)) = stroke {
        color.stroke(&mut c);
        c.set_line_width(n(s.width));
        match s.dash {
            StrokeDash::Dashed => {
                c.set_dash_pattern([n(3.0 * s.width), n(3.0 * s.width)], 0.0);
            }
            StrokeDash::Dotted => {
                c.set_line_cap(pdf_writer::types::LineCapStyle::RoundCap);
                c.set_dash_pattern([0.0, n(2.0 * s.width)], 0.0);
            }
            _ => {}
        }
        // A rectangle's corners are square; a polygon's sharp ones are rounded so they stay
        // inside the box.
        if !matches!(path, PathSpec::Rect { .. }) {
            c.set_line_join(pdf_writer::types::LineJoinStyle::RoundJoin);
        }
    }
    if let Some(color) = fill {
        color.fill(&mut c);
    }
    let bounds = match path {
        PathSpec::Line { from, to } => {
            c.move_to(n(from.x.get()), n(from.y.get()));
            c.line_to(n(to.x.get()), n(to.y.get()));
            span(&[*from, *to])
        }
        PathSpec::Rect { rect: r, radius } => {
            rounded(&mut c, r, *radius);
            *r
        }
        PathSpec::Ellipse { rect: r } => {
            ellipse(&mut c, r);
            *r
        }
        PathSpec::Polygon { points, closed } => {
            if let Some((first, rest)) = points.split_first() {
                c.move_to(n(first.x.get()), n(first.y.get()));
                for p in rest {
                    c.line_to(n(p.x.get()), n(p.y.get()));
                }
                if *closed {
                    c.close_path();
                }
            }
            span(points)
        }
        _ => RectPt::default(),
    };
    match (fill.is_some(), stroke.is_some()) {
        (true, true) => c.fill_nonzero_and_stroke(),
        (true, false) => c.fill_nonzero(),
        _ => c.stroke(),
    };
    // Half the stroke reaches past the path, a square corner's miter √2 times that: a whole
    // width holds both.
    let margin = stroke.map_or(0.0, |(_, s)| s.width);
    let bbox = RectPt::new(
        bounds.x0 - laminapdf_model::Pt::new(margin),
        bounds.y0 - laminapdf_model::Pt::new(margin),
        bounds.x1 + laminapdf_model::Pt::new(margin),
        bounds.y1 + laminapdf_model::Pt::new(margin),
    );
    (c.finish().into_vec(), bbox)
}

/// The box that holds `points`.
fn span(points: &[PointPt]) -> RectPt {
    let xs = points.iter().map(|p| p.x.get());
    let ys = points.iter().map(|p| p.y.get());
    let (x0, x1) = (
        xs.clone().fold(f64::INFINITY, f64::min),
        xs.fold(f64::NEG_INFINITY, f64::max),
    );
    let (y0, y1) = (
        ys.clone().fold(f64::INFINITY, f64::min),
        ys.fold(f64::NEG_INFINITY, f64::max),
    );
    RectPt::from_points(x0, y0, x1, y1)
}

/// A rectangle with its corners rounded by `radius`; square corners for 0.
fn rounded(c: &mut Content, r: &RectPt, radius: f64) {
    let (x0, y0, x1, y1) = (r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get());
    if radius <= 0.0 {
        c.rect(n(x0), n(y0), n(x1 - x0), n(y1 - y0));
        return;
    }
    let k = radius * (1.0 - KAPPA);
    c.move_to(n(x0 + radius), n(y0));
    c.line_to(n(x1 - radius), n(y0));
    c.cubic_to(n(x1 - k), n(y0), n(x1), n(y0 + k), n(x1), n(y0 + radius));
    c.line_to(n(x1), n(y1 - radius));
    c.cubic_to(n(x1), n(y1 - k), n(x1 - k), n(y1), n(x1 - radius), n(y1));
    c.line_to(n(x0 + radius), n(y1));
    c.cubic_to(n(x0 + k), n(y1), n(x0), n(y1 - k), n(x0), n(y1 - radius));
    c.line_to(n(x0), n(y0 + radius));
    c.cubic_to(n(x0), n(y0 + k), n(x0 + k), n(y0), n(x0 + radius), n(y0));
    c.close_path();
}

/// The ellipse that fits `r`, as four Bézier curves.
fn ellipse(c: &mut Content, r: &RectPt) {
    let (cx, cy) = (
        f64::midpoint(r.x0.get(), r.x1.get()),
        f64::midpoint(r.y0.get(), r.y1.get()),
    );
    let (rx, ry) = ((r.x1.get() - r.x0.get()) / 2.0, (r.y1.get() - r.y0.get()) / 2.0);
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    c.move_to(n(cx + rx), n(cy));
    c.cubic_to(n(cx + rx), n(cy + ky), n(cx + kx), n(cy + ry), n(cx), n(cy + ry));
    c.cubic_to(n(cx - kx), n(cy + ry), n(cx - rx), n(cy + ky), n(cx - rx), n(cy));
    c.cubic_to(n(cx - rx), n(cy - ky), n(cx - kx), n(cy - ry), n(cx), n(cy - ry));
    c.cubic_to(n(cx + kx), n(cy - ry), n(cx + rx), n(cy - ky), n(cx + rx), n(cy));
    c.close_path();
}

fn rect(r: RectPt) -> Object {
    Object::Array(
        [r.x0, r.y0, r.x1, r.y1]
            .iter()
            .map(|v| Object::number(v.get()))
            .collect(),
    )
}

fn invalid(message: &'static str) -> laminapdf_cos::Error {
    laminapdf_cos::Error::InvalidStructure {
        kind: "added content",
        message,
    }
}

fn build_error(e: crate::Error) -> laminapdf_cos::Error {
    match e {
        crate::Error::Cos(e) => e,
        _ => invalid("an added page can't be copied"),
    }
}
