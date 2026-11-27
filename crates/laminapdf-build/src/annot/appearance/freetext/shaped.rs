//! Text boxes in any font: `laminapdf-text`'s layout (bidi, shaping, UAX #14 line breaking and
//! fallback fonts), with fonts given by their bytes embedded as subsets in the appearance's
//! resources. The standard 14 faces, when a box uses one with fallbacks, are encoded as
//! [`simple`](super::simple) encodes them. Bold and italic in a font given by its bytes are drawn
//! bolder (filled and outlined) and slanted.

use laminapdf_cos::CosMut;
use laminapdf_ops::{AnnotFont, AppearanceRefusal, BlobStore, FreeText, RichText};
use laminapdf_text::{
    Alignment, EmbedPolicy, Font, FontKey, FontRegistry, HarfRust, LayoutOptions, Line, PdfPiece, Standard14,
    StyledSpan, break_spans, pdf_line_run,
};

use super::simple;

/// The four styles each font is listed in, so that runs never mix them: regular, bold, italic,
/// bold italic.
const STYLES: usize = 4;

/// What one entry of the layout's font list is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::annot::appearance) enum Face {
    /// A standard 14 face, encoded a byte per character.
    Standard(Standard14),
    /// A font given by its bytes, by its index among the box's embedded fonts, drawn bold and
    /// italic as the flags say.
    Embedded { font: usize, bold: bool, italic: bool },
}

/// The box's fonts: the layout's list and what each entry is.
pub(in crate::annot::appearance) struct Fonts {
    pub(in crate::annot::appearance) list: Vec<Font>,
    pub(in crate::annot::appearance) faces: Vec<Face>,
    /// The fonts given by their bytes, in the order `Face::Embedded` numbers them.
    pub(in crate::annot::appearance) embedded: Vec<Font>,
}

/// Whether the box needs this module: a font given by its bytes, or a character the standard 14
/// faces lack when fallbacks are given.
pub(in crate::annot::appearance) fn needed(text: &FreeText) -> bool {
    matches!(text.font, AnnotFont::Font(_))
        || (!text.fallbacks.is_empty() && simple::first_missing(&text.text).is_some())
}

/// The box's fonts, each style of each listed once.
///
/// # Errors
/// `FontInvalid` for bytes that aren't a font or that its licence keeps from being embedded;
/// `NeedsText` when the session lacks the bytes (the op checks that first).
pub(in crate::annot::appearance) fn fonts(
    text: &FreeText,
    blobs: Option<&BlobStore>,
) -> Result<Fonts, AppearanceRefusal> {
    let load = |id: &laminapdf_ops::BlobId| -> Result<Font, AppearanceRefusal> {
        let bytes = blobs.and_then(|b| b.get(id)).ok_or(AppearanceRefusal::NeedsText)?;
        let font = Font::from_bytes(bytes.clone()).map_err(|_| AppearanceRefusal::FontInvalid)?;
        // A licence that forbids embedding refuses here rather than when the subset is written.
        FontRegistry::new()
            .add(font.clone(), EmbedPolicy::Respect)
            .map_err(|e| match e {
                laminapdf_text::Error::FontNotEmbeddable { .. } => AppearanceRefusal::FontNotEmbeddable,
                _ => AppearanceRefusal::FontInvalid,
            })?;
        Ok(font)
    };
    let mut out = Fonts {
        list: Vec::new(),
        faces: Vec::new(),
        embedded: Vec::new(),
    };
    let push_embedded = |font: Font, out: &mut Fonts| {
        let index = out.embedded.len();
        out.embedded.push(font.clone());
        for style in 0..STYLES {
            out.list.push(font.clone());
            out.faces.push(Face::Embedded {
                font: index,
                bold: style & 1 == 1,
                italic: style & 2 == 2,
            });
        }
    };
    match text.font {
        AnnotFont::Standard(family) => {
            for style in 0..STYLES {
                let face = simple::face(family, style & 1 == 1, style & 2 == 2);
                out.list.push(Font::standard(face));
                out.faces.push(Face::Standard(face));
            }
        }
        AnnotFont::Font(ref id) => push_embedded(load(id)?, &mut out),
        _ => return Err(AppearanceRefusal::NeedsText),
    }
    for id in &text.fallbacks {
        push_embedded(load(id)?, &mut out);
    }
    Ok(out)
}

