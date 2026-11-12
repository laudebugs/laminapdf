//! The document's metadata, read from the Info dictionary and the XMP packet together:
//! [`Document::metadata`] (E1 plan §2).

use crate::DocumentMetadata;
use crate::document::Document;
use crate::error::Result;

impl Document {
    /// The document's metadata, read from the Info dictionary and the XMP packet together:
    /// the XMP side in a PDF 2.0 or PDF/A file, else the side modified last (Info on a tie),
    /// with the fields where they disagree.
    ///
    /// ```
    /// # let bytes = include_bytes!("../../../fixtures/golden/metadata.pdf").to_vec();
    /// let doc = laminapdf::Document::open_bytes(bytes, Default::default())?;
    /// let meta = doc.metadata()?;
    /// assert_eq!((meta.authors.len(), meta.disagreements.len()), (1, 0));
    /// # Ok::<(), laminapdf::Error>(())
    /// ```
    ///
    /// # Errors
    /// Reading errors (Info, the catalog, the packet's stream).
    pub fn metadata(&self) -> Result<DocumentMetadata> {
        Ok(laminapdf_ops::read_metadata(self.ops())?)
    }
}
