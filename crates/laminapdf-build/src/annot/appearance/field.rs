//! Form field widget appearances (ISO 32000-2 §12.7.4.3, the variable text rules; §12.7.5): a
//! widget's background and border from `/MK`, then its value inside `/Tx BMC … EMC`, upright in
//! the frame `/MK /R` turns and clipped to the box inside the border.
//!
//! Text fields, combo boxes and list boxes are laid out in the standard 14 face of the family
//! `/DA` names, as text boxes are ([`simple`]); with the `text` feature, a character those faces
//! lack is drawn in a fallback font given by its bytes ([`shaped`]). Auto-size (a `/DA` size of
//! 0) follows pdf.js and PDFium, the round's references, since Acrobat's rules are undocumented:
//! a single line takes the box's height and shrinks to fit its width, a multi-line field the
//! largest size from 12 down at which its lines fit.

use laminapdf_cos::{CosMut, Dict, ObjRef};
use laminapdf_ops::{AnnotColor, AppearanceRefusal, FieldAppearanceRequest, FieldContent, RichText, TextAlign};
use laminapdf_text::Standard14;
use pdf_writer::{Content, Name as PdfName, Str};

#[cfg(feature = "text")]
use super::freetext::shaped;
use super::freetext::{FontNames, LEADING, ascent, simple};
use super::{DeviceColor, Drawn, Paint, upright};
use crate::surface::n;

/// Space between the border and the text, points (pdf.js's and PDFium's 2).
const PADDING: f64 = 2.0;
/// A single line's height in font sizes, for auto-size (pdf.js's `LINE_FACTOR`).
const LINE_FACTOR: f64 = 1.35;
/// The size a multi-line field or list box starts from when `/DA` asks for auto-size.
const AUTO_MAX: f64 = 12.0;
/// The smallest size auto-size goes down to.
const AUTO_MIN: f64 = 4.0;
/// A chosen row's highlight: Acrobat's and pdf.js's light blue.
const HIGHLIGHT: DeviceColor = DeviceColor::Rgb(0.6, 0.75, 0.86);
/// Helvetica's descender (its AFM file's), for a face whose metrics lack one.
const DEFAULT_DESCENDER: i16 = -207;
/// `ZapfDingbats`' check mark `4` (glyph `a20`), and its advance in thousandths of an em, for a
/// caption whose code the encoding lacks.
const CHECK_GLYPH: &str = "a20";
const CHECK_ADVANCE: u16 = 846;
/// A `ZapfDingbats` symbol's height in ems, about its cap height: what centres it vertically.
const SYMBOL_HEIGHT: f64 = 0.7;
/// What a password field shows for each character.
const BULLET: char = '\u{2022}';

/// Whether this build can draw `request`: the standard 14 tier draws values whose characters are
/// all in `WinAnsiEncoding`; fallback fonts need the `text` feature.
///
/// # Errors
/// Why it can't.
pub(super) fn check(request: &FieldAppearanceRequest<'_>) -> Result<(), AppearanceRefusal> {
    let Some(text) = text_of(request) else {
        return Ok(());
    };
    if !shaped_needed(request, &text) {
        return match simple::first_missing(&text) {
            Some(_) => Err(AppearanceRefusal::GlyphMissing),
            None => Ok(()),
        };
    }
    #[cfg(feature = "text")]
    {
        let synthetic = synthetic(request, &text, AUTO_MAX);
        let fonts = shaped::fonts(&synthetic, request.blobs)?;
        shaped::lay_out(&synthetic, &fonts, None).map(drop)
    }
    #[cfg(not(feature = "text"))]
    Err(AppearanceRefusal::NeedsText)
}

