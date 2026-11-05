//! Tables drawn without lines (the "stream" of Camelot and Tabula): consecutive lines of text
//! each split by wide gaps into pieces that stack into the same columns. Running text never
//! splits like that, so three or more such lines with two or more columns are taken as a table,
//! with a lower confidence than a ruled one.

use laminapdf_model::{Pt, RectPt};

use super::{Glyph, Table, cell_text};
use crate::model::{Cell, TableDetector};

/// A gap wider than this many font sizes splits a line into cells (word spaces are a third).
const CELL_GAP: f64 = 1.0;

/// Lines further apart than this many font sizes don't belong to one table.
const ROW_GAP: f64 = 2.5;

const MIN_ROWS: usize = 3;
const MIN_COLS: usize = 2;
const CONFIDENCE: f64 = 0.4;

/// A piece of one line: its glyphs and their extent.
#[derive(Debug, Clone)]
struct Piece {
    glyphs: Vec<usize>,
    x0: f64,
    x1: f64,
}

/// A line of the free glyphs, split into pieces at wide gaps.
#[derive(Debug, Clone)]
struct Row {
    top: f64,
    bottom: f64,
    size: f64,
    pieces: Vec<Piece>,
}

fn rows(glyphs: &[Glyph], free: &[usize]) -> Vec<Row> {
    let mut order: Vec<usize> = free.to_vec();
    order.sort_by(|&a, &b| {
        let (ga, gb) = (glyphs.get(a), glyphs.get(b));
        let ya = ga.map_or(0.0, |g| g.bbox.y1.get());
        let yb = gb.map_or(0.0, |g| g.bbox.y1.get());
        ya.total_cmp(&yb)
    });
    let mut lines: Vec<Vec<usize>> = Vec::new();
    for i in order {
        let Some(g) = glyphs.get(i) else { continue };
        let joins = lines
            .last()
            .and_then(|l| l.first())
            .and_then(|&f| glyphs.get(f))
            .is_some_and(|f| (g.bbox.y1.get() - f.bbox.y1.get()).abs() <= 0.5 * f.size.max(g.size));
        match lines.last_mut() {
            Some(line) if joins => line.push(i),
            _ => lines.push(vec![i]),
        }
    }
    lines.into_iter().filter_map(|line| row(glyphs, line)).collect()
}

fn row(glyphs: &[Glyph], mut line: Vec<usize>) -> Option<Row> {
    line.sort_by(|&a, &b| {
        let xa = glyphs.get(a).map_or(0.0, |g| g.bbox.x0.get());
        let xb = glyphs.get(b).map_or(0.0, |g| g.bbox.x0.get());
        xa.total_cmp(&xb)
    });
    let first = glyphs.get(*line.first()?)?;
    let mut out = Row {
        top: f64::INFINITY,
        bottom: f64::NEG_INFINITY,
        size: first.size,
        pieces: Vec::new(),
    };
    for i in line {
        let Some(g) = glyphs.get(i) else { continue };
        if g.text.trim().is_empty() {
            continue;
        }
        out.top = out.top.min(g.bbox.y0.get());
        out.bottom = out.bottom.max(g.bbox.y1.get());
        match out.pieces.last_mut() {
            Some(p) if g.bbox.x0.get() - p.x1 <= CELL_GAP * g.size => {
                p.glyphs.push(i);
                p.x1 = p.x1.max(g.bbox.x1.get());
            }
            _ => out.pieces.push(Piece {
                glyphs: vec![i],
                x0: g.bbox.x0.get(),
                x1: g.bbox.x1.get(),
            }),
        }
    }
    (!out.pieces.is_empty()).then_some(out)
}

/// Which column of `columns` each piece of `row` falls in (by overlap), or `None` when a piece
/// overlaps none or two, or two pieces fall in one column.
fn place(columns: &[(f64, f64)], row: &Row) -> Option<Vec<usize>> {
    let mut used = vec![false; columns.len()];
    let mut out = Vec::with_capacity(row.pieces.len());
    for p in &row.pieces {
        let hits: Vec<usize> = (0..columns.len())
            .filter(|&c| columns.get(c).is_some_and(|&(x0, x1)| p.x0 < x1 && p.x1 > x0))
            .collect();
        let [c] = hits.as_slice() else { return None };
        let slot = used.get_mut(*c)?;
        if *slot {
            return None;
        }
        *slot = true;
        out.push(*c);
    }
    Some(out)
}

