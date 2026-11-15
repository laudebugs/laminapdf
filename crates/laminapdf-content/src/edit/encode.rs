//! The encoder (ADR 0022 §3): each part an edit touched is written from its source bytes, with
//! the bytes of every operator no edit changed copied as they were.

use laminapdf_syntax::is_regular;

use super::{ContentOp, ContentOps};

/// The content written again: one entry per part, in order.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Encoded {
    /// One entry per [`ContentOps::parts`] entry.
    pub parts: Vec<EncodedPart>,
}

/// One part of [`Encoded`].
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodedPart {
    /// No edit touched the part: its stream can stay as it is.
    Unchanged {
        /// The part's index in [`ContentOps::parts`].
        part: usize,
    },
    /// The part's new content, decoded.
    Written {
        /// The part's index in [`ContentOps::parts`].
        part: usize,
        /// The content.
        bytes: Vec<u8>,
    },
}

impl EncodedPart {
    /// The part's index in [`ContentOps::parts`].
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn part(&self) -> usize {
        match self {
            Self::Unchanged { part } | Self::Written { part, .. } => *part,
        }
    }

    /// The new content, or `None` for a part that stays as it is.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Unchanged { .. } => None,
            Self::Written { bytes, .. } => Some(bytes),
        }
    }
}

/// Where the encoder stops copying the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Cut {
    /// Written operators, from this index in [`ContentOps::operators`] while they share an anchor.
    Insert(usize),
    /// Source bytes an edit dropped, up to this offset.
    Hole(usize),
}

/// The parts that need writing (every part with `all`), written.
pub(super) fn encode(ops: &ContentOps, all: bool) -> Encoded {
    let parts = ops
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            if all || ops.dirty.get(index).copied().unwrap_or(false) {
                EncodedPart::Written {
                    part: index,
                    bytes: write_region(ops, part.start, part.end),
                }
            } else {
                EncodedPart::Unchanged { part: index }
            }
        })
        .collect();
    Encoded { parts }
}

/// The source from `start` to `end` with the edits in it applied.
fn write_region(ops: &ContentOps, start: usize, end: usize) -> Vec<u8> {
    let mut cuts: Vec<(usize, Cut)> = ops
        .removed
        .iter()
        .filter(|hole| hole.start < end && start < hole.end)
        .map(|hole| (hole.start.max(start), Cut::Hole(hole.end.min(end))))
        .collect();
    let mut previous_anchor = None;
    for (index, op) in ops.ops.iter().enumerate() {
        if op.span.is_some() {
            previous_anchor = None;
            continue;
        }
        // Consecutive written operators at one anchor are written together.
        if previous_anchor != Some(op.token_start) && (start..end).contains(&op.token_start) {
            cuts.push((op.token_start, Cut::Insert(index)));
        }
        previous_anchor = Some(op.token_start);
    }
    cuts.sort_unstable();
    let source = &ops.source;
    let mut out = Vec::with_capacity(end.saturating_sub(start));
    let mut at = start;
    for (position, cut) in cuts {
        if position > at {
            out.extend_from_slice(source.get(at..position).unwrap_or_default());
            at = position;
        }
        match cut {
            Cut::Hole(hole_end) => at = at.max(hole_end),
            Cut::Insert(first) => {
                let anchor = ops.ops.get(first).map(|op| op.token_start);
                let run = ops
                    .ops
                    .get(first..)
                    .unwrap_or_default()
                    .iter()
                    .take_while(|op| op.span.is_none() && Some(op.token_start) == anchor);
                if out.last().copied().is_some_and(is_regular) {
                    out.push(b'\n');
                }
                write_canonical(run, &mut out);
            }
        }
    }
    out.extend_from_slice(source.get(at..end).unwrap_or_default());
    out
}

/// `ops` written canonically, each followed by a newline.
pub(super) fn canonical(ops: &[ContentOp]) -> Vec<u8> {
    let mut out = Vec::new();
    write_canonical(ops.iter(), &mut out);
    out
}

fn write_canonical<'a>(ops: impl Iterator<Item = &'a ContentOp>, out: &mut Vec<u8>) {
    for op in ops {
        if let Some(image) = &op.inline_image {
            // An inline image as written: its operands, if any, are strays `BI` doesn't take.
            out.extend_from_slice(image);
        } else {
            for operand in &op.operands {
                operand.write_to(out);
                out.push(b' ');
            }
            out.extend_from_slice(&op.keyword);
        }
        out.push(b'\n');
    }
}
