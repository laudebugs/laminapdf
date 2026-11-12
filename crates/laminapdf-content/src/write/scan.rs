//! The `q`/`Q` balance scan the writer runs before it wraps a page (ISO 32000-2 §8.4.2): tokens
//! only, through [`ContentIter`], so strings, comments and inline images never count, and no
//! operator is interpreted.

use laminapdf_cos::Result;
use laminapdf_syntax::{Limits, is_whitespace};

use crate::lexer::{ContentIter, Lexed};
use crate::operator::Operator;

/// Where a token sits: which stream of a page's `/Contents` (0 for the first) and its byte
/// offset in that stream's decoded content.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct StreamOffset {
    /// The stream's index in `/Contents`.
    pub stream: usize,
    /// The byte offset in the stream's decoded content.
    pub offset: usize,
}

impl StreamOffset {
    /// The token at `offset` in stream `stream`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(stream: usize, offset: usize) -> Self {
        Self { stream, offset }
    }
}

/// What [`scan_balance`] found in a page's content streams.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Balance {
    /// How many `q` are still open at the end (`e`): the closing `Q`s a wrap needs besides its own.
    pub depth: usize,
    /// Every `Q` read at depth 0, in order. A renderer ignores such a `Q` (there is no saved state
    /// to restore), but inside a wrap it would restore the wrap's own state, so the writer blanks
    /// exactly these tokens before it wraps.
    pub unmatched: Vec<StreamOffset>,
    /// Whether the content ends inside a text object (`BT` without its `ET`).
    pub in_text: bool,
    /// Whether the content is already wrapped: the first stream holds only `q`, the depth first
    /// returns to 0 at the end of a stream holding only `Q`s (and an `ET`), nothing after that
    /// draws or changes state outside a `q … Q` group, and nothing is unmatched or left open.
    pub wrapped: bool,
}

/// Scans `streams`, the decoded content streams of one page in `/Contents` order, for `q`/`Q`
/// balance. Streams are read as one content, joined by a newline as renderers join them
/// (§7.8.2), so an operator's operands may sit in the previous stream.
///
/// ```
/// use laminapdf_content::{Limits, StreamOffset, scan_balance};
///
/// let balance = scan_balance(&[b"q 2 0 0 2 0 0 cm Q Q", b"q 1 0 0 rg"], &Limits::default())?;
/// assert_eq!(balance.depth, 1);
/// assert_eq!(balance.unmatched.len(), 1);
/// assert_eq!(balance.unmatched[0].offset, 19);
/// # Ok::<(), laminapdf_content::Error>(())
/// ```
///
/// # Errors
/// A hard limit of the syntax layer ([`Limits::max_string_length`],
/// [`Limits::max_nesting_depth`]), as [`ContentIter`] reports it.
///
/// Added in 0.2.0.
pub fn scan_balance(streams: &[&[u8]], limits: &Limits) -> Result<Balance> {
    let mut joined = Vec::with_capacity(streams.iter().map(|s| s.len() + 1).sum());
    let mut starts = Vec::with_capacity(streams.len());
    for (i, stream) in streams.iter().enumerate() {
        if i > 0 {
            joined.push(b'\n');
        }
        starts.push(joined.len());
        joined.extend_from_slice(stream);
    }
    let locate = |offset: usize| {
        let stream = starts.partition_point(|&start| start <= offset).saturating_sub(1);
        let start = starts.get(stream).copied().unwrap_or(0);
        StreamOffset {
            stream,
            offset: offset - start.min(offset),
        }
    };

    let mut balance = Balance::default();
    // Per stream: whether every operator in it is `Q` or `ET` (a wrap's closing stream).
    let mut closing_only = vec![true; streams.len()];
    let mut closed_at: Option<usize> = None;
    let mut leaked = false;
    for item in ContentIter::new(&joined).with_limits(limits) {
        let Lexed::Op(op) = item? else { continue };
        let at = locate(op.offset);
        if !matches!(op.operator, Operator::Restore | Operator::EndText)
            && let Some(flag) = closing_only.get_mut(at.stream)
        {
            *flag = false;
        }
        let outside = closed_at.is_some() && balance.depth == 0;
        match op.operator {
            Operator::Save => balance.depth += 1,
            Operator::Restore if balance.depth == 0 => balance.unmatched.push(at),
            Operator::Restore => {
                balance.depth -= 1;
                if balance.depth == 0 && closed_at.is_none() {
                    closed_at = Some(at.stream);
                }
            }
            other => {
                leaked |= outside;
                match other {
                    Operator::BeginText => balance.in_text = true,
                    Operator::EndText => balance.in_text = false,
                    _ => {}
                }
            }
        }
    }
    let opens_with_q = streams.first().is_some_and(|s| trim(s) == b"q");
    balance.wrapped = opens_with_q
        && closed_at.is_some_and(|s| closing_only.get(s).copied().unwrap_or(false))
        && !leaked
        && balance.unmatched.is_empty()
        && balance.depth == 0
        && !balance.in_text;
    Ok(balance)
}

