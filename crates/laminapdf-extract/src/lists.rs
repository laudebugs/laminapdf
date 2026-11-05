//! List items ([ADR 0030] §3): a block that starts with a bullet or a number, its later lines
//! hanging to the right of the marker; the depth from the marker's indent.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use crate::model::{Block, BlockKind};

/// The bullets: ADR 0030 §3's five, and the round and square ones word processors draw.
const BULLETS: [char; 8] = ['•', '◦', '▪', '–', '*', '●', '○', '■'];

/// The longest lowercase Roman numeral taken as a marker (`viii`).
const MAX_ROMAN: usize = 4;

/// Whether `label` is a number a list counts with: up to three digits, one lowercase letter or
/// a short lowercase Roman numeral. Uppercase letters are left out, so a wrapped line starting
/// with an initial ("A. Smith") is never a list item.
fn counter(label: &str) -> bool {
    let digits = !label.is_empty() && label.len() <= 3 && label.chars().all(|c| c.is_ascii_digit());
    let letter = label.len() == 1 && label.chars().all(|c| c.is_ascii_lowercase());
    let roman = !label.is_empty() && label.len() <= MAX_ROMAN && label.chars().all(|c| matches!(c, 'i' | 'v' | 'x'));
    digits || letter || roman
}

/// Whether `word` (no spaces) is a list marker: a bullet, or a counter followed by `.` or `)`,
/// or a counter in parentheses.
pub(crate) fn is_marker(word: &str) -> bool {
    let mut chars = word.chars();
    if let (Some(c), None) = (chars.next(), chars.next())
        && BULLETS.contains(&c)
    {
        return true;
    }
    if let Some(inner) = word.strip_prefix('(').and_then(|w| w.strip_suffix(')')) {
        return counter(inner);
    }
    word.strip_suffix(['.', ')']).is_some_and(counter)
}

/// The marker that starts `text` and the text after it, when `text` starts with a marker
/// followed by a space and more text.
pub(crate) fn split_marker(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    let (word, rest) = text.split_once(char::is_whitespace)?;
    let rest = rest.trim_start();
    (is_marker(word) && !rest.is_empty()).then_some((word, rest))
}

/// Whether a block's lines after its first hang to the right of its left edge (the marker's):
/// a block of one line has none to check.
fn hangs(block: &Block) -> bool {
    let Some(first) = block.spans.iter().map(|s| s.baseline.get()).min_by(f64::total_cmp) else {
        return false;
    };
    block
        .spans
        .iter()
        .filter(|s| s.baseline.get() - first > 0.5 * s.size.get())
        .all(|s| s.bbox.x0.get() > block.bbox.x0.get() + INDENT_TOLERANCE)
}

/// Makes each paragraph of `blocks` that starts with a marker and hangs a list item, its text
/// the text after the marker, then gives the items their depths page by page.
pub(crate) fn assign(blocks: &mut [Block]) {
    for block in blocks.iter_mut() {
        if block.kind != BlockKind::Paragraph || !hangs(block) {
            continue;
        }
        let Some((marker, rest)) = split_marker(&block.text) else {
            continue;
        };
        let (marker, rest) = (marker.to_owned(), rest.to_owned());
        block.kind = BlockKind::ListItem { marker, depth: 0 };
        block.text = rest;
    }
    for page in blocks.chunk_by_mut(|a, b| a.page == b.page) {
        assign_depths(page);
    }
}

/// Indents closer than this, in points, are one depth.
const INDENT_TOLERANCE: f64 = 3.0;

/// Gives each list-item block of one page its depth: consecutive items in reading order, each
/// below the last, form one list, and an item indented past the one before it goes one level
/// deeper. A block that isn't an item, or an item above the last (the next column), starts a
/// new list.
fn assign_depths(blocks: &mut [Block]) {
    // The left edges of the open levels, outermost first, and the last item's top.
    let mut levels: Vec<f64> = Vec::new();
    let mut last_top: Option<f64> = None;
    for block in blocks {
        let (x, top) = (block.bbox.x0.get(), block.bbox.y0.get());
        let BlockKind::ListItem { depth, .. } = &mut block.kind else {
            levels.clear();
            last_top = None;
            continue;
        };
        if last_top.is_some_and(|t| top < t) {
            levels.clear();
        }
        while levels.last().is_some_and(|&l| x < l - INDENT_TOLERANCE) {
            levels.pop();
        }
        if levels.last().is_none_or(|&l| x > l + INDENT_TOLERANCE) {
            levels.push(x);
        }
        *depth = u8::try_from(levels.len().saturating_sub(1)).unwrap_or(u8::MAX);
        last_top = Some(top);
    }
}

#[cfg(test)]
mod tests {
    use super::{is_marker, split_marker};

    #[test]
    fn bullets_and_counters_are_markers() {
        for m in ["•", "–", "*", "1.", "12)", "(3)", "a.", "(b)", "iv.", "(ii)"] {
            assert!(is_marker(m), "{m}");
        }
    }

    #[test]
    fn words_initials_and_years_are_not_markers() {
        for m in ["A.", "word.", "2024.", "(1", "ab.", "••", "1"] {
            assert!(!is_marker(m), "{m}");
        }
    }

    #[test]
    fn a_marker_needs_text_after_it() {
        assert_eq!(split_marker("1. First step"), Some(("1.", "First step")));
        assert_eq!(split_marker("•  Apples"), Some(("•", "Apples")));
        assert_eq!(split_marker("1."), None);
        assert_eq!(split_marker("In 2024. Then"), None);
    }
}
