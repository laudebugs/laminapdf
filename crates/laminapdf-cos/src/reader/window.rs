//! Parsing over windows of the input (ADR 0005 §2).
//!
//! An in-memory input ([`ByteSource::as_slice`](crate::ByteSource::as_slice)) is parsed in
//! place: every helper here hands the whole slice to its closure once, with absolute offsets,
//! exactly as the reader did before windows existed. Any other source is read in
//! [`WINDOW`]-sized windows: a parse runs on the window holding its offset, and when the
//! parser reports that it looked past the window's end ([`Parser::reached_end`]) it runs again
//! on a window twice as long (only the new part is read), until it no longer does, the window
//! ends the input, or it would pass [`Limits::max_read_window`]. A parse that never looked past
//! the end saw only bytes that are the same in the window and in the whole input, so its
//! result is the one an in-memory parse gives; offsets inside it are made absolute again with
//! [`rebase`].
//!
//! A stream's data is not read through a growing window: [`read_indirect`] checks its
//! `/Length` with a small read at the declared end and, when that is wrong, searches forward
//! for `endstream`, with the same checks [`Parser::indirect_object`] makes.

use std::ops::Range;

use laminapdf_syntax::{
    IndirectHead, IndirectObject, Limit, Limits, ObjRef, Parser, Position, StreamHead, endstream_at, find,
    stream_data_end,
};

use crate::error::{Error, Result};
use crate::scan::rfind;
use crate::source::{Fetch, Pending, Source, WINDOW};

/// How the first window of a parse is chosen.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reach {
    /// The cached [`WINDOW`] holding the offset: for objects and xref sections.
    Object,
    /// Exactly this many bytes from the offset, not cached: for a quick look at a scattered
    /// offset (an `endstream` at a declared stream end), where a whole window would be waste.
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

/// [`Error::LimitExceeded`] for [`Limits::max_read_window`] at input offset `offset`.
pub(crate) fn read_window_exceeded(offset: usize) -> Error {
    Error::LimitExceeded {
        limit: Limit::ReadWindow,
        at: Some(Position::offset(offset)),
    }
}

/// `Some` result, or `None` when the read hit [`Limits::max_read_window`] (tolerant paths:
/// the item counts as unreadable); other errors are returned.
pub(crate) fn within_read_window<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.is_read_window_limit() => Ok(None),
        Err(e) => Err(e),
    }
}

/// Runs `parse` on input bytes from `offset`. `parse` returns its result and whether it
/// looked past the end of the data it was given; see the module docs.
///
/// # Errors
/// [`Error::DataPending`] / [`Error::Source`] when the source can't deliver a window;
/// [`Error::LimitExceeded`] for [`Limits::max_read_window`] when the parse still needs more.
pub(crate) fn parse_at<T>(
    source: &Source,
    offset: usize,
    reach: Reach,
    limits: &Limits,
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
    let max = limits.max_read_window().max(1);
    let (mut window, fetch) = match reach {
        Reach::Object => (
            source.read(offset - offset % WINDOW, WINDOW, Fetch::Cached)?,
            Fetch::Cached,
        ),
        Reach::Probe(len) => (source.read(offset, len.clamp(1, max), Fetch::Exact)?, Fetch::Exact),
    };
    loop {
        let view = View {
            data: window.bytes(),
            pos: offset.saturating_sub(window.start()),
            base: window.start(),
        };
        let (value, reached_end) = parse(view);
        if !reached_end || window.at_end() {
            return Ok(value);
        }
        let len = window.bytes().len();
        if len >= max {
            return Err(read_window_exceeded(offset));
        }
        let more = source.read(window.end(), len.max(1).min(max - len), fetch)?;
        window = window.append(&more);
    }
}