/// Draws `request`'s widget and writes its form; returns it with the fonts given by their bytes
/// it embeds, by name.
pub(super) fn draw(
    cos: &mut CosMut<'_>,
    request: &FieldAppearanceRequest<'_>,
) -> laminapdf_cos::Result<(ObjRef, Vec<(String, ObjRef)>)> {
    let paint = Paint {
        color: DeviceColor::of(request.color),
        opacity: 1.0,
        intent: request.intent,
    };
    let in_intent = |c: AnnotColor| {
        DeviceColor::of(c).map(|c| {
            Paint {
                color: Some(c),
                ..paint
            }
            .color_or(c)
        })
    };
    let r = request.rect;
    let mut drawn = Drawn::new(r, 1.0, false);
    let border = request
        .border
        .and_then(in_intent)
        .filter(|_| request.border_width > 0.0);
    let width = if border.is_some() { request.border_width } else { 0.0 };
    {
        let c = &mut drawn.content;
        if let Some(bg) = request.background.and_then(in_intent) {
            bg.fill(c);
            c.rect(n(r.x0), n(r.y0), n(r.x1 - r.x0), n(r.y1 - r.y0));
            c.fill_nonzero();
        }
        if let Some(bc) = border {
            bc.stroke(c);
            c.set_line_width(n(width));
            c.rect(
                n(r.x0 + width / 2.0),
                n(r.y0 + width / 2.0),
                n((r.x1 - r.x0 - width).max(0.0)),
                n((r.y1 - r.y0 - width).max(0.0)),
            );
            c.stroke();
        }
    }
    let frame = upright(r, request.rotation);
    let inset = width + PADDING;
    let mut fonts = FontNames::default();
    let mut embedded = Vec::new();
    let c = &mut drawn.content;
    c.begin_marked_content(PdfName(b"Tx"));
    c.save_state();
    c.transform(frame.matrix.map(n));
    // The text is clipped to the box inside the border (§12.7.4.3).
    c.rect(
        n(width),
        n(width),
        n((frame.width - 2.0 * width).max(0.0)),
        n((frame.height - 2.0 * width).max(0.0)),
    );
    c.clip_nonzero();
    c.end_path();
    let box_ = Inner {
        width: frame.width,
        height: frame.height,
        inset,
    };
    let color = paint.color_or(DeviceColor::Gray(0.0));
    match request.content {
        FieldContent::Button { caption, on } => {
            if on {
                color.fill(c);
                button(c, &box_, caption, request.size, &mut fonts);
            }
        }
        FieldContent::List { .. } => list(c, &box_, request, color, &mut fonts),
        _ => {
            color.fill(c);
            embedded = text(c, &box_, request, &mut fonts, cos)?;
        }
    }
    c.restore_state();
    c.end_marked_content();
    drawn.fonts = merge(fonts.write(cos)?, &embedded);
    let (form, _) = drawn.write(cos)?;
    Ok((form, embedded))
}

/// The box the content goes in, upright: its width and height, and the space kept inside the
/// border.
struct Inner {
    width: f64,
    height: f64,
    inset: f64,
}

impl Inner {
    /// The width text may take.
    fn room(&self) -> f64 {
        (self.width - 2.0 * self.inset).max(0.0)
    }

    /// The height text may take.
    fn tall(&self) -> f64 {
        (self.height - 2.0 * self.inset).max(0.0)
    }
}

/// The text a request shows, as the layout takes it: a password's bullets; a list's rows, each a
/// paragraph; `None` for a button.
fn text_of(request: &FieldAppearanceRequest<'_>) -> Option<RichText> {
    match request.content {
        FieldContent::Text { text, password, .. } => {
            let shown: String = if password {
                text.chars().map(|_| BULLET).collect()
            } else {
                text.to_owned()
            };
            Some(RichText::plain(&shown))
        }
        FieldContent::List { rows, .. } => Some(RichText::plain(&rows.join("\n"))),
        _ => None,
    }
}

/// Whether the value goes through the shaped layout: fonts are given, and a character the
/// standard 14 faces lack needs one, or the file claims PDF/A and every character must be drawn in
/// an embedded font. A list box's rows stay in the standard tier, so a row only a fallback draws
/// is refused (`annotation-glyph-missing`) and a PDF/A list box warns; a comb drawn through the
/// shaped layout is one line, without its cells.
fn shaped_needed(request: &FieldAppearanceRequest<'_>, text: &RichText) -> bool {
    !request.fallbacks.is_empty()
        && matches!(request.content, FieldContent::Text { .. })
        && (request.embed_fonts || simple::first_missing(text).is_some())
}

/// A single-line baseline that centres the face's ascent and descent in the box's height.
fn centred_baseline(face: Standard14, size: f64, height: f64) -> f64 {
    let ascent = ascent(face, size);
    let descent = -f64::from(face.metrics().descender.unwrap_or(DEFAULT_DESCENDER)) * size / 1000.0;
    f64::midpoint(height - ascent, descent)
}

/// Where a line `width` wide starts in `room` from `inset`.
fn aligned(align: TextAlign, inset: f64, room: f64, width: f64) -> f64 {
    match align {
        TextAlign::Center => inset + (room - width) / 2.0,
        TextAlign::Right => inset + room - width,
        TextAlign::Left => inset,
    }
}

