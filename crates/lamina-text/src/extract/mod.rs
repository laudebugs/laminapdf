//! Text extraction, read side (todos B9b, B9d; ISO 32000-2 §9.4–9.10, §14.6, §14.9.4;
//! research 03 §2.6, §3–5, §8.1): a page's text-showing operators to glyphs with the text each
//! stands for, their boxes in page space, and runs of them.
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
//! assert_eq!(&*text.runs[0].font_name, "Courier");
//! # Ok::<(), lamina_cos::Error>(())
//! ```
//!
//! [`TextExtractor::page`] runs the page through [`lamina_content::interpret_page`] and
//! decodes every shown string with its font: codes by the font's encoding or CMap, widths
//! from `/Widths`, `/W` and `/W2` (which move the text matrix, so the interpreter sees the
//! right positions across `Tj` sequences), and Unicode by ToUnicode, then encoding and glyph
//! names, then the embedded font's `cmap` (see `font.rs`).
//!
//! Each [`Glyph`] keeps its code, text, origin and advance in user space, its size, and a
//! [`Quad`] in page space (ADR 0027: the displayed `CropBox`, top-left origin, y down,
//! `/Rotate` and `/UserUnit` applied; [`PageText::frame`] maps back): the glyph's width across
//! the font's ascent and descent (descriptor, `/FontBBox`, AFM, else a default), through the
//! text rendering matrix; vertical fonts place it by the `/W2` position vector. Glyphs inside
//! marked content carry the innermost MCID (§14.7.5) and the outermost `ActualText` span
//! (§14.9.4); text drawn in rendering mode 3 or 7 is `invisible` (OCR layers).
//!
//! [`TextRun`]s group consecutive glyphs of one font, size, direction, MCID, `ActualText`
//! span and visibility that follow each other on one baseline (see `runs.rs`). A run inside
//! an `ActualText` span takes the span's text (the first run of the span; later runs of it
//! add none) and keeps its glyphs' boxes. Other runs join their glyphs' text, with a space
//! where the pen jumps forward over a quarter of the size; a run with right-to-left
//! characters is taken to be drawn in visual order and is put in logical order by UAX #9 over
//! grapheme clusters (`BidiParagraph`), as pdf.js does. [`PageText::lines`] joins the runs in
//! content order, a new line where the baseline moves by over half the size.
//!
//! Fonts are parsed once per [`TextExtractor`] (keyed by font object). Problems follow
//! ADR 0003 and are warnings on the document: `font-invalid` (a malformed font dictionary,
//! decoded with defaults), `cmap-invalid` (malformed CMap or ToUnicode entries skipped, an
//! unsupported predefined CMap, a `/UseCMap` chain past [`MAX_USECMAP_DEPTH`]) and
//! `unmapped-glyph` (a font with glyphs no rule maps; they extract as U+FFFD). Strict mode
//! turns each into an error; the content limits of `lamina-content` bound the work.

mod cmap;
mod font;
mod runs;

pub use cmap::{CMap, Code};
pub use font::MAX_USECMAP_DEPTH;
pub use runs::TextRun;

use std::sync::Arc;

use lamina_content::{MarkedContent, Matrix, TextAdvance, TextItem, TextShow, Visitor};
use lamina_cos::{Error, Result, Warning};
use lamina_model::{PageFrame, PointPt, UserSpacePoint};
use lamina_syntax::{ObjRef, Object};

use font::FontCache;

/// A glyph's box in page space: the baseline's start and end, then the top's end and start.
/// For unrotated horizontal text that is bottom-left, bottom-right, top-right, top-left.
pub type Quad = [PointPt; 4];

/// One shown glyph.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Glyph {
    /// The text it stands for: possibly several characters (a ligature), possibly none (a
    /// glyph mapped to no text), U+FFFD when nothing maps it (`mapped` is `false`). Not
    /// replaced by `ActualText`: see the glyph's run.
    pub text: String,
    /// Whether a rule mapped the code (else `text` is U+FFFD).
    pub mapped: bool,
    /// The character code.
    pub code: Code,
    /// The glyph origin in raw user space (the text rendering matrix's translation, §9.4.4).
    pub origin: UserSpacePoint,
    /// How far the glyph moved the pen, a vector in user space (with `Tc`, `Tw` and `Th`).
    pub advance: (f64, f64),
    /// The glyph's box in page space.
    pub quad: Quad,
    /// The size the glyph is drawn at in user space: `Tfs` scaled by the text and current
    /// transformation matrices.
    pub size: f64,
    /// Whether the font writes vertically.
    pub vertical: bool,
    /// The font's object, when the font dictionary is indirect.
    pub font: Option<ObjRef>,
    /// The font's `/BaseFont`.
    pub font_name: Arc<str>,
    /// The innermost marked-content identifier around the glyph (`/MCID`, §14.7.5).
    pub mcid: Option<i64>,
    /// The `ActualText` span the glyph is in: an index into [`PageText::actual_texts`].
    pub actual_text: Option<usize>,
    /// Drawn in text rendering mode 3 or 7: neither filled nor stroked (OCR layers).
    pub invisible: bool,
}

/// The text a page shows, in content order.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PageText {
    /// Every glyph, in the order the content shows them.
    pub glyphs: Vec<Glyph>,
    /// The glyphs grouped into runs, in content order.
    pub runs: Vec<TextRun>,
    /// The text of each `ActualText` span (`BDC` with `/ActualText`), in content order.
    pub actual_texts: Vec<String>,
    /// Glyphs no rule mapped (their text is U+FFFD).
    pub unmapped: usize,
    /// The page's frame: [`PageFrame::to_user_space`] maps a quad back to user space.
    pub frame: PageFrame,
}

