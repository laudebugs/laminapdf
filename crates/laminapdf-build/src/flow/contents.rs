//! Contents pages and the outline (ADR 0032 §3): every heading gets a named destination, the
//! outline nests them by level, and a contents page lists them with the pages they start on.
//!
//! The page numbers come from laying the flow out: the contents page is drawn with the numbers
//! of the last layout, and the flow is laid out again until no number changes, at most
//! [`MAX_PASSES`] times; then it fails with `flow-layout-unstable`.

use super::model::{FlowBlock, Inline, Paragraph, TextStyle};
use crate::error::{Error, Result};
use crate::nav::{LinkTarget, OutlineEntry};

/// How many times a flow with a contents page is laid out before it gives up.
pub(crate) const MAX_PASSES: usize = 4;

/// A contents page: the flow's headings up to a level, each indented by its level, with dot
/// leaders and the page it starts on at the end of its line, and a link to the heading.
/// Tagged `TOC`, each entry a `TOCI` holding its `Link`s; the leaders are `Layout` artifacts.
///
/// The numbers are the document's page numbers, as `{page}` counts them. Entries are laid out
/// left to right, the numbers at the right edge.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Contents {
    pub(crate) style: TextStyle,
    pub(crate) levels: u8,
    pub(crate) indent: f64,
}

impl Contents {
    /// A contents page in `style` listing headings of levels 1 and 2, each level indented 12
    /// points more than the one above it.
    #[must_use]
    pub fn new(style: &TextStyle) -> Self {
        Self {
            style: style.clone(),
            levels: 2,
            indent: 12.0,
        }
    }

    /// Listing headings of levels 1 to `levels` (1 to 6).
    #[must_use]
    pub fn with_levels(mut self, levels: u8) -> Self {
        self.levels = levels.clamp(1, 6);
        self
    }

    /// Indenting each level `indent` points more than the one above it.
    #[must_use]
    pub const fn with_indent(mut self, indent: f64) -> Self {
        self.indent = indent;
        self
    }
}

/// A heading of the flow: its level and its text, in flow order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Heading {
    pub(crate) level: u8,
    pub(crate) text: String,
}

/// The flow's headings, in order, including those inside `Keep` groups.
pub(crate) fn headings(blocks: &[FlowBlock]) -> Vec<Heading> {
    let mut out = Vec::new();
    collect(blocks, &mut out);
    out
}

fn collect(blocks: &[FlowBlock], out: &mut Vec<Heading>) {
    for block in blocks {
        match block {
            FlowBlock::Heading(level, p) => out.push(Heading {
                level: (*level).clamp(1, 6),
                text: plain_text(p),
            }),
            FlowBlock::Keep(inner) => collect(inner, out),
            _ => {}
        }
    }
}

/// `paragraph`'s text without its footnotes, line breaks as spaces.
pub(crate) fn plain_text(paragraph: &Paragraph) -> String {
    let mut text = String::new();
    for inline in &paragraph.inlines {
        match inline {
            Inline::Text(t, _) | Inline::Link(t, _, _) | Inline::Code(t, _) => text.push_str(t),
            Inline::LineBreak => text.push(' '),
            Inline::Note(_) => {}
        }
    }
    text.trim().to_owned()
}

/// The named destination of heading `n` of the flow whose first page is `first`: unique in
/// the document, since every flow adds at least one page.
pub(crate) fn destination(first: usize, n: usize) -> String {
    format!("flow{first}-h{n}")
}

/// Whether `blocks` hold a contents page (so the flow needs more than one layout).
pub(crate) fn has_contents(blocks: &[FlowBlock]) -> bool {
    blocks.iter().any(|b| match b {
        FlowBlock::Contents(_) => true,
        FlowBlock::Keep(inner) => has_contents(inner),
        _ => false,
    })
}

/// Lays out until the page numbers settle: `lay` takes the numbers to print and returns its
/// output and the numbers it found; the first numbers are `initial`. Returns the output of the
/// layout whose numbers were the ones it printed.
///
/// # Errors
/// What `lay` returns; [`Error::FlowLayoutUnstable`] when the numbers still change after
/// [`MAX_PASSES`] layouts.
pub(crate) fn settle<T>(initial: Vec<usize>, mut lay: impl FnMut(&[usize]) -> Result<(T, Vec<usize>)>) -> Result<T> {
    let mut numbers = initial;
    for _ in 0..MAX_PASSES {
        let (out, found) = lay(&numbers)?;
        if found == numbers {
            return Ok(out);
        }
        numbers = found;
    }
    Err(Error::FlowLayoutUnstable)
}

/// The outline of `headings`, each going to `target(n)`: an item's children are the headings
/// after it of a deeper level, up to the next one of its level or higher.
pub(crate) fn outline(headings: &[Heading], target: impl Fn(usize) -> LinkTarget) -> Vec<OutlineEntry> {
    let items: Vec<(u8, OutlineEntry)> = headings
        .iter()
        .enumerate()
        .map(|(n, h)| (h.level, OutlineEntry::new(h.text.clone(), target(n))))
        .collect();
    nest(&items)
}

fn nest(items: &[(u8, OutlineEntry)]) -> Vec<OutlineEntry> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some((level, entry)) = items.get(i) {
        let end = items
            .iter()
            .skip(i + 1)
            .position(|(l, _)| l <= level)
            .map_or(items.len(), |p| i + 1 + p);
        let children = nest(items.get(i + 1..end).unwrap_or_default());
        out.push(entry.clone().with_children(children));
        i = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_repeat_until_the_numbers_settle() {
        let mut calls = 0;
        let out = settle(vec![1, 1], |numbers| {
            calls += 1;
            // Every layout finds the headings on pages 2 and 3, so the second one settles.
            Ok((numbers.to_vec(), vec![2, 3]))
        })
        .unwrap();
        assert_eq!((out, calls), (vec![2, 3], 2));
    }

    #[test]
    fn a_layout_that_never_settles_fails_after_four() {
        let mut calls = 0;
        let result = settle(vec![1], |numbers| {
            calls += 1;
            Ok(((), vec![numbers[0] + 1]))
        });
        assert!(matches!(result, Err(Error::FlowLayoutUnstable)));
        assert_eq!(calls, MAX_PASSES);
    }

    #[test]
    fn the_outline_nests_headings_by_level() {
        let h = |level, text: &str| Heading {
            level,
            text: text.to_owned(),
        };
        let headings = [
            h(1, "A"),
            h(2, "A.1"),
            h(3, "A.1.a"),
            h(2, "A.2"),
            h(1, "B"),
            h(3, "B.x"),
        ];
        let tree = outline(&headings, |n| LinkTarget::Named(destination(0, n)));
        let shape: Vec<(String, Vec<(String, usize)>)> = tree
            .iter()
            .map(|e| {
                let kids = e
                    .children()
                    .iter()
                    .map(|c| (c.title().to_owned(), c.children().len()))
                    .collect();
                (e.title().to_owned(), kids)
            })
            .collect();
        assert_eq!(
            shape,
            vec![
                ("A".to_owned(), vec![("A.1".to_owned(), 1), ("A.2".to_owned(), 0)]),
                ("B".to_owned(), vec![("B.x".to_owned(), 0)]),
            ]
        );
    }
}
