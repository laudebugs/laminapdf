//! Tables in a flow (ADR 0032 §3): header rows repeated at the top of every page the table
//! continues on, body rows kept whole, and footer rows (totals) kept with the last body row.
//!
//! Column widths are worked out once for the whole table, so a long table never changes width
//! from one page to the next: fixed columns first, then automatic ones at their widest content
//! (narrowed, widest first, when they don't fit), then fractions sharing what is left.

use std::rc::Rc;

use laminapdf_text::Alignment;

use super::measure::{MLine, Source};
use super::model::{Paragraph, TextStyle};
use crate::error::Result;
use crate::surface::color::Color;

/// How wide a column is.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ColumnWidth {
    /// A width in points.
    Fixed(f64),
    /// A share of the width the fixed and automatic columns leave: two columns of `Fraction(1.0)`
    /// and `Fraction(2.0)` take a third and two thirds of it.
    Fraction(f64),
    /// As wide as the column's widest cell (cells spanning columns aside), narrowed to fit.
    Auto,
}

/// Where a cell's text sits between the row's top and bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum VerticalAlign {
    /// At the top (the default).
    #[default]
    Top,
    /// In the middle.
    Middle,
    /// At the bottom.
    Bottom,
}

/// A table cell: paragraphs, across one column or more.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TableCell {
    pub(crate) paragraphs: Vec<Paragraph>,
    pub(crate) span: u32,
    pub(crate) align: Option<Alignment>,
    pub(crate) valign: VerticalAlign,
}

impl TableCell {
    /// A cell holding `paragraph`, one column wide, aligned as the paragraph is.
    #[must_use]
    pub fn new(paragraph: Paragraph) -> Self {
        Self {
            paragraphs: vec![paragraph],
            span: 1,
            align: None,
            valign: VerticalAlign::Top,
        }
    }

    /// A cell holding `text` in `style`.
    #[must_use]
    pub fn plain(text: impl Into<String>, style: &TextStyle) -> Self {
        Self::new(Paragraph::plain(text, style))
    }

    /// An empty cell.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            paragraphs: Vec::new(),
            ..Self::new(Paragraph::new(super::model::ParagraphStyle::new()))
        }
    }

    /// With another paragraph after the ones it has.
    #[must_use]
    pub fn paragraph(mut self, paragraph: Paragraph) -> Self {
        self.paragraphs.push(paragraph);
        self
    }

    /// Across `columns` columns (`/ColSpan`; at least 1, at most those left in the row).
    #[must_use]
    pub fn with_span(mut self, columns: u32) -> Self {
        self.span = columns.max(1);
        self
    }

    /// With every paragraph aligned to `align` (numbers: [`Alignment::End`]).
    #[must_use]
    pub const fn with_align(mut self, align: Alignment) -> Self {
        self.align = Some(align);
        self
    }

    /// With its text placed `valign` in the row.
    #[must_use]
    pub const fn with_vertical_align(mut self, valign: VerticalAlign) -> Self {
        self.valign = valign;
        self
    }
}

/// A row of cells. A row with fewer cells than the table has columns ends in empty cells; cells
/// past the last column are left out.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TableRow {
    pub(crate) cells: Vec<TableCell>,
}

impl TableRow {
    /// A row with no cells yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A row of one plain cell per text, all in `style`.
    #[must_use]
    pub fn plain<S: Into<String>>(texts: impl IntoIterator<Item = S>, style: &TextStyle) -> Self {
        Self {
            cells: texts.into_iter().map(|t| TableCell::plain(t, style)).collect(),
        }
    }

    /// With `cell` after the cells it has.
    #[must_use]
    pub fn cell(mut self, cell: TableCell) -> Self {
        self.cells.push(cell);
        self
    }
}

/// A line drawn by a table: its width in points and its colour.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TableRule {
    pub(crate) width: f64,
    pub(crate) color: Color,
}

