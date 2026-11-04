//! Page ranges ([ADR 0031] §3): qpdf's grammar, pdftk's handles and an inline `file:range`.
//!
//! A range is groups joined by commas: a page `7`, the last page `z` (or `end`), the n-th page
//! from the end `r3`, or a span `a-b` of those, counting down when `a > b`. A group with an `x`
//! in front removes its pages from the groups before it (from every page when it comes first).
//! `:odd` or `:even` at the end keeps every other page **of the result**, starting with its
//! first or second page. Ranges count pages from 1, as people do ([ADR 0027] §6); [`resolve`]
//! turns one into page numbers once the document's page count is known.
//!
//! A handle (`A=fronts.pdf`) names an input with one or more uppercase letters, so it never
//! collides with a range: `A1-5`, `Bz-1`, or `B` alone for all of B's pages.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md
//! [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
//! [`resolve`]: PageRange::resolve

use std::fmt;

/// A range that doesn't parse, or names a page the document lacks: what is wrong, and the
/// 0-based byte offset in the range text where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeError {
    /// The byte offset in the range text.
    pub position: usize,
    /// What is wrong: one lowercase sentence without a final period.
    pub message: String,
}

impl fmt::Display for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at character {})", self.message, self.position + 1)
    }
}

impl std::error::Error for RangeError {}

/// One end of a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bound {
    /// A page number, from 1.
    Page(u32),
    /// The n-th page from the end, from 1 (`z` is 1).
    FromEnd(u32),
}

/// One group: a page or a span, kept or removed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    exclude: bool,
    start: Bound,
    end: Bound,
    /// Where each bound starts in the text, for errors.
    at: (usize, usize),
}

/// Every other page of the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parity {
    Odd,
    Even,
}

/// A parsed page range; [`PageRange::resolve`] gives its pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRange {
    groups: Vec<Group>,
    parity: Option<Parity>,
}

impl PageRange {
    /// Every page, in order: the range `1-z`.
    #[must_use]
    pub fn all() -> Self {
        Self {
            groups: vec![Group {
                exclude: false,
                start: Bound::Page(1),
                end: Bound::FromEnd(1),
                at: (0, 0),
            }],
            parity: None,
        }
    }

    /// Parses `text`.
    ///
    /// # Errors
    ///
    /// A [`RangeError`] at the first character that doesn't fit the grammar.
    pub fn parse(text: &str) -> Result<Self, RangeError> {
        let mut cursor = Cursor { text, at: 0 };
        let mut groups = Vec::new();
        loop {
            groups.push(cursor.group()?);
            if !cursor.eat(",") {
                break;
            }
        }
        let parity = if cursor.eat(":") {
            let at = cursor.at;
            if cursor.eat("odd") {
                Some(Parity::Odd)
            } else if cursor.eat("even") {
                Some(Parity::Even)
            } else {
                return Err(error(at, "expected `odd` or `even` after `:`"));
            }
        } else {
            None
        };
        if cursor.at < text.len() {
            return Err(error(cursor.at, "unexpected character"));
        }
        Ok(Self { groups, parity })
    }

    /// The page numbers (from 1) the range selects in a document of `page_count` pages, in
    /// order; a page named twice is selected twice.
    ///
    /// # Errors
    ///
    /// A [`RangeError`] at the first bound that names a page the document lacks.
    pub fn resolve(&self, page_count: u32) -> Result<Vec<u32>, RangeError> {
        let mut pages: Vec<u32> = Vec::new();
        for (i, group) in self.groups.iter().enumerate() {
            let start = page(group.start, page_count, group.at.0)?;
            let end = page(group.end, page_count, group.at.1)?;
            let span: Vec<u32> = if start <= end {
                (start..=end).collect()
            } else {
                (end..=start).rev().collect()
            };
            if group.exclude {
                if i == 0 {
                    pages = (1..=page_count).collect();
                }
                pages.retain(|p| !span.contains(p));
            } else {
                pages.extend(span);
            }
        }
        Ok(match self.parity {
            None => pages,
            Some(parity) => {
                let skip = usize::from(parity == Parity::Even);
                pages.into_iter().skip(skip).step_by(2).collect()
            }
        })
    }
}

/// A bound's page number in a document of `page_count` pages.
fn page(bound: Bound, page_count: u32, at: usize) -> Result<u32, RangeError> {
    let (n, past) = match bound {
        Bound::Page(n) => (Some(n), n > page_count),
        Bound::FromEnd(n) => (page_count.checked_sub(n).map(|p| p + 1), n > page_count),
    };
    match n {
        Some(n) if !past => Ok(n),
        _ => Err(error(at, format!("the document has {page_count} pages"))),
    }
}

/// A range argument with an optional handle in front: `A1-5`, `Bz-1`, `B`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleRange {
    /// The handle: one or more uppercase ASCII letters.
    pub handle: Option<String>,
    /// The range; `None` for every page.
    pub range: Option<PageRange>,
}

