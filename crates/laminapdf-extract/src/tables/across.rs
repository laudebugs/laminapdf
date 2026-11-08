//! Tables that go on over a page break: the part at the foot of one page and the part at the top
//! of the next are linked (`continued_by`, `continues`), so the renderings can write one table.
//! Each part stays a block of its own, with its page's box.

use std::collections::BTreeMap;

use laminapdf_model::RectPt;

use super::lattice::SNAP;
use crate::model::{Block, BlockKind, Cell, Extraction, TableDetector, header_row_count};

/// The first part's bottom edge must be at least this far down its page (a share of the height).
const BOTTOM: f64 = 0.75;
/// The second part's top edge must be at most this far down its page.
const TOP: f64 = 0.25;
/// The longest line that may stand between the parts when it says the table goes on.
const CONTINUED_MAX_CHARS: usize = 60;

/// Links every pair of table parts that one table split over a page break. Runs once furniture
/// is gone and block IDs are final.
pub(crate) fn link(out: &mut Extraction) {
    // Each page's first and last block, found once: blocks are grouped by page.
    let mut ends: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    for (i, block) in out.blocks.iter().enumerate() {
        ends.entry(block.page).and_modify(|e| e.1 = i).or_insert((i, i));
    }
    // The first part of each chain, by the index of any of its parts: a later part's header is
    // compared with the table's own header, wherever the chain began.
    let mut head: BTreeMap<usize, usize> = BTreeMap::new();
    let heights: Vec<(usize, f64)> = out.pages.iter().map(|p| (p.index, p.height.get())).collect();
    for pair in heights.windows(2) {
        let [(page, height), (next, next_height)] = pair else {
            continue;
        };
        if *next != page + 1 {
            continue;
        }
        let (Some(&(_, last)), Some(&(first, _))) = (ends.get(page), ends.get(next)) else {
            continue;
        };
        let Some((a, b)) = candidates(&out.blocks, last, first) else {
            continue;
        };
        let start = head.get(&a).copied().unwrap_or(a);
        let (Some(part), Some(second), Some(chain_head)) =
            (out.blocks.get(a), out.blocks.get(b), out.blocks.get(start))
        else {
            continue;
        };
        if !(at_foot(&part.bbox, *height) && at_top(&second.bbox, *next_height)) {
            continue;
        }
        let Some(repeated) = joinable(part, second, &chain_head.cells) else {
            continue;
        };
        let (part_id, second_id) = (part.id.clone(), second.id.clone());
        if let Some(part) = out.blocks.get_mut(a) {
            part.continued_by = Some(second_id);
        }
        if let Some(second) = out.blocks.get_mut(b) {
            second.continues = Some(part_id);
            second
                .cells
                .iter_mut()
                .filter(|c| c.row < repeated)
                .for_each(|c| c.header = true);
        }
        head.insert(b, start);
    }
}

/// The indices of a page's last table and the next page's first table, given those pages' last
/// and first blocks, when nothing but one short "continued" line stands between them.
fn candidates(blocks: &[Block], last: usize, first: usize) -> Option<(usize, usize)> {
    let (page, next) = (blocks.get(last)?.page, blocks.get(first)?.page);
    let is_table = |i: usize| blocks.get(i).is_some_and(|b| matches!(b.kind, BlockKind::Table { .. }));
    let is_continued = |i: usize| blocks.get(i).is_some_and(says_continued);
    match (is_table(last), is_table(first)) {
        (true, true) => Some((last, first)),
        // One "continued" line at the foot of the first page or the top of the next.
        (false, true) if last > 0 && is_continued(last) && is_table(last - 1) => blocks
            .get(last - 1)
            .filter(|b| b.page == page)
            .map(|_| (last - 1, first)),
        (true, false) if is_continued(first) && is_table(first + 1) => blocks
            .get(first + 1)
            .filter(|b| b.page == next)
            .map(|_| (last, first + 1)),
        _ => None,
    }
}

