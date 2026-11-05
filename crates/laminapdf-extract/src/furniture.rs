//! Running headers, footers and page numbers ([ADR 0030] §4): blocks in the top or bottom 10%
//! of a page that repeat at a similar height on at least three pages once digits are ignored,
//! and blocks there that hold only a page number. They need every page, so they are found once
//! the pages are read, and moved from `blocks` to `dropped`.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Block, BlockKind, DropReason, Dropped, Extraction};

/// The share of the page's height at its top and bottom where headers and footers sit.
const MARGIN: f64 = 0.1;

/// Headers whose middles are closer than this share of the page's height are at one position.
const SAME_HEIGHT: f64 = 0.02;

/// A line repeated on at least this many pages is a running header or footer.
const MIN_PAGES: usize = 3;

/// The longest number taken as a page number, in digits.
const MAX_DIGITS: usize = 5;

/// The longest Roman numeral taken as a page number (`xxviii`).
const MAX_ROMAN: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Zone {
    Top,
    Bottom,
}

/// Whether `word` (lowercase) is a page number: digits, or a Roman numeral.
fn number(word: &str) -> bool {
    let digits = word.len() <= MAX_DIGITS && word.chars().all(|c| c.is_ascii_digit());
    let roman = word.len() <= MAX_ROMAN && word.chars().all(|c| matches!(c, 'i' | 'v' | 'x' | 'l' | 'c'));
    !word.is_empty() && (digits || roman)
}

/// Whether `text` holds only a page number: `12`, `xiv`, `- 12 -`, `[12]`, `Page 12`,
/// `12 of 40`, `12 / 40`, `p. 4`.
pub(crate) fn is_page_number(text: &str) -> bool {
    let lower = text
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '–' | '—' | '[' | ']' | '(' | ')'))
        .to_lowercase();
    let rest = ["page ", "p. ", "p "]
        .iter()
        .find_map(|prefix| lower.strip_prefix(prefix))
        .unwrap_or(&lower);
    let words: Vec<&str> = rest.split_whitespace().collect();
    match words.as_slice() {
        [n] => match n.split_once('/') {
            Some((a, b)) => number(a) && number(b),
            None => number(n),
        },
        [a, "of" | "/", b] => number(a) && number(b),
        _ => false,
    }
}

/// `text` with its digits left out and its spaces collapsed: what a running header keeps from
/// page to page.
fn key(text: &str) -> String {
    let kept: String = text.chars().filter(|c| !c.is_ascii_digit()).collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A block that could be furniture.
struct Candidate {
    block: usize,
    page: usize,
    zone: Zone,
    /// Its middle as a share of the page's height, in [`SAME_HEIGHT`] steps.
    bucket: i64,
}

/// The zone `block` lies wholly in, on a page `height` tall.
fn zone(block: &Block, height: f64) -> Option<Zone> {
    if height <= 0.0 {
        return None;
    }
    if block.bbox.y1.get() <= MARGIN * height {
        Some(Zone::Top)
    } else if block.bbox.y0.get() >= (1.0 - MARGIN) * height {
        Some(Zone::Bottom)
    } else {
        None
    }
}

/// Why each block is furniture, by its index in `out.blocks`.
fn reasons(out: &Extraction) -> BTreeMap<usize, DropReason> {
    let heights: BTreeMap<usize, f64> = out.pages.iter().map(|p| (p.index, p.height.get())).collect();
    let mut found = BTreeMap::new();
    let mut groups: BTreeMap<(Zone, String), Vec<Candidate>> = BTreeMap::new();
    for (i, block) in out.blocks.iter().enumerate() {
        if matches!(block.kind, BlockKind::Table { .. }) {
            continue;
        }
        let Some(&height) = heights.get(&block.page) else {
            continue;
        };
        let Some(zone) = zone(block, height) else {
            continue;
        };
        if is_page_number(&block.text) {
            found.insert(i, DropReason::PageNumber);
            continue;
        }
        let key = key(&block.text);
        if !key.chars().any(char::is_alphanumeric) {
            continue;
        }
        let middle = f64::midpoint(block.bbox.y0.get(), block.bbox.y1.get()) / height;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a share of the page in [0, 1], in 2% steps"
        )]
        let bucket = (middle / SAME_HEIGHT).round() as i64;
        groups.entry((zone, key)).or_default().push(Candidate {
            block: i,
            page: block.page,
            zone,
            bucket,
        });
    }
    for candidates in groups.values() {
        let mut pages: BTreeMap<i64, BTreeSet<usize>> = BTreeMap::new();
        for c in candidates {
            pages.entry(c.bucket).or_default().insert(c.page);
        }
        for c in candidates {
            let near: BTreeSet<usize> = (c.bucket - 1..=c.bucket + 1)
                .filter_map(|b| pages.get(&b))
                .flatten()
                .copied()
                .collect();
            if near.len() >= MIN_PAGES {
                let reason = match c.zone {
                    Zone::Top => DropReason::RunningHeader,
                    Zone::Bottom => DropReason::RunningFooter,
                };
                found.insert(c.block, reason);
            }
        }
    }
    found
}

/// Moves the running headers, footers and page numbers of `out` to its `dropped`, each page's
/// entries after the hidden text dropped there.
pub(crate) fn remove(out: &mut Extraction, keep_spans: bool) {
    let found = reasons(out);
    if found.is_empty() {
        return;
    }
    let blocks = std::mem::take(&mut out.blocks);
    for (i, block) in blocks.into_iter().enumerate() {
        let Some(&reason) = found.get(&i) else {
            out.blocks.push(block);
            continue;
        };
        out.dropped.push(Dropped {
            reason,
            page: block.page,
            bbox: block.bbox,
            chars: block
                .spans
                .iter()
                .map(|s| s.text.chars().filter(|c| !c.is_whitespace()).count())
                .sum(),
            spans: if keep_spans { block.spans } else { Vec::new() },
        });
    }
    let position: BTreeMap<usize, usize> = out.pages.iter().enumerate().map(|(n, p)| (p.index, n)).collect();
    out.dropped
        .sort_by_key(|d| position.get(&d.page).copied().unwrap_or(usize::MAX));
}

#[cfg(test)]
mod tests {
    use super::{is_page_number, key};

    #[test]
    fn page_numbers_in_their_usual_forms() {
        for t in [
            "12",
            "xiv",
            "IV",
            "- 12 -",
            "– 3 –",
            "[7]",
            "Page 12",
            "page 3 of 40",
            "12 / 40",
            "3/9",
            "p. 4",
        ] {
            assert!(is_page_number(t), "{t}");
        }
    }

    #[test]
    fn text_with_a_number_is_not_a_page_number() {
        for t in ["Chapter 3", "12 apples", "2024 Annual Report", "Page", "123456", ""] {
            assert!(!is_page_number(t), "{t}");
        }
    }

    #[test]
    fn a_running_headers_key_ignores_its_digits() {
        assert_eq!(key("Annual report 2024 —  page 3"), key("Annual report 2024 — page 14"));
    }
}
