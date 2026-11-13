//! Paragraphs prepared for breaking and broken into lines at a frame's width, through
//! `laminapdf-text`'s `break_spans`. Each line keeps its shaping: it is drawn from the same
//! result, never shaped again.

use laminapdf_text::{Font, HarfRust, LayoutOptions, Line, StyledSpan, break_spans};

use super::model::{Inline, Paragraph, ParagraphStyle, TextStyle};
use crate::error::Result;
use crate::nav::LinkTarget;
use crate::surface::color::Color;
use crate::surface::text::FontHandle;

/// How a span's text is drawn, besides what shaping needs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Look {
    pub(crate) color: Option<Color>,
    /// An index into [`Source::links`].
    pub(crate) link: Option<usize>,
    /// The span's own language, tagged as a `Span` with `/Lang` when it isn't the paragraph's.
    pub(crate) lang: Option<String>,
    /// For a footnote's marker, the note's index in the flow (tagged as a `Reference`).
    pub(crate) note: Option<usize>,
    /// Computer code (tagged `Code`).
    pub(crate) code: bool,
    /// Struck through.
    pub(crate) strike: bool,
}

/// A paragraph's text with its spans, ready to break at any width.
#[derive(Debug, Clone)]
pub(crate) struct Source {
    pub(crate) text: String,
    /// The fonts the spans name, each once.
    pub(crate) fonts: Vec<FontHandle>,
    spans: Vec<StyledSpan>,
    /// One per span.
    pub(crate) looks: Vec<Look>,
    pub(crate) links: Vec<LinkTarget>,
    pub(crate) style: ParagraphStyle,
    /// The paragraph's language: its first piece's.
    pub(crate) lang: Option<String>,
    /// The size of an empty line (the first style's).
    size: f64,
    /// Its footnotes' bodies, in order; the first is the flow's note [`Source::note_base`].
    pub(crate) notes: Vec<Paragraph>,
    /// The flow's index of its first footnote.
    pub(crate) note_base: usize,
    /// Each footnote marker's first byte and its note's index in the flow.
    pub(crate) markers: Vec<(usize, usize)>,
    /// The style of the last text pushed, for a marker after it.
    last_style: Option<TextStyle>,
}

/// A line of a paragraph and its height.
#[derive(Debug, Clone)]
pub(crate) struct MLine {
    pub(crate) line: Line,
    /// From the line's top to the next line's top, points.
    pub(crate) height: f64,
    /// From the line's top to its baseline, points.
    pub(crate) baseline: f64,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "sizes and widths are f32 in laminapdf-text"
)]
fn f32_of(v: f64) -> f32 {
    v as f32
}

impl Source {
    /// `paragraph` with its inlines joined into one text; a line break is U+2028, which ends a
    /// line but not the bidi paragraph. A paragraph with footnotes needs [`Source::numbered`].
    pub(crate) fn new(paragraph: &Paragraph) -> Self {
        Self::numbered(paragraph, 0)
    }

    /// `paragraph` as [`Source::new`] has it, its footnotes numbered from the flow's note
    /// `note_base` (0-based; the marker shows `note_base + 1`).
    pub(crate) fn numbered(paragraph: &Paragraph, note_base: usize) -> Self {
        let mut source = Self {
            text: String::new(),
            fonts: Vec::new(),
            spans: Vec::new(),
            looks: Vec::new(),
            links: Vec::new(),
            style: paragraph.style.clone(),
            lang: None,
            size: 0.0,
            notes: Vec::new(),
            note_base,
            markers: Vec::new(),
            last_style: None,
        };
        // The paragraph's language and empty-line size are its first text's, even after a note.
        let first = first_style(paragraph);
        let paragraph_lang = first.as_ref().and_then(|s| s.language.clone());
        source.size = first.as_ref().map_or(0.0, |s| s.size);
        for inline in &paragraph.inlines {
            let (text, style, link) = match inline {
                Inline::Text(text, style) => (text.as_str(), style, None),
                Inline::Link(text, style, target) => (text.as_str(), style, Some(target)),
                Inline::Code(text, style) => {
                    source.push(text, style, None, paragraph_lang.as_deref());
                    if let Some(look) = source.looks.last_mut() {
                        look.code = true;
                    }
                    continue;
                }
                Inline::LineBreak => {
                    // In the style of the text before it, so its line keeps that height.
                    let Some(prev) = source.spans.last().cloned() else {
                        continue;
                    };
                    let look = source.looks.last().cloned();
                    let start = source.text.len();
                    source.text.push('\u{2028}');
                    let mut span = prev;
                    span.range = start..source.text.len();
                    source.spans.push(span);
                    source.looks.extend(look.map(|l| Look {
                        link: None,
                        note: None,
                        code: false,
                        strike: false,
                        ..l
                    }));
                    continue;
                }
                Inline::Note(body) => {
                    source.marker(body, paragraph_lang.as_deref());
                    continue;
                }
            };
            let link = link.map(|target| {
                source.links.push(target.clone());
                source.links.len() - 1
            });
            source.push(text, style, link, paragraph_lang.as_deref());
        }
        source.lang = paragraph_lang;
        source
    }

