//! A file's revisions (ISO 32000-2 §7.5.6): each incremental update appends one, and each
//! earlier revision is a prefix of the file that opens like a file of its own.

use std::collections::BTreeSet;

use laminapdf_cos::OpenOptions;

use crate::{Document, Error, Result};

/// One revision of the file, oldest first ([`Document::revisions`]): its cross-reference section,
/// where it ends, and the objects it adds, changes and frees.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RevisionSummary {
    /// Its 0-based index; 0 is the original file.
    pub index: u32,
    /// The byte offset of the cross-reference section that starts it.
    pub section: u64,
    /// Its length in bytes: the prefix of the file that holds it.
    pub bytes: u64,
    /// Objects it adds (for the first revision, every object).
    pub added: Vec<u32>,
    /// Objects an earlier revision had that it rewrites.
    pub changed: Vec<u32>,
    /// Objects it frees.
    pub freed: Vec<u32>,
    /// A signature covers exactly this revision: its `/ByteRange` ends where the revision does.
    /// The signature is counted, not verified.
    pub signed: bool,
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl Document {
    /// How many revisions the file has: 1 for a file never updated, or one whose
    /// cross-reference table was rebuilt; each incremental update adds one (ISO 32000-2 §7.5.6).
    ///
    /// # Errors
    /// `data-pending` for a source read in windows, which has to be read whole.
    ///
    /// Added in 0.2.0.
    pub fn revision_count(&self) -> Result<u32> {
        Ok(to_u32(self.ops().revision_ends()?.len()))
    }

    /// The file's revisions, oldest first (ISO 32000-2 §7.5.6), each with what its
    /// cross-reference sections list: a linearized file's first-page section is part of its
    /// first revision. Empty for a file whose cross-reference table had to be rebuilt.
    ///
    /// ```
    /// # use laminapdf::{Document, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello-incremental.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let revisions = doc.revisions()?;
    /// assert_eq!(revisions.len(), 2);
    /// println!("the update changed objects {:?}", revisions[1].changed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `data-pending` for a source read in windows, which has to be read whole; reading errors.
    ///
    /// Added in 0.6.0.
    pub fn revisions(&self) -> Result<Vec<RevisionSummary>> {
        let index = self.revision_index()?;
        let signed: BTreeSet<u64> = if index.is_empty() {
            BTreeSet::new()
        } else {
            let planner = laminapdf_save::SavePlanner::new(self.ops())?;
            planner
                .constraint()
                .signatures
                .iter()
                .filter_map(|s| s.covers)
                .collect()
        };
        Ok(Self::revision_list(&index, &signed))
    }

    /// The revisions with what each added, changed and freed, marked signed where a signature's
    /// byte range ends at `signed`'s offsets: `reveal` passes none, and so builds no save
    /// planner.
    pub(crate) fn revision_list(index: &[laminapdf_cos::Revision], signed: &BTreeSet<u64>) -> Vec<RevisionSummary> {
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let mut out = Vec::with_capacity(index.len());
        for revision in index.iter().cloned() {
            let (changed, added): (Vec<u32>, Vec<u32>) = revision.in_use.iter().partition(|n| seen.contains(n));
            seen.extend(&revision.in_use);
            out.push(RevisionSummary {
                index: to_u32(out.len()),
                section: revision.section,
                bytes: revision.end,
                added,
                changed,
                freed: revision.freed,
                signed: signed.contains(&revision.end),
            });
        }
        out
    }

    /// Revision `index` of the file (0 is the original) opened as a document of its own: the
    /// prefix of the file that ends with that revision's `%%EOF`. The latest revision is
    /// `revision_count() - 1`, the file itself.
    ///
    /// # Errors
    /// `revision-index-out-of-range` past the last revision; what [`Document::open_bytes`]
    /// returns for the prefix.
    ///
    /// Added in 0.2.0.
    pub fn open_revision(&self, index: u32, options: OpenOptions) -> Result<Self> {
        let ends = self.ops().revision_ends()?;
        let count = to_u32(ends.len());
        let end = usize::try_from(index)
            .ok()
            .and_then(|i| ends.get(i))
            .and_then(|&end| usize::try_from(end).ok())
            .ok_or(Error::RevisionIndexOutOfRange { index, count })?;
        let bytes = self.ops().whole_input()?;
        let prefix = bytes.get(..end).unwrap_or(&bytes).to_vec();
        Self::open_bytes(prefix, options)
    }
}
