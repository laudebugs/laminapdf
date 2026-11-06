//! Reading order from the structure tree ([ADR 0030] §2): the tree's elements in `/K` order,
//! each block-level element one block, its marked content (`(page, MCID)`) mapped to the runs
//! that carry it. Headings, list items and tables take their kinds from the tree's types
//! (ISO 32000-2 §14.8.4), through the `RoleMap`.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use std::collections::BTreeMap;

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{Document, PageId, RectPt, StructElem, StructKid, StructTree};

use crate::error::ExtractError;
use crate::lines::{Piece, lines, mostly_rtl};
use crate::model::{Block, BlockKind, Cell, ExtractedPage, Span, StructRef, TableDetector};
use crate::text_fix::{join_lines, visible_chars};
use crate::visibility::normalised;

/// What a block made from the tree is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitKind {
    Paragraph,
    Heading(u8),
    ListItem { depth: u8 },
    Caption,
    Footnote,
    Table,
    Other,
}

/// A block-level element: what its marked content becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unit {
    pub(crate) kind: UnitKind,
    /// The standard types from the root down to the element.
    pub(crate) path: Vec<String>,
    pub(crate) elem: Option<ObjRef>,
}

/// A table cell's place (§14.8.4.8.3; spans from the `/A` attributes, §14.8.5.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CellPlace {
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) row_span: usize,
    pub(crate) col_span: usize,
    pub(crate) header: bool,
}

/// What a piece of marked content is inside its block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Body,
    /// A list item's `Lbl`: its marker.
    Label,
    Cell(CellPlace),
}

/// Where one marked-content sequence goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mark {
    /// Its place in the tree's order.
    pub(crate) order: usize,
    /// Its block, an index into [`TaggedIndex::units`].
    pub(crate) unit: usize,
    pub(crate) role: Role,
}

/// The structure tree, indexed for extraction.
#[derive(Debug, Clone, Default)]
pub(crate) struct TaggedIndex {
    marks: BTreeMap<(usize, i64), Mark>,
    pub(crate) units: Vec<Unit>,
}

/// Types that group blocks and are never a block themselves (§14.8.4.2–4), unless they hold
/// marked content directly.
const GROUPING: [&str; 13] = [
    "Document",
    "DocumentFragment",
    "Part",
    "Sect",
    "Div",
    "Art",
    "Aside",
    "BlockQuote",
    "TOC",
    "L",
    "NonStruct",
    "Private",
    "Index",
];

/// The block a standard type starts, if it starts one.
fn block_kind(kind: &str, list_depth: usize) -> Option<UnitKind> {
    let heading = kind
        .strip_prefix('H')
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=6).contains(n));
    Some(match kind {
        _ if heading.is_some() => UnitKind::Heading(heading.unwrap_or(1)),
        "H" | "Title" => UnitKind::Heading(1),
        "P" | "TOCI" | "Lbl" => UnitKind::Paragraph,
        "LI" => UnitKind::ListItem {
            depth: u8::try_from(list_depth.saturating_sub(1)).unwrap_or(u8::MAX),
        },
        "Caption" => UnitKind::Caption,
        "Note" | "FENote" => UnitKind::Footnote,
        "Table" => UnitKind::Table,
        "Figure" | "Formula" | "Form" => UnitKind::Other,
        _ => return None,
    })
}

/// A table being read: the row the next `TD`/`TH` goes in, and the next free column per row
/// (cells spanning rows take columns in the rows below).
#[derive(Debug, Clone, Default)]
struct TableWalk {
    row: Option<usize>,
    taken: BTreeMap<usize, Vec<bool>>,
}

impl TableWalk {
    fn next_row(&mut self) {
        self.row = Some(self.row.map_or(0, |r| r + 1));
    }