impl HandleRange {
    /// Parses `text`.
    ///
    /// # Errors
    ///
    /// A [`RangeError`] in the range after the handle, with its offset in `text`.
    pub fn parse(text: &str) -> Result<Self, RangeError> {
        let letters = text.bytes().take_while(u8::is_ascii_uppercase).count();
        let (handle, rest) = text.split_at_checked(letters).unwrap_or((text, ""));
        let range = if rest.is_empty() {
            None
        } else {
            let range = PageRange::parse(rest).map_err(|e| RangeError {
                position: e.position + letters,
                ..e
            })?;
            Some(range)
        };
        Ok(Self {
            handle: (letters > 0).then(|| handle.to_owned()),
            range,
        })
    }
}

/// A handle definition, `A=fronts.pdf`: the handle and the input. `None` when `text` isn't
/// one.
#[must_use]
pub fn handle_definition(text: &str) -> Option<(&str, &str)> {
    let (handle, input) = text.split_once('=')?;
    let is_handle = !handle.is_empty() && handle.bytes().all(|b| b.is_ascii_uppercase());
    (is_handle && !input.is_empty()).then_some((handle, input))
}

/// An input with an inline range, `file.pdf:1-z:odd`: the input and its range. The range
/// starts after the first `:` whose remainder parses, so a path with a colon (`C:\a.pdf:1-3`)
/// keeps it; `text` without one is an input with no range.
#[must_use]
pub fn inline_range(text: &str) -> (&str, Option<PageRange>) {
    for (at, _) in text.match_indices(':') {
        let (input, rest) = text.split_at(at);
        if let (false, Some(range)) = (input.is_empty(), rest.get(1..).and_then(|r| PageRange::parse(r).ok())) {
            return (input, Some(range));
        }
    }
    (text, None)
}

/// A [`RangeError`] at `position`.
fn error(position: usize, message: impl Into<String>) -> RangeError {
    RangeError {
        position,
        message: message.into(),
    }
}

/// A position in the range text.
struct Cursor<'a> {
    text: &'a str,
    at: usize,
}