/// Whether `block` is a short line that says a table goes on: "continued", "cont." or "cont'd".
fn says_continued(block: &Block) -> bool {
    if !matches!(block.kind, BlockKind::Paragraph | BlockKind::Caption | BlockKind::Other)
        || block.text.chars().count() > CONTINUED_MAX_CHARS
    {
        return false;
    }
    let text = block.text.to_lowercase().replace('\u{2019}', "'");
    // Whole words only, so "discontinued" doesn't count.
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '.'))
        .map(|word| word.trim_end_matches('.'))
        .any(|word| matches!(word, "continued" | "cont" | "cont'd"))
}

fn at_foot(bbox: &RectPt, page_height: f64) -> bool {
    bbox.y1.get() >= BOTTOM * page_height
}

fn at_top(bbox: &RectPt, page_height: f64) -> bool {
    bbox.y0.get() <= TOP * page_height
}

/// `Some(repeated)` when `second` can go on from `first`: the same detector and columns, and a
/// header that fits the table's own (`header`, its first part's cells). `repeated` is how many
/// of the second part's first rows repeat that header.
fn joinable(first: &Block, second: &Block, header: &[Cell]) -> Option<usize> {
    let (
        BlockKind::Table { cols, detector, .. },
        BlockKind::Table {
            cols: cols2,
            detector: detector2,
            ..
        },
    ) = (&first.kind, &second.kind)
    else {
        return None;
    };
    if cols != cols2 || detector != detector2 || *detector == TableDetector::Tagged {
        return None;
    }
    let a = columns(&first.cells, &first.bbox, *cols)?;
    let b = columns(&second.cells, &second.bbox, *cols)?;
    let aligned = match detector {
        TableDetector::Lattice => edges_match(&a, &b),
        _ => overlaps_match(&a, &b),
    };
    if !aligned {
        return None;
    }
    header_fits(header, &second.cells)
}

/// Each column's left and right edge, measured from the table's left edge: the cells that start
/// in it and span no other. `None` when a column has no such cell.
fn columns(cells: &[Cell], bbox: &RectPt, cols: usize) -> Option<Vec<(f64, f64)>> {
    let left = bbox.x0.get();
    (0..cols)
        .map(|col| {
            let mut own = cells.iter().filter(|c| c.col == col && c.col_span == 1);
            let first = own.next()?;
            let start = (first.bbox.x0.get(), first.bbox.x1.get());
            let (x0, x1) = own.fold(start, |(x0, x1), c| (x0.min(c.bbox.x0.get()), x1.max(c.bbox.x1.get())));
            Some((x0 - left, x1 - left))
        })
        .collect()
}

/// Ruled tables: every column edge within `SNAP` of the other part's.
fn edges_match(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.iter()
        .zip(b)
        .all(|(p, q)| (p.0 - q.0).abs() <= SNAP && (p.1 - q.1).abs() <= SNAP)
}

/// Unruled tables: every column overlaps the same column of the other part and no other.
fn overlaps_match(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    let overlaps = |p: &(f64, f64), q: &(f64, f64)| p.0 < q.1 && q.0 < p.1;
    a.iter()
        .enumerate()
        .all(|(i, p)| b.iter().enumerate().all(|(j, q)| overlaps(p, q) == (i == j)))
}

/// `Some(repeated)` when the second part's header fits the table's (`first`, its first part's
/// cells): it repeats all of the header's rows (`repeated` is their count), neither has a
/// header (`0`), or only the table does and the second part's first rows differ (`0`). `None`
/// when the second part has a header of its own: it starts a new table.
fn header_fits(first: &[Cell], second: &[Cell]) -> Option<usize> {
    let rows = header_row_count(first);
    let repeats = rows > 0 && (0..rows).all(|row| row_texts(first, row) == row_texts(second, row));
    if repeats {
        Some(rows)
    } else if header_row_count(second) > 0 {
        None
    } else {
        Some(0)
    }
}

