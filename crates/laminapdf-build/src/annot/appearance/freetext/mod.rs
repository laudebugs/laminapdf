//! `FreeText` appearances (ISO 32000-2 §12.5.6.6): the box's fill and border, the callout's
//! leader line and ending, then the text, upright as the page is shown and clipped to the box
//! but for typewriter text, which never wraps and widens the drawing instead.
//! The text is laid out by [`simple`] in the standard 14 fonts, which every build that draws text
//! boxes has, or by [`shaped`] (feature `text`) for fonts given by their bytes and fallback
//! fonts, as the owner chose to keep the WebAssembly modules in budget.

#[cfg(feature = "text")]
mod shaped;
mod simple;

pub(super) use simple::win_ansi;

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object};
use laminapdf_model::UserSpaceRect;
use laminapdf_ops::{AnnotFont, AppearanceRefusal, BlobStore, FreeText, StandardFamily};
use laminapdf_text::Standard14;
#[cfg(feature = "text")]
use pdf_writer::types::TextRenderingMode;
use pdf_writer::{Content, Name as PdfName, Str};

use super::{DeviceColor, Frame, Paint};
use crate::surface::n;

/// Space between the border and the text, points.
const PADDING: f64 = 2.0;
/// Space between a box with no border, typewriter text among them, and its text: other apps
/// size such boxes to their text, so any more would wrap or clip it.
const BARE_PADDING: f64 = 1.0;
/// From one baseline to the next, in font sizes.
const LEADING: f64 = 1.2;
/// The slant of synthetic italic: tan 12°.
#[cfg(feature = "text")]
const SLANT: f64 = 0.212_556_56;
/// The outline width of synthetic bold, in font sizes.
#[cfg(feature = "text")]
const EMBOLDEN: f64 = 0.03;

/// Whether this build can draw `text`: the standard 14 tier draws boxes whose characters are all
/// in `WinAnsiEncoding`; a font given by its bytes, or fallbacks a character needs, need the
/// `text` feature.
///
/// # Errors
/// Why it can't.
pub(super) fn check(text: &FreeText, blobs: Option<&BlobStore>) -> Result<(), AppearanceRefusal> {
    #[cfg(feature = "text")]
    if shaped::needed(text) {
        let fonts = shaped::fonts(text, blobs)?;
        // Laid out at the box's width: a character no font has fails here, before anything is
        // written.
        return shaped::lay_out(text, &fonts, wraps(text).then(|| frame(text).width)).map(drop);
    }
    #[cfg(not(feature = "text"))]
    {
        let _ = blobs;
        if matches!(text.font, AnnotFont::Font(_)) {
            return Err(AppearanceRefusal::NeedsText);
        }
        if simple::first_missing(&text.text).is_some() && !text.fallbacks.is_empty() {
            return Err(AppearanceRefusal::NeedsText);
        }
    }
    match simple::first_missing(&text.text) {
        Some(_) => Err(AppearanceRefusal::GlyphMissing),
        None => Ok(()),
    }
}

/// Draws `text` and writes its form, with the fonts it uses, into the overlay.
pub(super) fn draw(
    cos: &mut CosMut<'_>,
    text: &FreeText,
    paint: &Paint,
    blobs: Option<&BlobStore>,
) -> laminapdf_cos::Result<(ObjRef, UserSpaceRect)> {
    let color = paint.color_or(DeviceColor::Gray(0.0));
    let mut drawn = super::drawing::freetext_frame(text, paint, color);
    let frame = frame(text);
    let inset = text.drawn_border().width.max(0.0) / 2.0;
    let r = text.rect;
    let c = &mut drawn.content;
    c.save_state();
    // Typewriter text isn't wrapped, so it isn't clipped either: the box grows to hold it.
    if wraps(text) {
        c.rect(
            n(r.x0 + inset),
            n(r.y0 + inset),
            n((r.x1 - r.x0 - 2.0 * inset).max(0.0)),
            n((r.y1 - r.y0 - 2.0 * inset).max(0.0)),
        );
        c.clip_nonzero();
        c.end_path();
    }
    c.transform(frame.matrix.map(n));
    color.fill(c);
    color.stroke(c);
    let mut fonts = FontNames::default();
    #[cfg(feature = "text")]
    if shaped::needed(text) {
        let (registered, extent) = text_tier(c, text, &frame, &mut fonts, blobs, cos)?;
        c.restore_state();
        grow(&mut drawn.bbox, text, &frame, extent);
        drawn.fonts = fonts.write(cos)?;
        let written = drawn.write(cos)?;
        if let Some(registered) = registered {
            registered.registry.finish_fonts(cos).map_err(text_error)?;
        }
        return Ok(written);
    }
    let _ = blobs;
    let extent = standard_tier(c, text, &frame, &mut fonts);
    c.restore_state();
    grow(&mut drawn.bbox, text, &frame, extent);
    drawn.fonts = fonts.write(cos)?;
    drawn.write(cos)
}

/// Whether the box's lines wrap at its width: everything but typewriter text, whose lines end
/// only where its paragraphs do.
fn wraps(text: &FreeText) -> bool {
    !matches!(text.intent, laminapdf_ops::FreeTextIntent::Typewriter)
}

