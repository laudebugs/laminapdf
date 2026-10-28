//! Byte searches used to locate file-structure keywords (`startxref`, `obj`, `trailer`, …).

use lamina_syntax::is_regular;

/// Finds `needle` in `hay` at or after `from`.
pub(crate) fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    let tail = hay.get(from..)?;
    tail.windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

/// Finds the last `needle` in `hay`.
pub(crate) fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

/// True when `data[pos]` is not a regular character (keyword boundary), or is EOF.
pub(crate) fn is_boundary(data: &[u8], pos: usize) -> bool {
    data.get(pos).is_none_or(|&b| !is_regular(b))
}