impl TableRule {
    /// A rule `width` points wide in `color`.
    #[must_use]
    pub const fn new(width: f64, color: Color) -> Self {
        Self { width, color }
    }
}

/// A table (`Table`): header rows (`THead`), body rows (`TBody`) and footer rows (`TFoot`).
///
/// The header rows start the table and are drawn again at the top of every page it continues
/// on; those copies are `Pagination` artifacts, so the structure tree holds the header once.
/// A body row never splits: one that doesn't fit starts the next page. The footer rows end the
/// table, on the page of the last body row. Header cells are `TH` with `/Scope /Column`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Table {
    pub(crate) columns: Vec<ColumnWidth>,
    pub(crate) header: Vec<TableRow>,
    pub(crate) body: Vec<TableRow>,
    pub(crate) footer: Vec<TableRow>,
    pub(crate) padding: f64,
    pub(crate) frame: Option<TableRule>,
    pub(crate) header_rule: Option<TableRule>,
    pub(crate) row_rule: Option<TableRule>,
    pub(crate) header_background: Option<Color>,
    pub(crate) space_before: f64,
    pub(crate) space_after: f64,
}

impl Table {
    /// A table with these columns (a table with none draws no cells), no rows, 4 points of padding in each cell, no rules, and
    /// 6 points of space after it.
    #[must_use]
    pub fn new(columns: impl IntoIterator<Item = ColumnWidth>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            header: Vec::new(),
            body: Vec::new(),
            footer: Vec::new(),
            padding: 4.0,
            frame: None,
            header_rule: None,
            row_rule: None,
            header_background: None,
            space_before: 0.0,
            space_after: 6.0,
        }
    }

    /// With a header row after the header rows it has.
    #[must_use]
    pub fn header(mut self, row: TableRow) -> Self {
        self.header.push(row);
        self
    }

    /// With a body row after the body rows it has.
    #[must_use]
    pub fn row(mut self, row: TableRow) -> Self {
        self.body.push(row);
        self
    }

    /// With a footer row after the footer rows it has.
    #[must_use]
    pub fn footer(mut self, row: TableRow) -> Self {
        self.footer.push(row);
        self
    }

    /// With `padding` points between each cell's edges and its text.
    #[must_use]
    pub const fn with_padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    /// With a frame around the table's part on each page.
    #[must_use]
    pub fn with_frame(mut self, rule: TableRule) -> Self {
        self.frame = Some(rule);
        self
    }

    /// With a rule under the header rows (on every page).
    #[must_use]
    pub fn with_header_rule(mut self, rule: TableRule) -> Self {
        self.header_rule = Some(rule);
        self
    }

    /// With a rule between rows.
    #[must_use]
    pub fn with_row_rules(mut self, rule: TableRule) -> Self {
        self.row_rule = Some(rule);
        self
    }

    /// With the header rows on a background of `color`.
    #[must_use]
    pub fn with_header_background(mut self, color: Color) -> Self {
        self.header_background = Some(color);
        self
    }

    /// The table without its rows: what drawing it needs once its cells are sources.
    pub(crate) fn look(&self) -> Self {
        Self {
            columns: self.columns.clone(),
            header: Vec::new(),
            body: Vec::new(),
            footer: Vec::new(),
            padding: self.padding,
            frame: self.frame.clone(),
            header_rule: self.header_rule.clone(),
            row_rule: self.row_rule.clone(),
            header_background: self.header_background.clone(),
            space_before: self.space_before,
            space_after: self.space_after,
        }
    }

    /// With `before` and `after` points of space around the table.
    #[must_use]
    pub const fn with_space(mut self, before: f64, after: f64) -> Self {
        self.space_before = before;
        self.space_after = after;
        self
    }
}

/// Which part of the table a row is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Head,
    Body,
    Foot,
}