/// The unruled tables among the glyphs `free`.
pub(crate) fn tables(glyphs: &[Glyph], free: &[usize]) -> Vec<Table> {
    let rows = rows(glyphs, free);
    let mut out = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        let Some(first) = rows.get(i) else { break };
        if first.pieces.len() < MIN_COLS {
            i += 1;
            continue;
        }
        let mut columns: Vec<(f64, f64)> = first.pieces.iter().map(|p| (p.x0, p.x1)).collect();
        let mut run = vec![(i, (0..first.pieces.len()).collect::<Vec<_>>())];
        let mut j = i + 1;
        while let (Some(prev), Some(next)) = (rows.get(j - 1), rows.get(j)) {
            if next.pieces.len() < MIN_COLS || next.top - prev.bottom > ROW_GAP * prev.size {
                break;
            }
            let Some(placed) = place(&columns, next) else { break };
            for (p, &c) in next.pieces.iter().zip(&placed) {
                if let Some(col) = columns.get_mut(c) {
                    *col = (col.0.min(p.x0), col.1.max(p.x1));
                }
            }
            run.push((j, placed));
            j += 1;
        }
        if run.len() >= MIN_ROWS {
            if let Some(table) = build(glyphs, &rows, &run, &columns) {
                out.push(table);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn build(glyphs: &[Glyph], rows: &[Row], run: &[(usize, Vec<usize>)], columns: &[(f64, f64)]) -> Option<Table> {
    let mut cells = Vec::new();
    let mut members = Vec::new();
    let (mut top, mut bottom) = (f64::INFINITY, f64::NEG_INFINITY);
    for (r, (index, placed)) in run.iter().enumerate() {
        let row = rows.get(*index)?;
        top = top.min(row.top);
        bottom = bottom.max(row.bottom);
        for (c, &(x0, x1)) in columns.iter().enumerate() {
            let inside: Vec<usize> = placed
                .iter()
                .position(|&pc| pc == c)
                .and_then(|p| row.pieces.get(p))
                .map(|p| p.glyphs.clone())
                .unwrap_or_default();
            cells.push(Cell {
                row: r,
                col: c,
                row_span: 1,
                col_span: 1,
                header: false,
                text: cell_text(glyphs, &inside),
                bbox: RectPt::new(Pt::new(x0), Pt::new(row.top), Pt::new(x1), Pt::new(row.bottom)),
            });
            members.extend(inside);
        }
    }
    let (x0, x1) = (columns.first()?.0, columns.last()?.1);
    Some(Table {
        bbox: RectPt::new(Pt::new(x0), Pt::new(top), Pt::new(x1), Pt::new(bottom)),
        rows: run.len(),
        cols: columns.len(),
        cells,
        detector: TableDetector::Stream,
        confidence: CONFIDENCE,
        ambiguous: false,
        glyphs: members,
    })
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{Glyph, tables};

    /// One glyph per word, 6 pt wide per character.
    fn word(text: &str, x: f64, y: f64) -> Glyph {
        #[expect(clippy::cast_precision_loss, reason = "short test words")]
        let w = 6.0 * text.len() as f64;
        Glyph {
            text: text.to_owned(),
            bbox: RectPt::new(Pt::new(x), Pt::new(y - 10.0), Pt::new(x + w), Pt::new(y)),
            size: 10.0,
            bold: false,
        }
    }

    #[test]
    fn three_aligned_rows_of_two_columns_are_a_table() {
        let glyphs = [
            word("Item", 72.0, 100.0),
            word("Price", 300.0, 100.0),
            word("Tea", 72.0, 114.0),
            word("2.50", 300.0, 114.0),
            word("Cake", 72.0, 128.0),
            word("4.00", 300.0, 128.0),
        ];
        let free: Vec<usize> = (0..glyphs.len()).collect();
        let found = tables(&glyphs, &free);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].rows, found[0].cols), (3, 2));
        let texts: Vec<&str> = found[0].cells.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(texts, ["Item", "Price", "Tea", "2.50", "Cake", "4.00"]);
    }

    #[test]
    fn running_text_is_not_a_table() {
        let glyphs = [
            word("The", 72.0, 100.0),
            word("cat", 94.0, 100.0),
            word("sat", 72.0, 114.0),
            word("down", 94.0, 114.0),
        ];
        let free: Vec<usize> = (0..glyphs.len()).collect();
        assert_eq!(tables(&glyphs, &free).len(), 0);
    }
}
