//! A file's revisions (ISO 32000-2 §7.5.6): each incremental update appends one, and each
//! earlier revision is a prefix of the file that opens like a file of its own.

use laminapdf_cos::OpenOptions;

use crate::{Document, Error, Result};

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl Document {
    /// How many revisions the file has: 1 for a file never updated, or one whose
    /// cross-reference table was rebuilt; each incremental update adds one (ISO 32000-2 §7.5.6).
    ///
    /// # Errors
    /// `data-pending` for a source read in windows, which has to be read whole.
    pub fn revision_count(&self) -> Result<u32> {
        Ok(to_u32(self.ops().revision_ends()?.len()))
    }

    /// Revision `index` of the file (0 is the original) opened as a document of its own: the
    /// prefix of the file that ends with that revision's `%%EOF`. The latest revision is
    /// `revision_count() - 1`, the file itself.
    ///
    /// # Errors
    /// `revision-index-out-of-range` past the last revision; what [`Document::open_bytes`]
    /// returns for the prefix.
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