/// A cell as the flow holds it: its paragraphs as sources, and its element.
#[derive(Debug, Clone)]
pub(crate) struct CellItem {
    pub(crate) sources: Vec<usize>,
    pub(crate) col: usize,
    pub(crate) span: usize,
    pub(crate) valign: VerticalAlign,
    pub(crate) node: usize,
}

/// A row as the flow holds it.
#[derive(Debug, Clone)]
pub(crate) struct RowItem {
    pub(crate) section: Section,
    pub(crate) cells: Vec<CellItem>,
}

/// A table as the flow holds it: its rows in order (header, body, footer) and its look
/// ([`Table::look`]).
#[derive(Debug, Clone)]
pub(crate) struct TableItem {
    pub(crate) table: Table,
    pub(crate) rows: Vec<RowItem>,
}

impl TableItem {
    /// How many rows of `section` it has.
    pub(crate) fn count(&self, section: Section) -> usize {
        self.rows.iter().filter(|r| r.section == section).count()
    }
}

/// A paragraph of a cell, broken at the cell's width.
#[derive(Debug, Clone)]
pub(crate) struct CellBlock {
    pub(crate) source: usize,
    pub(crate) lines: Vec<Rc<MLine>>,
    /// From the cell's text top to the block's first line, points.
    pub(crate) top: f64,
}

/// A cell laid out.
#[derive(Debug, Clone)]
pub(crate) struct CellLayout {
    /// The cell's left edge from the table's, points.
    pub(crate) x: f64,
    /// From the row's top to the cell's text top (the padding and the vertical alignment).
    pub(crate) top: f64,
    pub(crate) blocks: Vec<CellBlock>,
    /// The cell's index in its row.
    pub(crate) cell: usize,
}

/// A row laid out.
#[derive(Debug, Clone)]
pub(crate) struct RowLayout {
    pub(crate) height: f64,
    pub(crate) cells: Vec<CellLayout>,
}

/// A table laid out at a width.
#[derive(Debug, Clone)]
pub(crate) struct TableLayout {
    pub(crate) width: f64,
    pub(crate) rows: Vec<RowLayout>,
}

impl TableLayout {
    /// The height of the rows of `section`.
    pub(crate) fn height(&self, item: &TableItem, section: Section) -> f64 {
        item.rows
            .iter()
            .zip(&self.rows)
            .filter(|(r, _)| r.section == section)
            .map(|(_, l)| l.height)
            .sum()
    }
}

/// `item` laid out `available` points wide.
pub(crate) fn layout(item: &TableItem, sources: &[Source], available: f64) -> Result<TableLayout> {
    let padding = item.table.padding.max(0.0);
    let natural = natural_widths(item, sources, padding)?;
    let widths = column_widths(&item.table.columns, &natural, available);
    let mut starts = Vec::with_capacity(widths.len() + 1);
    let mut x = 0.0;
    for w in &widths {
        starts.push(x);
        x += w;
    }
    starts.push(x);
    let mut rows = Vec::with_capacity(item.rows.len());
    for row in &item.rows {
        let mut cells = Vec::with_capacity(row.cells.len());
        for (n, cell) in row.cells.iter().enumerate() {
            let left = starts.get(cell.col).copied().unwrap_or(x);
            let right = starts.get(cell.col + cell.span).copied().unwrap_or(x);
            let width = right - left;
            // An automatic column is exactly as wide as its widest line: the allowance keeps
            // rounding (the breaker measures in f32) from wrapping that line's last glyph.
            let text_width = (width - 2.0 * padding).max(1.0) + 0.01;
            let mut blocks = Vec::with_capacity(cell.sources.len());
            let mut y = 0.0;
            let mut after: Option<f64> = None;
            for &s in &cell.sources {
                let Some(source) = sources.get(s) else { continue };
                if let Some(a) = after {
                    y += a.max(source.style.space_before);
                }
                let lines: Vec<Rc<MLine>> = source.lines(0, text_width)?.into_iter().map(Rc::new).collect();
                let top = y;
                y += lines.iter().map(|l| l.height).sum::<f64>();
                after = Some(source.style.space_after);
                blocks.push(CellBlock { source: s, lines, top });
            }
            cells.push((
                cell.valign,
                CellLayout {
                    x: left,
                    top: padding,
                    blocks,
                    cell: n,
                },
                y,
            ));
        }
        let content = cells.iter().map(|(_, _, h)| *h).fold(0.0, f64::max);
        let height = content + 2.0 * padding;
        let cells = cells
            .into_iter()
            .map(|(valign, mut cell, h)| {
                let slack = content - h;
                cell.top += match valign {
                    VerticalAlign::Top => 0.0,
                    VerticalAlign::Middle => slack / 2.0,
                    VerticalAlign::Bottom => slack,
                };
                cell
            })
            .collect();
        rows.push(RowLayout { height, cells });
    }
    Ok(TableLayout { width: x, rows })
}

