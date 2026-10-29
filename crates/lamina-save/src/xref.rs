//! Cross-reference sections (§7.5.4 tables, §7.5.8 streams) shared by both writers.

use std::collections::BTreeMap;

use lamina_cos::{Dict, Object, Stream};

use crate::error::{Result, SaveError};
use crate::legacy::XrefEol;

/// One xref entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Row {
    /// A free entry: the next free object number and the generation to use if reused.
    Free { next: u32, generation: u16 },
    /// An object at a byte offset.
    InFile { offset: u64, generation: u16 },
    /// An object inside an object stream (generation 0, §7.5.7).
    InStream { stream: u32, index: u16 },
}

/// Entries by object number.
pub(crate) type Rows = BTreeMap<u32, Row>;

/// Links the free entries into the free list §7.5.4 describes: object 0 points at the
/// lowest free number, each free entry at the next, the last back at 0. Object 0 is added
/// (generation 65 535) when absent.
pub(crate) fn link_free_list(rows: &mut Rows) {
    rows.entry(0).or_insert(Row::Free {
        next: 0,
        generation: u16::MAX,
    });
    let free: Vec<u32> = rows
        .iter()
        .filter(|(_, row)| matches!(row, Row::Free { .. }))
        .map(|(&n, _)| n)
        .collect();
    let next_of = free.iter().skip(1).copied().chain(std::iter::once(0));
    for (&num, next) in free.iter().zip(next_of) {
        if let Some(Row::Free { next: link, .. }) = rows.get_mut(&num) {
            *link = next;
        }
    }
}

/// Runs of consecutive object numbers: `(first, count)`.
fn runs(rows: &Rows) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &num in rows.keys() {
        match out.last_mut() {
            Some((first, count)) if first.checked_add(*count) == Some(num) => *count += 1,
            _ => out.push((num, 1)),
        }
    }
    out
}

fn too_big(message: &'static str) -> SaveError {
    SaveError::Cos(lamina_cos::Error::InvalidStructure {
        kind: "xref section",
        message,
    })
}

/// A classic section: `xref`, then one subsection per run of consecutive numbers, each entry
/// 20 bytes ending in `eol`. The `trailer` keyword is the caller's.
///
/// # Errors
/// [`lamina_cos::Error::InvalidStructure`] for an offset over ten digits or an entry that a
/// table cannot express (an object-stream member).
pub(crate) fn classic_table(rows: &Rows, eol: XrefEol) -> Result<Vec<u8>> {
    let mut out = b"xref\n".to_vec();
    for (first, count) in runs(rows) {
        out.extend_from_slice(format!("{first} {count}\n").as_bytes());
        for num in first..first.saturating_add(count) {
            let entry = match rows.get(&num) {
                Some(Row::Free { next, generation }) => format!("{next:010} {generation:05} f"),
                Some(Row::InFile { offset, generation }) if *offset < 10_000_000_000 => {
                    format!("{offset:010} {generation:05} n")
                }
                Some(Row::InFile { .. }) => return Err(too_big("offset over ten digits")),
                Some(Row::InStream { .. }) | None => return Err(too_big("object-stream member in a table")),
            };
            out.extend_from_slice(entry.as_bytes());
            out.extend_from_slice(eol.bytes());
        }
    }
    Ok(out)
}

/// An xref stream's dictionary and Flate-compressed rows (`/W [1 n 2]`, `n` the bytes the
/// largest second field needs, at least 4 like the compact writer), with `/Index` listing the
/// runs. `/Size` and the trailer keys are the caller's, added after these.
pub(crate) fn stream_table(rows: &Rows) -> (Dict, Vec<u8>) {
    let field = |row: &Row| match *row {
        Row::Free { next, generation } => (0u8, u64::from(next), generation),
        Row::InFile { offset, generation } => (1, offset, generation),
        Row::InStream { stream, index } => (2, u64::from(stream), index),
    };
    let widest = rows.values().map(|r| field(r).1).max().unwrap_or(0);
    let width = usize::try_from(8 - widest.leading_zeros() / 8).unwrap_or(8).max(4);
    let mut data = Vec::with_capacity(rows.len() * (3 + width));
    for row in rows.values() {
        let (kind, second, third) = field(row);
        let bytes = second.to_be_bytes();
        data.push(kind);
        data.extend_from_slice(bytes.get(8 - width..).unwrap_or(&bytes));
        data.extend_from_slice(&third.to_be_bytes());
    }
    let index: Vec<Object> = runs(rows)
        .into_iter()
        .flat_map(|(first, count)| [Object::from(first), Object::from(count)])
        .collect();
    let width = i64::try_from(width).unwrap_or(8);
    let dict: Dict = [
        ("Type", Object::name("XRef")),
        ("W", vec![1.into(), width.into(), 2.into()].into()),
        ("Index", Object::Array(index)),
    ]
    .into_iter()
    .collect();
    (dict, data)
}

/// The xref stream object's bytes: `dict` (to which the caller added `/Size` and the trailer
/// keys) over `data`, Flate-compressed.
pub(crate) fn stream_object(num: u32, dict: Dict, data: &[u8]) -> Vec<u8> {
    let mut out = format!("{num} 0 obj\n").into_bytes();
    Object::Stream(Stream::flate(dict, data)).write_to(&mut out);
    out.extend_from_slice(b"\nendobj\n");
    out
}
