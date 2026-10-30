//! Text extraction, read side (todo B9b; ISO 32000-2 §9.4–9.10; research 03 §3–5, §8.1): a
//! page's text-showing operators to glyphs with the text each stands for.
//!
//! ```
//! # fn sample() -> Vec<u8> { std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/text-std14.pdf")).unwrap() }
//! use lamina_text::TextExtractor;
//!
//! let doc = lamina_model::Document::open(sample(), lamina_cos::OpenOptions::default())?;
//! let pages = doc.pages()?;
//! let page = pages.get(0).expect("a page");
//! let text = TextExtractor::new().page(&doc, &page)?;
//! assert_eq!(text.lines().first().map(String::as_str), Some("Courier: Hello, world! 0123456789"));
//! # Ok::<(), lamina_cos::Error>(())
//! ```
//!
//! [`TextExtractor::page`] runs the page through [`lamina_content::interpret_page`] and
//! decodes every shown string with its font: codes by the font's encoding or CMap, widths
//! from `/Widths`, `/W` and `/W2` (which move the text matrix, so the interpreter sees the
//! right positions across `Tj` sequences), and Unicode by ToUnicode, then encoding and glyph
//! names, then the embedded font's `cmap` (see `font.rs`). Each [`Glyph`] keeps its code,
//! text, origin and advance in user space, and size. [`PageText::lines`] joins them in
//! content order into lines, a new one where the baseline moves, with a space where the pen
//! jumps forward. Glyph boxes, runs, `ActualText` and MCIDs are todo B9d.
//!
//! Fonts are parsed once per [`TextExtractor`] (keyed by font object). Problems follow
//! ADR 0003 and are warnings on the document: `font-invalid` (a malformed font dictionary,
//! decoded with defaults), `cmap-invalid` (malformed CMap or ToUnicode entries skipped, an
//! unsupported predefined CMap, a `/UseCMap` chain past [`MAX_USECMAP_DEPTH`]) and
//! `unmapped-glyph` (a font with glyphs no rule maps; they extract as U+FFFD). Strict mode
//! turns each into an error; the content limits of `lamina-content` bound the work.

mod cmap;
mod font;

pub use cmap::{CMap, Code};
pub use font::MAX_USECMAP_DEPTH;

use lamina_content::{Matrix, TextAdvance, TextItem, TextShow, Visitor};
use lamina_cos::{Error, Result, Warning};
use lamina_syntax::{ObjRef, Object};

use font::FontCache;

/// One shown glyph.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Glyph {
    /// The text it stands for: possibly several characters (a ligature), possibly none (a
    /// glyph mapped to no text), U+FFFD when nothing maps it (`mapped` is `false`).
    pub text: String,
    /// Whether a rule mapped the code (else `text` is U+FFFD).
    pub mapped: bool,
    /// The character code.
    pub code: Code,
    /// The glyph origin in user space (the text rendering matrix's translation, §9.4.4).
    pub origin: (f64, f64),
    /// How far the glyph moved the pen, in user space (with `Tc`, `Tw` and `Th`).
    pub advance: (f64, f64),
    /// The size the glyph is drawn at in user space: `Tfs` scaled by the text and current
    /// transformation matrices.
    pub size: f64,
    /// Whether the font writes vertically.
    pub vertical: bool,
    /// The font's object, when the font dictionary is indirect.
    pub font: Option<ObjRef>,
}

/// The glyphs a page shows, in content order.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct PageText {
    /// Every glyph, in the order the content shows them.
    pub glyphs: Vec<Glyph>,
    /// Glyphs no rule mapped (their text is U+FFFD).
    pub unmapped: usize,
}

/// A baseline moving by more than this fraction of the glyph size starts a new line.
const LINE_SHIFT: f64 = 0.5;

/// The pen jumping forward by more than this fraction of the size inserts a space.
const SPACE_GAP: f64 = 0.25;

impl PageText {
    /// The page's text as lines, in content order: a new line where the baseline moves by
    /// more than half the glyph size (or the writing direction turns); a space where the pen
    /// jumps forward by more than a quarter of it between two glyphs that have none. Glyphs
    /// drawn backwards on one baseline (right-to-left runs in logical order) stay on the line.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut line = String::new();
        // The previous glyph's end, its writing direction (unit vector) and size.
        let mut previous: Option<(Point, Point, f64)> = None;
        for glyph in &self.glyphs {
            let direction = unit(glyph.advance)
                .or(previous.map(|(_, d, _)| d))
                .unwrap_or((1.0, 0.0));
            if let Some((end, prev_direction, size)) = previous {
                let delta = (glyph.origin.0 - end.0, glyph.origin.1 - end.1);
                let size = size.max(glyph.size).max(f64::EPSILON);
                let shift = (delta.1 * prev_direction.0 - delta.0 * prev_direction.1).abs();
                let turned = direction.0 * prev_direction.0 + direction.1 * prev_direction.1 < 0.9;
                if shift > LINE_SHIFT * size || turned {
                    lines.push(std::mem::take(&mut line));
                } else {
                    let gap = delta.0 * prev_direction.0 + delta.1 * prev_direction.1;
                    let spaced = line.ends_with(char::is_whitespace) || glyph.text.starts_with(char::is_whitespace);
                    if gap > SPACE_GAP * size && !spaced && !line.is_empty() {
                        line.push(' ');
                    }
                }
            }
            line.push_str(&glyph.text);
            let end = (glyph.origin.0 + glyph.advance.0, glyph.origin.1 + glyph.advance.1);
            previous = Some((end, direction, glyph.size));
        }
        lines.push(line);
        lines.retain(|l| !l.trim().is_empty());
        lines
    }
}

