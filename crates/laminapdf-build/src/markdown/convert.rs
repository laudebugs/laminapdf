//! Markdown events to flow blocks. The document is read twice: once to collect the footnote
//! definitions (a reference can come before its definition, and a flow needs the note where
//! it is cited) and the headings' slugs (for `#slug` links), then to build the blocks.

use std::collections::BTreeMap;

use pulldown_cmark::{Alignment as Align, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::{FrontMatter, MarkdownBuildOptions, Styles};
use crate::error::Warning;
use crate::flow::{
    ColumnWidth, Figure, FlowBlock, List, ListItem, ListKind, Paragraph, ParagraphStyle, Table, TableCell, TableRow,
    destination, plain_text,
};
use crate::nav::LinkTarget;
use laminapdf_text::Alignment;

/// What the Markdown became.
#[derive(Debug, Default)]
pub(crate) struct Document {
    pub(crate) blocks: Vec<FlowBlock>,
    pub(crate) warnings: Vec<Warning>,
}

/// `CommonMark` with the extensions ADR 0032 §6 names: tables, footnotes, strikethrough, task
/// lists and front matter.
fn parser(markdown: &str) -> Parser<'_> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    Parser::new_ext(markdown, options)
}

/// The front matter of `markdown`, if it has any.
pub(crate) fn front_matter(markdown: &str) -> FrontMatter {
    let mut inside = false;
    let mut text = String::new();
    for event in parser(markdown) {
        match event {
            Event::Start(Tag::MetadataBlock(_)) => inside = true,
            Event::End(TagEnd::MetadataBlock(_)) => break,
            Event::Text(t) if inside => text.push_str(&t),
            _ => {}
        }
    }
    FrontMatter::parse(&text)
}

/// `markdown` as flow blocks set in `styles`, for a flow whose first page is the document's
/// page `first`.
pub(crate) fn convert(markdown: &str, styles: &Styles, options: &MarkdownBuildOptions, first: usize) -> Document {
    let mut gather = Converter::new(styles, options, first, BTreeMap::new(), BTreeMap::new());
    gather.run(markdown);
    let slugs = slugs(&gather.headings);
    let mut build = Converter::new(styles, options, first, gather.definitions, slugs);
    build.run(markdown);
    let blocks = match build.frames.pop() {
        Some(Frame::Blocks(blocks)) => blocks,
        _ => Vec::new(),
    };
    Document {
        blocks,
        warnings: build.warnings,
    }
}

/// Each heading's slug, as GitHub makes them (lower case, spaces to hyphens, other
/// punctuation dropped, `-1`, `-2` … after a repeat), to its index.
fn slugs(headings: &[String]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (n, text) in headings.iter().enumerate() {
        let base = slug(text);
        let count = seen.entry(base.clone()).or_insert(0);
        let slug = if *count == 0 { base } else { format!("{base}-{count}") };
        *count += 1;
        out.insert(slug, n);
    }
    out
}

pub(crate) fn slug(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// A container being built.
enum Frame {
    /// The document's blocks.
    Blocks(Vec<FlowBlock>),
    /// A block quote's blocks.
    Quote(Vec<FlowBlock>),
    List(List),
    Item(Item),
    Table(TableFrame),
    /// A footnote definition: its label and paragraphs.
    Note(String, Vec<Paragraph>),
}

#[derive(Default)]
struct Item {
    paragraph: Option<Paragraph>,
    nested: Option<List>,
    marker: Option<String>,
}

struct TableFrame {
    table: Table,
    aligns: Vec<Align>,
    row: TableRow,
    column: usize,
}

/// An image being read: its target, its alternate text, where it starts.
struct ImageAt {
    target: String,
    alt: String,
    offset: usize,
}

struct Converter<'a> {
    styles: &'a Styles,
    options: &'a MarkdownBuildOptions,
    first: usize,
    /// Footnote bodies by label (from the first pass).
    notes: BTreeMap<String, Paragraph>,
    /// Heading indices by slug (from the first pass).
    slugs: BTreeMap<String, usize>,
    frames: Vec<Frame>,
    /// The paragraph, heading or cell being written.
    inline: Option<Paragraph>,
    heading: Option<u8>,
    bold: u32,
    italic: u32,
    strike: u32,
    links: Vec<Option<LinkTarget>>,
    code: Option<String>,
    image: Option<ImageAt>,
    meta: bool,
    /// What the first pass collects.
    definitions: BTreeMap<String, Paragraph>,
    headings: Vec<String>,
    warnings: Vec<Warning>,
}

