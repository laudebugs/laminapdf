//! What a flow is made of (ADR 0032 §3): blocks of styled text, lists and figures, and the page
//! template they flow through.

use laminapdf_text::Alignment;

use crate::nav::LinkTarget;
use crate::page::PageSize;
use crate::surface::color::Color;
use crate::surface::image::Image;
use crate::surface::text::FontHandle;

/// How a stretch of text looks: its fonts (the first, then fallbacks for characters it lacks),
/// size, colour, baseline rise, language, and whether it is struck through.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TextStyle {
    pub(crate) fonts: Vec<FontHandle>,
    pub(crate) size: f64,
    pub(crate) color: Option<Color>,
    pub(crate) rise: f64,
    pub(crate) language: Option<String>,
    pub(crate) strike: bool,
}

impl TextStyle {
    /// Text in `font` at `size` points, in the default colour, on the baseline.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(font: &FontHandle, size: f64) -> Self {
        Self {
            fonts: vec![font.clone()],
            size,
            color: None,
            rise: 0.0,
            language: None,
            strike: false,
        }
    }

    /// With `font` as a fallback for characters the fonts before it lack.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_fallback(mut self, font: &FontHandle) -> Self {
        self.fonts.push(font.clone());
        self
    }

    /// With the size set, in points.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// With the fill colour set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// With the baseline raised by `rise` points (negative: lowered), for a superscript.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_rise(mut self, rise: f64) -> Self {
        self.rise = rise;
        self
    }

    /// With the language set (BCP 47): used for shaping and tagged as the span's `/Lang`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// With a line through the text (deleted text), drawn as decoration in its colour.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_strikethrough(mut self, strike: bool) -> Self {
        self.strike = strike;
        self
    }

    /// The size in points.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn size(&self) -> f64 {
        self.size
    }
}

/// A piece of a paragraph.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Inline {
    /// Text in a style.
    Text(String, TextStyle),
    /// Text in a style that links to `target`.
    Link(String, TextStyle, LinkTarget),
    /// Computer code in a style (tagged `Code`).
    Code(String, TextStyle),
    /// A line break inside the paragraph.
    LineBreak,
    /// A footnote: a raised number here, in the style of the text before it, and the body at
    /// the foot of the page that holds the number.
    Note(Box<Paragraph>),
}

/// How a paragraph sits in the flow.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParagraphStyle {
    pub(crate) align: Alignment,
    pub(crate) line_height: f64,
    pub(crate) space_before: f64,
    pub(crate) space_after: f64,
    pub(crate) keep_with_next: bool,
    pub(crate) widows: usize,
    pub(crate) orphans: usize,
    pub(crate) break_before: bool,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self {
            align: Alignment::Start,
            line_height: 1.2,
            space_before: 0.0,
            space_after: 6.0,
            keep_with_next: false,
            widows: 2,
            orphans: 2,
            break_before: false,
        }
    }
}

impl ParagraphStyle {
    /// The defaults: start-aligned, lines 1.2 × the size apart, 6 points after, two lines at
    /// least at the foot and the head of a page.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// With the alignment set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// With the distance between baselines set, as a multiple of a line's largest size.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_line_height(mut self, factor: f64) -> Self {
        self.line_height = factor;
        self
    }

    /// With the space above and below set, in points (the larger of two neighbours' spaces
    /// apply, never both).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_space(mut self, before: f64, after: f64) -> Self {
        self.space_before = before;
        self.space_after = after;
        self
    }

    /// Kept on the same page as the block after it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_keep_with_next(mut self, keep: bool) -> Self {
        self.keep_with_next = keep;
        self
    }

    /// With the fewest lines left at the head of a page (`widows`) and at its foot (`orphans`)
    /// when the paragraph splits; at least 1 each.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_widows_and_orphans(mut self, widows: usize, orphans: usize) -> Self {
        self.widows = widows.max(1);
        self.orphans = orphans.max(1);
        self
    }

    /// Starts on a new page.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_break_before(mut self, brk: bool) -> Self {
        self.break_before = brk;
        self
    }
}

/// A paragraph: inline pieces in a paragraph style.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Paragraph {
    pub(crate) inlines: Vec<Inline>,
    pub(crate) style: ParagraphStyle,
}

