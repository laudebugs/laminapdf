//! A flow's blocks flattened into the items the paginator places, each with the structure
//! element its content belongs to.

use super::contents::{Contents, Heading, destination, headings};
use super::measure::Source;
use super::model::{Figure, FlowBlock, Inline, List, ListKind, Paragraph, ParagraphStyle};
use super::table::{CellItem, RowItem, Section, Table, TableItem, TableRow};
use crate::nav::LinkTarget;
use crate::tag::{TableScope, Tag, TagKind};

/// What an item draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Content {
    /// The lines of [`Items::sources`]`[i]`.
    Para(usize),
    /// [`Items::figures`]`[i]`.
    Figure(usize),
    /// [`Items::tables`]`[i]`.
    Table(usize),
    /// Space of a height, points.
    Spacer(f64),
    /// The end of a page.
    Break,
}

/// One thing the paginator places.
#[derive(Debug, Clone)]
pub(crate) struct Item {
    pub(crate) content: Content,
    pub(crate) space_before: f64,
    pub(crate) space_after: f64,
    pub(crate) keep_with_next: bool,
    /// The `Keep` group it belongs to, if any.
    pub(crate) keep: Option<usize>,
    pub(crate) break_before: bool,
    pub(crate) widows: usize,
    pub(crate) orphans: usize,
    /// Its left indent, points (lists).
    pub(crate) indent: f64,
    /// The element its content belongs to: an index into [`Items::nodes`] (`None`: the
    /// document's). A figure draws its own `Figure` element inside it.
    pub(crate) node: Option<usize>,
    /// A list item's marker: a source, drawn in the indent beside its first line, and its element.
    pub(crate) label: Option<(usize, usize)>,
    /// The heading level, for `{section}`.
    pub(crate) heading: Option<u8>,
    /// A contents entry's page number: a source drawn at the end of its last line.
    pub(crate) entry: Option<usize>,
    /// Room kept at the end of each line, points (a contents entry's number).
    pub(crate) reserve: f64,
}

/// A footnote: its body (a source, its number first) and its `Note` element.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NoteItem {
    pub(crate) source: usize,
    pub(crate) node: usize,
}

/// A structure element and its parent (`None`: the document's).
#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub(crate) tag: Tag,
    pub(crate) parent: Option<usize>,
}

/// Everything the paginator and the drawing read.
#[derive(Debug, Default)]
pub(crate) struct Items {
    pub(crate) entries: Vec<Item>,
    pub(crate) sources: Vec<Source>,
    pub(crate) figures: Vec<Figure>,
    pub(crate) tables: Vec<TableItem>,
    pub(crate) nodes: Vec<Node>,
    /// The flow's footnotes, in order.
    pub(crate) notes: Vec<NoteItem>,
    /// The flow's headings, in order.
    pub(crate) headings: Vec<Heading>,
    /// Each heading's item, in order.
    pub(crate) heading_at: Vec<usize>,
    /// The page number a contents page prints for each heading.
    numbers: Vec<usize>,
    /// The flow's first page: its headings' destinations are named after it.
    first: usize,
    keeps: usize,
}

impl Items {
    /// `blocks` flattened; a contents page prints `numbers` (one per heading, in order) for the
    /// flow whose first page is the document's page `first`.
    pub(crate) fn new(blocks: &[FlowBlock], numbers: &[usize], first: usize) -> Self {
        let mut items = Self {
            headings: headings(blocks),
            numbers: numbers.to_vec(),
            first,
            ..Self::default()
        };
        for block in blocks {
            items.block(block, None, None, 0.0);
        }
        items
    }

    fn node(&mut self, tag: Tag, parent: Option<usize>) -> usize {
        self.nodes.push(Node { tag, parent });
        self.nodes.len() - 1
    }

    fn block(&mut self, block: &FlowBlock, parent: Option<usize>, keep: Option<usize>, indent: f64) {
        match block {
            FlowBlock::Paragraph(p) => {
                let node = self.node(Tag::new(TagKind::P), parent);
                self.paragraph(p, node, keep, indent, None, None);
            }
            FlowBlock::Heading(level, p) => {
                let level = (*level).clamp(1, 6);
                let kind = [
                    TagKind::H1,
                    TagKind::H2,
                    TagKind::H3,
                    TagKind::H4,
                    TagKind::H5,
                    TagKind::H6,
                ];
                let kind = kind.get(usize::from(level - 1)).copied().unwrap_or(TagKind::H6);
                let node = self.node(Tag::new(kind), parent);
                let mut p = p.clone();
                p.style.keep_with_next = true;
                self.heading_at.push(self.entries.len());
                self.paragraph(&p, node, keep, indent, None, Some(level));
            }
            FlowBlock::List(list) => self.list(list, parent, keep, indent),
            FlowBlock::Figure(figure) => {
                self.figures.push(figure.clone());
                let mut item = Self::item(Content::Figure(self.figures.len() - 1), parent, keep, indent);
                item.space_after = figure.space_after;
                self.entries.push(item);
            }
            FlowBlock::Table(table) => self.table(table, parent, keep, indent),
            FlowBlock::Contents(contents) => self.contents(contents, parent, keep, indent),
            FlowBlock::Spacer(height) => {
                let item = Self::item(Content::Spacer(height.max(0.0)), None, keep, indent);
                self.entries.push(item);
            }
            FlowBlock::PageBreak => {
                let item = Self::item(Content::Break, None, keep, indent);
                self.entries.push(item);
            }
            FlowBlock::Keep(blocks) => {
                let group = keep.unwrap_or_else(|| {
                    self.keeps += 1;
                    self.keeps - 1
                });
                for b in blocks {
                    self.block(b, parent, Some(group), indent);
                }
            }
        }
    }

