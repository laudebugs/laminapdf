//! Parsing over windows of the input (ADR 0005 §2).
//!
//! An in-memory input ([`ByteSource::as_slice`](crate::ByteSource::as_slice)) is parsed in
//! place: every helper here hands the whole slice to its closure once, with absolute offsets,
//! exactly as the reader did before windows existed. Any other source is read in
//! [`WINDOW`]-sized windows: a parse runs on the window holding its offset, and when the
//! parser reports that it looked past the window's end ([`Parser::reached_end`]) it runs again
//! on a window twice as long, until it no longer does or the window ends the input. A parse
//! that never looked past the end saw only bytes that are the same in the window and in the
//! whole input, so its result is the one an in-memory parse gives; offsets inside it are made
//! absolute again with [`rebase`].
//!
//! [`Parser::reached_end`]: lamina_syntax::Parser::reached_end

use std::ops::Range;

use lamina_syntax::{Limit, find};

use crate::error::{Error, Result};
use crate::scan::rfind;
use crate::source::{Pending, Source, WINDOW};

/// How the first window of a parse is chosen.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reach {
    /// The cached [`WINDOW`] holding the offset: for objects and xref sections.
    Object,
    /// Exactly this many bytes from the offset, not cached: for a quick look at many
    /// scattered offsets (checking every xref entry's `n g obj` header), where reading whole
    /// windows would mean reading the whole file.
    Probe(usize),
}

/// What a parse closure works on: `data[pos]` is the input byte at the parse's offset, and
/// `data[0]` is input byte `base`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct View<'w> {
    pub(crate) data: &'w [u8],
    pub(crate) pos: usize,
    pub(crate) base: usize,
}

