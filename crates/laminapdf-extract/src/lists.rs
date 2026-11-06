//! List items ([ADR 0030] §3): a block that starts with a bullet or a number, its later lines
//! hanging to the right of the marker; the depth from the marker's indent.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use crate::model::{Block, BlockKind};

/// Bullets prose never starts a line with: ADR 0030 §3's `•`, `◦` and `▪`, and the round and
/// square ones word processors draw.
const BULLETS: [char; 6] = ['•', '◦', '▪', '●', '○', '■'];

/// Bullets prose can start a line with too (a dash before a clause, an attribution, a
/// footnote's asterisk): ADR 0030 §3's other two.
const WEAK_BULLETS: [char; 2] = ['–', '*'];

/// The largest Roman numeral a list counts to (`xxxix`).
const MAX_ROMAN_ITEM: u32 = 39;

/// The Roman digits, largest first, with the subtractive pairs.
const ROMAN: [(u32, &str); 9] = [
    (100, "c"),
    (90, "xc"),
    (50, "l"),
    (40, "xl"),
    (10, "x"),
    (9, "ix"),
    (5, "v"),
    (4, "iv"),
    (1, "i"),
];

/// The value of `word` as a lowercase Roman numeral in its standard form (`iv`, not `iiii`),
/// below 400; `None` for anything else.
pub(crate) fn roman(word: &str) -> Option<u32> {
    let mut rest = word;
    let mut value = 0;
    while let Some(&(n, digits)) = ROMAN.iter().find(|(_, d)| rest.starts_with(d)) {
        rest = rest.get(digits.len()..)?;
        value += n;
    }
    // Written back in the standard form, it must give `word` again.
    let mut standard = String::new();
    let mut left = value;
    for (n, digits) in ROMAN {
        while left >= n {
            standard.push_str(digits);
            left -= n;
        }
    }
    (!word.is_empty() && rest.is_empty() && value < 400 && standard == word).then_some(value)
}

/// Whether `label` is a number a list counts with: up to three digits, one lowercase letter or
/// a lowercase Roman numeral. Uppercase letters are left out, so a wrapped line starting with
/// an initial ("A. Smith") is never a list item.
fn counter(label: &str) -> bool {
    let digits = !label.is_empty() && label.len() <= 3 && label.chars().all(|c| c.is_ascii_digit());
    let letter = label.len() == 1 && label.chars().all(|c| c.is_ascii_lowercase());
    digits || letter || roman(label).is_some_and(|n| n <= MAX_ROMAN_ITEM)
}

/// How a list marker can be told from prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Marker {
    /// A bullet prose never starts a line with.
    Bullet,
    /// A dash or an asterisk, which prose can start a line with too.
    WeakBullet,
    /// A counter followed by `.` or `)`, or in parentheses.
    Counter,
}

/// What `word` (no spaces) is as a list marker, if it is one.
pub(crate) fn marker(word: &str) -> Option<Marker> {
    let mut chars = word.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if BULLETS.contains(&c) {
            return Some(Marker::Bullet);
        }
        if WEAK_BULLETS.contains(&c) {
            return Some(Marker::WeakBullet);
        }
    }
    let counted = match word.strip_prefix('(').and_then(|w| w.strip_suffix(')')) {
        Some(inner) => counter(inner),
        None => word.strip_suffix(['.', ')']).is_some_and(counter),
    };
    counted.then_some(Marker::Counter)
}

/// Whether `word` (no spaces) is a list marker.
pub(crate) fn is_marker(word: &str) -> bool {
    marker(word).is_some()
}

/// The marker that starts `text`, what it is, and the text after it, when `text` starts with
/// a marker followed by a space and more text.
pub(crate) fn split_marker(text: &str) -> Option<(&str, Marker, &str)> {
    let text = text.trim_start();
    let (word, rest) = text.split_once(char::is_whitespace)?;
    let rest = rest.trim_start();
    let kind = marker(word)?;
    (!rest.is_empty()).then_some((word, kind, rest))
}

/// Indents closer than this, in points, are one depth.
const INDENT_TOLERANCE: f64 = 3.0;

/// How a block's lines after its first sit against its left edge (the marker's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// One line.
    OneLine,
    /// Every later line hangs to the right of the marker.
    Hanging,
    /// A later line starts at the marker: a paragraph that happens to start with one.
    Flush,
}