/// Draws a text field's or combo box's text; returns the fonts given by their bytes it embeds.
#[cfg_attr(
    not(feature = "text"),
    expect(clippy::unnecessary_wraps, reason = "only fallback fonts, with `text`, can fail")
)]
fn text(
    c: &mut Content,
    b: &Inner,
    request: &FieldAppearanceRequest<'_>,
    fonts: &mut FontNames,
    cos: &mut CosMut<'_>,
) -> laminapdf_cos::Result<Vec<(String, ObjRef)>> {
    let FieldContent::Text { multiline, comb, .. } = request.content else {
        return Ok(Vec::new());
    };
    let Some(value) = text_of(request) else {
        return Ok(Vec::new());
    };
    #[cfg(feature = "text")]
    if shaped_needed(request, &value) {
        return shaped_text(c, b, request, &value, multiline, fonts, cos);
    }
    let _ = &cos;
    let face = simple::face(request.family, false, false);
    if let Some(cells) = comb {
        comb_text(c, b, request, &value, cells, face, fonts);
        return Ok(Vec::new());
    }
    let size = match request.size {
        Some(size) => size,
        None if multiline => fit_lines(b, |size| {
            simple::lay_out(&value, request.family, size, b.room()).map(|l| l.len())
        }),
        None => {
            let by_height = b.tall() / LINE_FACTOR;
            let width = simple::lay_out(&value, request.family, by_height, f64::INFINITY)
                .ok()
                .and_then(|lines| lines.first().map(|l| l.width))
                .unwrap_or(0.0);
            fit_width(by_height, width, b.room())
        }
    };
    let limit = if multiline { b.room() } else { f64::INFINITY };
    let Ok(lines) = simple::lay_out(&value, request.family, size, limit) else {
        // `check` refused a character no face has before the op wrote anything.
        return Ok(Vec::new());
    };
    let first = if multiline {
        b.height - b.inset - ascent(face, size)
    } else {
        centred_baseline(face, size, b.height)
    };
    c.begin_text();
    let mut current = None;
    for (i, line) in lines.iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "a field holds far fewer lines than 2^52")]
        let baseline = first - i as f64 * size * LEADING;
        let x = aligned(request.align, b.inset, b.room(), line.width);
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, n(x), n(baseline)]);
        for piece in &line.pieces {
            if current != Some(piece.face) {
                let name = fonts.standard(piece.face);
                c.set_font(PdfName(name.as_bytes()), n(size));
                current = Some(piece.face);
            }
            c.show(Str(&piece.bytes));
        }
    }
    c.end_text();
    Ok(Vec::new())
}

/// Auto-size on one line: `by_height`, the size the box's height gives, unless text `width` wide at
/// that size overflows `room`, then smaller in proportion.
fn fit_width(by_height: f64, width: f64, room: f64) -> f64 {
    if width > room && width > 0.0 {
        by_height * room / width
    } else {
        by_height
    }
}

/// The largest size from [`AUTO_MAX`] down, by halves, at which `lines(size)` lines fit the box's
/// height; [`AUTO_MIN`] when none does.
fn fit_lines(b: &Inner, lines: impl Fn(f64) -> Result<usize, char>) -> f64 {
    let mut size = AUTO_MAX;
    while size > AUTO_MIN {
        #[expect(clippy::cast_precision_loss, reason = "a field holds far fewer lines than 2^52")]
        let height = lines(size).map_or(f64::INFINITY, |count| count.max(1) as f64 * size * LEADING);
        if height <= b.tall() {
            return size;
        }
        size -= 0.5;
    }
    AUTO_MIN
}

/// A comb field: each character centred in its own of `cells` equal cells across the whole
/// width (§12.7.5.3), at most `cells` of them.
fn comb_text(
    c: &mut Content,
    b: &Inner,
    request: &FieldAppearanceRequest<'_>,
    value: &RichText,
    cells: u32,
    face: Standard14,
    fonts: &mut FontNames,
) {
    let cell = b.width / f64::from(cells);
    let chars = value.to_plain();
    // Auto-size: the box's height, then small enough that the widest character fits its cell.
    let size = request.size.unwrap_or_else(|| {
        let by_height = b.tall() / LINE_FACTOR;
        let widest = chars
            .chars()
            .filter_map(simple::win_ansi)
            .filter_map(|code| laminapdf_text::Encoding::WinAnsi.glyph_name(code))
            .filter_map(|name| face.width(name))
            .max()
            .map_or(0.0, |w| f64::from(w) / 1000.0);
        fit_width(by_height, widest * by_height, cell)
    });
    let baseline = centred_baseline(face, size, b.height);
    let name = fonts.standard(face);
    c.begin_text();
    c.set_font(PdfName(name.as_bytes()), n(size));
    for (i, ch) in chars
        .chars()
        .filter(|&ch| !Standard14::draws_nothing(ch))
        .take(cells as usize)
        .enumerate()
    {
        let Some(code) = simple::win_ansi(ch) else { continue };
        let glyph = laminapdf_text::Encoding::WinAnsi.glyph_name(code).unwrap_or("space");
        let width = f64::from(face.width(glyph).unwrap_or(0)) * size / 1000.0;
        #[expect(clippy::cast_precision_loss, reason = "a comb has far fewer cells than 2^52")]
        let x = i as f64 * cell + (cell - width) / 2.0;
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, n(x), n(baseline)]);
        c.show(Str(&[code]));
    }
    c.end_text();
}