/// What the text covers in the upright frame: `[x0, y0, x1, y1]`.
type Extent = Option<[f64; 4]>;

/// Takes in a line from `x` `width` wide, its baseline at `baseline`, reaching `descent` below.
fn cover(extent: &mut Extent, x: f64, width: f64, baseline: f64, descent: f64, top: f64) {
    let [x0, y0, x1, y1] = extent.unwrap_or([x, baseline - descent, x + width, top]);
    *extent = Some([x0.min(x), y0.min(baseline - descent), x1.max(x + width), y1.max(top)]);
}

/// Grows `bbox`, the form's box, to hold typewriter text that runs past the box; text that wraps
/// is clipped to the box and needs nothing.
fn grow(bbox: &mut UserSpaceRect, text: &FreeText, frame: &Frame, extent: Extent) {
    let (Some([x0, y0, x1, y1]), false) = (extent, wraps(text)) else {
        return;
    };
    let m = frame.matrix;
    for (u, v) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
        let (x, y) = (m[0] * u + m[2] * v + m[4], m[1] * u + m[3] * v + m[5]);
        *bbox = UserSpaceRect::from_corners(bbox.x0.min(x), bbox.y0.min(y), bbox.x1.max(x), bbox.y1.max(y));
    }
}

/// The upright frame of `text`'s box on a page shown turned by its `rotation` (clockwise).
fn frame(text: &FreeText) -> Frame {
    super::upright(text.rect, text.rotation)
}

/// The padding inside `text`'s border, points.
fn padding(text: &FreeText) -> f64 {
    let border = text.drawn_border().width;
    if border > 0.0 { border + PADDING } else { BARE_PADDING }
}

/// Where a line `width` points wide starts in a box `room` points wide.
fn aligned(text: &FreeText, room: f64, width: f64) -> f64 {
    match text.align {
        laminapdf_ops::TextAlign::Center => (room - width) / 2.0,
        laminapdf_ops::TextAlign::Right => room - width,
        laminapdf_ops::TextAlign::Left => 0.0,
    }
}

/// Draws the text in the standard 14 faces of the box's family.
fn standard_tier(c: &mut Content, text: &FreeText, frame: &Frame, fonts: &mut FontNames) -> Extent {
    let family = match text.font {
        AnnotFont::Standard(family) => family,
        _ => StandardFamily::Helvetica,
    };
    let pad = padding(text);
    let room = frame.width - 2.0 * pad;
    let limit = if wraps(text) { room } else { f64::INFINITY };
    let Ok(lines) = simple::lay_out(&text.text, family, text.size, limit) else {
        // `check` refused a character no face has before the op wrote anything.
        return None;
    };
    let face = simple::face(family, false, false);
    let ascent = ascent(face, text.size);
    let descent = -f64::from(face.metrics().descender.unwrap_or(-207)) * text.size / 1000.0;
    let mut extent = None;
    c.begin_text();
    let mut current = None;
    for (i, line) in lines.iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "a box holds far fewer lines than 2^52")]
        let baseline = frame.height - pad - ascent - i as f64 * text.size * LEADING;
        let x = pad + aligned(text, room, line.width);
        if line.pieces.is_empty() {
            continue;
        }
        cover(&mut extent, x, line.width, baseline, descent, frame.height);
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, n(x), n(baseline)]);
        for piece in &line.pieces {
            if current != Some(piece.face) {
                let name = fonts.standard(piece.face);
                c.set_font(PdfName(name.as_bytes()), n(text.size));
                current = Some(piece.face);
            }
            c.show(Str(&piece.bytes));
        }
    }
    c.end_text();
    extent
}

/// A standard 14 face's ascent at `size`, points.
fn ascent(face: Standard14, size: f64) -> f64 {
    f64::from(face.metrics().ascender.unwrap_or(718)) * size / 1000.0
}