impl Paragraph {
    /// An empty paragraph in `style`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(style: ParagraphStyle) -> Self {
        Self {
            inlines: Vec::new(),
            style,
        }
    }

    /// One stretch of `text` in `style`, in the default paragraph style.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn plain(text: impl Into<String>, style: &TextStyle) -> Self {
        Self::new(ParagraphStyle::default()).text(text, style)
    }

    /// With `text` in `style` added.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>, style: &TextStyle) -> Self {
        self.inlines.push(Inline::Text(text.into(), style.clone()));
        self
    }

    /// With `text` in `style` added as a link to `target`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn link(mut self, text: impl Into<String>, style: &TextStyle, target: LinkTarget) -> Self {
        self.inlines.push(Inline::Link(text.into(), style.clone(), target));
        self
    }

    /// With a footnote added here: notes are numbered from 1 through the flow, the number is
    /// drawn raised in the style of the text before it, and `body` goes at the foot of the
    /// page that holds the number, after the number again.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn note(mut self, body: Paragraph) -> Self {
        self.inlines.push(Inline::Note(Box::new(body)));
        self
    }

    /// With `text` in `style` added as computer code (a `Code` element).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn code(mut self, text: impl Into<String>, style: &TextStyle) -> Self {
        self.inlines.push(Inline::Code(text.into(), style.clone()));
        self
    }

    /// With a line break added.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn line_break(mut self) -> Self {
        self.inlines.push(Inline::LineBreak);
        self
    }

    /// With the paragraph style set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_style(mut self, style: ParagraphStyle) -> Self {
        self.style = style;
        self
    }
}

/// How a list numbers its items.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ListKind {
    /// A bullet (•) before each item.
    #[default]
    Bullet,
    /// `1.`, `2.`, … before each item.
    Numbered,
}

/// One item of a list: a paragraph, and optionally a list nested under it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ListItem {
    pub(crate) paragraph: Paragraph,
    pub(crate) nested: Option<List>,
    pub(crate) marker: Option<String>,
}

impl ListItem {
    /// An item of one paragraph.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(paragraph: Paragraph) -> Self {
        Self {
            paragraph,
            nested: None,
            marker: None,
        }
    }

    /// With its own marker instead of the list's bullet or number, such as a task list's ☐.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_marker(mut self, marker: impl Into<String>) -> Self {
        self.marker = Some(marker.into());
        self
    }

    /// With `list` nested under the item.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_nested(mut self, list: List) -> Self {
        self.nested = Some(list);
        self
    }
}

/// A bulleted or numbered list, its items indented with their markers in the hanging indent.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct List {
    pub(crate) kind: ListKind,
    pub(crate) items: Vec<ListItem>,
    pub(crate) indent: f64,
    pub(crate) marker: TextStyle,
    pub(crate) start: usize,
}

impl List {
    /// An empty list of `kind`, its markers in `marker`, items indented 18 points.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(kind: ListKind, marker: &TextStyle) -> Self {
        Self {
            kind,
            items: Vec::new(),
            indent: 18.0,
            marker: marker.clone(),
            start: 1,
        }
    }

    /// Numbered from `start` (1 by default).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_start(mut self, start: usize) -> Self {
        self.start = start;
        self
    }

    /// With `item` added.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn item(mut self, item: ListItem) -> Self {
        self.items.push(item);
        self
    }

    /// With the indent set, in points: where the items' text starts, the markers before it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_indent(mut self, indent: f64) -> Self {
        self.indent = indent;
        self
    }
}

/// An image with its alternate text, at a width (default: the frame's) and the height its
/// aspect ratio gives.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Figure {
    pub(crate) image: Image,
    pub(crate) alt: String,
    pub(crate) width: Option<f64>,
    pub(crate) space_after: f64,
}

impl Figure {
    /// `image` described by `alt`, as wide as the frame.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(image: Image, alt: impl Into<String>) -> Self {
        Self {
            image,
            alt: alt.into(),
            width: None,
            space_after: 6.0,
        }
    }

    /// With the width set, in points (at most the frame's).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }
}

/// A block of a flow.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum FlowBlock {
    /// A paragraph (`P`).
    Paragraph(Paragraph),
    /// A heading of level 1 to 6 (`H1`–`H6`), kept with the block after it.
    Heading(u8, Paragraph),
    /// A list (`L`).
    List(List),
    /// A figure (`Figure`), moved whole to the next page when it doesn't fit.
    Figure(Figure),
    /// Empty space of a height, in points; dropped at the top of a page.
    Spacer(f64),
    /// Ends the page.
    PageBreak,
    /// Blocks kept on one page.
    Keep(Vec<FlowBlock>),
    /// A table (`Table`), its header rows repeated on every page it continues on.
    Table(super::table::Table),
    /// A contents page (`TOC`): the flow's headings with the pages they start on.
    Contents(super::contents::Contents),
    /// A block quotation (`BlockQuote`): its blocks indented 18 points.
    Quote(Vec<FlowBlock>),
    /// A block of computer code: its text in `style`, a line per line of the text, indented 12
    /// points and wrapped at the body's width (a `P` holding a `Code` element). A tab becomes
    /// spaces up to the next multiple of four columns; elsewhere in a flow a tab is one space.
    CodeBlock(String, TextStyle),
    /// A horizontal rule across the body (a `Layout` artifact), with 6 points above and below.
    Rule,
}