impl PageText {
    /// The page's text as lines, in content order (see the module docs): runs joined, a new
    /// line where the baseline moves by more than half the size or the direction turns, a
    /// space where the pen jumps forward by more than a quarter of it between runs that have
    /// none. Each line is trimmed; empty lines are dropped.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        runs::lines(&self.glyphs, &self.runs)
    }

    /// The same text without invisible glyphs (`Tr` 3 and 7): the drawn text only.
    #[must_use]
    pub fn visible(&self) -> Self {
        let glyphs: Vec<Glyph> = self.glyphs.iter().filter(|g| !g.invisible).cloned().collect();
        let runs = runs::runs(&glyphs, &self.actual_texts);
        Self {
            glyphs,
            runs,
            ..self.clone()
        }
    }
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

    /// The text `page` of `doc` shows (its content and every Form XObject it draws).
    ///
    /// # Errors
    /// What [`lamina_content::interpret_page`] returns (the content limits, strict mode, an
    /// undecodable content stream), and [`Error::LimitExceeded`] or [`Error::Strict`] while
    /// reading a font or the page's boxes.
    pub fn page(&mut self, doc: &lamina_model::Document, page: &lamina_model::Page<'_>) -> Result<PageText> {
        let frame = page.frame()?;
        let mut collector = Collector {
            doc,
            fonts: &mut self.fonts,
            frame,
            glyphs: Vec::new(),
            actual_texts: Vec::new(),
            marked: Vec::new(),
            unmapped: 0,
            error: None,
        };
        lamina_content::interpret_page(doc, page, &mut collector)?;
        if let Some(error) = collector.error {
            return Err(error);
        }
        let runs = runs::runs(&collector.glyphs, &collector.actual_texts);
        Ok(PageText {
            glyphs: collector.glyphs,
            runs,
            actual_texts: collector.actual_texts,
            unmapped: collector.unmapped,
            frame,
        })
    }
}

/// One open marked-content sequence.
#[derive(Debug, Clone, Copy)]
struct Marked {
    mcid: Option<i64>,
    actual_text: Option<usize>,
}

struct Collector<'a> {
    doc: &'a lamina_cos::Document,
    fonts: &'a mut FontCache,
    frame: PageFrame,
    glyphs: Vec<Glyph>,
    actual_texts: Vec<String>,
    marked: Vec<Marked>,
    unmapped: usize,
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
        let font_name: Arc<str> = Arc::from(font.name.as_str());
        let state = &show.state.text;
        let vertical = font.vertical();
        let invisible = matches!(state.render_mode, 3 | 7);
        let mcid = self.marked.iter().rev().find_map(|m| m.mcid);
        let actual_text = self.marked.iter().find_map(|m| m.actual_text);
        let (ascent, descent) = font.extent;
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
                    // A `TJ` number moves by −n/1000 · Tfs (and Th horizontally), §9.4.3.
                    let shift = -n / 1000.0 * state.font_size;
                    let advance = if vertical {
                        TextAdvance { tx: 0.0, ty: shift }
                    } else {
                        TextAdvance {
                            tx: shift * state.horizontal_scaling,
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
                        // The box in glyph space (thousandths), relative to the pen: the width
                        // across the ascent and descent, moved by the position vector when
                        // writing vertically (§9.7.4.3).
                        let (left, bottom) = if vertical {
                            let (vx, vy) = font.position_vector(code);
                            (-vx, -vy)
                        } else {
                            (0.0, 0.0)
                        };
                        let corner = |x: f64, y: f64| {
                            let (ux, uy) = trm.transform_point((left + x) / 1000.0, (bottom + y) / 1000.0);
                            self.frame.to_page(UserSpacePoint::new(ux, uy))
                        };
                        let quad = [
                            corner(0.0, descent),
                            corner(w0, descent),
                            corner(w0, ascent),
                            corner(0.0, ascent),
                        ];
                        let (text, mapped) = if let Some(text) = font.unicode(code) {
                            (text, true)
                        } else {
                            unmapped = true;
                            self.unmapped += 1;
                            (char::REPLACEMENT_CHARACTER.to_string(), false)
                        };
                        self.glyphs.push(Glyph {
                            text,
                            mapped,
                            code,
                            origin: UserSpacePoint::new(trm.e, trm.f),
                            advance: (x1 - x0, y1 - y0),
                            quad,
                            size: trm.c.hypot(trm.d),
                            vertical,
                            font: font_ref.object,
                            font_name: Arc::clone(&font_name),
                            mcid,
                            actual_text,
                            invisible,
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

    /// The `/ActualText` of a property list, decoded as a text string (§7.9.2.2).
    fn actual_text(&self, marked: &MarkedContent<'_>) -> Result<Option<String>> {
        let Some(value) = marked.properties.and_then(|p| p.get("ActualText")) else {
            return Ok(None);
        };
        let value = match self.doc.deref(value) {
            Ok(resolved) => resolved.into_arc(),
            Err(error @ (Error::LimitExceeded { .. } | Error::Strict { .. } | Error::DataPending { .. })) => {
                return Err(error);
            }
            Err(_) => return Ok(None),
        };
        Ok(match &*value {
            Object::String(s) => Some(lamina_syntax::text::decode_text_string(&s.bytes)),
            _ => None,
        })
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

    fn begin_marked(&mut self, marked: &MarkedContent<'_>) {
        let actual_text = match self.actual_text(marked) {
            Ok(Some(text)) => {
                self.actual_texts.push(text);
                Some(self.actual_texts.len() - 1)
            }
            Ok(None) => None,
            Err(error) => {
                self.error.get_or_insert(error);
                None
            }
        };
        self.marked.push(Marked {
            mcid: marked.mcid,
            actual_text,
        });
    }

    fn end_marked(&mut self) {
        self.marked.pop();
    }
}