impl Cursor<'_> {
    fn rest(&self) -> &str {
        self.text.get(self.at..).unwrap_or_default()
    }

    /// Moves past `token` when the text continues with it.
    fn eat(&mut self, token: &str) -> bool {
        let found = self.rest().starts_with(token);
        if found {
            self.at += token.len();
        }
        found
    }

    fn group(&mut self) -> Result<Group, RangeError> {
        let exclude = self.eat("x");
        let start_at = self.at;
        let start = self.bound()?;
        let (end, end_at) = if self.eat("-") {
            let end_at = self.at;
            (self.bound()?, end_at)
        } else {
            (start, start_at)
        };
        Ok(Group {
            exclude,
            start,
            end,
            at: (start_at, end_at),
        })
    }

    fn bound(&mut self) -> Result<Bound, RangeError> {
        let at = self.at;
        if self.eat("end") || self.eat("z") {
            return Ok(Bound::FromEnd(1));
        }
        let from_end = self.eat("r");
        let digits = self.rest().bytes().take_while(u8::is_ascii_digit).count();
        let number = self.rest().get(..digits).unwrap_or_default();
        if number.is_empty() {
            let expected = if from_end {
                "a page count after `r`"
            } else {
                "a page number, `z` or `r<n>`"
            };
            return Err(error(self.at, format!("expected {expected}")));
        }
        let Ok(n) = number.parse::<u32>() else {
            return Err(error(self.at, "page number too large"));
        };
        if n == 0 {
            return Err(error(at, "pages count from 1"));
        }
        self.at += digits;
        Ok(if from_end { Bound::FromEnd(n) } else { Bound::Page(n) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn pages(range: &str, count: u32) -> Result<Vec<u32>, RangeError> {
        PageRange::parse(range)?.resolve(count)
    }

    fn at(range: &str, count: u32) -> usize {
        pages(range, count).map_or_else(|e| e.position, |_| usize::MAX)
    }

    // The examples of qpdf's manual (cli.rst, "Page Ranges"), on a 15-page document.
    #[test]
    fn qpdfs_examples_should_select_its_pages() {
        let cases: [(&str, Vec<u32>); 8] = [
            ("1,6,4", vec![1, 6, 4]),
            ("3-7", vec![3, 4, 5, 6, 7]),
            ("7-3", vec![7, 6, 5, 4, 3]),
            ("1-z", (1..=15).collect()),
            ("z-1", (1..=15).rev().collect()),
            ("r3-r1", vec![13, 14, 15]),
            ("1-z:even", (2..=15).step_by(2).collect()),
            ("1-15,x3-5,x9", vec![1, 2, 6, 7, 8, 10, 11, 12, 13, 14, 15]),
        ];
        for (range, expected) in cases {
            assert_eq!(pages(range, 15), Ok(expected), "{range}");
        }
    }

    #[test]
    fn odd_and_even_should_count_in_the_result_not_the_document() {
        assert_eq!(pages("2-7:odd", 10), Ok(vec![2, 4, 6]));
        assert_eq!(pages("2-7:even", 10), Ok(vec![3, 5, 7]));
        assert_eq!(pages("z-1:odd", 5), Ok(vec![5, 3, 1]));
    }

    #[test]
    fn end_should_be_z_and_a_leading_exclusion_should_start_from_every_page() {
        assert_eq!(pages("3-end", 5), Ok(vec![3, 4, 5]));
        assert_eq!(pages("x1", 4), Ok(vec![2, 3, 4]));
        assert_eq!(pages("x1,x4", 4), Ok(vec![2, 3]));
    }

    #[test]
    fn a_page_named_twice_should_be_selected_twice() {
        assert_eq!(pages("1,1-2", 3), Ok(vec![1, 1, 2]));
    }

    #[test]
    fn syntax_errors_should_point_at_their_character() {
        assert_eq!(at("", 5), 0);
        assert_eq!(at("1-", 5), 2);
        assert_eq!(at("1,,2", 5), 2);
        assert_eq!(at("1-3:all", 5), 4);
        assert_eq!(at("0", 5), 0);
        assert_eq!(at("r", 5), 1);
        assert_eq!(at("N", 5), 0);
        assert_eq!(at("1 ", 5), 1);
        assert_eq!(at("99999999999", 5), 0);
    }

    #[test]
    fn a_page_past_the_end_should_point_at_its_bound() {
        assert_eq!(at("1-6", 5), 2);
        assert_eq!(at("r6", 5), 0);
        let error = pages("2,9", 5).unwrap_err();
        assert_eq!(error.to_string(), "the document has 5 pages (at character 3)");
    }

    #[test]
    fn handles_should_be_uppercase_letters_before_the_range() {
        let parsed = HandleRange::parse("Bz-1").unwrap();
        assert_eq!(parsed.handle.as_deref(), Some("B"));
        assert_eq!(parsed.range.unwrap().resolve(3), Ok(vec![3, 2, 1]));
        assert_eq!(
            HandleRange::parse("AB").unwrap(),
            HandleRange {
                handle: Some("AB".to_owned()),
                range: None
            }
        );
        assert_eq!(HandleRange::parse("1-2").unwrap().handle, None);
        assert_eq!(HandleRange::parse("A1-").unwrap_err().position, 3);
    }

    #[test]
    fn handle_definitions_should_need_uppercase_letters_and_an_input() {
        assert_eq!(handle_definition("A=fronts.pdf"), Some(("A", "fronts.pdf")));
        assert_eq!(handle_definition("ab=x.pdf"), None);
        assert_eq!(handle_definition("A="), None);
        assert_eq!(handle_definition("a.pdf"), None);
    }

    #[test]
    fn an_inline_range_should_start_after_the_first_colon_whose_rest_parses() {
        let (input, range) = inline_range("file.pdf:1-z:odd");
        assert_eq!((input, range.map(|r| r.resolve(4))), ("file.pdf", Some(Ok(vec![1, 3]))));
        let (input, range) = inline_range(r"C:\a.pdf:2");
        assert_eq!((input, range.map(|r| r.resolve(4))), (r"C:\a.pdf", Some(Ok(vec![2]))));
        assert_eq!(inline_range("plain.pdf"), ("plain.pdf", None));
        assert_eq!(inline_range(":3"), (":3", None));
    }

    /// A random range text: one to four groups, each maybe excluded, with page numbers up to
    /// 30, `z`, `end` or `r<n>` ends, and maybe a parity suffix.
    fn range_text() -> impl Strategy<Value = String> {
        let bound = prop_oneof![
            (1u32..=30).prop_map(|n| n.to_string()),
            Just("z".to_owned()),
            Just("end".to_owned()),
            (1u32..=30).prop_map(|n| format!("r{n}")),
        ];
        let group = (any::<bool>(), bound.clone(), proptest::option::of(bound)).prop_map(|(x, a, b)| {
            format!(
                "{}{a}{}",
                if x { "x" } else { "" },
                b.map(|b| format!("-{b}")).unwrap_or_default()
            )
        });
        let suffix = prop_oneof![Just(""), Just(":odd"), Just(":even")];
        (proptest::collection::vec(group, 1..=4), suffix).prop_map(|(groups, suffix)| groups.join(",") + suffix)
    }

    proptest! {
        #[test]
        fn every_page_should_be_in_the_document(text in range_text(), count in 1u32..=30) {
            let range = PageRange::parse(&text).unwrap();
            if let Ok(pages) = range.resolve(count) {
                prop_assert!(pages.iter().all(|p| (1..=count).contains(p)));
            }
        }

        #[test]
        fn a_reversed_span_should_be_the_span_reversed(a in 1u32..=30, b in 1u32..=30) {
            let forward = pages(&format!("{}-{}", a.min(b), a.max(b)), 30).unwrap();
            let backward = pages(&format!("{}-{}", a.max(b), a.min(b)), 30).unwrap();
            prop_assert_eq!(forward.into_iter().rev().collect::<Vec<_>>(), backward);
        }

        #[test]
        fn an_excluded_page_should_never_be_selected(a in 1u32..=20, x in 1u32..=20) {
            let pages = pages(&format!("1-{a},1-z,x{x}"), 20).unwrap();
            prop_assert!(!pages.contains(&x));
        }
    }
}