/// A list box: its rows from its top row, a chosen row on a highlight across the box.
fn list(c: &mut Content, b: &Inner, request: &FieldAppearanceRequest<'_>, color: DeviceColor, fonts: &mut FontNames) {
    let FieldContent::List { rows, selected, top } = request.content else {
        return;
    };
    let size = request.size.unwrap_or(AUTO_MAX);
    let step = size * LEADING;
    let face = simple::face(request.family, false, false);
    let name = fonts.standard(face);
    for (shown, (index, row)) in rows.iter().enumerate().skip(top).enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "a list shows far fewer rows than 2^52")]
        let row_top = b.height - b.inset - shown as f64 * step;
        if row_top <= b.inset {
            break;
        }
        if selected.contains(&index) {
            HIGHLIGHT.fill(c);
            c.rect(n(b.inset), n(row_top - step), n(b.room()), n(step));
            c.fill_nonzero();
        }
        let Ok(lines) = simple::lay_out(&RichText::plain(row), request.family, size, f64::INFINITY) else {
            continue;
        };
        let Some(line) = lines.first() else { continue };
        color.fill(c);
        c.begin_text();
        c.set_font(PdfName(name.as_bytes()), n(size));
        let x = aligned(request.align, b.inset, b.room(), line.width);
        let baseline = row_top - ascent(face, size) - (step - size) / 2.0;
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, n(x), n(baseline)]);
        for piece in &line.pieces {
            c.show(Str(&piece.bytes));
        }
        c.end_text();
    }
}

/// A check box's or radio button's on state: `caption` in `ZapfDingbats`, centred, at `size` or
/// as large as fits.
fn button(c: &mut Content, b: &Inner, caption: char, size: Option<f64>, fonts: &mut FontNames) {
    let face = Standard14::ZapfDingbats;
    let Ok(code) = u8::try_from(u32::from(caption)) else {
        return;
    };
    let glyph = face.builtin_encoding().glyph_name(code).unwrap_or(CHECK_GLYPH);
    let advance = f64::from(face.width(glyph).unwrap_or(CHECK_ADVANCE)) / 1000.0;
    let size = size.unwrap_or_else(|| (b.width.min(b.height) - 2.0 * b.inset).max(1.0) * 0.8 / advance.max(0.5));
    let name = fonts.standard(face);
    c.begin_text();
    c.set_font(PdfName(name.as_bytes()), n(size));
    let x = (b.width - advance * size) / 2.0;
    let baseline = (b.height - size * SYMBOL_HEIGHT) / 2.0;
    c.set_text_matrix([1.0, 0.0, 0.0, 1.0, n(x), n(baseline)]);
    c.show(Str(&[code]));
    c.end_text();
}

/// The `/Font` resources with the fonts given by their bytes added under their names.
fn merge(mut fonts: Dict, embedded: &[(String, ObjRef)]) -> Dict {
    for (name, r) in embedded {
        fonts.insert(laminapdf_cos::Name::from(name.as_str()), laminapdf_cos::Object::Ref(*r));
    }
    fonts
}

/// The text box the shaped layout reads: the value in the field's family at `size`, with its
/// fallbacks and alignment; when every character must be embedded, in the first fallback with
/// the rest after it.
#[cfg(feature = "text")]
fn synthetic(request: &FieldAppearanceRequest<'_>, text: &RichText, size: f64) -> laminapdf_ops::FreeText {
    let (font, fallbacks) = match request.fallbacks.split_first() {
        Some((first, rest)) if request.embed_fonts => (laminapdf_ops::AnnotFont::Font(*first), rest.to_vec()),
        _ => (
            laminapdf_ops::AnnotFont::Standard(request.family),
            request.fallbacks.to_vec(),
        ),
    };
    laminapdf_ops::FreeText::new(request.rect, text.clone())
        .with_font(font, size)
        .with_fallbacks(fallbacks)
        .with_align(request.align)
}