impl<'a> Converter<'a> {
    fn new(
        styles: &'a Styles,
        options: &'a MarkdownBuildOptions,
        first: usize,
        notes: BTreeMap<String, Paragraph>,
        slugs: BTreeMap<String, usize>,
    ) -> Self {
        Self {
            styles,
            options,
            first,
            notes,
            slugs,
            frames: vec![Frame::Blocks(Vec::new())],
            inline: None,
            heading: None,
            bold: 0,
            italic: 0,
            strike: 0,
            links: Vec::new(),
            code: None,
            image: None,
            meta: false,
            definitions: BTreeMap::new(),
            headings: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn run(&mut self, markdown: &str) {
        for (event, range) in parser(markdown).into_offset_iter() {
            self.event(event, range.start);
        }
    }

    fn event(&mut self, event: Event<'_>, offset: usize) {
        match event {
            Event::Start(tag) => self.start(tag, offset),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(text) => {
                let style = self.styles.mono();
                self.with_inline(|p| p.code(text.to_string(), &style));
            }
            Event::InlineHtml(_) => self.warnings.push(Warning::MarkdownHtmlIgnored { offset }),
            Event::SoftBreak => self.text(" "),
            Event::HardBreak => self.with_inline(Paragraph::line_break),
            Event::Rule => self.push_block(FlowBlock::Rule),
            Event::FootnoteReference(label) => match self.notes.get(label.as_ref()).cloned() {
                Some(body) => self.with_inline(|p| p.note(body)),
                None => self.text(&format!("[^{label}]")),
            },
            Event::TaskListMarker(checked) => {
                let (boxed, plain) = if checked { ('☑', "[x]") } else { ('☐', "[ ]") };
                let marker = if self.styles.has(boxed) {
                    boxed.to_string()
                } else {
                    plain.to_owned()
                };
                if let Some(Frame::Item(item)) = self.frames.last_mut() {
                    item.marker = Some(marker);
                }
            }
            // Block HTML is reported once, at its start; maths isn't enabled.
            Event::Html(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>, offset: usize) {
        match tag {
            Tag::Paragraph | Tag::TableCell => self.inline = Some(Paragraph::new(ParagraphStyle::new())),
            Tag::Heading { level, .. } => {
                self.heading = Some(level_of(level));
                self.inline = Some(Paragraph::new(ParagraphStyle::new().with_space(10.0, 4.0)));
            }
            Tag::BlockQuote(_) => self.frames.push(Frame::Quote(Vec::new())),
            Tag::CodeBlock(_) => self.code = Some(String::new()),
            Tag::HtmlBlock => self.warnings.push(Warning::MarkdownHtmlIgnored { offset }),
            Tag::List(start) => {
                let kind = if start.is_some() {
                    ListKind::Numbered
                } else {
                    ListKind::Bullet
                };
                let start = start.and_then(|s| usize::try_from(s).ok()).unwrap_or(1);
                let list = List::new(kind, &self.styles.text(false, false)).with_start(start);
                self.frames.push(Frame::List(list));
            }
            Tag::Item => self.frames.push(Frame::Item(Item::default())),
            Tag::FootnoteDefinition(label) => self.frames.push(Frame::Note(label.to_string(), Vec::new())),
            Tag::Table(aligns) => {
                let table = Table::new(aligns.iter().map(|_| ColumnWidth::Auto));
                self.frames.push(Frame::Table(TableFrame {
                    table,
                    aligns,
                    row: TableRow::new(),
                    column: 0,
                }));
            }
            Tag::TableHead | Tag::TableRow => {
                if let Some(Frame::Table(t)) = self.frames.last_mut() {
                    t.row = TableRow::new();
                    t.column = 0;
                }
            }
            Tag::Emphasis => self.italic += 1,
            Tag::Strong => self.bold += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Link { dest_url, .. } => {
                let target = self.target(&dest_url);
                self.links.push(target);
            }
            Tag::Image { dest_url, .. } => {
                self.image = Some(ImageAt {
                    target: dest_url.to_string(),
                    alt: String::new(),
                    offset,
                });
            }
            Tag::MetadataBlock(_) => self.meta = true,
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                if let Some(p) = self.inline.take().filter(|p| !p.inlines.is_empty()) {
                    self.push_block(FlowBlock::Paragraph(p));
                }
            }
            TagEnd::Heading(level) => {
                let p = self.inline.take().unwrap_or_default();
                self.heading = None;
                self.headings.push(plain_text(&p));
                self.push_block(FlowBlock::Heading(level_of(level), p));
            }
            TagEnd::BlockQuote(_) => {
                if let Some(Frame::Quote(blocks)) = self.frames.pop() {
                    self.push_block(FlowBlock::Quote(blocks));
                }
            }
            TagEnd::CodeBlock => {
                let code = self.code.take().unwrap_or_default();
                self.push_block(FlowBlock::CodeBlock(code, self.styles.mono()));
            }
            TagEnd::List(_) => {
                if let Some(Frame::List(list)) = self.frames.pop() {
                    self.push_block(FlowBlock::List(list));
                }
            }
            TagEnd::Item => self.end_item(),
            TagEnd::FootnoteDefinition => {
                if let Some(Frame::Note(label, paragraphs)) = self.frames.pop() {
                    self.definitions.insert(label, joined(paragraphs));
                }
            }
            TagEnd::TableCell => self.end_cell(),
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(Frame::Table(t)) = self.frames.last_mut() {
                    let row = std::mem::take(&mut t.row);
                    t.table = if matches!(tag, TagEnd::TableHead) {
                        t.table.clone().header(row)
                    } else {
                        t.table.clone().row(row)
                    };
                }
            }
            TagEnd::Table => {
                if let Some(Frame::Table(t)) = self.frames.pop() {
                    self.push_block(FlowBlock::Table(t.table));
                }
            }
            TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
            TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link => {
                self.links.pop();
            }
            TagEnd::Image => self.end_image(),
            TagEnd::MetadataBlock(_) => self.meta = false,
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        if self.meta {
            return;
        }
        if let Some(code) = &mut self.code {
            code.push_str(text);
            return;
        }
        if let Some(image) = &mut self.image {
            image.alt.push_str(text);
            return;
        }
        let mut style = match self.heading {
            Some(level) => self.styles.heading(level),
            None => self.styles.text(self.bold > 0, self.italic > 0),
        };
        if self.strike > 0 {
            style = style.with_strikethrough(true);
        }
        let link = self.links.last().cloned().flatten();
        self.with_inline(|p| match link {
            Some(target) => p.link(text, &style, target),
            None => p.text(text, &style),
        });
    }

    /// Adds to the paragraph being written, starting one where Markdown has text outside a
    /// paragraph (a tight list's items).
    fn with_inline(&mut self, add: impl FnOnce(Paragraph) -> Paragraph) {
        let p = self
            .inline
            .take()
            .unwrap_or_else(|| Paragraph::new(ParagraphStyle::new()));
        self.inline = Some(add(p));
    }

    /// A link's target: a URL, or a `#slug` to that heading's destination (no link for a slug
    /// no heading has).
    fn target(&self, url: &str) -> Option<LinkTarget> {
        match url.strip_prefix('#') {
            Some(slug) => self
                .slugs
                .get(slug)
                .map(|&n| LinkTarget::Named(destination(self.first, n))),
            None => Some(LinkTarget::Uri(url.to_owned())),
        }
    }

    fn push_block(&mut self, block: FlowBlock) {
        match self.frames.last_mut() {
            Some(Frame::Blocks(blocks) | Frame::Quote(blocks)) => blocks.push(block),
            Some(Frame::Item(item)) => match block {
                FlowBlock::Paragraph(p) => {
                    item.paragraph = Some(match item.paragraph.take() {
                        Some(before) => joined(vec![before, p]),
                        None => p,
                    });
                }
                FlowBlock::List(list) if item.nested.is_none() => item.nested = Some(list),
                // A list item holds a paragraph and a nested list: code, quotes and the rest
                // become text of its paragraph.
                other => {
                    let text = block_text(&other);
                    if !text.is_empty() {
                        let style = self.styles.text(false, false);
                        let p = Paragraph::plain(text, &style);
                        item.paragraph = Some(match item.paragraph.take() {
                            Some(before) => joined(vec![before, p]),
                            None => p,
                        });
                    }
                }
            },
            Some(Frame::Note(_, paragraphs)) => {
                if let FlowBlock::Paragraph(p) = block {
                    paragraphs.push(p);
                }
            }
            Some(Frame::List(_) | Frame::Table(_)) | None => {}
        }
    }

    fn end_item(&mut self) {
        if let Some(p) = self.inline.take().filter(|p| !p.inlines.is_empty()) {
            self.push_block(FlowBlock::Paragraph(p));
        }
        let Some(Frame::Item(item)) = self.frames.pop() else {
            return;
        };
        let mut entry = ListItem::new(item.paragraph.unwrap_or_default());
        if let Some(nested) = item.nested {
            entry = entry.with_nested(nested);
        }
        if let Some(marker) = item.marker {
            entry = entry.with_marker(marker);
        }
        if let Some(Frame::List(list)) = self.frames.last_mut() {
            *list = list.clone().item(entry);
        }
    }

    fn end_cell(&mut self) {
        let p = self.inline.take().unwrap_or_default();
        if let Some(Frame::Table(t)) = self.frames.last_mut() {
            let align = match t.aligns.get(t.column) {
                Some(Align::Center) => Some(Alignment::Center),
                Some(Align::Right) => Some(Alignment::End),
                Some(Align::Left) => Some(Alignment::Start),
                Some(Align::None) | None => None,
            };
            let mut cell = TableCell::new(p);
            if let Some(align) = align {
                cell = cell.with_align(align);
            }
            t.row = std::mem::take(&mut t.row).cell(cell);
            t.column += 1;
        }
    }

    /// An image becomes a figure between the text before and after it, when the caller
    /// supplied it; else it is left out with `markdown-image-missing`.
    fn end_image(&mut self) {
        let Some(image) = self.image.take() else { return };
        let Some(bytes) = self.options.images.get(&image.target) else {
            self.warnings
                .push(Warning::MarkdownImageMissing { offset: image.offset });
            return;
        };
        let style = self.inline.as_ref().map(|p| p.style.clone());
        if let Some(p) = self.inline.take().filter(|p| !p.inlines.is_empty()) {
            self.push_block(FlowBlock::Paragraph(p));
        }
        self.push_block(FlowBlock::Figure(Figure::new(bytes.clone(), image.alt)));
        self.inline = style.map(Paragraph::new);
    }
}

fn level_of(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Paragraphs as one, a line break between them.
fn joined(paragraphs: Vec<Paragraph>) -> Paragraph {
    let mut out: Option<Paragraph> = None;
    for p in paragraphs {
        out = Some(match out {
            Some(mut before) => {
                before = before.line_break();
                before.inlines.extend(p.inlines);
                before
            }
            None => p,
        });
    }
    out.unwrap_or_default()
}

/// A block's text, for a block a list item can't hold.
fn block_text(block: &FlowBlock) -> String {
    match block {
        FlowBlock::CodeBlock(text, _) => text.trim_end().to_owned(),
        FlowBlock::Paragraph(p) | FlowBlock::Heading(_, p) => plain_text(p),
        FlowBlock::Quote(blocks) => blocks.iter().map(block_text).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }
}