/// Runs `parse` on input bytes from `offset`. `parse` returns its result and whether it
/// looked past the end of the data it was given; see the module docs.
///
/// # Errors
/// [`Error::DataPending`] / [`Error::Source`] when the source can't deliver a window.
pub(crate) fn parse_at<T>(
    source: &Source,
    offset: usize,
    reach: Reach,
    mut parse: impl FnMut(View<'_>) -> (T, bool),
) -> Result<T> {
    if let Some(data) = source.in_memory() {
        return Ok(parse(View {
            data,
            pos: offset,
            base: 0,
        })
        .0);
    }
    let (start, mut len) = match reach {
        Reach::Object => (offset - offset % WINDOW, WINDOW),
        Reach::Probe(len) => (offset, len.max(1)),
    };
    loop {
        let window = match reach {
            Reach::Object => source.window(start, len)?,
            Reach::Probe(_) => source.exact(start, len)?,
        };
        let view = View {
            data: window.bytes(),
            pos: offset.saturating_sub(window.start()),
            base: window.start(),
        };
        let (value, reached_end) = parse(view);
        if !reached_end || window.at_end() {
            return Ok(value);
        }
        len = len.saturating_mul(2);
    }
}

/// `e` with the byte offsets it carries moved by `base`: from a window to the input.
pub(crate) fn rebase(e: Error, base: usize) -> Error {
    use lamina_syntax::Error as Syntax;
    if base == 0 {
        return e;
    }
    match e {
        Error::Syntax(Syntax::UnexpectedEof { offset }) => Error::Syntax(Syntax::UnexpectedEof {
            offset: offset.saturating_add(base),
        }),
        Error::Syntax(Syntax::Syntax { offset, message }) => Error::Syntax(Syntax::Syntax {
            offset: offset.saturating_add(base),
            message,
        }),
        // The syntax limits carry a byte offset; the others an object number or nothing.
        Error::LimitExceeded {
            limit: limit @ (Limit::NestingDepth | Limit::StringLength),
            at: Some(at),
        } => Error::LimitExceeded {
            limit,
            at: Some(at.saturating_add(u64::try_from(base).unwrap_or(u64::MAX))),
        },
        other => other,
    }
}

/// The offset of the last `needle` in the input, searching back from the end one window at a
/// time (so a `startxref` near the end costs one window).
///
/// Once a window is missing, the search can't stop before the start of the input without
/// it, so the windows before it are asked for in the same [`Error::DataPending`]: a file
/// without the keyword (one that needs repair, which reads everything) costs one round, not
/// one per window.
///
/// # Errors
/// As [`parse_at`].
pub(crate) fn rfind_input(source: &Source, needle: &[u8]) -> Result<Option<usize>> {
    if let Some(data) = source.in_memory() {
        return Ok(rfind(data, needle));
    }
    let overlap = needle.len().saturating_sub(1);
    let mut pending = Pending::default();
    for index in (0..source.len().div_ceil(WINDOW)).rev() {
        // A match that starts in this window and ends in the next is found here; one that
        // starts in the next window would have been found already.
        match source.window(index * WINDOW, WINDOW + overlap) {
            Ok(window) if pending.is_empty() => {
                if let Some(p) = rfind(window.bytes(), needle) {
                    return Ok(Some(window.start() + p));
                }
            }
            Ok(_) => {}
            Err(e) => pending.absorb(e)?,
        }
    }
    pending.finish()?;
    Ok(None)
}

/// The offset of the first `needle` at or after `from`, searching forward one window at a
/// time.
///
/// # Errors
/// As [`parse_at`].
pub(crate) fn find_input(source: &Source, needle: &[u8], from: usize) -> Result<Option<usize>> {
    if let Some(data) = source.in_memory() {
        return Ok(find(data, needle, from));
    }
    let overlap = needle.len().saturating_sub(1);
    for index in from / WINDOW..source.len().div_ceil(WINDOW) {
        let window = source.window(index * WINDOW, WINDOW + overlap)?;
        let at = from.saturating_sub(window.start());
        if let Some(p) = find(window.bytes(), needle, at) {
            return Ok(Some(window.start() + p));
        }
        if window.at_end() {
            break;
        }
    }
    Ok(None)
}

/// Calls `visit` for every [`WINDOW`] of the input in order, in overlapping chunks: each view
/// holds up to `back` bytes before its window and `ahead` bytes after it, and the range
/// passed with it is the window's own part of the view (matches that start there belong to
/// this chunk). An in-memory input is one chunk.
///
/// Missing windows don't stop the walk: every one is collected and reported in one
/// [`Error::DataPending`] (repair needs them all), and `visit` is not called again once one is
/// missing, as the walk will fail anyway.
///
/// # Errors
/// [`Error::DataPending`] / [`Error::Source`] as [`parse_at`]; `visit`'s errors.
pub(crate) fn for_each_chunk(
    source: &Source,
    (back, ahead): (usize, usize),
    mut visit: impl FnMut(View<'_>, Range<usize>) -> Result<()>,
) -> Result<()> {
    if let Some(data) = source.in_memory() {
        return visit(View { data, pos: 0, base: 0 }, 0..data.len());
    }
    let total = source.len();
    let mut pending = Pending::default();
    for index in 0..total.div_ceil(WINDOW) {
        let own_start = index * WINDOW;
        let start = own_start.saturating_sub(back);
        let window = match source.window(start, own_start - start + WINDOW + ahead) {
            Ok(window) => window,
            Err(e) => {
                pending.absorb(e)?;
                continue;
            }
        };
        if !pending.is_empty() {
            continue;
        }
        let base = window.start();
        let own = own_start - base..(own_start + WINDOW).min(total) - base;
        let view = View {
            data: window.bytes(),
            pos: own.start,
            base,
        };
        if let Err(e) = visit(view, own) {
            pending.absorb(e)?;
        }
    }
    pending.finish()
}

#[cfg(test)]
mod tests {
    use lamina_syntax::{Object, Parser};

    use super::*;
    use crate::source::tests::NoSlice;

    /// `len` spaces with `needle` written at each of `at`.
    fn spaces_with(len: usize, needle: &[u8], at: &[usize]) -> Vec<u8> {
        let mut data = vec![b' '; len];
        for &p in at {
            data[p..p + needle.len()].copy_from_slice(needle);
        }
        data
    }

    #[test]
    fn rfind_input_finds_the_last_match_across_a_window_boundary() {
        let data = spaces_with(3 * WINDOW, b"startxref", &[10, WINDOW - 4]);
        let (source, _) = NoSlice::source(data);
        assert_eq!(rfind_input(&source, b"startxref"), Ok(Some(WINDOW - 4)));
    }

    #[test]
    fn rfind_input_stops_at_the_last_window_when_the_match_is_there() {
        let data = spaces_with(4 * WINDOW, b"startxref", &[4 * WINDOW - 30]);
        let (source, inner) = NoSlice::source(data);
        rfind_input(&source, b"startxref").unwrap();
        assert_eq!(inner.reads(), 1);
    }

    #[test]
    fn find_input_finds_the_first_match_across_a_window_boundary() {
        let data = spaces_with(3 * WINDOW, b"endstream", &[2 * WINDOW - 3, 2 * WINDOW + 100]);
        let (source, _) = NoSlice::source(data);
        assert_eq!(find_input(&source, b"endstream", 5), Ok(Some(2 * WINDOW - 3)));
    }

    #[test]
    fn find_input_finds_nothing_before_from() {
        let data = spaces_with(2 * WINDOW, b"endstream", &[100]);
        let (source, _) = NoSlice::source(data);
        assert_eq!(find_input(&source, b"endstream", 101), Ok(None));
    }

    #[test]
    fn parse_at_doubles_the_window_until_the_object_fits() {
        // A 150 000-byte string starting near the end of the first window.
        let mut data = vec![b' '; WINDOW - 100];
        data.extend_from_slice(b"(");
        data.extend(std::iter::repeat_n(b'z', 150_000));
        data.extend_from_slice(b") ");
        let total = data.len();
        let (source, _) = NoSlice::source(data);
        let mut windows = Vec::new();
        let parsed = parse_at(&source, WINDOW - 100, Reach::Object, |v| {
            windows.push(v.data.len());
            let mut parser = Parser::new(v.data, v.pos);
            let obj = parser.object().ok();
            (obj, parser.reached_end())
        })
        .unwrap();
        let len = parsed.and_then(|o| match o {
            Object::String(s) => Some(s.bytes.len()),
            _ => None,
        });
        // The third window would be 4 × 64 KiB but is cut at the end of the input.
        assert_eq!((len, windows), (Some(150_000), vec![WINDOW, 2 * WINDOW, total]));
    }

    #[test]
    fn rebase_moves_byte_offsets_but_not_object_numbers() {
        let syntax = Error::Syntax(lamina_syntax::Error::Syntax {
            offset: 5,
            message: "m",
        });
        let resolve = Error::LimitExceeded {
            limit: Limit::ResolveDepth,
            at: Some(5),
        };
        assert_eq!(
            (rebase(syntax, 100), rebase(resolve.clone(), 100)),
            (
                Error::Syntax(lamina_syntax::Error::Syntax {
                    offset: 105,
                    message: "m"
                }),
                resolve
            )
        );
    }

    #[test]
    fn for_each_chunk_gives_each_window_its_own_part_with_overlap() {
        let (source, _) = NoSlice::source(vec![b'a'; 2 * WINDOW + 10]);
        let mut seen = Vec::new();
        for_each_chunk(&source, (1, 3), |view, own| {
            seen.push((view.base, view.data.len(), own));
            Ok(())
        })
        .unwrap();
        assert_eq!(
            seen,
            vec![
                (0, WINDOW + 3, 0..WINDOW),
                (WINDOW - 1, WINDOW + 4, 1..WINDOW + 1),
                (2 * WINDOW - 1, 11, 1..11),
            ]
        );
    }
}
