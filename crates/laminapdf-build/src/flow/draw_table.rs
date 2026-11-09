//! Drawing a table's rows and rules. Header rows drawn again on a later page are `Pagination`
//! artifacts, so the structure tree holds the header once, as the PDF Association's *Tagged
//! PDF Best Practice Guide: Syntax* recommends for table headers repeated on each page; rules
//! and the header background are `Layout` artifacts.

use std::collections::BTreeMap;
use std::rc::Rc;

use laminapdf_model::{FillRule, PointPt, Pt, RectPt};

use super::emit::{draw_line, ensure};
use super::items::{Content, Items};
use super::model::Flow;
use super::table::{Section, TableItem, TableLayout, TableRule};
use crate::error::Result;
use crate::surface::Surface;
use crate::surface::style::StrokeStyle;
use crate::tag::{ArtifactKind, ElementId};

/// A row to draw: `row` of item `item`'s table laid out as `layout`, its top at `top` and the
/// table's left at `x`.
#[derive(Debug, Clone)]
pub(crate) struct RowAt {
    pub(crate) item: usize,
    pub(crate) layout: Rc<TableLayout>,
    pub(crate) row: usize,
    pub(crate) x: f64,
    pub(crate) top: f64,
    pub(crate) repeat: bool,
}

/// The rows of one table drawn on one page, for its rules.
#[derive(Debug)]
pub(crate) struct Part {
    item: usize,
    layout: Rc<TableLayout>,
    x: f64,
    top: f64,
    /// Each row's bottom and section, in order.
    rows: Vec<(f64, Section)>,
}

impl Part {
    /// Whether `at` continues this part.
    pub(crate) const fn holds(&self, at: &RowAt) -> bool {
        self.item == at.item
    }
}

fn table_of(items: &Items, item: usize) -> Option<&TableItem> {
    match items.entries.get(item)?.content {
        Content::Table(t) => items.tables.get(t),
        _ => None,
    }
}

/// Draws `at`'s row, its cells in their elements (or a `Pagination` artifact for a repeated
/// header row), and adds it to `part`, which it starts when `None`.
pub(crate) fn draw_row(
    s: &mut Surface,
    flow: &Flow,
    items: &Items,
    at: &RowAt,
    part: &mut Option<Part>,
    open: &mut BTreeMap<usize, ElementId>,
    order: &mut Vec<ElementId>,
) -> Result<()> {
    let layout = &at.layout;
    let Some(table) = table_of(items, at.item) else {
        return Ok(());
    };
    let (Some(row), Some(laid)) = (table.rows.get(at.row), layout.rows.get(at.row)) else {
        return Ok(());
    };
    if row.section == Section::Head
        && let Some(color) = &table.table.header_background
    {
        let rect = RectPt::from_points(at.x, at.top, layout.width, laid.height);
        layout_artifact(s, flow.tagged, |s| {
            s.fill_color(color.clone())
                .path(|p| {
                    p.rect(rect);
                })
                .fill(FillRule::NonZero);
        })?;
    }
    let padding = table.table.padding.max(0.0);
    for cell in &laid.cells {
        let Some(node) = row.cells.get(cell.cell).map(|c| c.node) else {
            continue;
        };
        let draw = |s: &mut Surface, tagged: bool| -> Result<()> {
            for block in &cell.blocks {
                let Some(source) = items.sources.get(block.source) else {
                    continue;
                };
                let mut y = at.top + cell.top + block.top;
                for line in &block.lines {
                    draw_line(s, tagged, source, line, at.x + cell.x + padding, y);
                    y += line.height;
                }
            }
            Ok(())
        };
        if !flow.tagged {
            draw(s, false)?;
        } else if at.repeat {
            s.artifact(ArtifactKind::Pagination, |s| draw(s, false))?;
        } else {
            // An empty cell is still opened, so every row has its cells.
            let id = ensure(s, items, node, open, order)?;
            s.resume(id, |s| draw(s, true))?;
        }
    }
    let bottom = at.top + laid.height;
    match part {
        Some(p) => p.rows.push((bottom, row.section)),
        None => {
            *part = Some(Part {
                item: at.item,
                layout: Rc::clone(layout),
                x: at.x,
                top: at.top,
                rows: vec![(bottom, row.section)],
            });
        }
    }
    Ok(())
}

/// Draws `part`'s rules: between rows, under the header rows, and the frame.
pub(crate) fn draw_rules(s: &mut Surface, flow: &Flow, items: &Items, part: &Part) -> Result<()> {
    let Some(table) = table_of(items, part.item).map(|t| &t.table) else {
        return Ok(());
    };
    if table.frame.is_none() && table.header_rule.is_none() && table.row_rule.is_none() {
        return Ok(());
    }
    let (left, right) = (part.x, part.x + part.layout.width);
    let bottom = part.rows.last().map_or(part.top, |r| r.0);
    layout_artifact(s, flow.tagged, |s| {
        for pair in part.rows.windows(2) {
            let [(y, above), (_, below)] = pair else { continue };
            let under_header = *above == Section::Head && *below != Section::Head;
            let rule = match (&table.header_rule, under_header) {
                (Some(rule), true) => Some(rule),
                _ => table.row_rule.as_ref(),
            };
            if let Some(rule) = rule {
                line(s, rule, (left, *y), (right, *y));
            }
        }
        if let Some(rule) = &table.frame {
            stroke(s, rule, |p| {
                p.rect(RectPt::from_points(left, part.top, right - left, bottom - part.top));
            });
        }
    })
}

fn line(s: &mut Surface, rule: &TableRule, from: (f64, f64), to: (f64, f64)) {
    stroke(s, rule, |p| {
        p.move_to(PointPt::from_points(from.0, from.1))
            .line_to(PointPt::from_points(to.0, to.1));
    });
}

fn stroke(s: &mut Surface, rule: &TableRule, build: impl FnOnce(&mut crate::surface::path::PathBuilder)) {
    let style = StrokeStyle {
        width: Pt::new(rule.width.max(0.0)),
        ..StrokeStyle::default()
    };
    s.stroke_color(rule.color.clone()).path(build).stroke(&style);
}

/// Runs `draw` inside a `Layout` artifact when the flow is tagged, its colours restored after.
/// The save is outside the artifact: marked content can't span a `q` or `Q`, so the builder
/// would end the artifact there and start another.
fn layout_artifact(s: &mut Surface, tagged: bool, draw: impl FnOnce(&mut Surface)) -> Result<()> {
    s.save();
    let drawn = if tagged {
        s.artifact(ArtifactKind::Layout, |s| {
            draw(s);
            Ok(())
        })
    } else {
        draw(s);
        Ok(())
    };
    s.restore();
    drawn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_holds_rows_of_its_own_table_only() {
        let layout = Rc::new(TableLayout {
            width: 10.0,
            rows: Vec::new(),
        });
        let part = Part {
            item: 3,
            layout,
            x: 0.0,
            top: 0.0,
            rows: Vec::new(),
        };
        let at = |item| RowAt {
            item,
            layout: Rc::new(TableLayout {
                width: 10.0,
                rows: Vec::new(),
            }),
            row: 0,
            x: 0.0,
            top: 0.0,
            repeat: false,
        };
        assert!(part.holds(&at(3)));
        assert!(!part.holds(&at(4)));
    }
}
