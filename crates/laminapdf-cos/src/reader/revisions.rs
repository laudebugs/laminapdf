//! Where each revision of a file ends (ISO 32000-2 §7.5.6): an incremental update appends its
//! objects, a cross-reference section and a trailer after the previous revision's `%%EOF`, so
//! each earlier revision is a prefix of the file that reads like a file of its own.

use super::Document;
use crate::error::Result;

/// The marker that ends a revision (§7.5.5).
const EOF_MARKER: &[u8] = b"%%EOF";

/// The length of `bytes[from..]`'s first line ending at its start: `\r\n`, `\n` or `\r`.
fn line_end(bytes: &[u8], from: usize) -> usize {
    match (bytes.get(from), bytes.get(from + 1)) {
        (Some(b'\r'), Some(b'\n')) => 2,
        (Some(b'\r' | b'\n'), _) => 1,
        _ => 0,
    }
}

/// Just past the first `%%EOF` line in `bytes[from..to]`, or `None` when there is none.
fn end_of_eof_line(bytes: &[u8], from: usize, to: usize) -> Option<usize> {
    let window = bytes.get(from..to.min(bytes.len()))?;
    let at = window.windows(EOF_MARKER.len()).position(|w| w == EOF_MARKER)?;
    let after = from + at + EOF_MARKER.len();
    Some(after + line_end(bytes, after))
}

/// The revisions' ends in a file of `len` bytes whose cross-reference sections sit at
/// `sections` (newest first, as `/Prev` reaches them), oldest first.
///
/// A section starts a revision only when it sits past every older section: a linearized file's
/// first-page section comes before the section its `/Prev` names (Annex F), so it belongs to the
/// first revision. Each revision but the newest ends just past the first `%%EOF` line between
/// its section and the next revision's; one with no such line merges into the next. The newest
/// ends at the end of the file.
pub(crate) fn ends(bytes: &[u8], sections: &[usize]) -> Vec<u64> {
    let mut starts: Vec<usize> = Vec::new();
    for &offset in sections.iter().rev() {
        if starts.last().is_none_or(|&last| offset > last) {
            starts.push(offset);
        }
    }
    let len = bytes.len();
    let mut out: Vec<u64> = starts
        .windows(2)
        .filter_map(|pair| match *pair {
            [this, next] => end_of_eof_line(bytes, this, next),
            _ => None,
        })
        .filter_map(|end| u64::try_from(end).ok())
        .collect();
    out.push(u64::try_from(len).unwrap_or(u64::MAX));
    out
}

impl Document {
    /// Each revision's length in bytes, oldest first: the prefix of the file that holds that
    /// revision, the last being the whole file. A file never updated, or one whose
    /// cross-reference table had to be rebuilt by scanning (no `/Prev` chain to follow), has one
    /// revision. A linearized file's first-page section is part of its first revision.
    ///
    /// # Errors
    /// What [`Document::whole_input`] returns for a source that isn't in memory.
    pub fn revision_ends(&self) -> Result<Vec<u64>> {
        let bytes = self.whole_input()?;
        if self.was_repaired() || self.xref_section_offsets().is_empty() {
            return Ok(vec![u64::try_from(bytes.len()).unwrap_or(u64::MAX)]);
        }
        Ok(ends(&bytes, self.xref_section_offsets()))
    }
}

#[cfg(test)]
mod tests {
    use super::ends;

    #[test]
    fn a_file_never_updated_is_one_revision() {
        let file = b"%PDF-1.7\nxref\ntrailer\nstartxref\n9\n%%EOF\n";
        assert_eq!(ends(file, &[9]), vec![file.len() as u64]);
    }

    #[test]
    fn an_update_ends_the_first_revision_after_its_eof_line() {
        let first = b"%PDF-1.7\nxref\ntrailer\nstartxref\n9\n%%EOF\r\n".to_vec();
        let mut file = first.clone();
        file.extend_from_slice(b"1 0 obj 1 endobj\nxref\ntrailer\nstartxref\n60\n%%EOF\n");
        let update = first.len() + 17;
        assert_eq!(ends(&file, &[update, 9]), vec![first.len() as u64, file.len() as u64]);
    }

    #[test]
    fn a_first_page_section_before_the_section_it_names_is_no_revision_of_its_own() {
        // Linearized: the newest section (offset 9) names an older one later in the file.
        let file = b"%PDF-1.7\nxref /Prev 40 startxref 9 %%EOF\nlots of objects xref startxref 40 %%EOF\n";
        assert_eq!(ends(file, &[9, 40]), vec![file.len() as u64]);
    }

    #[test]
    fn a_revision_without_an_eof_line_merges_into_the_next() {
        let file = b"%PDF-1.7\nxref trailer startxref 9\nxref trailer startxref 33\n%%EOF\n";
        assert_eq!(ends(file, &[33, 9]), vec![file.len() as u64]);
    }
}
