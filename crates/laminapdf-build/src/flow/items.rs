//! A flow's blocks flattened into the items the paginator places, each with the structure
//! element its content belongs to.

use super::measure::Source;
use super::model::{Figure, FlowBlock, List, ListKind, Paragraph, ParagraphStyle};
use super::table::{CellItem, RowItem, Section, Table, TableItem, TableRow};
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
    keeps: usize,
}

impl Items {
    /// `blocks` flattened.
    pub(crate) fn new(blocks: &[FlowBlock]) -> Self {
        let mut items = Self::default();
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
            table: table.clone(),
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
            let tag = if section == Section::Head {
                Tag::new(TagKind::TH).with_scope(TableScope::Column)
            } else {
                Tag::new(TagKind::TD)
            };
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
                let source = Source::new(&p);
                if let (Some(lang), Some(n)) = (source.lang.clone(), self.nodes.get_mut(node))
                    && n.tag.lang().is_none()
                {
                    n.tag = n.tag.clone().with_lang(lang);
                }
                self.sources.push(source);
                sources.push(self.sources.len() - 1);
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
        let source = Source::new(p);
        // The paragraph's own language goes on its element; its other languages on spans.
        if let (Some(lang), Some(n)) = (source.lang.clone(), self.nodes.get_mut(node)) {
            n.tag = n.tag.clone().with_lang(lang);
        }
        self.sources.push(source);
        let mut item = Self::item(Content::Para(self.sources.len() - 1), Some(node), keep, indent);
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