/// The texts of the cells that start in `row`, by column, white space collapsed.
fn row_texts(cells: &[Cell], row: usize) -> Vec<(usize, String)> {
    let mut texts: Vec<(usize, String)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.text.split_whitespace().collect::<Vec<_>>().join(" ")))
        .collect();
    texts.sort();
    texts
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{edges_match, header_fits, overlaps_match, says_continued};
    use crate::model::{Block, BlockKind, Cell};

    fn cell(row: usize, col: usize, text: &str, header: bool) -> Cell {
        Cell {
            row,
            col,
            row_span: 1,
            col_span: 1,
            header,
            text: text.to_owned(),
            bbox: RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(1.0), Pt::new(1.0)),
        }
    }

    fn paragraph(text: &str) -> Block {
        Block {
            id: "p1-b0".to_owned(),
            kind: BlockKind::Paragraph,
            text: text.to_owned(),
            page: 1,
            bbox: RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(1.0), Pt::new(1.0)),
            bbox_norm: None,
            struct_path: Vec::new(),
            struct_ref: None,
            spans: Vec::new(),
            confidence: 1.0,
            cells: Vec::new(),
            continues: None,
            continued_by: None,
            from_model: false,
        }
    }

    #[test]
    fn a_short_continued_line_is_recognised_in_any_case_and_form() {
        assert!(says_continued(&paragraph("Table 1 (continued)")));
        assert!(says_continued(&paragraph("CONT'D")));
        assert!(says_continued(&paragraph("Prices, cont.")));
        assert!(!says_continued(&paragraph("Discontinued products")));
        assert!(!says_continued(&paragraph("Table 1")));
        assert!(!says_continued(&paragraph(&format!("continued {}", "x".repeat(60)))));
    }

    #[test]
    fn ruled_columns_line_up_within_the_snap_distance() {
        assert!(edges_match(
            &[(0.0, 100.0), (100.0, 200.0)],
            &[(2.0, 101.0), (101.0, 202.5)]
        ));
        assert!(!edges_match(
            &[(0.0, 100.0), (100.0, 200.0)],
            &[(0.0, 80.0), (80.0, 180.0)]
        ));
    }

    #[test]
    fn unruled_columns_overlap_their_own_column_only() {
        assert!(overlaps_match(
            &[(0.0, 40.0), (60.0, 90.0)],
            &[(5.0, 30.0), (55.0, 95.0)]
        ));
        assert!(!overlaps_match(
            &[(0.0, 40.0), (60.0, 90.0)],
            &[(0.0, 70.0), (75.0, 95.0)]
        ));
    }

    #[test]
    fn a_repeated_header_joins_and_is_reported() {
        let first = [
            cell(0, 0, "Item", true),
            cell(0, 1, "Price", true),
            cell(1, 0, "Tea", false),
        ];
        let repeated = [
            cell(0, 0, "Item", false),
            cell(0, 1, " Price ", false),
            cell(1, 0, "Jam", false),
        ];
        assert_eq!(header_fits(&first, &repeated), Some(1));
    }

    #[test]
    fn a_two_row_header_repeats_only_when_both_rows_do() {
        let first = [
            cell(0, 0, "Item", true),
            cell(1, 0, "(EUR)", true),
            cell(2, 0, "Tea", false),
        ];
        let both = [
            cell(0, 0, "Item", false),
            cell(1, 0, "(EUR)", false),
            cell(2, 0, "Jam", false),
        ];
        assert_eq!(header_fits(&first, &both), Some(2));
        let one = [cell(0, 0, "Item", true), cell(1, 0, "Jam", false)];
        assert_eq!(header_fits(&first, &one), None);
    }

    #[test]
    fn rows_without_a_header_join_and_a_different_header_does_not() {
        let first = [cell(0, 0, "Item", true), cell(1, 0, "Tea", false)];
        let plain = [cell(0, 0, "Jam", false), cell(1, 0, "Bread", false)];
        assert_eq!(header_fits(&first, &plain), Some(0));
        let other = [cell(0, 0, "Total", true), cell(1, 0, "9.00", false)];
        assert_eq!(header_fits(&first, &other), None);
        let headless = [cell(0, 0, "Tea", false)];
        assert_eq!(header_fits(&headless, &plain), Some(0));
        assert_eq!(header_fits(&headless, &other), None);
    }
}