/// [`parse_at`] with a [`Parser`] at `offset` under `limits`: `parse` gets the parser and the
/// input offset of its data's first byte, and its error comes back with input offsets.
///
/// # Errors
/// Outer: as [`parse_at`]. Inner: `parse`'s error.
pub(crate) fn parse_syntax_at<T>(
    source: &Source,
    offset: usize,
    reach: Reach,
    limits: &Limits,
    mut parse: impl FnMut(&mut Parser<'_>, usize) -> Result<T>,
) -> Result<Result<T>> {
    parse_at(source, offset, reach, limits, |v| {
        let mut parser = Parser::new(v.data, v.pos).with_limits(limits);
        let result = parse(&mut parser, v.base).map_err(|e| rebase(e, v.base));
        (result, parser.reached_end())
    })
}

/// [`Parser::indirect_object`] at `offset` over the whole input. On a source read in windows,
/// a stream's extent is found as described in the module docs.
///
/// # Errors
/// Outer: the source's errors and [`Limits::max_read_window`] for the object's head. Inner:
/// the parse error an in-memory parse gives, or [`Limits::max_read_window`] when the stream's
/// data or the search for its `endstream` would pass it.
pub(crate) fn read_indirect(
    source: &Source,
    offset: usize,
    limits: &Limits,
    resolve_length: &mut dyn FnMut(ObjRef) -> Option<i64>,
) -> Result<Result<IndirectObject>> {
    if let Some(data) = source.in_memory() {
        let mut parser = Parser::new(data, offset).with_limits(limits);
        return Ok(parser.indirect_object(resolve_length).map_err(Error::from));
    }
    let head = parse_syntax_at(source, offset, Reach::Object, limits, |p, base| {
        Ok(match p.indirect_head(resolve_length)? {
            IndirectHead::Stream(mut head) => {
                head.data_start += base;
                IndirectHead::Stream(head)
            }
            object @ IndirectHead::Object(_) => object,
        })
    })?;
    match head {
        Ok(IndirectHead::Object(object)) => Ok(Ok(object)),
        Ok(IndirectHead::Stream(head)) => stream_from(source, offset, limits, head),
        Err(e) => Ok(Err(e)),
    }
}

/// Bytes read first to check for `endstream` at a declared stream end.
const ENDSTREAM_PROBE: usize = 32;

/// The stream `head` (of the object at `offset`) with its data, from a source read in windows.
fn stream_from(source: &Source, offset: usize, limits: &Limits, head: StreamHead) -> Result<Result<IndirectObject>> {
    let total = source.len()?;
    let max = limits.max_read_window();
    // A declared end past the input is wrong without looking (in memory, the check finds
    // nothing there either); a long white-space run before `endstream` over the read
    // window counts as no `endstream`.
    let declared_end = match head.declared_end().filter(|&end| end <= total) {
        Some(end) => {
            let probe = parse_at(source, end, Reach::Probe(ENDSTREAM_PROBE), limits, |v| {
                let mut reached_end = false;
                (endstream_at(v.data, v.pos, &mut reached_end), reached_end)
            });
            within_read_window(probe)?.unwrap_or(false).then_some(end)
        }
        None => None,
    };
    let end = if let Some(end) = declared_end {
        end
    } else {
        let endstream = match find_input(source, b"endstream", head.data_start, max.saturating_add(9))? {
            Search::Found(at) => at,
            Search::Absent => {
                return Ok(Err(Error::Syntax(laminapdf_syntax::Error::UnexpectedEof {
                    at: Position::offset(total),
                })));
            }
            Search::Beyond => return Ok(Err(read_window_exceeded(offset))),
        };
        // The EOL before `endstream` is at most two bytes.
        let from = endstream.saturating_sub(2).max(head.data_start);
        let before = source.read(from, endstream - from, Fetch::Exact)?;
        from + stream_data_end(before.bytes(), head.data_start.saturating_sub(from), endstream - from)
    };
    let len = end - head.data_start;
    if len > max {
        return Ok(Err(read_window_exceeded(offset)));
    }
    let data = source.read(head.data_start, len, Fetch::Cached)?.into_vec();
    Ok(Ok(head.into_object(data, declared_end.is_none())))
}

/// `e`, from a parse of a window that starts at input byte `base`, with its window-relative
/// offsets made absolute: those of syntax errors and of the syntax limits (nesting, string
/// length). Every other position (an object number, a read-window limit's or any other
/// error's offset, which the reader computes in input terms) is left as it is.
pub(crate) fn rebase(e: Error, base: usize) -> Error {
    let shift = |p: Position| p.shifted(u64::try_from(base).unwrap_or(u64::MAX));
    match e {
        Error::Syntax(_)
        | Error::LimitExceeded {
            limit: Limit::NestingDepth | Limit::StringLength,
            ..
        } => e.map_position(shift),
        other => other,
    }
}

/// Walks `windows` in order, each read with `overlap` bytes of the next, until `look` finds
/// something. Once a window is missing, the walk goes on for as many more windows as it had
/// read (at least one) and reports them all: a search that ends near where it started costs
/// one more round, a long one a number of rounds logarithmic in its length, and none asks for
/// much more than it would read.
///
/// # Errors
/// [`Error::DataPending`] / [`Error::Source`] as [`Source::read`].
fn search_windows(
    source: &Source,
    windows: impl Iterator<Item = usize>,
    overlap: usize,
    mut look: impl FnMut(&crate::source::Window<'_>) -> Option<usize>,
) -> Result<Option<usize>> {
    let mut pending = Pending::default();
    let (mut read, mut asked) = (0usize, 0usize);
    for index in windows {
        match source.read(index * WINDOW, WINDOW + overlap, Fetch::Cached) {
            Ok(window) if pending.is_empty() => {
                if let Some(found) = look(&window) {
                    return Ok(Some(found));
                }
                read += 1;
            }
            Ok(_) => asked += 1,
            Err(e) => {
                pending.absorb(e)?;
                asked += 1;
            }
        }
        if !pending.is_empty() && (asked >= read.max(1) || pending.is_full()) {
            break;
        }
    }
    pending.finish()?;
    Ok(None)
}

/// The offset of the last `needle` in the input, searching back from the end one window at a
/// time (so a `startxref` near the end costs one window).
///
/// # Errors
/// As [`search_windows`].
pub(crate) fn rfind_input(source: &Source, needle: &[u8]) -> Result<Option<usize>> {
    if let Some(data) = source.in_memory() {
        return Ok(rfind(data, needle));
    }
    // A match that starts in a window and ends in the next is found with that window; one
    // that starts in the next window would have been found already.
    let windows = (0..source.len()?.div_ceil(WINDOW)).rev();
    search_windows(source, windows, needle.len().saturating_sub(1), |window| {
        rfind(window.bytes(), needle).map(|p| window.start() + p)
    })
}

/// What [`find_input`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Search {
    /// The match's offset.
    Found(usize),
    /// No match up to the end of the input.
    Absent,
    /// No match within the bound; the input goes on.
    Beyond,
}

/// The first `needle` that starts at or after `from`, searching forward one window at a time.
/// A source read in windows is searched only for matches starting before `from + bound`;
/// an in-memory input is searched to its end.
///
/// # Errors
/// As [`search_windows`].
pub(crate) fn find_input(source: &Source, needle: &[u8], from: usize, bound: usize) -> Result<Search> {
    if let Some(data) = source.in_memory() {
        return Ok(find(data, needle, from).map_or(Search::Absent, Search::Found));
    }
    let total = source.len()?;
    let limit = from.saturating_add(bound).min(total);
    let windows = from / WINDOW..limit.div_ceil(WINDOW);
    let found = search_windows(source, windows, needle.len().saturating_sub(1), |window| {
        let at = from.saturating_sub(window.start());
        find(window.bytes(), needle, at)
            .map(|p| window.start() + p)
            .filter(|&p| p < limit)
    })?;
    Ok(match found {
        Some(at) => Search::Found(at),
        None if limit < total => Search::Beyond,
        None => Search::Absent,
    })
}

/// Calls `visit` for every [`WINDOW`] of the input in order, in overlapping chunks: each view
/// holds up to `back` bytes before its window and `ahead` bytes after it, and the range
/// passed with it is the window's own part of the view (matches that start there belong to
/// this chunk). An in-memory input is one chunk.
///
/// Missing windows don't stop the walk: they are collected (up to one round's worth) and
/// reported in one [`Error::DataPending`] (repair needs them all), and `visit` is not called
/// again once one is missing, as the walk will fail anyway. A `DataPending` from `visit`
/// (for bytes past its chunk) is collected the same way.
///
/// # Errors
/// [`Error::DataPending`] / [`Error::Source`] as [`Source::read`]; `visit`'s errors.
pub(crate) fn for_each_chunk(
    source: &Source,
    (back, ahead): (usize, usize),
    mut visit: impl FnMut(View<'_>, Range<usize>) -> Result<()>,
) -> Result<()> {
    if let Some(data) = source.in_memory() {
        return visit(View { data, pos: 0, base: 0 }, 0..data.len());
    }
    let total = source.len()?;
    let mut pending = Pending::default();
    for index in 0..total.div_ceil(WINDOW) {
        let own_start = index * WINDOW;
        let start = own_start.saturating_sub(back);
        let result = source
            .read(start, own_start - start + WINDOW + ahead, Fetch::Cached)
            .and_then(|window| {
                if !pending.is_empty() {
                    return Ok(());
                }
                let base = window.start();
                let own = own_start - base..(own_start + WINDOW).min(total) - base;
                let view = View {
                    data: window.bytes(),
                    pos: own.start,
                    base,
                };
                visit(view, own)
            });
        if let Err(e) = result {
            pending.absorb(e)?;
            if pending.is_full() {
                break;
            }
        }
    }
    pending.finish()
}

#[cfg(test)]
mod tests {
    use laminapdf_syntax::Object;

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
        assert_eq!(
            find_input(&source, b"endstream", 5, usize::MAX),
            Ok(Search::Found(2 * WINDOW - 3))
        );
    }

    #[test]
    fn find_input_finds_nothing_before_from() {
        let data = spaces_with(2 * WINDOW, b"endstream", &[100]);
        let (source, _) = NoSlice::source(data);
        assert_eq!(find_input(&source, b"endstream", 101, usize::MAX), Ok(Search::Absent));
    }

    #[test]
    fn parse_at_doubles_the_window_until_the_object_fits() {
        // A 150 000-byte string starting near the end of the first window.
        let mut data = vec![b' '; WINDOW - 100];
        data.extend_from_slice(b"(");
        data.extend(std::iter::repeat_n(b'z', 150_000));
        data.extend_from_slice(b") ");
        let total = data.len();
        let (source, inner) = NoSlice::source(data);
        let mut windows = Vec::new();
        let parsed = parse_at(&source, WINDOW - 100, Reach::Object, &Limits::default(), |v| {
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
        // The third window would be 4 × 64 KiB but is cut at the end of the input; every byte
        // is read once (one call per window: the source serves whole reads).
        assert_eq!(
            (len, windows, inner.reads()),
            (Some(150_000), vec![WINDOW, 2 * WINDOW, total], total.div_ceil(WINDOW))
        );
    }

    #[test]
    fn rebase_keeps_the_absolute_offset_of_a_read_window_limit() {
        assert_eq!(rebase(read_window_exceeded(500), 100), read_window_exceeded(500));
    }

    #[test]
    fn rebase_moves_byte_offsets_but_not_object_numbers() {
        let syntax = Error::Syntax(laminapdf_syntax::Error::Syntax {
            at: Position::FileOffset(5),
            message: "m",
        });
        let resolve = Error::LimitExceeded {
            limit: Limit::ResolveDepth,
            at: Some(Position::Object(5)),
        };
        assert_eq!(
            (rebase(syntax, 100), rebase(resolve.clone(), 100)),
            (
                Error::Syntax(laminapdf_syntax::Error::Syntax {
                    at: Position::FileOffset(105),
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

    #[test]
    fn parse_at_stops_growing_at_max_read_window() {
        // An unterminated string: the parser always wants more.
        let mut data = b"(".to_vec();
        data.extend(std::iter::repeat_n(b'z', 5 * WINDOW));
        let (source, inner) = NoSlice::source(data);
        let limits = Limits::default().with_max_read_window(2 * WINDOW);
        let result = parse_at(&source, 0, Reach::Object, &limits, |v| {
            let mut parser = Parser::new(v.data, v.pos);
            let _ = parser.object();
            ((), parser.reached_end())
        });
        assert_eq!((result, inner.reads()), (Err(read_window_exceeded(0)), 2));
    }

    #[test]
    fn find_input_reports_beyond_when_the_bound_ends_before_the_input() {
        let data = spaces_with(4 * WINDOW, b"endstream", &[3 * WINDOW]);
        let (source, _) = NoSlice::source(data);
        assert_eq!(find_input(&source, b"endstream", 0, WINDOW), Ok(Search::Beyond));
    }

    /// Window 0 and the last window are there; every other one is missing.
    struct HeadAndTail(usize);

    impl crate::ByteSource for HeadAndTail {
        fn len(&self) -> u64 {
            u64::try_from(self.0).unwrap()
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::result::Result<usize, crate::SourceError> {
            let at = usize::try_from(offset).unwrap();
            if at < WINDOW || at >= (self.0 - 1) / WINDOW * WINDOW {
                buf.fill(b' ');
                return Ok(buf.len());
            }
            Err(crate::SourceError::Pending {
                offset,
                len: u64::try_from(buf.len()).unwrap(),
            })
        }
    }

    #[test]
    fn a_backward_search_asks_for_as_many_windows_as_it_has_read() {
        // 10 windows; the last 1 is there, so the first round asks for one more.
        let source = Source::from(HeadAndTail(10 * WINDOW));
        let w = u64::try_from(WINDOW).unwrap();
        assert_eq!(
            rfind_input(&source, b"startxref"),
            Err(Error::DataPending {
                ranges: vec![(8 * w, w)]
            })
        );
    }
}
