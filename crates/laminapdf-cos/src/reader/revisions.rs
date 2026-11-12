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

/// The offsets of the cross-reference sections that start a revision, oldest first, from
/// `sections` (newest first, as `/Prev` reaches them). A section starts a revision only when it
/// sits past every older section: a linearized file's first-page section comes before the
/// section its `/Prev` names (Annex F), so it belongs to the first revision.
///
/// Added in 0.2.0.
#[must_use]
pub fn revision_starts(sections: &[usize]) -> Vec<usize> {
    let mut starts: Vec<usize> = Vec::new();
    for &offset in sections.iter().rev() {
        if starts.last().is_none_or(|&last| offset > last) {
            starts.push(offset);
        }
    }
    starts
}

/// The revisions in `bytes`, whose cross-reference sections sit at `sections` (newest first),
/// oldest first, each as the offset of the section that starts it and where it ends. Each
/// revision but the newest ends just past the first `%%EOF` line between its section and the
/// next revision's ([`revision_starts`]); one with no such line merges into the next. The
/// newest ends at the end of the file.
fn groups(bytes: &[u8], sections: &[usize]) -> Vec<(usize, u64)> {
    let starts = revision_starts(sections);
    let mut out = Vec::new();
    let mut first = None;
    for (k, &start) in starts.iter().enumerate() {
        let group = *first.get_or_insert(start);
        let end = match starts.get(k + 1) {
            Some(&next) => end_of_eof_line(bytes, start, next),
            None => Some(bytes.len()),
        };
        if let Some(end) = end {
            out.push((group, u64::try_from(end).unwrap_or(u64::MAX)));
            first = None;
        }
    }
    out
}

/// The revisions' ends ([`groups`]), oldest first.
pub(crate) fn ends(bytes: &[u8], sections: &[usize]) -> Vec<u64> {
    groups(bytes, sections).into_iter().map(|(_, end)| end).collect()
}

/// One revision of a file ([`Document::revision_index`]): the cross-reference section that
/// starts it, where it ends, and the objects its sections list.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Revision {
    /// The offset of the cross-reference section that starts this revision.
    pub section: u64,
    /// Its length in bytes: the prefix of the file that holds it ([`Document::revision_ends`]).
    pub end: u64,
    /// The object numbers its sections list in use: the objects it adds or changes (for the
    /// first revision, every object).
    pub in_use: Vec<u32>,
    /// The object numbers its sections list as free, object 0 aside.
    pub freed: Vec<u32>,
}

impl Document {
    /// The file's revisions, oldest first, with what each one's cross-reference sections list
    /// (§7.5.6), grouped as [`Document::revision_ends`] groups them: a linearized file's
    /// first-page section is part of its first revision, and a revision with no `%%EOF` line
    /// merges into the next. Empty for a file whose cross-reference table had to be
    /// rebuilt by scanning, which has no sections to read.
    ///
    /// # Errors
    /// What [`Document::whole_input`] returns for a source that isn't in memory.
    ///
    /// Added in 0.2.0.
    pub fn revision_index(&self) -> Result<Vec<Revision>> {
        if self.was_repaired() || self.section_offsets.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = self.whole_input()?;
        let mut out: Vec<Revision> = groups(&bytes, &self.section_offsets)
            .into_iter()
            .map(|(start, end)| Revision {
                section: u64::try_from(start).unwrap_or(u64::MAX),
                end,
                in_use: Vec::new(),
                freed: Vec::new(),
            })
            .collect();
        for (&offset, objects) in self.section_offsets.iter().zip(&self.section_objects) {
            let offset = u64::try_from(offset).unwrap_or(u64::MAX);
            // The last revision starting at or before the section; a linearized file's
            // first-page section, before every other, is the first revision's.
            let index = out.iter().rposition(|r| r.section <= offset).unwrap_or(0);
            if let Some(revision) = out.get_mut(index) {
                revision.in_use.extend_from_slice(&objects.in_use);
                revision.freed.extend_from_slice(&objects.freed);
            }
        }
        for revision in &mut out {
            revision.in_use.sort_unstable();
            revision.in_use.dedup();
            revision.freed.sort_unstable();
            revision.freed.dedup();
        }
        Ok(out)
    }

    /// Each revision's length in bytes, oldest first: the prefix of the file that holds that
    /// revision, the last being the whole file. A file never updated, or one whose
    /// cross-reference table had to be rebuilt by scanning (no `/Prev` chain to follow), has one
    /// revision. A linearized file's first-page section is part of its first revision.
    ///
    /// # Errors
    /// What [`Document::whole_input`] returns for a source that isn't in memory.
    ///
    /// Added in 0.2.0.
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
    fn a_revision_without_an_eof_line_merges_but_the_others_keep_their_ends() {
        // Three sections; the second has no %%EOF line before the third, so it joins it.
        let file = b"%PDF-1.7\nxref startxref\n%%EOF\nxref startxref\nxref startxref\n%%EOF\n";
        let at: Vec<usize> = file
            .windows(6)
            .enumerate()
            .filter(|(_, w)| w == b"\nxref ")
            .map(|(i, _)| i + 1)
            .collect();
        let sections = [at[2], at[1], at[0]];
        let first = b"%PDF-1.7\nxref startxref\n%%EOF\n".len() as u64;
        assert_eq!(ends(file, &sections), vec![first, file.len() as u64]);
        assert_eq!(
            super::groups(file, &sections),
            vec![(at[0], first), (at[1], file.len() as u64)]
        );
    }

    #[test]
    fn a_revision_without_an_eof_line_merges_into_the_next() {
        let file = b"%PDF-1.7\nxref trailer startxref 9\nxref trailer startxref 33\n%%EOF\n";
        assert_eq!(ends(file, &[33, 9]), vec![file.len() as u64]);
    }
}
