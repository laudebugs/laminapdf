//! Markdown in, flowing pages out (ADR 0032 §6): `CommonMark` with tables, footnotes,
//! strikethrough and task lists, read with pulldown-cmark and mapped onto the flow layer.
//!
//! | Markdown | Flow | Tag |
//! | --- | --- | --- |
//! | `#` to `######` | [`FlowBlock::Heading`] | `H1`–`H6` |
//! | paragraph, hard break | [`FlowBlock::Paragraph`], a line break | `P` |
//! | `*em*`, `**strong**`, `` `code` ``, `~~strike~~` | spans in the italic, bold and mono fonts, struck through | `Span`, `Code` |
//! | link | a link: a URL, or `#slug` to that heading's destination | `Link` |
//! | image | [`FlowBlock::Figure`], its text as alt text, its bytes from [`MarkdownBuildOptions::with_image`] | `Figure` |
//! | list, task list | [`FlowBlock::List`] (an ordered list's start; ☐ and ☑) | `L`, `LI`, `Lbl`, `LBody` |
//! | block quote | [`FlowBlock::Quote`] | `BlockQuote` |
//! | code block | [`FlowBlock::CodeBlock`], a tab as spaces to the next multiple of four columns | `P` holding `Code` |
//! | thematic break | [`FlowBlock::Rule`] | a `Layout` artifact |
//! | table | [`FlowBlock::Table`], the first row as header | `Table`, `THead`, `TBody`, … |
//! | footnote | a note ([`Paragraph::note`]) | `Reference`, `Note` |
//! | front matter | the title, language, author and subject | `/Title`, `/Lang` |
//!
//! Raw HTML is dropped with `markdown-html-ignored`, and an image the caller didn't supply, or a
//! remote one, with `markdown-image-missing`: nothing is fetched. Both give the byte offset in
//! the Markdown, never its text. Maths is not parsed, so `$…$` stays text.
//!
//! ```
//! use laminapdf_build::markdown::MarkdownBuildOptions;
//! use laminapdf_build::{BuildOptions, Builder};
//!
//! let mut b = Builder::new(BuildOptions::new());
//! // Without fonts, the standard 14 Helvetica family and Courier set the text.
//! let report = b.markdown("# Notes\n\nPlain *Latin* text.\n", &MarkdownBuildOptions::new())?;
//! assert_eq!(report.pages.len(), 1);
//! b.finish()?;
//! # Ok::<(), laminapdf_build::Error>(())
//! ```
//!
//! [`FlowBlock::Heading`]: crate::flow::FlowBlock::Heading
//! [`FlowBlock::Paragraph`]: crate::flow::FlowBlock::Paragraph
//! [`FlowBlock::Figure`]: crate::flow::FlowBlock::Figure
//! [`FlowBlock::List`]: crate::flow::FlowBlock::List
//! [`FlowBlock::Quote`]: crate::flow::FlowBlock::Quote
//! [`FlowBlock::CodeBlock`]: crate::flow::FlowBlock::CodeBlock
//! [`FlowBlock::Rule`]: crate::flow::FlowBlock::Rule
//! [`FlowBlock::Table`]: crate::flow::FlowBlock::Table
//! [`Paragraph::note`]: crate::flow::Paragraph::note

mod convert;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use crate::builder::Builder;
use crate::error::Result;
use crate::flow::{Flow, FlowBlock, FlowReport, Margins, PageRegion, PageTemplate, TextStyle};
use crate::surface::image::Image;
use crate::surface::text::FontHandle;
use laminapdf_model::PageSize;
use laminapdf_text::Standard14;

/// The fonts a Markdown document is set in, by role. Each falls back to `fallbacks`, in order,
/// for characters it lacks (another script, say).
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MarkdownFonts {
    pub(crate) regular: FontHandle,
    pub(crate) bold: FontHandle,
    pub(crate) italic: FontHandle,
    pub(crate) bold_italic: FontHandle,
    pub(crate) mono: FontHandle,
    pub(crate) fallbacks: Vec<FontHandle>,
}