/// `bytes` without leading and trailing PDF white-space.
fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes.iter().position(|&b| !is_whitespace(b)).unwrap_or(bytes.len());
    let end = bytes.iter().rposition(|&b| !is_whitespace(b)).map_or(start, |i| i + 1);
    bytes.get(start..end).unwrap_or_default()
}

/// `content` with the `Q` at each of `offsets` replaced by a space, every other byte kept.
pub(super) fn blank(content: &[u8], offsets: impl IntoIterator<Item = usize>) -> Vec<u8> {
    let mut out = content.to_vec();
    for offset in offsets {
        if let Some(byte) = out.get_mut(offset)
            && *byte == b'Q'
        {
            *byte = b' ';
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(streams: &[&[u8]]) -> Balance {
        scan_balance(streams, &Limits::default()).unwrap_or_default()
    }

    fn at(stream: usize, offset: usize) -> StreamOffset {
        StreamOffset { stream, offset }
    }

    #[test]
    fn balanced_content_has_depth_zero_and_nothing_unmatched() {
        let b = scan(&[b"q 1 0 0 rg 0 0 10 10 re f Q"]);
        assert_eq!((b.depth, b.unmatched.len(), b.in_text), (0, 0, false));
    }

    #[test]
    fn surplus_q_is_counted_as_the_depth() {
        assert_eq!(scan(&[b"q q 2 0 0 2 0 0 cm", b"q Q"]).depth, 2);
    }

    #[test]
    fn a_q_at_depth_zero_is_unmatched_in_its_own_stream() {
        let b = scan(&[b"q 2 0 0 2 0 0 cm Q Q 0 0 1 rg", b"2 0 0 2 0 0 cm Q 0 0 10 10 re f"]);
        assert_eq!(b.unmatched, [at(0, 19), at(1, 15)]);
    }

    #[test]
    fn q_bytes_in_strings_comments_and_inline_images_do_not_count() {
        let b = scan(&[b"(Q) Tj % Q\nBI /W 1 /H 1 /BPC 8 /CS /G ID Q EI Q"]);
        assert_eq!(b.unmatched, [at(0, 46)]);
    }

    #[test]
    fn content_ending_inside_a_text_object_is_reported() {
        assert!(scan(&[b"BT /F1 12 Tf (Hi) Tj"]).in_text);
    }

    #[test]
    fn a_wrapped_page_with_appended_groups_is_wrapped() {
        assert!(scan(&[b"q\n", b"q 1 0 0 rg", b"\nQ\nQ\n", b"q /Fm1 Do Q"]).wrapped);
    }

    #[test]
    fn state_set_after_the_wrap_means_not_wrapped() {
        assert!(!scan(&[b"q\n", b"0 0 1 rg", b"\nQ\n", b"1 0 0 rg"]).wrapped);
    }

    #[test]
    fn a_lone_balanced_group_split_over_streams_is_not_a_wrap_when_the_first_stream_draws() {
        assert!(!scan(&[b"q 0 0 1 rg", b"Q"]).wrapped);
    }

    #[test]
    fn blank_replaces_only_the_q_bytes_at_the_offsets() {
        assert_eq!(blank(b"Q q Q", [0, 2, 4]), b"  q  ");
    }
}
