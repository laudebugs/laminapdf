//! Stamp appearances (ISO 32000-2 §12.5.6.12), upright as the page is shown. A standard stamp
//! is its name in capitals, drawn as paths from Lamina Sans Bold outlines ([`font`]), inside a
//! rounded frame of its colour: it needs no font, so every build draws it, and a PDF/A file's
//! fonts stay embedded. An image stamp is its picture fitted to the box. A dynamic stamp is its
//! lines in Helvetica, the first in bold, which needs the `freetext` feature.

mod font;
#[path = "generated/glyphs.rs"]
mod glyphs;

use laminapdf_cos::{CosMut, ObjRef, Object};
use laminapdf_model::UserSpaceRect;
use laminapdf_ops::{AppearanceRefusal, BlobStore, Stamp, StampKind, StampStyle};
use pdf_writer::Content;

use super::{DeviceColor, Drawn, Paint, upright};
use crate::image_writer;
use crate::surface::image::Image;
use crate::surface::n;

/// The frame's line width as a share of the stamp's height, and its bounds in points.
const LINE: (f64, f64, f64) = (0.06, 0.75, 4.0);
/// Space between the frame and the lettering, as a share of the stamp's height.
const PADDING: f64 = 0.12;
/// The largest share of the inner height a capital takes.
const CAPS: f64 = 0.62;

/// Whether this build can draw `stamp`.
///
/// # Errors
/// An image that isn't a JPEG or PNG; a dynamic stamp without the `freetext` feature, or with a
/// character Helvetica doesn't draw.
pub(super) fn check(stamp: &Stamp, blobs: Option<&BlobStore>) -> Result<(), AppearanceRefusal> {
    match &stamp.stamp {
        StampKind::Image(id) => {
            let bytes = blobs.and_then(|b| b.get(id)).ok_or(AppearanceRefusal::ImageInvalid)?;
            image_writer::decode(bytes)
                .map(drop)
                .ok_or(AppearanceRefusal::ImageInvalid)
        }
        StampKind::Dynamic(d) => dynamic::check(&d.lines),
        // Standard names and any kind a later version adds draw from what this one knows.
        _ => Ok(()),
    }
}

/// Draws `stamp` and writes its form into the overlay.
pub(super) fn draw(
    cos: &mut CosMut<'_>,
    stamp: &Stamp,
    paint: &Paint,
    blobs: Option<&BlobStore>,
) -> laminapdf_cos::Result<(ObjRef, UserSpaceRect)> {
    let frame = upright(stamp.rect, stamp.rotation);
    let mut drawn = Drawn::new(stamp.rect, paint.opacity, false);
    let c = &mut drawn.content;
    c.save_state();
    c.transform(frame.matrix.map(n));
    let (w, h) = (frame.width, frame.height);
    match &stamp.stamp {
        StampKind::Standard(name) => {
            let color = tone(paint, name.style());
            frame_box(c, w, h, color);
            lettering(c, name.lettering(), w, h);
        }
        StampKind::Dynamic(d) => {
            let color = tone(paint, d.style);
            frame_box(c, w, h, color);
            let fonts = dynamic::draw(c, &d.lines, w, h);
            drawn.fonts = dynamic::fonts(cos, fonts)?;
        }
        StampKind::Image(id) => {
            let image = blobs
                .and_then(|b| b.get(id))
                .and_then(|bytes| image_writer::decode(bytes));
            if let Some(image) = image
                && let Some(object) = image_writer::write(cos, &image)?
            {
                picture(c, &image, w, h);
                drawn.xobjects.insert("Im0", Object::Ref(object));
            }
        }
        // A kind a later version adds is drawn as an empty box rather than wrongly.
        _ => {}
    }
    c.restore_state();
    drawn.write(cos)
}

/// The stamp's colour: `/C` when given, else its style's.
fn tone(paint: &Paint, style: StampStyle) -> DeviceColor {
    paint.color_or(match style {
        StampStyle::Green => DeviceColor::Rgb(0.09, 0.47, 0.2),
        StampStyle::Red => DeviceColor::Rgb(0.76, 0.11, 0.11),
        StampStyle::Blue => DeviceColor::Rgb(0.11, 0.27, 0.66),
    })
}

/// The line width of a stamp `h` points high.
fn line_width(h: f64) -> f64 {
    (h * LINE.0).clamp(LINE.1, LINE.2)
}

