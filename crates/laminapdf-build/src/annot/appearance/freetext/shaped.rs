//! Text boxes in any font: `laminapdf-text`'s layout (bidi, shaping, UAX #14 line breaking and
//! fallback fonts), with fonts given by their bytes embedded as subsets in the appearance's
//! resources. The standard 14 faces, when a box uses one with fallbacks, are encoded as
//! [`simple`](super::simple) encodes them. Bold and italic in a font given by its bytes are drawn
//! bolder (filled and outlined) and slanted (E4e).

use laminapdf_cos::CosMut;
use laminapdf_ops::{AppearanceRefusal, BlobStore, FontRef, FreeText, RichText};
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
pub(super) enum Face {
    /// A standard 14 face, encoded a byte per character.
    Standard(Standard14),
    /// A font given by its bytes, by its index among the box's embedded fonts, drawn bold and
    /// italic as the flags say.
    Embedded { font: usize, bold: bool, italic: bool },
}

/// The box's fonts: the layout's list and what each entry is.
pub(super) struct Fonts {
    pub(super) list: Vec<Font>,
    pub(super) faces: Vec<Face>,
    /// The fonts given by their bytes, in the order `Face::Embedded` numbers them.
    pub(super) embedded: Vec<Font>,
}

/// Whether the box needs this module: a font given by its bytes, or a character the standard 14
/// faces lack when fallbacks are given.
pub(super) fn needed(text: &FreeText) -> bool {
    matches!(text.font, FontRef::Font(_)) || (!text.fallbacks.is_empty() && simple::first_missing(&text.text).is_some())
}

/// The box's fonts, each style of each listed once.
///
/// # Errors
/// `FontInvalid` for bytes that aren't a font or that its licence keeps from being embedded;
/// `NeedsText` when the session lacks the bytes (the op checks that first).
pub(super) fn fonts(text: &FreeText, blobs: Option<&BlobStore>) -> Result<Fonts, AppearanceRefusal> {
    let load = |id: &laminapdf_ops::BlobId| -> Result<Font, AppearanceRefusal> {
        let bytes = blobs.and_then(|b| b.get(id)).ok_or(AppearanceRefusal::NeedsText)?;
        let font = Font::from_bytes(bytes.clone()).map_err(|_| AppearanceRefusal::FontInvalid)?;
        // A licence that forbids embedding refuses here rather than when the subset is written.
        FontRegistry::new()
            .add(font.clone(), EmbedPolicy::Respect)
            .map_err(|_| AppearanceRefusal::FontInvalid)?;
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
        FontRef::Standard(family) => {
            for style in 0..STYLES {
                let face = simple::face(family, style & 1 == 1, style & 2 == 2);
                out.list.push(Font::standard(face));
                out.faces.push(Face::Standard(face));
            }
        }
        FontRef::Font(ref id) => push_embedded(load(id)?, &mut out),
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
            let from = joined.len();
            joined.push_str(&span.text);
            let list: Vec<usize> = (0..families).map(|f| f * STYLES + style).collect();
            spans.push(StyledSpan::new(from..joined.len()).with_fonts(list));
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

/// Lays the box's text out in `width` points.
///
/// # Errors
/// `GlyphMissing` when a character is in none of its span's fonts; `FontInvalid` when shaping
/// fails.
pub(super) fn lay_out(text: &FreeText, fonts: &Fonts, width: f64) -> Result<(String, Vec<Line>), AppearanceRefusal> {
    let (joined, spans) = spans(&text.text, fonts);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a box's width and size in points, far below f32::MAX"
    )]
    let options = LayoutOptions::new(width.max(1.0) as f32, text.size as f32).with_align(match text.align {
        laminapdf_ops::TextAlign::Center => Alignment::Center,
        laminapdf_ops::TextAlign::Right => Alignment::Right,
        laminapdf_ops::TextAlign::Left => Alignment::Left,
    });
    let lines = break_spans(&HarfRust, &fonts.list, &joined, &spans, &options).map_err(|e| match e {
        laminapdf_text::Error::MissingGlyph { .. } => AppearanceRefusal::GlyphMissing,
        _ => AppearanceRefusal::FontInvalid,
    })?;
    Ok((joined, lines))
}

/// The box's embedded fonts registered for the appearance, each referenced in the overlay.
pub(super) struct Registered {
    pub(super) registry: FontRegistry,
    pub(super) keys: Vec<FontKey>,
}

/// Registers `fonts`' embedded fonts and reserves their objects.
pub(super) fn register(
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
pub(super) fn pieces(
    run: &laminapdf_text::LineRun,
    text: &str,
    registered: &mut Registered,
    font: usize,
) -> Option<laminapdf_text::Result<Vec<PdfPiece>>> {
    let key = registered.keys.get(font).copied()?;
    Some(pdf_line_run(run, text, &mut registered.registry, key).map(|pdf| pdf.pieces))
}