/// The text as one string, paragraphs ending in `\n`, with a span per text span listing its
/// style's entry of each font in order.
fn spans(text: &RichText, fonts: &Fonts) -> (String, Vec<StyledSpan>) {
    let families = fonts.list.len() / STYLES;
    let mut joined = String::new();
    let mut spans = Vec::new();
    for (p, paragraph) in text.paragraphs.iter().enumerate() {
        let start = joined.len();
        for span in paragraph {
            if span.text.is_empty() {
                continue;
            }
            let style = usize::from(span.bold) + 2 * usize::from(span.italic);
            let every: Vec<usize> = (0..families).map(|f| f * STYLES + style).collect();
            // A standard 14 face draws only what `WinAnsiEncoding` encodes, though its AFM file
            // lists more glyphs: the rest goes to the fallbacks.
            let standard = matches!(fonts.faces.first(), Some(Face::Standard(_)));
            let mut rest = span.text.as_str();
            while let Some(first) = rest.chars().next() {
                let encodable = |ch: char| !standard || Standard14::draws_nothing(ch) || simple::win_ansi(ch).is_some();
                let kind = encodable(first);
                let end = rest.find(|ch: char| encodable(ch) != kind).unwrap_or(rest.len());
                let (piece, tail) = rest.split_at(end);
                let from = joined.len();
                joined.push_str(piece);
                let list = if kind {
                    every.clone()
                } else {
                    every.get(1..).unwrap_or_default().to_vec()
                };
                spans.push(StyledSpan::new(from..joined.len()).with_fonts(list));
                rest = tail;
            }
        }
        if p + 1 < text.paragraphs.len() {
            joined.push('\n');
            // The break belongs to the paragraph's last span, or to a span of its own when the
            // paragraph is empty.
            match spans.last_mut() {
                Some(last) if last.range.end == joined.len() - 1 && last.range.start >= start => {
                    last.range.end = joined.len();
                }
                _ => spans.push(StyledSpan::new(joined.len() - 1..joined.len())),
            }
        }
    }
    (joined, spans)
}

/// Lays the box's text out in `width` points, or without wrapping when `width` is `None`.
///
/// # Errors
/// `GlyphMissing` when a character is in none of its span's fonts; `FontInvalid` when shaping
/// fails.
pub(in crate::annot::appearance) fn lay_out(
    text: &FreeText,
    fonts: &Fonts,
    width: Option<f64>,
) -> Result<(String, Vec<Line>), AppearanceRefusal> {
    let (joined, spans) = spans(&text.text, fonts);
    // Unwrapped lines are laid out flush left; the caller aligns them in the box.
    let align = match (width, text.align) {
        (None, _) | (_, laminapdf_ops::TextAlign::Left) => Alignment::Left,
        (_, laminapdf_ops::TextAlign::Center) => Alignment::Center,
        (_, laminapdf_ops::TextAlign::Right) => Alignment::Right,
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a box's width and size in points, far below f32::MAX"
    )]
    let options = LayoutOptions::new(width.map_or(f32::MAX, |w| w.max(1.0) as f32), text.size as f32).with_align(align);
    let lines = break_spans(&HarfRust, &fonts.list, &joined, &spans, &options).map_err(|e| match e {
        laminapdf_text::Error::MissingGlyph { .. } => AppearanceRefusal::GlyphMissing,
        _ => AppearanceRefusal::FontInvalid,
    })?;
    Ok((joined, lines))
}

/// The box's embedded fonts registered for the appearance, each referenced in the overlay.
pub(in crate::annot::appearance) struct Registered {
    pub(in crate::annot::appearance) registry: FontRegistry,
    pub(in crate::annot::appearance) keys: Vec<FontKey>,
}

/// Registers `fonts`' embedded fonts and reserves their objects.
pub(in crate::annot::appearance) fn register(
    fonts: &Fonts,
    cos: &CosMut<'_>,
) -> laminapdf_text::Result<(Registered, Vec<laminapdf_cos::ObjRef>)> {
    let mut registry = FontRegistry::new();
    let mut keys = Vec::new();
    let mut refs = Vec::new();
    for font in &fonts.embedded {
        let key = registry.add(font.clone(), EmbedPolicy::Respect)?;
        refs.push(registry.reference(key, cos.document())?);
        keys.push(key);
    }
    Ok((Registered { registry, keys }, refs))
}

/// One run's pieces: `TJ` arrays of a font given by its bytes, with the `/ActualText` spans
/// `pdf_run` asks for. `None` when `font` was never registered, which [`register`] rules out.
pub(in crate::annot::appearance) fn pieces(
    run: &laminapdf_text::LineRun,
    text: &str,
    registered: &mut Registered,
    font: usize,
) -> Option<laminapdf_text::Result<Vec<PdfPiece>>> {
    let key = registered.keys.get(font).copied()?;
    Some(pdf_line_run(run, text, &mut registered.registry, key).map(|pdf| pdf.pieces))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_win_ansi_lacks_list_only_the_fallbacks() {
        // Helvetica's AFM file has Ł, but WinAnsiEncoding has no code for it. The list's fonts
        // only count here: two families of four styles, the first standard.
        let standard = Face::Standard(Standard14::Helvetica);
        let fallback = Face::Embedded {
            font: 0,
            bold: false,
            italic: false,
        };
        let fonts = Fonts {
            list: vec![Font::standard(Standard14::Helvetica); 2 * STYLES],
            faces: [[standard; STYLES], [fallback; STYLES]].concat(),
            embedded: Vec::new(),
        };
        let (joined, spans) = spans(&RichText::plain("aŁb"), &fonts);
        let lists: Vec<(&str, Vec<usize>)> = spans
            .iter()
            .map(|s| {
                (
                    joined.get(s.range.clone()).unwrap_or_default(),
                    s.fonts.clone().unwrap_or_default(),
                )
            })
            .collect();
        assert_eq!(lists, [("a", vec![0, 4]), ("Ł", vec![4]), ("b", vec![0, 4])]);
    }
}