    fn list(&mut self, list: &List, parent: Option<usize>, keep: Option<usize>, indent: f64) {
        let l = self.node(Tag::new(TagKind::L), parent);
        let marker_style = ParagraphStyle::new().with_space(0.0, 0.0);
        for (n, entry) in list.items.iter().enumerate() {
            let li = self.node(Tag::new(TagKind::LI), Some(l));
            let marker = match list.kind {
                ListKind::Bullet => "•".to_owned(),
                ListKind::Numbered => format!("{}.", n + 1),
            };
            self.sources
                .push(Source::single(&marker, &list.marker, marker_style.clone()));
            let lbl = self.node(Tag::new(TagKind::Lbl), Some(li));
            let body = self.node(Tag::new(TagKind::LBody), Some(li));
            let label = (self.sources.len() - 1, lbl);
            self.paragraph(&entry.paragraph, body, keep, indent + list.indent, Some(label), None);
            if let Some(nested) = &entry.nested {
                self.list(nested, Some(body), keep, indent + list.indent);
            }
        }
    }

    /// A table: its element, a `THead`, `TBody` and `TFoot` for the parts it has, a `TR` per
    /// row, and a `TH` (`/Scope /Column`) or `TD` per cell with its `/ColSpan`.
    fn table(&mut self, table: &Table, parent: Option<usize>, keep: Option<usize>, indent: f64) {
        let node = self.node(Tag::new(TagKind::Table), parent);
        let columns = table.columns.len();
        let mut rows = Vec::new();
        let parts = [
            (Section::Head, TagKind::THead, &table.header),
            (Section::Body, TagKind::TBody, &table.body),
            (Section::Foot, TagKind::TFoot, &table.footer),
        ];
        for (section, kind, part) in parts {
            if part.is_empty() {
                continue;
            }
            let group = self.node(Tag::new(kind), Some(node));
            for row in part {
                rows.push(self.table_row(row, section, group, columns));
            }
        }
        self.tables.push(TableItem {
            table: table.look(),
            rows,
        });
        let mut item = Self::item(Content::Table(self.tables.len() - 1), Some(node), keep, indent);
        item.space_before = table.space_before;
        item.space_after = table.space_after;
        self.entries.push(item);
    }