/// What a header or footer slot holds: text with fields `{page}`, `{pages}`, `{section}` and
/// `{title}` (`{{` and `}}` for braces).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct PageRegion {
    pub(crate) start: Option<String>,
    pub(crate) center: Option<String>,
    pub(crate) end: Option<String>,
}

impl PageRegion {
    /// An empty region.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// With the slot at the start side (left in left-to-right text).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_start(mut self, text: impl Into<String>) -> Self {
        self.start = Some(text.into());
        self
    }

    /// With the centre slot.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_center(mut self, text: impl Into<String>) -> Self {
        self.center = Some(text.into());
        self
    }

    /// With the slot at the end side.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_end(mut self, text: impl Into<String>) -> Self {
        self.end = Some(text.into());
        self
    }
}

/// A page's margins, in points.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Margins {
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
    pub(crate) left: f64,
}

impl Margins {
    /// Margins of `top`, `right`, `bottom` and `left` points.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same margin on every side.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn all(margin: f64) -> Self {
        Self::new(margin, margin, margin, margin)
    }
}

/// The pages a flow fills: their size and margins, a header and a footer (drawn in the top and
/// bottom margins), and a variant for the first page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PageTemplate {
    pub(crate) size: PageSize,
    pub(crate) margins: Margins,
    pub(crate) header: Option<PageRegion>,
    pub(crate) footer: Option<PageRegion>,
    pub(crate) furniture: Option<TextStyle>,
    pub(crate) first: Option<Box<PageTemplate>>,
}

impl PageTemplate {
    /// Pages of `size` with 72-point margins, no header and no footer.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(size: PageSize) -> Self {
        Self {
            size,
            margins: Margins::all(72.0),
            header: None,
            footer: None,
            furniture: None,
            first: None,
        }
    }

    /// With the margins set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_margins(mut self, margins: Margins) -> Self {
        self.margins = margins;
        self
    }

    /// With a header and a footer, drawn in `style`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_furniture(mut self, header: Option<PageRegion>, footer: Option<PageRegion>, style: &TextStyle) -> Self {
        self.header = header;
        self.footer = footer;
        self.furniture = Some(style.clone());
        self
    }

    /// With `first` used for the first page instead (a cover without a header, other margins).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_first_page(mut self, first: Self) -> Self {
        self.first = Some(Box::new(first));
        self
    }

    /// The template of page `index` (0-based).
    pub(crate) fn for_page(&self, index: usize) -> &Self {
        match (&self.first, index) {
            (Some(first), 0) => first,
            _ => self,
        }
    }

    /// The body's width.
    pub(crate) fn body_width(&self) -> f64 {
        (self.size.width.get() - self.margins.left - self.margins.right).max(0.0)
    }

    /// The body's top and bottom, page space.
    pub(crate) fn body_y(&self) -> (f64, f64) {
        (self.margins.top, self.size.height.get() - self.margins.bottom)
    }
}

/// Blocks and the template they flow through (ADR 0032 §3). Tagged by default (§4).
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Flow {
    pub(crate) blocks: Vec<FlowBlock>,
    pub(crate) template: PageTemplate,
    pub(crate) tagged: bool,
    pub(crate) section_level: u8,
    pub(crate) title: Option<String>,
    pub(crate) outline: bool,
}

impl Flow {
    /// An empty flow through `template`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(template: PageTemplate) -> Self {
        Self {
            blocks: Vec::new(),
            template,
            tagged: true,
            section_level: 1,
            title: None,
            outline: true,
        }
    }

    /// With `block` added.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn block(mut self, block: FlowBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// With tagging turned on or off (on by default).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_tagged(mut self, tagged: bool) -> Self {
        self.tagged = tagged;
        self
    }

    /// With the heading level `{section}` names (default 1).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_section_level(mut self, level: u8) -> Self {
        self.section_level = level;
        self
    }

    /// With the document outline (bookmarks) made from the flow's headings turned on or off
    /// (on by default): one item per heading, nested as the headings are.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_outline(mut self, outline: bool) -> Self {
        self.outline = outline;
        self
    }

    /// With the title `{title}` names.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}