impl MarkdownFonts {
    /// `regular` for every role: bold, italic and code text are set in it too.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(regular: &FontHandle) -> Self {
        Self {
            regular: regular.clone(),
            bold: regular.clone(),
            italic: regular.clone(),
            bold_italic: regular.clone(),
            mono: regular.clone(),
            fallbacks: Vec::new(),
        }
    }

    /// With the bold font (also headings').
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_bold(mut self, font: &FontHandle) -> Self {
        self.bold = font.clone();
        self
    }

    /// With the italic font.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_italic(mut self, font: &FontHandle) -> Self {
        self.italic = font.clone();
        self
    }

    /// With the bold italic font.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_bold_italic(mut self, font: &FontHandle) -> Self {
        self.bold_italic = font.clone();
        self
    }

    /// With the font for code.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_mono(mut self, font: &FontHandle) -> Self {
        self.mono = font.clone();
        self
    }

    /// With `font` as a fallback for characters every role's font lacks.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_fallback(mut self, font: &FontHandle) -> Self {
        self.fallbacks.push(font.clone());
        self
    }
}

/// How [`Builder::markdown`] lays a document out.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MarkdownBuildOptions {
    pub(crate) fonts: Option<MarkdownFonts>,
    pub(crate) size: f64,
    pub(crate) template: Option<PageTemplate>,
    pub(crate) page_size: PageSize,
    pub(crate) images: BTreeMap<String, Image>,
    pub(crate) contents: Option<u8>,
}

impl Default for MarkdownBuildOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownBuildOptions {
    /// The standard 14 fonts (Helvetica and Courier, Latin text only, not embedded), 10.5-point
    /// text, A4 pages with 56-point margins and "Page {page} of {pages}" in the footer, no
    /// images and no contents page.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fonts: None,
            size: 10.5,
            template: None,
            page_size: PageSize::A4,
            images: BTreeMap::new(),
            contents: None,
        }
    }

    /// With the fonts set; without them, the standard 14 fonts, which PDF/UA-1 and PDF/A
    /// refuse (`font-not-embedded`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_fonts(mut self, fonts: MarkdownFonts) -> Self {
        self.fonts = Some(fonts);
        self
    }

    /// With the body text's size, in points; headings scale from it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// With the default template's pages at `size` (A4 by default): 56-point margins and
    /// "Page {page} of {pages}" in the footer. A template set with
    /// [`MarkdownBuildOptions::with_template`] takes its own size instead.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_page_size(mut self, size: PageSize) -> Self {
        self.page_size = size;
        self
    }

    /// With the page template set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_template(mut self, template: PageTemplate) -> Self {
        self.template = Some(template);
        self
    }

    /// With `image` for the images whose target is `target` (as written in the Markdown).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_image(mut self, target: impl Into<String>, image: Image) -> Self {
        self.images.insert(target.into(), image);
        self
    }

    /// With a contents page listing headings of levels 1 to `levels` at the start.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_contents(mut self, levels: u8) -> Self {
        self.contents = Some(levels);
        self
    }
}

/// The target of every image `markdown` shows, as written (`chart.png`, `https://…`), once each
/// in order of first use: what a caller passes to [`MarkdownBuildOptions::with_image`]. Reading
/// the files, and refusing remote ones, is the caller's choice.
///
/// Added in 0.2.0.
#[must_use]
pub fn image_targets(markdown: &str) -> Vec<String> {
    convert::image_targets(markdown)
}

/// What the front matter says: plain `key: value` lines between `---` lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FrontMatter {
    pub(crate) title: Option<String>,
    pub(crate) lang: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) subject: Option<String>,
}

impl FrontMatter {
    /// Reads `title`, `lang`, `author` and `subject` from `text`; other keys and lines without
    /// a colon are left out, and quotes around a value are taken off.
    pub(crate) fn parse(text: &str) -> Self {
        let mut out = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            if value.is_empty() {
                continue;
            }
            let slot = match key.trim() {
                "title" => &mut out.title,
                "lang" => &mut out.lang,
                "author" => &mut out.author,
                "subject" => &mut out.subject,
                _ => continue,
            };
            *slot = Some(value.to_owned());
        }
        out
    }
}

/// The text styles a document is set in.
#[derive(Debug, Clone)]
pub(crate) struct Styles {
    fonts: MarkdownFonts,
    pub(crate) size: f64,
    lang: Option<String>,
}

impl Styles {
    /// The style of text in `font` at `size`, with the fallbacks and the document's language.
    pub(crate) fn style(&self, font: &FontHandle, size: f64) -> TextStyle {
        let mut style = TextStyle::new(font, size);
        for fallback in &self.fonts.fallbacks {
            style = style.with_fallback(fallback);
        }
        match &self.lang {
            Some(lang) => style.with_language(lang.clone()),
            None => style,
        }
    }