    fn table_row(&mut self, row: &TableRow, section: Section, group: usize, columns: usize) -> RowItem {
        let tr = self.node(Tag::new(TagKind::TR), Some(group));
        let mut cells = Vec::with_capacity(row.cells.len());
        let mut col = 0;
        for cell in &row.cells {
            if col >= columns {
                break;
            }
            let span = usize::try_from(cell.span).unwrap_or(1).clamp(1, columns - col);
            let tag = cell_tag(section);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a span is at most the column count, from a u32"
            )]
            let node = self.node(tag.with_col_span(span as u32), Some(tr));
            let mut sources = Vec::with_capacity(cell.paragraphs.len());
            for p in &cell.paragraphs {
                let mut p = p.clone();
                if let Some(align) = cell.align {
                    p.style = p.style.with_align(align);
                }
                let source = self.source(&p, node);
                if let (Some(lang), Some(n)) = (
                    self.sources.get(source).and_then(|s| s.lang.clone()),
                    self.nodes.get_mut(node),
                ) && n.tag.lang().is_none()
                {
                    n.tag = n.tag.clone().with_lang(lang);
                }
                sources.push(source);
            }
            cells.push(CellItem {
                sources,
                col,
                span,
                valign: cell.valign,
                node,
            });
            col += span;
        }
        // Missing cells are empty ones, so every row has a cell per column in the structure.
        while col < columns {
            let tag = cell_tag(section);
            let node = self.node(tag, Some(tr));
            cells.push(CellItem {
                sources: Vec::new(),
                col,
                span: 1,
                valign: super::table::VerticalAlign::Top,
                node,
            });
            col += 1;
        }
        RowItem { section, cells }
    }

    fn item(content: Content, node: Option<usize>, keep: Option<usize>, indent: f64) -> Item {
        let style = ParagraphStyle::default();
        Item {
            content,
            space_before: 0.0,
            space_after: 0.0,
            keep_with_next: false,
            keep,
            break_before: false,
            widows: style.widows,
            orphans: style.orphans,
            indent,
            node,
            label: None,
            heading: None,
            entry: None,
            reserve: 0.0,
        }
    }

    /// `p` as a source numbering its footnotes after the flow's earlier ones; each note's body
    /// becomes a `Note` element inside `node`, the element of the text that cites it.
    fn source(&mut self, p: &Paragraph, node: usize) -> usize {
        let source = Source::numbered(p, self.notes.len());
        let bodies = source.notes.clone();
        self.sources.push(source);
        let index = self.sources.len() - 1;
        for body in bodies {
            let number = self.notes.len() + 1;
            let element = self.node(Tag::new(TagKind::Note), Some(node));
            self.sources.push(Source::new(&note_body(&body, number)));
            self.notes.push(NoteItem {
                source: self.sources.len() - 1,
                node: element,
            });
        }
        index
    }

    /// A contents page (`TOC`): an entry (`TOCI`) per heading up to its levels, indented by
    /// level, its text and its page number links to the heading.
    fn contents(&mut self, contents: &Contents, parent: Option<usize>, keep: Option<usize>, indent: f64) {
        let toc = self.node(Tag::new(TagKind::TOC), parent);
        let style = ParagraphStyle::new().with_space(0.0, contents.style.size * 0.3);
        let listed: Vec<(usize, Heading)> = self
            .headings
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, h)| h.level <= contents.levels)
            .collect();
        for (n, heading) in listed {
            let target = || LinkTarget::Named(destination(self.first, n));
            let number = self.numbers.get(n).copied().unwrap_or(0).to_string();
            let text = Paragraph::new(style.clone()).link(heading.text, &contents.style, target());
            let number = Paragraph::new(ParagraphStyle::new()).link(number, &contents.style, target());
            let toci = self.node(Tag::new(TagKind::TOCI), Some(toc));
            self.sources.push(Source::new(&number));
            let number = self.sources.len() - 1;
            self.sources.push(Source::new(&text));
            let level_indent = f64::from(heading.level - 1) * contents.indent.max(0.0);
            let mut item = Self::item(
                Content::Para(self.sources.len() - 1),
                Some(toci),
                keep,
                indent + level_indent,
            );
            item.space_after = style.space_after;
            item.entry = Some(number);
            // Room for a number of four digits and the leaders' gap.
            item.reserve = contents.style.size * 3.0;
            self.entries.push(item);
        }
    }

    fn paragraph(
        &mut self,
        p: &Paragraph,
        node: usize,
        keep: Option<usize>,
        indent: f64,
        label: Option<(usize, usize)>,
        heading: Option<u8>,
    ) {
        let source = self.source(p, node);
        // The paragraph's own language goes on its element; its other languages on spans.
        if let (Some(lang), Some(n)) = (
            self.sources.get(source).and_then(|s| s.lang.clone()),
            self.nodes.get_mut(node),
        ) {
            n.tag = n.tag.clone().with_lang(lang);
        }
        let mut item = Self::item(Content::Para(source), Some(node), keep, indent);
        let s = &p.style;
        item.space_before = s.space_before;
        item.space_after = s.space_after;
        item.keep_with_next = s.keep_with_next;
        item.break_before = s.break_before;
        item.widows = s.widows;
        item.orphans = s.orphans;
        item.label = label;
        item.heading = heading;
        self.entries.push(item);
    }

    /// Whether item `i` must stay on the page of item `i + 1`.
    pub(crate) fn binds(&self, i: usize) -> bool {
        let (Some(a), Some(b)) = (self.entries.get(i), self.entries.get(i + 1)) else {
            return false;
        };
        a.keep_with_next || (a.keep.is_some() && a.keep == b.keep)
    }
}

/// A cell's element: `TH` heading its column in a header row, `TD` elsewhere.
fn cell_tag(section: Section) -> Tag {
    if section == Section::Head {
        Tag::new(TagKind::TH).with_scope(TableScope::Column)
    } else {
        Tag::new(TagKind::TD)
    }
}

/// A footnote's body as drawn at the foot of the page: its number, then its text, without
/// footnotes of its own (a note inside a note is left out).
fn note_body(body: &Paragraph, number: usize) -> Paragraph {
    let mut out = Paragraph::new(body.style.clone());
    let first = body.inlines.iter().find_map(|i| match i {
        Inline::Text(_, style) | Inline::Link(_, style, _) => Some(style.clone()),
        Inline::LineBreak | Inline::Note(_) => None,
    });
    if let Some(style) = first {
        out = out.text(format!("{number}\u{a0}"), &style);
    }
    out.inlines
        .extend(body.inlines.iter().filter(|i| !matches!(i, Inline::Note(_))).cloned());
    out
}