/// Each automatic column's widest one-column cell, padding included (just the padding for a
/// column with none); 0 for the other columns, which never read it.
fn natural_widths(item: &TableItem, sources: &[Source], padding: f64) -> Result<Vec<f64>> {
    let columns = &item.table.columns;
    let mut natural: Vec<f64> = columns
        .iter()
        .map(|c| if *c == ColumnWidth::Auto { 2.0 * padding } else { 0.0 })
        .collect();
    let auto = |col: usize| columns.get(col) == Some(&ColumnWidth::Auto);
    for cell in item
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .filter(|c| c.span == 1 && auto(c.col))
    {
        let mut widest = 0.0_f64;
        for source in cell.sources.iter().filter_map(|&s| sources.get(s)) {
            for line in source.lines(0, f64::from(f32::MAX))? {
                widest = widest.max(line.line.width);
            }
        }
        if let Some(n) = natural.get_mut(cell.col) {
            *n = n.max(widest + 2.0 * padding);
        }
    }
    Ok(natural)
}

/// The columns' widths in `available` points, from their kinds and natural widths.
fn column_widths(columns: &[ColumnWidth], natural: &[f64], available: f64) -> Vec<f64> {
    let clean = |v: f64| if v.is_finite() { v.max(0.0) } else { 0.0 };
    let available = clean(available);
    let fixed: f64 = columns
        .iter()
        .map(|c| match c {
            ColumnWidth::Fixed(w) => clean(*w),
            _ => 0.0,
        })
        .sum();
    // Fixed columns wider than the frame shrink in proportion.
    let fixed_scale = if fixed > available && fixed > 0.0 {
        available / fixed
    } else {
        1.0
    };
    let left = (available - fixed * fixed_scale).max(0.0);
    let autos: Vec<usize> = (0..columns.len())
        .filter(|&i| columns.get(i) == Some(&ColumnWidth::Auto))
        .collect();
    let weight = |c: &ColumnWidth| match c {
        ColumnWidth::Fraction(f) => clean(*f),
        _ => 0.0,
    };
    let weights: f64 = columns.iter().map(weight).sum();
    let fractions = columns.iter().filter(|c| weight(c) > 0.0).count();
    let mut wanted: Vec<f64> = autos.iter().map(|&i| natural.get(i).copied().unwrap_or(0.0)).collect();
    if wanted.iter().sum::<f64>() > left {
        // Automatic columns too wide together: the fraction columns share the narrowing as
        // columns that want any width, so a long cell can't leave them nothing.
        wanted.extend(std::iter::repeat_n(f64::INFINITY, fractions));
    }
    let mut shares = fair_share(&wanted, left);
    let fraction_total: f64 = shares.drain(autos.len()..).sum();
    let auto_widths = shares;
    let auto_total: f64 = auto_widths.iter().sum();
    let rest = if fraction_total > 0.0 {
        fraction_total
    } else {
        (left - auto_total).max(0.0)
    };
    let mut out = Vec::with_capacity(columns.len());
    let mut next_auto = auto_widths.into_iter();
    for c in columns {
        out.push(match c {
            ColumnWidth::Fixed(w) => clean(*w) * fixed_scale,
            ColumnWidth::Auto => next_auto.next().unwrap_or(0.0),
            ColumnWidth::Fraction(f) if weights > 0.0 => rest * clean(*f) / weights,
            ColumnWidth::Fraction(_) => 0.0,
        });
    }
    out
}

