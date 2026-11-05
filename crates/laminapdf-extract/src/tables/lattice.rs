//! Tables ruled with lines (the "lattice" of Camelot and pdfplumber): rules snapped to shared
//! coordinates and joined, connected rules grouped, each group's distinct coordinates taken as
//! the grid, and a missing inner rule read as a merged cell.

use laminapdf_model::{Pt, RectPt};

use super::edges::{Edges, Rule};
use super::{Glyph, Table, cell_text, header_rows};
use crate::model::{Cell, TableDetector};

/// Rules this close (in points) share a coordinate, and a rule may stop this short of a
/// crossing and still meet it (pdfplumber's `snap_tolerance` and `join_tolerance` defaults).
const SNAP: f64 = 3.0;

/// Rules shorter than this (in points) are ticks or underlines, not table rules.
const MIN_RULE: f64 = 6.0;

/// How sure a lattice table is: its grid is drawn, so only the cell boundaries are inferred.
const CONFIDENCE: f64 = 0.85;

/// A grid of at most this many cells could as well be a form's boxes.
const SMALL_GRID: usize = 4;
const SMALL_CONFIDENCE: f64 = 0.5;

/// Rules snapped: a run of rules whose coordinates lie within [`SNAP`] of the previous one
/// take their mean.
fn snap(rules: &mut [Rule]) {
    rules.sort_by(|a, b| a.at.total_cmp(&b.at));
    let mut start = 0;
    for end in 1..=rules.len() {
        let breaks = match (rules.get(end), rules.get(end.wrapping_sub(1))) {
            (Some(next), Some(prev)) => next.at - prev.at > SNAP,
            _ => true,
        };
        if breaks {
            if let Some(group) = rules.get_mut(start..end) {
                #[expect(clippy::cast_precision_loss, reason = "a page has far fewer than 2^52 rules")]
                let mean = group.iter().map(|r| r.at).sum::<f64>() / group.len() as f64;
                for r in group.iter_mut() {
                    r.at = mean;
                }
            }
            start = end;
        }
    }
}

/// Collinear rules that overlap or nearly meet, joined into one.
fn join(mut rules: Vec<Rule>) -> Vec<Rule> {
    rules.sort_by(|a, b| a.at.total_cmp(&b.at).then(a.from.total_cmp(&b.from)));
    let mut out: Vec<Rule> = Vec::with_capacity(rules.len());
    for r in rules {
        match out.last_mut() {
            Some(last) if (last.at - r.at).abs() < 1e-9 && r.from <= last.to + SNAP => last.to = last.to.max(r.to),
            _ => out.push(r),
        }
    }
    out.retain(|r| r.to - r.from >= MIN_RULE);
    out
}

/// Whether horizontal rule `h` and vertical rule `v` cross (or nearly meet).
fn crosses(h: &Rule, v: &Rule) -> bool {
    v.at >= h.from - SNAP && v.at <= h.to + SNAP && h.at >= v.from - SNAP && h.at <= v.to + SNAP
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while let Some(&p) = parent.get(i) {
        if p == i {
            break;
        }
        let grand = parent.get(p).copied().unwrap_or(p);
        if let Some(slot) = parent.get_mut(i) {
            *slot = grand;
        }
        i = grand;
    }
    i
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb
        && let Some(slot) = parent.get_mut(ra.max(rb))
    {
        *slot = ra.min(rb);
    }
}