fn shape(block: &Block) -> Shape {
    let Some(first) = block.spans.iter().map(|s| s.baseline.get()).min_by(f64::total_cmp) else {
        return Shape::OneLine;
    };
    let mut later = block
        .spans
        .iter()
        .filter(|s| s.baseline.get() - first > 0.5 * s.size.get())
        .peekable();
    if later.peek().is_none() {
        Shape::OneLine
    } else if later.all(|s| s.bbox.x0.get() > block.bbox.x0.get() + INDENT_TOLERANCE) {
        Shape::Hanging
    } else {
        Shape::Flush
    }
}

/// The marker of a paragraph that could be a list item, and whether it could be one on its
/// own (`false`: one line after a dash or an asterisk, an item only beside another such line).
fn candidate(block: &Block) -> Option<(&str, bool)> {
    if block.kind != BlockKind::Paragraph || !block.struct_path.is_empty() {
        return None;
    }
    let (marker, kind, _) = split_marker(&block.text)?;
    match (shape(block), kind) {
        (Shape::Flush, _) => None,
        (Shape::OneLine, Marker::WeakBullet) => Some((marker, false)),
        _ => Some((marker, true)),
    }
}

/// Makes each paragraph of `blocks` that starts with a marker and hangs a list item, its text
/// the text after the marker, then gives the items their depths page by page. A one-line
/// block after a dash or an asterisk is an item only next to another item on its page with the
/// same marker or under which it is indented, so a lone attribution (`– Mark Twain`) stays a
/// paragraph.
pub(crate) fn assign(blocks: &mut [Block]) {
    let items: Vec<bool> = {
        let found: Vec<Option<(&str, bool)>> = blocks.iter().map(candidate).collect();
        // Another item beside it on its page, with the same marker or with this one indented
        // under it.
        let beside = |i: usize, j: Option<usize>, marker: &str| {
            let (Some(here), Some(j)) = (blocks.get(i), j) else {
                return false;
            };
            let Some(((other, _), there)) = found.get(j).copied().flatten().zip(blocks.get(j)) else {
                return false;
            };
            let nested = here.bbox.x0.get() > there.bbox.x0.get() + INDENT_TOLERANCE;
            there.page == here.page && (other == marker || nested)
        };
        found
            .iter()
            .enumerate()
            .map(|(i, f)| match *f {
                Some((_, true)) => true,
                Some((marker, false)) => beside(i, i.checked_sub(1), marker) || beside(i, i.checked_add(1), marker),
                None => false,
            })
            .collect()
    };
    for (block, item) in blocks.iter_mut().zip(items) {
        if !item {
            continue;
        }
        let Some((marker, _, rest)) = split_marker(&block.text) else {
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
    use super::{Marker, is_marker, roman, split_marker};

    #[test]
    fn bullets_and_counters_are_markers() {
        for m in ["•", "–", "*", "1.", "12)", "(3)", "a.", "(b)", "iv.", "(ii)", "xii."] {
            assert!(is_marker(m), "{m}");
        }
    }

    #[test]
    fn words_initials_and_years_are_not_markers() {
        for m in ["A.", "word.", "2024.", "(1", "ab.", "••", "1", "iiii.", "vv)"] {
            assert!(!is_marker(m), "{m}");
        }
    }

    #[test]
    fn a_marker_needs_text_after_it() {
        assert_eq!(
            split_marker("1. First step"),
            Some(("1.", Marker::Counter, "First step"))
        );
        assert_eq!(split_marker("•  Apples"), Some(("•", Marker::Bullet, "Apples")));
        assert_eq!(split_marker("– so"), Some(("–", Marker::WeakBullet, "so")));
        assert_eq!(split_marker("1."), None);
        assert_eq!(split_marker("In 2024. Then"), None);
    }

    #[test]
    fn roman_numerals_in_their_standard_form_only() {
        assert_eq!(roman("xiv"), Some(14));
        assert_eq!(roman("xc"), Some(90));
        assert_eq!(roman("civ"), Some(104));
        for word in ["", "iiii", "vx", "civil", "li x", "IV"] {
            assert_eq!(roman(word), None, "{word}");
        }
    }
}