/// `wanted` widths fitted into `total`: when they don't fit, the narrow ones keep theirs and
/// the wide ones share what is left equally (so a column of numbers isn't squeezed to make room
/// for one of descriptions).
fn fair_share(wanted: &[f64], total: f64) -> Vec<f64> {
    if wanted.iter().sum::<f64>() <= total {
        return wanted.to_vec();
    }
    let mut out = wanted.to_vec();
    let mut open: Vec<usize> = (0..wanted.len()).collect();
    let mut left = total;
    while !open.is_empty() {
        #[expect(clippy::cast_precision_loss, reason = "column counts are small")]
        let share = left / open.len() as f64;
        let (fits, wide): (Vec<usize>, Vec<usize>) =
            open.iter().partition(|&&i| wanted.get(i).is_some_and(|w| *w <= share));
        if fits.is_empty() {
            for i in wide {
                if let Some(o) = out.get_mut(i) {
                    *o = share;
                }
            }
            break;
        }
        for i in fits {
            left -= wanted.get(i).copied().unwrap_or(0.0);
        }
        open = wide;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_then_auto_then_fractions() {
        let cols = [
            ColumnWidth::Fixed(50.0),
            ColumnWidth::Auto,
            ColumnWidth::Fraction(1.0),
            ColumnWidth::Fraction(3.0),
        ];
        let widths = column_widths(&cols, &[0.0, 30.0, 0.0, 0.0], 200.0);
        assert_eq!(widths, vec![50.0, 30.0, 30.0, 90.0]);
    }

    #[test]
    fn autos_that_do_not_fit_narrow_the_widest_first() {
        // A narrow numbers column keeps its width; the two wide ones share the rest.
        let widths = column_widths(&[ColumnWidth::Auto; 3], &[40.0, 300.0, 500.0], 300.0);
        assert_eq!(widths, vec![40.0, 130.0, 130.0]);
        assert!((widths.iter().sum::<f64>() - 300.0).abs() < 1e-9);
    }

    #[test]
    fn autos_that_fit_keep_their_widths() {
        let widths = column_widths(&[ColumnWidth::Auto, ColumnWidth::Auto], &[40.0, 60.0], 300.0);
        assert_eq!(widths, vec![40.0, 60.0]);
    }

    #[test]
    fn fixed_columns_wider_than_the_frame_shrink_in_proportion() {
        let cols = [ColumnWidth::Fixed(300.0), ColumnWidth::Fixed(100.0), ColumnWidth::Auto];
        let widths = column_widths(&cols, &[0.0, 0.0, 50.0], 200.0);
        assert_eq!(widths, vec![150.0, 50.0, 0.0]);
    }

    #[test]
    fn a_wide_auto_column_leaves_the_fraction_columns_their_share() {
        let cols = [
            ColumnWidth::Auto,
            ColumnWidth::Fraction(1.0),
            ColumnWidth::Fraction(3.0),
        ];
        let widths = column_widths(&cols, &[600.0, 0.0, 0.0], 480.0);
        // Three columns share 480 points; the fractions split their two thirds one to three.
        assert_eq!(widths, vec![160.0, 80.0, 240.0]);
    }

    #[test]
    fn bad_widths_count_as_zero() {
        let cols = [
            ColumnWidth::Fixed(f64::NAN),
            ColumnWidth::Fraction(-1.0),
            ColumnWidth::Fraction(1.0),
        ];
        assert_eq!(column_widths(&cols, &[0.0; 3], 90.0), vec![0.0, 0.0, 90.0]);
    }
}