    fn place(&mut self, row_span: usize, col_span: usize, header: bool) -> CellPlace {
        let row = *self.row.get_or_insert(0);
        let taken = self.taken.entry(row).or_default();
        let col = taken.iter().position(|t| !t).unwrap_or(taken.len());
        for r in row..row + row_span {
            let cols = self.taken.entry(r).or_default();
            if cols.len() < col + col_span {
                cols.resize(col + col_span, false);
            }
            for c in cols.iter_mut().skip(col).take(col_span) {
                *c = true;
            }
        }
        CellPlace {
            row,
            col,
            row_span,
            col_span,
            header,
        }
    }
}

/// The context an element is read in.
#[derive(Debug, Clone, Default)]
struct Context {
    path: Vec<String>,
    unit: Option<usize>,
    role: Option<Role>,
    list_depth: usize,
    page: Option<PageId>,
    /// The table being read, an index into the walk's tables.
    table: Option<usize>,
}

struct Builder<'a> {
    doc: &'a Document,
    tree: &'a StructTree,
    pages: BTreeMap<PageId, usize>,
    index: TaggedIndex,
    order: usize,
    tables: Vec<TableWalk>,
}

/// The most rows or columns one tagged cell may span: spans bound the grid's size, so a file
/// can't make one cell cost more than this squared.
const MAX_SPAN: usize = 64;

/// The integer `/A` attribute `key` of `elem` (`RowSpan`, `ColSpan`), from 1 to [`MAX_SPAN`].
fn span(doc: &Document, elem: &StructElem, key: &str) -> Result<usize, ExtractError> {
    let Some(object) = elem.object else {
        return Ok(1);
    };
    let reference = Object::Ref(object);
    let resolved = doc.deref(&reference)?;
    let Some(attrs) = resolved.as_dict().and_then(|d: &Dict| d.get("A")) else {
        return Ok(1);
    };
    let attrs = doc.deref(attrs)?;
    // `/A` is a dictionary or an array of them (§14.7.6).
    let lists: Vec<&Object> = match &*attrs {
        Object::Array(items) => items.iter().collect(),
        other => vec![other],
    };
    for list in lists {
        let list = doc.deref(list)?;
        if let Some(n) = list.as_dict().and_then(|d| d.get(key)).and_then(Object::as_int) {
            return Ok(usize::try_from(n).unwrap_or(1).clamp(1, MAX_SPAN));
        }
    }
    Ok(1)
}