/// The rounded frame inside `w` × `h`, stroked in `color`, which the lettering is then filled in.
fn frame_box(c: &mut Content, width: f64, height: f64, color: DeviceColor) {
    let lw = line_width(height);
    let (x0, y0, x1, y1) = (lw / 2.0, lw / 2.0, width - lw / 2.0, height - lw / 2.0);
    let r = (height.min(width) * 0.18)
        .min((x1 - x0) / 2.0)
        .min((y1 - y0) / 2.0)
        .max(0.0);
    let k = r * super::KAPPA;
    color.stroke(c);
    color.fill(c);
    c.set_line_width(n(lw));
    c.move_to(n(x0 + r), n(y0));
    c.line_to(n(x1 - r), n(y0));
    c.cubic_to(n(x1 - r + k), n(y0), n(x1), n(y0 + r - k), n(x1), n(y0 + r));
    c.line_to(n(x1), n(y1 - r));
    c.cubic_to(n(x1), n(y1 - r + k), n(x1 - r + k), n(y1), n(x1 - r), n(y1));
    c.line_to(n(x0 + r), n(y1));
    c.cubic_to(n(x0 + r - k), n(y1), n(x0), n(y1 - r + k), n(x0), n(y1 - r));
    c.line_to(n(x0), n(y0 + r));
    c.cubic_to(n(x0), n(y0 + r - k), n(x0 + r - k), n(y0), n(x0 + r), n(y0));
    c.close_path();
    c.stroke();
}

/// The room inside the frame: its width and height, and the inset on every side.
fn room(w: f64, h: f64) -> (f64, f64, f64) {
    let inset = line_width(h) + h * PADDING;
    ((w - 2.0 * inset).max(0.0), (h - 2.0 * inset).max(0.0), inset)
}

/// `text` in capitals, as large as fits, centred in the frame.
fn lettering(c: &mut Content, text: &str, w: f64, h: f64) {
    let (room_w, room_h, _) = room(w, h);
    let at_one = font::width(text, 1.0);
    if at_one <= 0.0 {
        return;
    }
    let size = (room_w / at_one).min(room_h * CAPS / font::cap_height(1.0)).max(0.0);
    let (tw, cap) = (font::width(text, size), font::cap_height(size));
    font::trace(c, text, size, (w - tw) / 2.0, (h - cap) / 2.0);
    c.fill_nonzero();
}

/// `image` fitted to `w` × `h` with its proportions kept, centred, upright as its file says.
fn picture(c: &mut Content, image: &Image, w: f64, h: f64) {
    let (iw, ih) = (f64::from(image.width()), f64::from(image.height()));
    let (iw, ih) = if image.orientation().swaps_sides() {
        (ih, iw)
    } else {
        (iw, ih)
    };
    if iw <= 0.0 || ih <= 0.0 {
        return;
    }
    let scale = (w / iw).min(h / ih);
    let (dw, dh) = (iw * scale, ih * scale);
    c.save_state();
    c.transform([n(dw), 0.0, 0.0, n(dh), n((w - dw) / 2.0), n((h - dh) / 2.0)]);
    c.transform(image_writer::upright_unit(image).to_array().map(n));
    c.x_object(pdf_writer::Name(b"Im0"));
    c.restore_state();
}

/// Dynamic stamps' text in the standard 14 Helvetica faces.
#[cfg(feature = "freetext")]
mod dynamic {
    use laminapdf_cos::{CosMut, Dict, Name, Object};
    use laminapdf_ops::AppearanceRefusal;
    use laminapdf_text::{Encoding, Standard14};
    use pdf_writer::{Content, Name as PdfName, Str};

    use super::room;
    use crate::annot::appearance::freetext::win_ansi;
    use crate::surface::n;

    /// The other lines' size against the first's.
    const SMALL: f64 = 0.5;
    /// From one baseline to the next, in font sizes.
    const LEADING: f64 = 1.15;
    /// Helvetica's cap height, in font sizes (its AFM's `CapHeight`).
    const CAP: f64 = 0.718;

    pub(super) fn check(lines: &[String]) -> Result<(), AppearanceRefusal> {
        match lines
            .iter()
            .flat_map(|l| l.chars())
            .find(|&ch| !Standard14::draws_nothing(ch) && win_ansi(ch).is_none())
        {
            Some(_) => Err(AppearanceRefusal::GlyphMissing),
            None => Ok(()),
        }
    }

    fn face(first: bool) -> Standard14 {
        if first {
            Standard14::HelveticaBold
        } else {
            Standard14::Helvetica
        }
    }

    /// A line's codes and its width at size 1.
    fn encode(line: &str, face: Standard14) -> (Vec<u8>, f64) {
        let codes: Vec<u8> = line
            .chars()
            .filter(|&ch| !Standard14::draws_nothing(ch))
            .filter_map(win_ansi)
            .collect();
        let width = codes
            .iter()
            .map(|&code| {
                f64::from(
                    face.width(Encoding::WinAnsi.glyph_name(code).unwrap_or("space"))
                        .unwrap_or(0),
                )
            })
            .sum::<f64>()
            / 1000.0;
        (codes, width)
    }