/// Draws the text through `laminapdf-text`'s layout; returns the registry of the fonts given by
/// their bytes, which must be written after the form.
#[cfg(feature = "text")]
fn text_tier(
    c: &mut Content,
    text: &FreeText,
    frame: &Frame,
    fonts: &mut FontNames,
    blobs: Option<&BlobStore>,
    cos: &CosMut<'_>,
) -> laminapdf_cos::Result<(Option<shaped::Registered>, Extent)> {
    let refused = |_| laminapdf_cos::Error::InvalidStructure {
        kind: "text box",
        message: "its text can't be laid out in its fonts",
    };
    let list = shaped::fonts(text, blobs).map_err(refused)?;
    let pad = padding(text);
    let room = frame.width - 2.0 * pad;
    let (joined, lines) = shaped::lay_out(text, &list, wraps(text).then_some(room)).map_err(refused)?;
    let mut extent = None;
    let (mut registered, refs) = shaped::register(&list, cos).map_err(text_error)?;
    for (i, r) in refs.into_iter().enumerate() {
        fonts.embedded(i, r);
    }
    let mut baseline = frame.height - pad - lines.first().map_or(text.size, |l| l.ascent);
    c.begin_text();
    for line in &lines {
        // Unwrapped lines are laid out flush left and aligned here, in the box's width.
        let start = if wraps(text) {
            pad + line.x
        } else {
            pad + aligned(text, room, line.width)
        };
        if !line.runs.is_empty() {
            cover(&mut extent, start, line.width, baseline, -line.descent, frame.height);
        }
        for index in line.drawing_order() {
            let Some(run) = line.runs.get(index) else { continue };
            let x = start + run.x;
            match list.faces.get(run.font) {
                Some(shaped::Face::Standard(face)) => {
                    let bytes: Vec<u8> = joined
                        .get(run.run.text_range.clone())
                        .unwrap_or_default()
                        .chars()
                        .filter(|&ch| !Standard14::draws_nothing(ch))
                        .filter_map(win_ansi)
                        .collect();
                    let name = fonts.standard(*face);
                    c.set_font(PdfName(name.as_bytes()), n(text.size));
                    c.set_text_matrix([1.0, 0.0, 0.0, 1.0, n(x), n(baseline)]);
                    c.show(Str(&bytes));
                }
                Some(shaped::Face::Embedded { font, bold, italic }) => {
                    let pieces = shaped::pieces(run, &joined, &mut registered, *font)
                        .transpose()
                        .map_err(text_error)?
                        .unwrap_or_default();
                    let name = FontNames::embedded_name(*font);
                    c.set_font(PdfName(name.as_bytes()), n(text.size));
                    let slant = if *italic { SLANT } else { 0.0 };
                    c.set_text_matrix([1.0, 0.0, n(slant), 1.0, n(x), n(baseline)]);
                    if *bold {
                        c.set_text_rendering_mode(TextRenderingMode::FillStroke);
                        c.set_line_width(n(text.size * EMBOLDEN));
                    }
                    show_pieces(c, &pieces, text.size);
                    if *bold {
                        c.set_text_rendering_mode(TextRenderingMode::Fill);
                    }
                }
                None => {}
            }
        }
        baseline -= (line.ascent - line.descent).max(text.size * LEADING);
    }
    c.end_text();
    Ok((Some(registered), extent))
}

/// A run's pieces: `/ActualText` spans where they need one, `Ts` rises, `TJ` arrays.
#[cfg(feature = "text")]
fn show_pieces(c: &mut Content, pieces: &[laminapdf_text::PdfPiece], size: f64) {
    let mut rise = 0.0_f32;
    for piece in pieces {
        if let Some(actual) = &piece.actual_text {
            c.begin_marked_content_with_properties(PdfName(b"Span"))
                .properties()
                .actual_text(pdf_writer::TextStr(actual));
        }
        for segment in &piece.segments {
            let wanted = n(segment.rise * size / 1000.0);
            if wanted.to_bits() != rise.to_bits() {
                c.set_rise(wanted);
                rise = wanted;
            }
            let mut show = c.show_positioned();
            let mut items = show.items();
            for element in &segment.tj {
                match element {
                    laminapdf_text::TjElement::Codes(codes) => {
                        items.show(Str(codes));
                    }
                    laminapdf_text::TjElement::Adjust(amount) => {
                        items.adjust(n(*amount));
                    }
                }
            }
        }
        if piece.actual_text.is_some() {
            c.end_marked_content();
        }
    }
    if rise != 0.0 {
        c.set_rise(0.0);
    }
}

/// A text error as the overlay's: the overlay's own, or a structure that can't be written.
#[cfg(feature = "text")]
fn text_error(e: laminapdf_text::Error) -> laminapdf_cos::Error {
    match e {
        laminapdf_text::Error::Cos(e) => e,
        _ => laminapdf_cos::Error::InvalidStructure {
            kind: "text box",
            message: "its fonts can't be embedded",
        },
    }
}

/// The fonts a form uses, named in order of first use: `S0`, `S1` … for standard 14 faces and
/// `E0`, `E1` … for fonts given by their bytes.
#[derive(Default)]
struct FontNames {
    standard: Vec<Standard14>,
    embedded: Vec<(usize, ObjRef)>,
}

impl FontNames {
    fn standard(&mut self, face: Standard14) -> String {
        let index = self.standard.iter().position(|f| *f == face).unwrap_or_else(|| {
            self.standard.push(face);
            self.standard.len() - 1
        });
        format!("S{index}")
    }

    #[cfg(feature = "text")]
    fn embedded(&mut self, font: usize, r: ObjRef) {
        self.embedded.push((font, r));
    }

    #[cfg(feature = "text")]
    fn embedded_name(font: usize) -> String {
        format!("E{font}")
    }

    /// The `/Font` resources: a non-embedded `/Type1` font in `WinAnsiEncoding` per standard
    /// face, written now, and the reserved objects of the fonts given by their bytes.
    fn write(self, cos: &mut CosMut<'_>) -> laminapdf_cos::Result<Dict> {
        let mut out = Dict::new();
        for (i, face) in self.standard.into_iter().enumerate() {
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
        for (font, r) in self.embedded {
            out.insert(Name::from(format!("E{font}").as_str()), Object::Ref(r));
        }
        Ok(out)
    }
}