impl Builder<'_> {
    /// Whether an element of type `kind` read in `ctx` starts a block: outside any block, or as
    /// a list item nested in another's body, or as a table's caption.
    fn starts_block(&self, ctx: &Context, kind: &str) -> bool {
        if GROUPING.contains(&kind) {
            return false;
        }
        match ctx.unit.and_then(|u| self.index.units.get(u)).map(|u| u.kind) {
            None => true,
            Some(UnitKind::ListItem { .. }) => kind == "LI",
            Some(UnitKind::Table) => kind == "Caption",
            Some(_) => false,
        }
    }

    fn unit(&mut self, kind: UnitKind, ctx: &Context, elem: &StructElem) -> usize {
        self.index.units.push(Unit {
            kind,
            path: ctx.path.clone(),
            elem: elem.object,
        });
        self.index.units.len() - 1
    }

    fn walk(&mut self, elem: &StructElem, parent: &Context) -> Result<(), ExtractError> {
        let kind = String::from_utf8_lossy(self.tree.standard_kind(&elem.kind)).into_owned();
        let mut ctx = parent.clone();
        ctx.path.push(kind.clone());
        if elem.page.is_some() {
            ctx.page.clone_from(&elem.page);
        }
        match kind.as_str() {
            "L" => ctx.list_depth += 1,
            "TR" => {
                if let Some(table) = ctx.table.and_then(|t| self.tables.get_mut(t)) {
                    table.next_row();
                }
            }
            "TH" | "TD" if ctx.table.is_some() => {
                let (rows, cols) = (span(self.doc, elem, "RowSpan")?, span(self.doc, elem, "ColSpan")?);
                if let Some(table) = ctx.table.and_then(|t| self.tables.get_mut(t)) {
                    ctx.role = Some(Role::Cell(table.place(rows, cols, kind == "TH")));
                }
            }
            "Lbl" if matches!(ctx.unit.and_then(|u| self.index.units.get(u)), Some(u) if matches!(u.kind, UnitKind::ListItem { .. })) =>
            {
                ctx.role = Some(Role::Label);
            }
            _ if self.starts_block(&ctx, &kind) => {
                if let Some(block) = block_kind(&kind, ctx.list_depth) {
                    // A block inside another (a nested item, a table's caption) is its own.
                    ctx.role = None;
                    ctx.table = None;
                    ctx.unit = Some(self.unit(block, &ctx, elem));
                    if block == UnitKind::Table {
                        self.tables.push(TableWalk::default());
                        ctx.table = Some(self.tables.len() - 1);
                    }
                }
            }
            _ => {}
        }
        for kid in &elem.kids {
            match kid {
                StructKid::Elem(child) => self.walk(child, &ctx)?,
                StructKid::Mcid { page, mcid } => {
                    let page = page.as_ref().or(ctx.page.as_ref());
                    let Some(&page) = page.and_then(|p| self.pages.get(p)) else {
                        continue;
                    };
                    // Marked content outside any block: the element holding it is one.
                    let unit = if let Some(unit) = ctx.unit {
                        unit
                    } else {
                        let unit = self.unit(UnitKind::Paragraph, &ctx, elem);
                        ctx.unit = Some(unit);
                        unit
                    };
                    let mark = Mark {
                        order: self.order,
                        unit,
                        role: ctx.role.unwrap_or(Role::Body),
                    };
                    self.order += 1;
                    self.index.marks.entry((page, *mcid)).or_insert(mark);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl TaggedIndex {
    /// The index of `doc`'s structure tree, `None` when it has none or the tree holds no marked
    /// content on a page of the document.
    pub(crate) fn read(doc: &Document) -> Result<Option<Self>, ExtractError> {
        let Some(tree) = doc.structure()? else {
            return Ok(None);
        };
        let pages = doc.pages()?;
        let page_index: BTreeMap<PageId, usize> = (0..pages.len())
            .filter_map(|i| pages.get(i).map(|p| (p.id().clone(), i)))
            .collect();
        let mut builder = Builder {
            doc,
            tree: &tree,
            pages: page_index,
            index: Self::default(),
            order: 0,
            tables: Vec::new(),
        };
        for root in &tree.roots {
            builder.walk(root, &Context::default())?;
        }
        Ok((!builder.index.marks.is_empty()).then_some(builder.index))
    }

    /// Where the marked content `mcid` of page `page` goes.
    pub(crate) fn mark(&self, page: usize, mcid: i64) -> Option<Mark> {
        self.marks.get(&(page, mcid)).copied()
    }
}

/// A plausibility check a page's tree failed ([ADR 0030] §2).
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Implausible {
    /// Its elements cover too little of the page's visible text.
    Coverage,
    /// Its order jumps back across columns too often.
    Order,
}

impl Implausible {
    pub(crate) const fn reason(self) -> &'static str {
        match self {
            Self::Coverage => "coverage",
            Self::Order => "order",
        }
    }
}

/// The share of a page's visible characters the tree must cover.
const MIN_COVERAGE: f64 = 0.8;

/// The share of block-to-block moves that may jump back across columns.
const MAX_BACKWARD: f64 = 0.1;

/// Whether moving from `prev` to `next` jumps back across a column: to a block wholly before
/// it in the line direction (left, or right on a right-to-left page) that starts higher up.
fn backward(prev: &RectPt, next: &RectPt, rtl: bool) -> bool {
    let before = if rtl { next.x0 >= prev.x1 } else { next.x1 <= prev.x0 };
    before && next.y0 < prev.y0
}

/// Checks a page read in the tree's order: `tagged` of `total` visible characters are in the
/// tree, and `boxes` are its blocks' boxes in the tree's order.
pub(crate) fn plausible(tagged: usize, total: usize, boxes: &[RectPt], rtl: bool) -> Result<(), Implausible> {
    #[expect(clippy::cast_precision_loss, reason = "character counts, far below 2^52")]
    if total > 0 && (tagged as f64) < MIN_COVERAGE * total as f64 {
        return Err(Implausible::Coverage);
    }
    let moves = boxes.len().saturating_sub(1);
    let back = boxes
        .windows(2)
        .filter(|w| matches!(w, [a, b] if backward(a, b, rtl)))
        .count();
    #[expect(clippy::cast_precision_loss, reason = "block counts, far below 2^52")]
    if moves > 0 && back as f64 > MAX_BACKWARD * moves as f64 {
        return Err(Implausible::Order);
    }
    Ok(())
}

/// A page's pieces and their spans, as `tree_blocks` takes them and gives back what it didn't use.
pub(crate) type Unread = (Vec<Piece>, Vec<Span>);

/// A page's blocks from the tree, and the pieces left for the layout.
pub(crate) struct TreeBlocks {
    pub(crate) blocks: Vec<Block>,
    /// The pieces (and their spans) the tree doesn't cover.
    pub(crate) rest: Unread,
    /// Their visible characters.
    pub(crate) untagged: usize,
    /// Whether the tree covers anything on the page.
    pub(crate) tagged: bool,
}

/// One block's pieces, in the tree's order, with their roles.
struct Group {
    unit: usize,
    pieces: Vec<(Role, Piece, Span)>,
    mcids: Vec<i64>,
}

fn union(boxes: impl IntoIterator<Item = RectPt>) -> RectPt {
    boxes.into_iter().reduce(|a, b| a.union(&b)).unwrap_or_default()
}

/// The text of `pieces` as a block reads it: its lines, joined.
fn read(pieces: &[Piece], dehyphenate: bool) -> String {
    let lines = lines(pieces);
    join_lines(lines.iter().map(|l| l.text.as_str()), dehyphenate)
}

/// A group's spans in its lines' order.
fn spans_in_order(items: &[(Role, Piece, Span)]) -> Vec<Span> {
    let pieces: Vec<Piece> = items.iter().map(|(_, p, _)| p.clone()).collect();
    lines(&pieces)
        .iter()
        .flat_map(|l| l.pieces.iter().filter_map(|&i| items.get(i).map(|(_, _, s)| s.clone())))
        .collect()
}

/// A table block's cells and text from its pieces' cell places.
fn table(items: &[(Role, Piece, Span)], dehyphenate: bool) -> (Vec<Cell>, usize, usize, String) {
    let mut cells: BTreeMap<(usize, usize), (CellPlace, Vec<Piece>)> = BTreeMap::new();
    for (role, piece, _) in items {
        if let Role::Cell(place) = role {
            cells
                .entry((place.row, place.col))
                .or_insert_with(|| (*place, Vec::new()))
                .1
                .push(piece.clone());
        }
    }
    let rows = cells.values().map(|(p, _)| p.row + p.row_span).max().unwrap_or(0);
    let cols = cells.values().map(|(p, _)| p.col + p.col_span).max().unwrap_or(0);
    let cells: Vec<Cell> = cells
        .into_values()
        .map(|(place, pieces)| Cell {
            row: place.row,
            col: place.col,
            row_span: place.row_span,
            col_span: place.col_span,
            header: place.header,
            text: read(&pieces, dehyphenate),
            bbox: union(pieces.iter().map(|p| p.bbox)),
        })
        .collect();
    let mut text_rows: Vec<Vec<&str>> = vec![Vec::new(); rows];
    for cell in &cells {
        if let Some(row) = text_rows.get_mut(cell.row) {
            row.push(&cell.text);
        }
    }
    let mut text = text_rows.iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\n");
    // Content in the table but in no cell (directly in a row, say) keeps its text, after the rows.
    let loose: Vec<Piece> = items
        .iter()
        .filter(|(r, _, _)| !matches!(r, Role::Cell(_)))
        .map(|(_, p, _)| p.clone())
        .collect();
    if !loose.is_empty() {
        text.push('\n');
        text.push_str(&read(&loose, dehyphenate));
    }
    (cells, rows, cols, text)
}

impl TaggedIndex {
    fn block(&self, group: &Group, info: &ExtractedPage, n: usize, dehyphenate: bool) -> Option<Block> {
        let unit = self.units.get(group.unit)?;
        let all: Vec<Piece> = group.pieces.iter().map(|(_, p, _)| p.clone()).collect();
        let bbox = union(all.iter().map(|p| p.bbox));
        let mut cells = Vec::new();
        let (kind, text) = match unit.kind {
            UnitKind::Table => {
                let (table_cells, rows, cols, text) = table(&group.pieces, dehyphenate);
                cells = table_cells;
                (
                    BlockKind::Table {
                        rows,
                        cols,
                        detector: TableDetector::Tagged,
                    },
                    text,
                )
            }
            UnitKind::ListItem { depth } => {
                let (label, body): (Vec<_>, Vec<_>) = group.pieces.iter().partition(|(r, _, _)| *r == Role::Label);
                let label: Vec<Piece> = label.into_iter().map(|(_, p, _)| p.clone()).collect();
                let body: Vec<Piece> = body.into_iter().map(|(_, p, _)| p.clone()).collect();
                let marker = read(&label, false).trim().to_owned();
                let marker = if marker.is_empty() { "•".to_owned() } else { marker };
                (BlockKind::ListItem { marker, depth }, read(&body, dehyphenate))
            }
            UnitKind::Heading(level) => (BlockKind::Heading { level }, read(&all, dehyphenate)),
            UnitKind::Caption => (BlockKind::Caption, read(&all, dehyphenate)),
            UnitKind::Footnote => (BlockKind::Footnote, read(&all, dehyphenate)),
            UnitKind::Other => (BlockKind::Other, read(&all, dehyphenate)),
            UnitKind::Paragraph => (BlockKind::Paragraph, read(&all, dehyphenate)),
        };
        let mut mcids = group.mcids.clone();
        mcids.sort_unstable();
        mcids.dedup();
        Some(Block {
            id: format!("p{}-b{n}", info.index),
            kind,
            text,
            page: info.index,
            bbox,
            bbox_norm: normalised(&bbox, info.width, info.height),
            struct_path: unit.path.clone(),
            struct_ref: unit.elem.map(|o| StructRef {
                obj: o.num,
                r#gen: o.gen_nr,
                mcids,
            }),
            spans: spans_in_order(&group.pieces),
            confidence: TREE_CONFIDENCE,
            cells,
        })
    }
}

/// How sure the tree is of a block (ADR 0030 §1).
const TREE_CONFIDENCE: f64 = 1.0;

/// The blocks the tree gives page `info.index`'s `pieces` (with their `spans`), in its order,
/// and the pieces it doesn't cover. With `check`, a tree that fails a plausibility check gives
/// nothing: the error carries the pieces back.
pub(crate) fn tree_blocks(
    tree: &TaggedIndex,
    info: &ExtractedPage,
    pieces: Vec<Piece>,
    spans: Vec<Span>,
    dehyphenate: bool,
    check: bool,
) -> Result<TreeBlocks, (Implausible, Unread)> {
    let marks: Vec<Option<Mark>> = pieces
        .iter()
        .map(|p| p.mcid.and_then(|m| tree.mark(info.index, m)))
        .collect();
    // Artifacts (kept when furniture isn't dropped) are neither tagged nor untagged content.
    let content = |p: &&Piece| !p.artifact;
    let total: usize = pieces.iter().filter(content).map(|p| visible_chars(&p.text)).sum();
    let tagged: usize = pieces
        .iter()
        .zip(&marks)
        .filter(|(p, m)| m.is_some() && !p.artifact)
        .map(|(p, _)| visible_chars(&p.text))
        .sum();
    if tagged == 0 {
        return Ok(TreeBlocks {
            blocks: Vec::new(),
            rest: (pieces, spans),
            untagged: total,
            tagged: false,
        });
    }
    // The blocks, by their first marked content, each with its pieces' indices.
    let mut units: BTreeMap<usize, (usize, Vec<usize>)> = BTreeMap::new();
    for (i, mark) in marks.iter().enumerate() {
        if let Some(mark) = mark {
            let entry = units.entry(mark.unit).or_insert((mark.order, Vec::new()));
            entry.0 = entry.0.min(mark.order);
            entry.1.push(i);
        }
    }
    let mut order: Vec<(usize, usize, Vec<usize>)> =
        units.into_iter().map(|(u, (first, idx))| (first, u, idx)).collect();
    order.sort_by_key(|(first, _, _)| *first);
    if check {
        let boxes: Vec<RectPt> = order
            .iter()
            .map(|(_, _, idx)| union(idx.iter().filter_map(|&i| pieces.get(i)).map(|p| p.bbox)))
            .collect();
        let text: String = pieces.iter().map(|p| p.text.as_str()).collect();
        if let Err(why) = plausible(tagged, total, &boxes, mostly_rtl(&text)) {
            return Err((why, (pieces, spans)));
        }
    }
    let mut slots: Vec<Option<(Piece, Span)>> = pieces.into_iter().zip(spans).map(Some).collect();
    let groups: Vec<Group> = order
        .into_iter()
        .map(|(_, unit, idx)| {
            let mut group = Group {
                unit,
                pieces: Vec::new(),
                mcids: Vec::new(),
            };
            for i in idx {
                let (Some(Some((piece, span))), Some(Some(mark))) = (slots.get_mut(i).map(Option::take), marks.get(i))
                else {
                    continue;
                };
                group.mcids.extend(piece.mcid);
                group.pieces.push((mark.role, piece, span));
            }
            group
        })
        .collect();
    let (rest_pieces, rest_spans): (Vec<Piece>, Vec<Span>) = slots.into_iter().flatten().unzip();
    let untagged = rest_pieces.iter().filter(content).map(|p| visible_chars(&p.text)).sum();
    let blocks = groups
        .iter()
        .enumerate()
        .filter_map(|(n, g)| tree.block(g, info, n, dehyphenate))
        .collect();
    Ok(TreeBlocks {
        blocks,
        rest: (rest_pieces, rest_spans),
        untagged,
        tagged: true,
    })
}

#[cfg(test)]
mod tests {
    use laminapdf_model::RectPt;

    use super::{Implausible, TableWalk, UnitKind, block_kind, plausible};

    fn rect(x: f64, y: f64) -> RectPt {
        RectPt::from_points(x, y, 100.0, 20.0)
    }

    #[test]
    fn block_types_become_kinds() {
        assert_eq!(block_kind("H2", 0), Some(UnitKind::Heading(2)));
        assert_eq!(block_kind("LI", 2), Some(UnitKind::ListItem { depth: 1 }));
        assert_eq!(block_kind("Span", 0), None);
        assert_eq!(block_kind("H9", 0), None);
    }

    #[test]
    fn spanning_cells_take_columns_in_the_rows_below() {
        let mut t = TableWalk::default();
        t.next_row();
        let a = t.place(2, 1, true);
        let b = t.place(1, 1, true);
        t.next_row();
        let c = t.place(1, 1, false);
        assert_eq!((a.col, b.col, c.col, c.row), (0, 1, 1, 1));
    }

    #[test]
    fn a_tree_covering_too_little_or_jumping_back_is_implausible() {
        assert_eq!(plausible(70, 100, &[], false), Err(Implausible::Coverage));
        // Right column, then the left one from the top: one move of one jumps back.
        assert_eq!(
            plausible(100, 100, &[rect(300.0, 100.0), rect(50.0, 50.0)], false),
            Err(Implausible::Order)
        );
        // Left column top to bottom, then the right one from the top: forward.
        let columns = [rect(50.0, 50.0), rect(50.0, 100.0), rect(300.0, 50.0)];
        assert_eq!(plausible(100, 100, &columns, false), Ok(()));
    }
}