    /// Body text, bold and italic as asked.
    pub(crate) fn text(&self, bold: bool, italic: bool) -> TextStyle {
        let f = &self.fonts;
        let font = match (bold, italic) {
            (false, false) => &f.regular,
            (true, false) => &f.bold,
            (false, true) => &f.italic,
            (true, true) => &f.bold_italic,
        };
        self.style(font, self.size)
    }

    /// Code, a little smaller than the body.
    pub(crate) fn mono(&self) -> TextStyle {
        self.mono_at(self.size)
    }

    /// Code among text of `size`, a little smaller than it.
    pub(crate) fn mono_at(&self, size: f64) -> TextStyle {
        self.style(&self.fonts.mono, size * 0.9)
    }

    /// A heading of `level` (1 to 6), in the bold font.
    pub(crate) fn heading(&self, level: u8) -> TextStyle {
        const SCALE: [f64; 6] = [2.0, 1.6, 1.3, 1.15, 1.0, 1.0];
        let scale = SCALE.get(usize::from(level.saturating_sub(1))).copied().unwrap_or(1.0);
        self.style(&self.fonts.bold, self.size * scale)
    }

    /// Whether every font of the regular style has `ch`.
    pub(crate) fn has(&self, ch: char) -> bool {
        self.fonts.regular.font().glyph_for(ch).is_some()
    }
}

impl Builder {
    /// Lays `markdown` out as a flow and adds its pages after any already added, as
    /// [`Builder::flow`] does; the front matter's title, language, author and subject become
    /// the document's metadata and language. Returns the pages added and what was worked
    /// around: the flow's warnings and `markdown-html-ignored`, `markdown-image-missing`.
    ///
    /// # Errors
    /// As [`Builder::flow`]; `missing-glyph` for a character no font has (the standard 14
    /// fonts set Latin text only).
    ///
    /// Added in 0.2.0.
    pub fn markdown(&mut self, markdown: &str, options: &MarkdownBuildOptions) -> Result<FlowReport> {
        let fonts = match &options.fonts {
            Some(fonts) => fonts.clone(),
            None => MarkdownFonts::new(&self.standard_font(Standard14::Helvetica)?)
                .with_bold(&self.standard_font(Standard14::HelveticaBold)?)
                .with_italic(&self.standard_font(Standard14::HelveticaOblique)?)
                .with_bold_italic(&self.standard_font(Standard14::HelveticaBoldOblique)?)
                .with_mono(&self.standard_font(Standard14::Courier)?),
        };
        let first = self.page_count();
        let front = convert::front_matter(markdown);
        let styles = Styles {
            fonts,
            size: options.size,
            lang: front.lang.clone(),
        };
        let mut document = convert::convert(markdown, &styles, options, first);
        let small = styles.style(&styles.fonts.regular, options.size * 0.8);
        let template = options.template.clone().unwrap_or_else(|| {
            let footer = PageRegion::new().with_center("Page {page} of {pages}");
            PageTemplate::new(options.page_size)
                .with_margins(Margins::all(56.0))
                .with_furniture(None, Some(footer), &small)
        });
        let mut flow = Flow::new(template);
        if let Some(title) = &front.title {
            flow = flow.with_title(title.clone());
        }
        if let Some(levels) = options.contents {
            let contents = crate::flow::Contents::new(&styles.text(false, false)).with_levels(levels);
            document.blocks.insert(0, FlowBlock::Contents(contents));
        }
        for block in document.blocks {
            flow = flow.block(block);
        }
        let mut report = self.flow(&flow)?;
        report.warnings.splice(0..0, document.warnings);
        // Set once the flow is laid out, so a failed one leaves the metadata as it was.
        if front != FrontMatter::default() {
            // Merged into what the caller set: front matter overrides only the keys it has.
            let mut metadata = self.metadata_so_far().cloned().unwrap_or_default();
            if let Some(title) = front.title {
                metadata = metadata.with_title(title);
            }
            if let Some(author) = front.author {
                metadata = metadata.with_author(author);
            }
            if let Some(subject) = front.subject {
                metadata = metadata.with_subject(subject);
            }
            self.metadata(metadata);
        }
        if let Some(lang) = front.lang {
            self.lang(lang);
        }
        Ok(report)
    }
}
