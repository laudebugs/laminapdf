//! Footnotes on the page (ADR 0032 §3): a line that cites notes fits only with its notes, which
//! go at the foot of its page, above the footer, under a short rule. A line whose notes don't
//! fit moves to the next page with them.

use std::rc::Rc;

use super::items::Content;
use super::measure::MLine;
use super::paginate::{Lines, Pager, Placed};
use crate::error::{Error, Result};

/// The space above a page's first note, which holds the rule, points.
pub(crate) const NOTE_GAP: f64 = 12.0;
/// The space between two notes, points.
pub(crate) const NOTE_SPACING: f64 = 2.0;
/// The rule's length, points (at most the body's width).
const RULE_LENGTH: f64 = 72.0;

impl Pager<'_> {
    /// The notes `line` of item `i` cites, in order.
    pub(super) fn line_notes(&self, i: usize, line: &MLine) -> Vec<usize> {
        let source = match self.items.entries.get(i).map(|it| it.content) {
            Some(Content::Para(s)) => self.items.sources.get(s),
            _ => None,
        };
        source.map_or_else(Vec::new, |s| {
            s.markers
                .iter()
                .filter(|(at, _)| line.line.range.contains(at))
                .map(|(_, note)| *note)
                .collect()
        })
    }

    /// The notes cells of row `row` of table `t` cite, in order.
    pub(super) fn row_notes(&self, t: usize, row: usize) -> Vec<usize> {
        let Some(row) = self.items.tables.get(t).and_then(|t| t.rows.get(row)) else {
            return Vec::new();
        };
        row.cells
            .iter()
            .flat_map(|c| &c.sources)
            .filter_map(|&s| self.items.sources.get(s))
            .flat_map(|s| s.markers.iter().map(|(_, note)| *note))
            .collect()
    }

    /// The room `notes` take at the page's foot on top of the page's notes and `cited` (notes
    /// already counted for lines not placed yet): their heights, and the gap above the first.
    pub(super) fn notes_cost(&self, cited: &[usize], notes: &[usize]) -> Result<f64> {
        if notes.is_empty() {
            return Ok(0.0);
        }
        let mut cost = 0.0;
        for &note in notes {
            cost += self.note_lines(note)?.iter().map(|l| l.height).sum::<f64>() + NOTE_SPACING;
        }
        if self.notes.is_empty() && cited.is_empty() {
            cost += NOTE_GAP;
        }
        Ok(cost)
    }

    /// Puts `note` at the foot of the current page.
    pub(super) fn add_note(&mut self, note: usize) -> Result<()> {
        let cost = self.notes_cost(&[], &[note])?;
        let lines = self.note_lines(note)?;
        self.notes.push((note, lines));
        self.notes_height += cost;
        Ok(())
    }

    /// `note`'s body broken at the body's width.
    fn note_lines(&self, note: usize) -> Result<Lines> {
        let source = self.items.notes.get(note).ok_or(Error::InvalidGeometry {
            what: "a flow names no footnote",
        })?;
        let width = self.page_template().body_width().max(1.0);
        self.source_lines(source.source, 0, width)
    }

    /// Draws the current page's notes at its foot, under their rule, and starts the next
    /// page's.
    pub(super) fn finish_notes(&mut self) {
        if self.notes.is_empty() {
            return;
        }
        let template = self.page_template();
        let x = template.margins.left;
        let width = template.body_width();
        let mut top = template.body_y().1 - self.notes_height + NOTE_GAP;
        let mut placed = vec![Placed::NoteRule {
            x,
            y: top - NOTE_GAP / 2.0,
            width: RULE_LENGTH.min(width),
        }];
        for (note, lines) in std::mem::take(&mut self.notes) {
            for line in lines.iter() {
                placed.push(Placed::NoteLine {
                    note,
                    line: Rc::clone(line),
                    x,
                    top,
                });
                top += line.height;
            }
            top += NOTE_SPACING;
        }
        self.notes_height = 0.0;
        if let Some(page) = self.pages.last_mut() {
            page.placed.extend(placed);
        }
    }
}