/// A point or vector `(x, y)` in user space.
type Point = (f64, f64);

fn unit((x, y): Point) -> Option<Point> {
    let length = x.hypot(y);
    (length > f64::EPSILON).then(|| (x / length, y / length))
}

/// Extracts text page by page, parsing each font once.
#[derive(Debug, Default)]
pub struct TextExtractor {
    fonts: FontCache,
}

impl TextExtractor {
    /// An extractor with an empty font cache. Use one per document: fonts are cached by
    /// object number.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The glyphs `page` of `doc` shows (its content and every Form XObject it draws).
    ///
    /// # Errors
    /// What [`lamina_content::interpret_page`] returns (the content limits, strict mode, an
    /// undecodable content stream), and [`Error::LimitExceeded`] or [`Error::Strict`] while
    /// reading a font.
    pub fn page(&mut self, doc: &lamina_model::Document, page: &lamina_model::Page<'_>) -> Result<PageText> {
        let mut collector = Collector {
            doc,
            fonts: &mut self.fonts,
            text: PageText::default(),
            error: None,
        };
        lamina_content::interpret_page(doc, page, &mut collector)?;
        match collector.error {
            Some(error) => Err(error),
            None => Ok(collector.text),
        }
    }
}

struct Collector<'a> {
    doc: &'a lamina_cos::Document,
    fonts: &'a mut FontCache,
    text: PageText,
    /// The first error met inside a visitor call (which can't return one); the walk goes on
    /// and [`TextExtractor::page`] returns it.
    error: Option<Error>,
}

impl Collector<'_> {
    fn show(&mut self, show: &TextShow<'_>) -> Result<TextAdvance> {
        let Some(font_ref) = show.font() else {
            return Ok(TextAdvance::default());
        };
        let Some(dict) = font_ref.dict.as_deref().and_then(Object::as_dict) else {
            return Ok(TextAdvance::default());
        };
        let font = self.fonts.get(self.doc, font_ref.object, dict)?;
        let state = &show.state.text;
        let vertical = font.vertical();
        let mut tm = show.tm;
        let mut total = TextAdvance::default();
        let mut unmapped = false;
        let step = |advance: TextAdvance, tm: &mut Matrix, total: &mut TextAdvance| {
            *tm = Matrix::translate(advance.tx, advance.ty).multiply(tm);
            *total = *total + advance;
        };
        for item in show.items() {
            match item {
                TextItem::Adjust(n) => {
                    let advance = state.glyph_advance(0.0, 0.0, n, false, vertical);
                    // `glyph_advance` adds `Tc` per glyph; an adjustment is no glyph.
                    let advance = if vertical {
                        TextAdvance {
                            tx: 0.0,
                            ty: advance.ty - state.char_spacing,
                        }
                    } else {
                        TextAdvance {
                            tx: advance.tx - state.char_spacing * state.horizontal_scaling,
                            ty: 0.0,
                        }
                    };
                    step(advance, &mut tm, &mut total);
                }
                TextItem::Text(bytes) => {
                    let mut rest = bytes;
                    while let Some((code, used)) = font.next_code(rest) {
                        rest = rest.get(used..).unwrap_or_default();
                        let (w0, w1) = font.displacement(code);
                        let advance = state.glyph_advance(
                            w0 / 1000.0,
                            w1 / 1000.0,
                            0.0,
                            font::ExtractFont::is_word_space(code),
                            vertical,
                        );
                        let user = tm.multiply(&show.state.ctm);
                        let trm = state.text_space_matrix().multiply(&user);
                        let (x0, y0) = user.transform_point(0.0, 0.0);
                        let (x1, y1) = user.transform_point(advance.tx, advance.ty);
                        let (text, mapped) = if let Some(text) = font.unicode(code) {
                            (text, true)
                        } else {
                            unmapped = true;
                            self.text.unmapped += 1;
                            (char::REPLACEMENT_CHARACTER.to_string(), false)
                        };
                        self.text.glyphs.push(Glyph {
                            text,
                            mapped,
                            code,
                            origin: (trm.e, trm.f),
                            advance: (x1 - x0, y1 - y0),
                            size: trm.c.hypot(trm.d),
                            vertical,
                            font: font_ref.object,
                        });
                        step(advance, &mut tm, &mut total);
                    }
                }
            }
        }
        if unmapped {
            self.doc.report(Warning::UnmappedGlyph {
                object: font_ref.object.map(|r| r.num),
            })?;
        }
        Ok(total)
    }
}

impl Visitor for Collector<'_> {
    fn show_text(&mut self, show: &TextShow<'_>) -> TextAdvance {
        match self.show(show) {
            Ok(advance) => advance,
            Err(error) => {
                self.error.get_or_insert(error);
                TextAdvance::default()
            }
        }
    }
}
