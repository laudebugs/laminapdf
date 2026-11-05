//! Markdown: `CommonMark` with GitHub's pipe tables. Text is escaped only where it would change
//! the structure: a block that starts like a heading, list item, quote or rule, `|` in a table
//! cell, and `<` before something a renderer would read as HTML.

use std::fmt::Write as _;

use super::{Rendered, Writer};
use crate::model::{Block, BlockKind, Cell, Extraction};

/// How Markdown is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct MarkdownOptions {
    /// Write `<!-- page N -->` (1-based) before each page's first block.
    pub page_markers: bool,
}

impl MarkdownOptions {
    /// No page markers.
    #[must_use]
    pub const fn new() -> Self {
        Self { page_markers: false }
    }

    /// Page markers on or off.
    #[must_use]
    pub const fn with_page_markers(mut self, on: bool) -> Self {
        self.page_markers = on;
        self
    }
}

pub(super) fn render(extraction: &Extraction, options: MarkdownOptions) -> Rendered {
    let mut w = Writer::default();
    let mut page = None;
    let mut list = ListIndent::default();
    for (i, block) in extraction.blocks.iter().enumerate() {
        if i > 0 {
            w.push("\n\n");
        }
        if options.page_markers && page != Some(block.page) {
            let number = extraction
                .pages
                .iter()
                .find(|p| p.index == block.page)
                .map_or(block.page.saturating_add(1), |p| p.number);
            w.push(&format!("<!-- page {number} -->\n\n"));
            page = Some(block.page);
        }
        w.block(&block_markdown(block, &mut list));
    }
    w.finish()
}

/// Where each open level of a list puts its text, in spaces: `CommonMark` nests an item under
/// another only when it is indented to the other's text (3 spaces under `1. `, 2 under `- `).
#[derive(Debug, Default)]
struct ListIndent {
    columns: Vec<usize>,
}

impl ListIndent {
    /// The indent of an item at `depth` whose marker is written as `marker`.
    fn item(&mut self, depth: usize, marker: &str) -> usize {
        self.columns.truncate(depth);
        let indent = self.columns.last().copied().unwrap_or(0);
        self.columns.push(indent + marker.chars().count() + 1);
        indent
    }
}

fn block_markdown(block: &Block, list: &mut ListIndent) -> String {
    if !matches!(block.kind, BlockKind::ListItem { .. }) {
        list.columns.clear();
    }
    match &block.kind {
        BlockKind::Heading { level } => {
            let level = usize::from((*level).clamp(1, 6));
            format!("{} {}", "#".repeat(level), escape_inline(&block.text))
        }
        BlockKind::ListItem { marker, depth } => {
            let marker = list_marker(marker);
            let indent = " ".repeat(list.item(usize::from(*depth), marker));
            format!("{indent}{marker} {}", escape_inline(&block.text))
        }
        BlockKind::Table { rows, cols, .. } => table(&block.cells, *rows, *cols),
        _ => escape_block(&block.text),
    }
}

/// A numbered marker (`1.`, `2)`) stays; any bullet becomes `-`.
fn list_marker(marker: &str) -> &str {
    let numbered = marker.len() > 1
        && marker.ends_with(['.', ')'])
        && marker
            .get(..marker.len() - 1)
            .is_some_and(|n| n.chars().all(|c| c.is_ascii_digit()));
    if numbered { marker } else { "-" }
}