    /// Draws the lines centred in the frame, as large as fits; returns the faces used.
    pub(super) fn draw(c: &mut Content, lines: &[String], w: f64, h: f64) -> Vec<Standard14> {
        let (room_w, room_h, _) = room(w, h);
        let encoded: Vec<(Vec<u8>, f64, f64)> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let (codes, width) = encode(l, face(i == 0));
                (codes, width, if i == 0 { 1.0 } else { SMALL })
            })
            .collect();
        let widest = encoded.iter().map(|(_, wd, s)| wd * s).fold(0.0, f64::max);
        // Each line's cap height, and a line's leading below every line but the last.
        let tall: f64 = encoded.iter().map(|(_, _, s)| s * CAP).sum::<f64>()
            + encoded.iter().skip(1).map(|(_, _, s)| s * (LEADING - CAP)).sum::<f64>();
        if widest <= 0.0 || tall <= 0.0 {
            return Vec::new();
        }
        let size = (room_w / widest).min(room_h / tall);
        // The first line's capitals start at the top of the block, centred in the frame.
        let mut top = f64::midpoint(h, tall * size);
        c.begin_text();
        for (i, (codes, width, share)) in encoded.iter().enumerate() {
            let s = size * share;
            if i > 0 {
                top -= s * (LEADING - CAP);
            }
            let baseline = top - s * CAP;
            c.set_font(PdfName(if i == 0 { b"S0" } else { b"S1" }), n(s));
            c.set_text_matrix([1.0, 0.0, 0.0, 1.0, n((w - width * s) / 2.0), n(baseline)]);
            c.show(Str(codes));
            top = baseline;
        }
        c.end_text();
        let mut faces = vec![face(true)];
        if lines.len() > 1 {
            faces.push(face(false));
        }
        faces
    }

    /// The `/Font` resources: `S0` for the bold face, `S1` for the regular one.
    pub(super) fn fonts(cos: &mut CosMut<'_>, faces: Vec<Standard14>) -> laminapdf_cos::Result<Dict> {
        let mut out = Dict::new();
        for (i, face) in faces.into_iter().enumerate() {
            let font: Dict = [
                ("Type", Object::from(Name::from("Font"))),
                ("Subtype", Name::from("Type1").into()),
                ("BaseFont", Name::from(face.base_font()).into()),
                ("Encoding", Name::from("WinAnsiEncoding").into()),
            ]
            .into_iter()
            .collect();
            out.insert(
                Name::from(format!("S{i}").as_str()),
                Object::Ref(cos.create(Object::Dict(font))?),
            );
        }
        Ok(out)
    }
}

/// Without the `freetext` feature a dynamic stamp can't be laid out.
#[cfg(not(feature = "freetext"))]
mod dynamic {
    use laminapdf_cos::{CosMut, Dict};
    use laminapdf_ops::AppearanceRefusal;
    use pdf_writer::Content;

    pub(super) fn check(_: &[String]) -> Result<(), AppearanceRefusal> {
        Err(AppearanceRefusal::NeedsText)
    }

    /// No faces: `check` refused every dynamic stamp before anything was drawn.
    pub(super) const fn draw(_: &mut Content, _: &[String], _: f64, _: f64) -> Vec<core::convert::Infallible> {
        Vec::new()
    }

    #[expect(clippy::unnecessary_wraps, reason = "the same signature as the freetext build's")]
    pub(super) fn fonts(_: &mut CosMut<'_>, _: Vec<core::convert::Infallible>) -> laminapdf_cos::Result<Dict> {
        Ok(Dict::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laminapdf_ops::StampName;

    const PAINT: Paint = Paint {
        color: None,
        opacity: 1.0,
        intent: None,
    };

    fn content(stamp: &Stamp) -> String {
        let frame = upright(stamp.rect, stamp.rotation);
        let mut c = Content::new();
        frame_box(&mut c, frame.width, frame.height, tone(&PAINT, StampStyle::Green));
        if let StampKind::Standard(name) = stamp.stamp {
            lettering(&mut c, name.lettering(), frame.width, frame.height);
        }
        String::from_utf8(c.finish().into_vec()).unwrap()
    }

    #[test]
    fn a_standard_stamp_strokes_its_frame_and_fills_its_name() {
        let stamp = Stamp::new(
            UserSpaceRect::from_corners(0.0, 0.0, 200.0, 50.0),
            StampKind::Standard(StampName::Approved),
        );
        let drawn = content(&stamp);
        assert!(drawn.contains("\nS\n") && drawn.ends_with('f'));
    }

    #[test]
    fn the_lettering_fits_the_frame() {
        for name in StampName::ALL {
            let (w, h) = (150.0, 40.0);
            let (room_w, room_h, _) = room(w, h);
            let at_one = font::width(name.lettering(), 1.0);
            let size = (room_w / at_one).min(room_h * CAPS / font::cap_height(1.0));
            assert!(font::width(name.lettering(), size) <= room_w + 1e-9, "{name:?}");
        }
    }
}