    /// One stretch of `text` in `style`: a list marker, a header or footer slot.
    pub(crate) fn single(text: &str, style: &TextStyle, paragraph: ParagraphStyle) -> Self {
        Self::new(&Paragraph::plain(text, style).with_style(paragraph))
    }

    fn push(&mut self, text: &str, style: &TextStyle, link: Option<usize>, paragraph_lang: Option<&str>) {
        self.last_style = Some(style.clone());
        let fonts: Vec<usize> = style.fonts.iter().map(|f| self.font_index(f)).collect();
        let start = self.text.len();
        // A tab outside a code block (whose tabs are expanded to tab stops first) is a space: it
        // is drawn as one, and a font with CFF outlines can't give the space glyph a second text.
        if text.contains('\t') {
            self.text.extend(text.chars().map(|c| if c == '\t' { ' ' } else { c }));
        } else {
            self.text.push_str(text);
        }
        let mut span = StyledSpan::new(start..self.text.len())
            .with_fonts(fonts)
            .with_size(f32_of(style.size))
            .with_rise(f32_of(style.rise));
        if let Some(lang) = &style.language {
            span = span.with_language(lang.clone());
        }
        self.spans.push(span);
        self.looks.push(Look {
            color: style.color.clone(),
            link,
            lang: style.language.clone().filter(|l| Some(l.as_str()) != paragraph_lang),
            note: None,
            code: false,
            strike: style.strike,
        });
    }

    /// A footnote's marker: its number, raised and smaller, in the style of the text before
    /// it (or the note body's first style when it starts the paragraph).
    fn marker(&mut self, body: &Paragraph, paragraph_lang: Option<&str>) {
        let index = self.note_base + self.notes.len();
        // A note with no text anywhere to style its number after has no marker, and no number.
        let Some(base) = self.last_style.clone().or_else(|| first_style(body)) else {
            return;
        };
        self.notes.push(body.clone());
        let style = base
            .clone()
            .with_size(base.size * 0.65)
            .with_rise(base.rise + base.size * 0.35);
        let start = self.text.len();
        self.markers.push((start, index));
        self.push(&(index + 1).to_string(), &style, None, paragraph_lang);
        // The text after the marker is in the style before it, and so is a second marker.
        self.last_style = Some(base);
        if let Some(look) = self.looks.last_mut() {
            look.note = Some(index);
            look.lang = None;
        }
    }

    fn font_index(&mut self, font: &FontHandle) -> usize {
        if let Some(i) = self.fonts.iter().position(|f| f.key == font.key) {
            return i;
        }
        self.fonts.push(font.clone());
        self.fonts.len() - 1
    }

    /// The text from byte `from` broken into lines `width` points wide, their ranges in the
    /// whole text.
    pub(crate) fn lines(&self, from: usize, width: f64) -> Result<Vec<MLine>> {
        let text = self.text.get(from..).unwrap_or_default();
        let spans: Vec<StyledSpan> = self
            .spans
            .iter()
            .filter(|s| s.range.end > from || (s.range.is_empty() && s.range.start == from))
            .map(|s| {
                let mut s = s.clone();
                s.range = s.range.start.max(from) - from..s.range.end.max(from) - from;
                s
            })
            .collect();
        let fonts: Vec<Font> = self.fonts.iter().map(|f| f.font.clone()).collect();
        let options = LayoutOptions::new(f32_of(width), f32_of(self.size)).with_align(self.style.align);
        let lines = break_spans(&HarfRust, &fonts, text, &spans, &options)?;
        Ok(lines
            .into_iter()
            .map(|mut line| {
                shift(&mut line, from);
                let size = line.runs.iter().map(|r| f64::from(r.size)).fold(self.size, f64::max);
                let extent = line.ascent - line.descent;
                let height = (self.style.line_height * size).max(extent);
                let baseline = (height - extent) / 2.0 + line.ascent;
                MLine { line, height, baseline }
            })
            .collect())
    }

    /// The look of the span holding byte `at`.
    pub(crate) fn look_at(&self, at: usize) -> Option<&Look> {
        let i = self.spans.partition_point(|s| s.range.end <= at);
        self.looks.get(i)
    }
}

/// Moves `line`'s byte offsets by `by` (from a tail of the text to the whole text). Span indices
/// stay those of the tail's list; the flow reads looks by byte, never by them.
fn shift(line: &mut Line, by: usize) {
    if by == 0 {
        return;
    }
    line.range = line.range.start + by..line.range.end + by;
    line.content = line.content.start + by..line.content.end + by;
    for run in &mut line.runs {
        run.run.text_range = run.run.text_range.start + by..run.run.text_range.end + by;
        for glyph in &mut run.run.glyphs {
            glyph.cluster += by;
        }
    }
}

/// The style of `paragraph`'s first text.
pub(crate) fn first_style(paragraph: &Paragraph) -> Option<TextStyle> {
    paragraph.inlines.iter().find_map(|i| match i {
        Inline::Text(_, style) | Inline::Link(_, style, _) | Inline::Code(_, style) => Some(style.clone()),
        Inline::LineBreak | Inline::Note(_) => None,
    })
}