/// Escapes `<` where a renderer would start HTML: before a letter, `/`, `!` or `?`.
fn escape_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<'
            && chars
                .peek()
                .is_some_and(|n| n.is_ascii_alphabetic() || matches!(n, '/' | '!' | '?'))
        {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A paragraph: escapes a start a renderer would read as structure (`#`, `>`, `-`, `+`, `*`,
/// `_`, `=`, a fence, or a number followed by `.` or `)`).
fn escape_block(text: &str) -> String {
    let inline = escape_inline(text);
    let digits = inline.chars().take_while(char::is_ascii_digit).count();
    let after_digits = inline.get(digits..).unwrap_or_default();
    let structural = inline.starts_with(['#', '>', '-', '+', '*', '_', '=', '`', '~'])
        || (digits > 0 && after_digits.starts_with(['.', ')']));
    if !structural {
        return inline;
    }
    if digits > 0 {
        // Escape the punctuation, not the number: `1\. Introduction`.
        let (number, rest) = inline.split_at(digits);
        return format!("{number}\\{rest}");
    }
    format!("\\{inline}")
}

/// A table: a pipe table when no cell spans, else HTML (pipe tables can't span).
fn table(cells: &[Cell], rows: usize, cols: usize) -> String {
    if cells.iter().any(|c| c.row_span > 1 || c.col_span > 1) {
        return html_table(cells, rows);
    }
    let mut grid = vec![vec![String::new(); cols]; rows];
    for cell in cells {
        if let Some(slot) = grid.get_mut(cell.row).and_then(|r| r.get_mut(cell.col)) {
            *slot = escape_inline(&cell.text).replace('|', "\\|");
        }
    }
    let line = |row: &[String]| format!("| {} |", row.join(" | "));
    let mut out = Vec::with_capacity(rows + 1);
    // Pipe tables need a header row: the first row, header or not.
    let mut rows_iter = grid.iter();
    if let Some(first) = rows_iter.next() {
        out.push(line(first));
        out.push(format!("|{}|", vec![" --- "; cols].join("|")));
    }
    out.extend(rows_iter.map(|r| line(r)));
    out.join("\n")
}

fn html_table(cells: &[Cell], rows: usize) -> String {
    let mut out = String::from("<table>\n");
    for row in 0..rows {
        out.push_str("<tr>");
        for cell in cells.iter().filter(|c| c.row == row) {
            let tag = if cell.header { "th" } else { "td" };
            out.push('<');
            out.push_str(tag);
            if cell.row_span > 1 {
                let _ = write!(out, " rowspan=\"{}\"", cell.row_span);
            }
            if cell.col_span > 1 {
                let _ = write!(out, " colspan=\"{}\"", cell.col_span);
            }
            out.push('>');
            out.push_str(&escape_html(&cell.text));
            let _ = write!(out, "</{tag}>");
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>");
    out
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{escape_block, escape_inline, list_marker, table};
    use crate::model::Cell;

    fn cell(row: usize, col: usize, text: &str, span: (usize, usize)) -> Cell {
        Cell {
            row,
            col,
            row_span: span.0,
            col_span: span.1,
            header: row == 0,
            text: text.to_owned(),
            bbox: RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(1.0), Pt::new(1.0)),
        }
    }

    #[test]
    fn text_that_looks_like_structure_is_escaped() {
        assert_eq!(escape_block("# not a heading"), "\\# not a heading");
        assert_eq!(escape_block("1. Introduction"), "1\\. Introduction");
        assert_eq!(escape_block("2024 was good"), "2024 was good");
        assert_eq!(escape_block("__________"), "\\__________");
        assert_eq!(escape_inline("a <b> and a < b"), "a \\<b> and a < b");
    }

    #[test]
    fn bullets_become_dashes_and_numbers_stay() {
        assert_eq!(list_marker("•"), "-");
        assert_eq!(list_marker("12."), "12.");
        assert_eq!(list_marker("a)"), "-");
    }

    #[test]
    fn a_table_without_spans_is_a_pipe_table_with_pipes_escaped() {
        let cells = [
            cell(0, 0, "A", (1, 1)),
            cell(0, 1, "B|C", (1, 1)),
            cell(1, 0, "1", (1, 1)),
            cell(1, 1, "2", (1, 1)),
        ];
        assert_eq!(table(&cells, 2, 2), "| A | B\\|C |\n| --- | --- |\n| 1 | 2 |");
    }

    #[test]
    fn a_table_with_a_span_is_html() {
        let cells = [
            cell(0, 0, "Both", (1, 2)),
            cell(1, 0, "a < b", (1, 1)),
            cell(1, 1, "2", (1, 1)),
        ];
        assert_eq!(
            table(&cells, 2, 2),
            "<table>\n<tr><th colspan=\"2\">Both</th></tr>\n<tr><td>a &lt; b</td><td>2</td></tr>\n</table>"
        );
    }
}
