//! Tables: ruled ones from the page's lines ([`lattice`]), and, on request, unruled ones whose
//! text lines up in columns ([`stream`]). No table beats a wrong table, so both refuse anything
//! that doesn't look like a grid of text: a page border or a form's field boxes give none.

mod edges;
mod lattice;
mod stream;

use laminapdf_model::RectPt;

pub(crate) use edges::{Collector, Edges};

use crate::lines::{SPACE, mostly_rtl};
use crate::model::{Cell, TableDetector};

/// One visible glyph, as tables see it.
#[derive(Debug, Clone)]
pub(crate) struct Glyph {
    pub(crate) text: String,
    /// Its box in page space.
    pub(crate) bbox: RectPt,
    /// The font size in points.
    pub(crate) size: f64,
    /// Whether its font is bold (by name: `Bold`, `Black`, `Heavy`, `Semibold`).
    pub(crate) bold: bool,
}

/// A table found on a page.
#[derive(Debug, Clone)]
pub(crate) struct Table {
    pub(crate) bbox: RectPt,
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    /// Row by row, left to right.
    pub(crate) cells: Vec<Cell>,
    pub(crate) detector: TableDetector,
    pub(crate) confidence: f64,
    /// The rules described merged cells that aren't rectangles; the grid's cells were kept.
    pub(crate) ambiguous: bool,
    /// The indices of the glyphs inside its cells.
    pub(crate) glyphs: Vec<usize>,
}

/// Which detectors run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TableMode {
    /// No table detection: every block is text.
    Off,
    /// Ruled tables only (the default).
    #[default]
    Lattice,
    /// Ruled tables, then text aligned in at least three rows and two columns outside them. It
    /// finds tables drawn without lines, and sometimes columns of text that aren't one.
    LatticeAndStream,
}

/// The page's tables, ruled ones first, then (with [`TableMode::LatticeAndStream`]) unruled
/// ones in what the ruled ones left.
/// `limited` is set when [`lattice::tables`] stopped at `max_cells`.
pub(crate) fn detect(
    mode: TableMode,
    edges: &Edges,
    glyphs: &[Glyph],
    max_cells: usize,
    limited: &mut bool,
) -> Vec<Table> {
    if mode == TableMode::Off {
        return Vec::new();
    }
    let mut tables = lattice::tables(edges, glyphs, max_cells, limited);
    if mode == TableMode::LatticeAndStream {
        let taken: std::collections::BTreeSet<usize> = tables.iter().flat_map(|t| t.glyphs.iter().copied()).collect();
        let free: Vec<usize> = (0..glyphs.len()).filter(|i| !taken.contains(i)).collect();
        tables.extend(stream::tables(glyphs, &free));
    }
    tables
}

/// The glyphs `members` grouped into lines, top to bottom: a glyph joins the line whose first
/// glyph's bottom lies within half a font size of its own. Both cell text and unruled rows
/// read lines this way.
pub(crate) fn group_lines(glyphs: &[Glyph], members: &[usize]) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = members.iter().copied().filter(|&i| i < glyphs.len()).collect();
    order.sort_by(|&a, &b| {
        let y = |i: usize| glyphs.get(i).map_or(0.0, |g| g.bbox.y1.get());
        y(a).total_cmp(&y(b))
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
    lines
}

/// The text of the glyphs `members`, line by line, each line in reading order (right to left
/// when most of its letters are), a space where the pen jumps, lines joined by a space.
pub(crate) fn cell_text(glyphs: &[Glyph], members: &[usize]) -> String {
    let mut out = String::new();
    for line in group_lines(glyphs, members) {
        let mut line: Vec<&Glyph> = line.iter().filter_map(|&i| glyphs.get(i)).collect();
        line.sort_by(|a, b| a.bbox.x0.get().total_cmp(&b.bbox.x0.get()));
        if mostly_rtl(&line.iter().map(|g| g.text.as_str()).collect::<String>()) {
            line.reverse();
        }
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
        let mut prev: Option<&Glyph> = None;
        for g in line {
            if let Some(p) = prev {
                let gap = (g.bbox.x0.get() - p.bbox.x1.get()).max(p.bbox.x0.get() - g.bbox.x1.get());
                if gap > SPACE * g.size
                    && !out.ends_with(char::is_whitespace)
                    && !g.text.starts_with(char::is_whitespace)
                {
                    out.push(' ');
                }
            }
            out.push_str(&g.text);
            prev = Some(g);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How many rows at the top are header rows: the first row, when its text is all bold and a
/// later row's isn't, or when a filled area shades it and not the row below. 0 otherwise.
pub(crate) fn header_rows(
    cells: &[Cell],
    members: &[Vec<usize>],
    glyphs: &[Glyph],
    fills: &[RectPt],
    xs: &[f64],
    ys: &[f64],
) -> usize {
    let row_bold = |row: usize| -> Option<bool> {
        let mut seen = false;
        for (cell, inside) in cells.iter().zip(members) {
            if cell.row != row {
                continue;
            }
            for g in inside.iter().filter_map(|&i| glyphs.get(i)) {
                if g.text.trim().is_empty() {
                    continue;
                }
                seen = true;
                if !g.bold {
                    return Some(false);
                }
            }
        }
        seen.then_some(true)
    };
    let rows = ys.len().saturating_sub(1);
    let bold_head = row_bold(0) == Some(true) && (1..rows).any(|r| row_bold(r) == Some(false));
    let shaded = |row: usize| -> bool {
        let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) = (xs.first(), xs.last(), ys.get(row), ys.get(row + 1)) else {
            return false;
        };
        fills.iter().any(|f| {
            let w = (f.x1.get().min(x1) - f.x0.get().max(x0)).max(0.0);
            let h = (f.y1.get().min(y1) - f.y0.get().max(y0)).max(0.0);
            w * h >= 0.8 * (x1 - x0) * (y1 - y0)
        })
    };
    let shaded_head = rows > 1 && shaded(0) && !shaded(1);
    usize::from(bold_head || shaded_head)
}

/// Whether a font name is a bold face's.
pub(crate) fn is_bold(font: &str) -> bool {
    let lower = font.to_ascii_lowercase();
    ["bold", "black", "heavy", "semibold", "demi"]
        .iter()
        .any(|w| lower.contains(w))
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{Glyph, cell_text, is_bold};

    fn glyph(text: &str, x: f64, y: f64) -> Glyph {
        Glyph {
            text: text.to_owned(),
            bbox: RectPt::new(Pt::new(x), Pt::new(y - 10.0), Pt::new(x + 6.0), Pt::new(y)),
            size: 10.0,
            bold: false,
        }
    }

    #[test]
    fn a_cells_text_reads_line_by_line_with_spaces_at_gaps() {
        let glyphs = [
            glyph("b", 106.0, 40.0),
            glyph("a", 100.0, 40.0),
            glyph("c", 120.0, 40.0),
            glyph("d", 100.0, 60.0),
        ];
        assert_eq!(cell_text(&glyphs, &[0, 1, 2, 3]), "ab c d");
    }

    #[test]
    fn a_right_to_left_cell_reads_from_the_right() {
        // Two Hebrew words drawn left to right as separate pieces: the right one is first.
        let glyphs = [glyph("שלום", 100.0, 40.0), glyph("עולם", 60.0, 40.0)];
        assert_eq!(cell_text(&glyphs, &[0, 1]), "שלום עולם");
    }

    #[test]
    fn bold_faces_are_recognised_by_name() {
        assert!(is_bold("Helvetica-Bold") && is_bold("NotoSans-SemiBold") && !is_bold("Times-Roman"));
    }
}