/// The groups of rules that touch one another: each a candidate table.
fn components(h: &[Rule], v: &[Rule]) -> Vec<(Vec<Rule>, Vec<Rule>)> {
    let n = h.len() + v.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for (i, hr) in h.iter().enumerate() {
        for (j, vr) in v.iter().enumerate() {
            if crosses(hr, vr) {
                union(&mut parent, i, h.len() + j);
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, (Vec<Rule>, Vec<Rule>)> = std::collections::BTreeMap::new();
    for (i, r) in h.iter().enumerate() {
        groups.entry(find(&mut parent, i)).or_default().0.push(*r);
    }
    for (j, r) in v.iter().enumerate() {
        groups.entry(find(&mut parent, h.len() + j)).or_default().1.push(*r);
    }
    groups.into_values().collect()
}

/// The distinct coordinates of `rules`, ascending.
fn coordinates(rules: &[Rule]) -> Vec<f64> {
    let mut at: Vec<f64> = rules.iter().map(|r| r.at).collect();
    at.sort_by(f64::total_cmp);
    at.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    at
}

/// Whether a rule at `at` covers `from..to`.
fn covered(rules: &[Rule], at: f64, from: f64, to: f64) -> bool {
    rules
        .iter()
        .any(|r| (r.at - at).abs() < 1e-9 && r.from <= from + SNAP && r.to >= to - SNAP)
}

/// The cells' row and column spans: neighbouring grid cells with no rule between them are one
/// cell. `None` when a merged region isn't a rectangle (the rules don't describe a table cleanly).
fn spans(h: &[Rule], v: &[Rule], xs: &[f64], ys: &[f64]) -> Option<Vec<(usize, usize, usize, usize)>> {
    let (rows, cols) = (ys.len() - 1, xs.len() - 1);
    let at = |r: usize, c: usize| r * cols + c;
    let mut parent: Vec<usize> = (0..rows * cols).collect();
    for r in 0..rows {
        let (top, bottom) = (ys.get(r)?, ys.get(r + 1)?);
        for c in 0..cols.saturating_sub(1) {
            if !covered(v, *xs.get(c + 1)?, *top, *bottom) {
                union(&mut parent, at(r, c), at(r, c + 1));
            }
        }
    }
    for c in 0..cols {
        let (left, right) = (xs.get(c)?, xs.get(c + 1)?);
        for r in 0..rows.saturating_sub(1) {
            if !covered(h, *ys.get(r + 1)?, *left, *right) {
                union(&mut parent, at(r, c), at(r + 1, c));
            }
        }
    }
    // Per root: (min row, min col, max row, max col, count).
    let mut regions: std::collections::BTreeMap<usize, (usize, usize, usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for r in 0..rows {
        for c in 0..cols {
            let root = find(&mut parent, at(r, c));
            let e = regions.entry(root).or_insert((r, c, r, c, 0));
            *e = (e.0.min(r), e.1.min(c), e.2.max(r), e.3.max(c), e.4 + 1);
        }
    }
    let mut out = Vec::with_capacity(regions.len());
    for (r0, c0, r1, c1, count) in regions.into_values() {
        if count != (r1 - r0 + 1) * (c1 - c0 + 1) {
            return None;
        }
        out.push((r0, c0, r1 - r0 + 1, c1 - c0 + 1));
    }
    out.sort_unstable();
    Some(out)
}

/// The tables the page's rules draw, each with the glyphs inside its cells. `ambiguous` is set
/// on a table whose merged cells weren't rectangles; its cells are then the grid's, unmerged.
/// `limited` is set when the page had more rules, or a grid more cells, than `max_cells`
/// allows; what was past the limit gave no table.
pub(crate) fn tables(edges: &Edges, glyphs: &[Glyph], max_cells: usize, limited: &mut bool) -> Vec<Table> {
    let (mut h, mut v) = (edges.horizontal.clone(), edges.vertical.clone());
    snap(&mut h);
    snap(&mut v);
    let (h, v) = (join(h), join(v));
    // Grouping compares every horizontal rule with every vertical one.
    if h.len().saturating_mul(v.len()) > max_cells.saturating_mul(16) {
        *limited = true;
        return Vec::new();
    }
    let mut out = Vec::new();
    for (gh, gv) in components(&h, &v) {
        let (ys, xs) = (coordinates(&gh), coordinates(&gv));
        if ys.len() < 3 || xs.len() < 3 {
            continue;
        }
        if (ys.len() - 1).saturating_mul(xs.len() - 1) > max_cells {
            *limited = true;
            continue;
        }
        if let Some(table) = grid(&gh, &gv, &xs, &ys, edges, glyphs) {
            out.push(table);
        }
    }
    out
}

fn grid(h: &[Rule], v: &[Rule], xs: &[f64], ys: &[f64], edges: &Edges, glyphs: &[Glyph]) -> Option<Table> {
    let (rows, cols) = (ys.len() - 1, xs.len() - 1);
    let merged = spans(h, v, xs, ys);
    let ambiguous = merged.is_none();
    let regions = merged.unwrap_or_else(|| (0..rows).flat_map(|r| (0..cols).map(move |c| (r, c, 1, 1))).collect());
    let rect = |r0: usize, c0: usize, rs: usize, cs: usize| -> Option<RectPt> {
        Some(RectPt::new(
            Pt::new(*xs.get(c0)?),
            Pt::new(*ys.get(r0)?),
            Pt::new(*xs.get(c0 + cs)?),
            Pt::new(*ys.get(r0 + rs)?),
        ))
    };
    let mut cells = Vec::with_capacity(regions.len());
    let mut members = Vec::with_capacity(regions.len());
    for (row, col, row_span, col_span) in regions {
        let bbox = rect(row, col, row_span, col_span)?;
        let inside: Vec<usize> = (0..glyphs.len())
            .filter(|&i| glyphs.get(i).is_some_and(|g| contains_centre(&bbox, &g.bbox)))
            .collect();
        cells.push(Cell {
            row,
            col,
            row_span,
            col_span,
            header: false,
            text: cell_text(glyphs, &inside),
            bbox,
        });
        members.push(inside);
    }
    // An empty ruled grid is a form or a frame, not a table.
    if cells.iter().filter(|c| !c.text.is_empty()).count() < 2 {
        return None;
    }
    let bbox = rect(0, 0, rows, cols)?;
    let header = header_rows(&cells, &members, glyphs, &edges.fills, xs, ys);
    for cell in &mut cells {
        cell.header = cell.row < header;
    }
    let confidence = if rows * cols <= SMALL_GRID {
        SMALL_CONFIDENCE
    } else {
        CONFIDENCE
    };
    Some(Table {
        bbox,
        rows,
        cols,
        cells,
        detector: TableDetector::Lattice,
        confidence,
        ambiguous,
        glyphs: members.into_iter().flatten().collect(),
    })
}

/// Whether `outer` holds the centre of `inner`.
pub(crate) fn contains_centre(outer: &RectPt, inner: &RectPt) -> bool {
    let cx = f64::midpoint(inner.x0.get(), inner.x1.get());
    let cy = f64::midpoint(inner.y0.get(), inner.y1.get());
    cx >= outer.x0.get() && cx <= outer.x1.get() && cy >= outer.y0.get() && cy <= outer.y1.get()
}

#[cfg(test)]
mod tests {
    use super::{Rule, covered, join, snap, spans};

    fn rule(at: f64, from: f64, to: f64) -> Rule {
        Rule { at, from, to }
    }

    #[test]
    fn rules_within_the_snap_tolerance_share_a_coordinate() {
        let mut rules = vec![
            rule(100.0, 0.0, 50.0),
            rule(101.5, 50.0, 100.0),
            rule(200.0, 0.0, 100.0),
        ];
        snap(&mut rules);
        assert!((rules[0].at - 100.75).abs() < 1e-9 && (rules[1].at - 100.75).abs() < 1e-9);
        assert!((rules[2].at - 200.0).abs() < 1e-9);
    }

    #[test]
    fn collinear_rules_that_nearly_meet_are_joined() {
        let joined = join(vec![
            rule(10.0, 0.0, 40.0),
            rule(10.0, 42.0, 100.0),
            rule(10.0, 120.0, 200.0),
        ]);
        assert_eq!(joined, vec![rule(10.0, 0.0, 100.0), rule(10.0, 120.0, 200.0)]);
    }

    #[test]
    fn short_rules_are_left_out() {
        assert_eq!(join(vec![rule(10.0, 0.0, 4.0)]), Vec::new());
    }

    #[test]
    fn a_rule_covers_a_span_it_reaches_within_tolerance() {
        let rules = [rule(10.0, 0.0, 98.0)];
        assert!(covered(&rules, 10.0, 0.0, 100.0));
        assert!(!covered(&rules, 10.0, 0.0, 150.0));
    }

    #[test]
    fn a_missing_inner_rule_merges_two_cells() {
        let xs = [0.0, 50.0, 100.0];
        let ys = [0.0, 20.0, 40.0];
        // Every horizontal rule; the middle vertical rule only on the second row.
        let h = [rule(0.0, 0.0, 100.0), rule(20.0, 0.0, 100.0), rule(40.0, 0.0, 100.0)];
        let v = [rule(0.0, 0.0, 40.0), rule(50.0, 20.0, 40.0), rule(100.0, 0.0, 40.0)];
        let merged = spans(&h, &v, &xs, &ys).unwrap();
        assert_eq!(merged, vec![(0, 0, 1, 2), (1, 0, 1, 1), (1, 1, 1, 1)]);
    }

    #[test]
    fn a_merged_region_that_is_not_a_rectangle_is_refused() {
        let xs = [0.0, 50.0, 100.0];
        let ys = [0.0, 20.0, 40.0];
        // No middle vertical on row 0, no middle horizontal under column 0: an L shape.
        let h = [rule(0.0, 0.0, 100.0), rule(20.0, 50.0, 100.0), rule(40.0, 0.0, 100.0)];
        let v = [rule(0.0, 0.0, 40.0), rule(50.0, 20.0, 40.0), rule(100.0, 0.0, 40.0)];
        assert_eq!(spans(&h, &v, &xs, &ys), None);
    }
}