/// A text field's text in its family with fallback fonts, through `laminapdf-text`'s layout;
/// returns the embedded fonts by the names its resources give them.
#[cfg(feature = "text")]
fn shaped_text(
    c: &mut Content,
    b: &Inner,
    request: &FieldAppearanceRequest<'_>,
    value: &RichText,
    multiline: bool,
    fonts: &mut FontNames,
    cos: &mut CosMut<'_>,
) -> laminapdf_cos::Result<Vec<(String, ObjRef)>> {
    use super::freetext::{ShapedText, baselines, draw_shaped_line, text_error};
    let refused = |_| laminapdf_cos::Error::InvalidStructure {
        kind: "form field",
        message: "its value can't be laid out in its fonts",
    };
    let width = multiline.then_some(b.room());
    let size = match request.size {
        Some(size) => size,
        None if multiline => fit_lines(b, |size| {
            let synthetic = synthetic(request, value, size);
            shaped::fonts(&synthetic, request.blobs)
                .and_then(|list| shaped::lay_out(&synthetic, &list, width))
                .map(|(_, lines)| lines.len())
                .map_err(|_| ' ')
        }),
        None => {
            // The box's height, then narrower to fit its width, as the standard tier does.
            let by_height = b.tall() / LINE_FACTOR;
            let probe = synthetic(request, value, by_height);
            let width = shaped::fonts(&probe, request.blobs)
                .and_then(|list| shaped::lay_out(&probe, &list, None))
                .ok()
                .and_then(|(_, lines)| lines.first().map(|l| l.width))
                .unwrap_or(0.0);
            fit_width(by_height, width, b.room())
        }
    };
    let synthetic = synthetic(request, value, size);
    let list = shaped::fonts(&synthetic, request.blobs).map_err(refused)?;
    let (joined, lines) = shaped::lay_out(&synthetic, &list, width).map_err(refused)?;
    let (mut registered, refs) = shaped::register(&list, cos).map_err(text_error)?;
    let mut embedded = Vec::new();
    for (i, r) in refs.into_iter().enumerate() {
        fonts.embedded(i, r);
        embedded.push((FontNames::embedded_name(i), r));
    }
    let face = simple::face(request.family, false, false);
    let first = if multiline {
        b.height - b.inset - lines.first().map_or(size, |l| l.ascent)
    } else {
        centred_baseline(face, size, b.height)
    };
    c.begin_text();
    for (line, baseline) in lines.iter().zip(baselines(&lines, first, size)) {
        let x = if multiline {
            b.inset + line.x
        } else {
            aligned(request.align, b.inset, b.room(), line.width)
        };
        let mut shaped = ShapedText {
            joined: &joined,
            list: &list,
            registered: &mut registered,
            fonts,
            size,
        };
        draw_shaped_line(c, &mut shaped, line, x, baseline)?;
    }
    c.end_text();
    registered.registry.finish_fonts(cos).map_err(text_error)?;
    Ok(embedded)
}

/// `request`'s family, as a standard face, regular.
#[cfg(test)]
fn regular(family: laminapdf_ops::StandardFamily) -> Standard14 {
    simple::face(family, false, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_centred_baseline_puts_the_middle_of_the_face_at_the_middle_of_the_box() {
        let face = regular(laminapdf_ops::StandardFamily::Helvetica);
        let baseline = centred_baseline(face, 10.0, 20.0);
        let middle = baseline + (ascent(face, 10.0) - 2.07) / 2.0;
        assert!((middle - 10.0).abs() < 1e-9, "{middle}");
    }

    #[test]
    fn a_line_wider_than_its_room_shrinks_in_proportion() {
        assert_eq!(
            [fit_width(20.0, 300.0, 150.0), fit_width(20.0, 100.0, 150.0)],
            [10.0, 20.0]
        );
    }

    #[test]
    fn auto_size_finds_the_largest_size_whose_lines_fit() {
        let b = Inner {
            width: 100.0,
            height: 30.0,
            inset: 2.0,
        };
        // Two lines at any size: they fit once 2 × size × 1.2 ≤ 26.
        assert!((fit_lines(&b, |_| Ok(2)) - 10.5).abs() < 1e-9);
    }
}
