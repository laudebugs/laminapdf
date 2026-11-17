//! The files a document carries (ISO 32000-2 §7.11, §14.13, §12.3.5): reading them, their
//! bytes with the checksum checked, a portfolio's schema, and the edit that attaches a file.
//! [`AttachFile`](crate::AttachFile), [`UpdateAttachment`](crate::UpdateAttachment) and
//! [`DeleteAttachment`](crate::DeleteAttachment) change them through [`Document::apply`].
//!
//! Lamina PDF never opens an attached file: [`Document::attachment_bytes`] gives its bytes, and
//! saving them is the caller's.

use laminapdf_model::{AttachedFile, AttachmentBytes, AttachmentId, Collection};
use laminapdf_ops::{AttachFile, BlobId};

use crate::document::Document;
use crate::error::Result;

impl Document {
    /// Every file the document carries, each once with what holds it: the document's list
    /// (the `/EmbeddedFiles` tree, what viewers show), the document, a page or an annotation it
    /// is associated with (`/AF`), and `FileAttachment` annotations.
    ///
    /// ```
    /// # use laminapdf::{Document, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/examples/pdf_a3_invoice_with_xml.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// for file in doc.attachments()? {
    ///     println!("{:?}: {:?} bytes, {:?}", file.name, file.size, file.relationship);
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Reading errors (the catalog, the page tree).
    ///
    /// Added in 0.6.0.
    pub fn attachments(&self) -> Result<Vec<AttachedFile>> {
        Ok(self.ops().attachments()?)
    }

    /// The bytes of the attached file `id`, exactly as attached, and whether they match the
    /// checksum the file states. The size is checked against the document's
    /// [`Limits`](crate::Limits) first, so a file too large for the device is refused without
    /// being read.
    ///
    /// ```
    /// # use laminapdf::{Checksum, Document, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/examples/pdf_a3_invoice_with_xml.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let xml = doc.attachments()?.remove(0);
    /// let file = doc.attachment_bytes(&xml.id)?;
    /// assert!(file.bytes.starts_with(b"<?xml"));
    /// assert_eq!(file.checksum, Checksum::Matches);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `unknown-attachment` when the document holds no such attachment or its file isn't
    /// embedded; `limit-exceeded` for a file over
    /// [`Limits::max_stream_decoded`](crate::Limits::max_stream_decoded); decoding errors.
    ///
    /// Added in 0.6.0.
    pub fn attachment_bytes(&self, id: &AttachmentId) -> Result<AttachmentBytes> {
        self.ops()
            .attachment_bytes(id)?
            .ok_or_else(|| laminapdf_ops::Error::UnknownAttachment(id.clone()).into())
    }

    /// The portfolio the document is (`/Collection`): the columns of its schema, how its files
    /// are sorted, how it asks to be shown, and each file with its values; `None` for a
    /// document that isn't one.
    ///
    /// # Errors
    /// Reading errors (the catalog).
    ///
    /// Added in 0.6.0.
    pub fn collection(&self) -> Result<Option<Collection>> {
        Ok(self.ops().collection()?)
    }

    /// The edit that attaches `bytes` as `name`, of type `mime` (such as `text/xml`), to the
    /// document's list: the bytes are kept for the session and the op names them by their
    /// SHA-256. Set its dates, description and [`AttachTarget`](crate::AttachTarget) before
    /// applying it with [`Document::apply`]; the dates come from the caller, never a clock.
    /// Keeping the bytes changes nothing to save or undo.
    ///
    /// ```
    /// # use laminapdf::{AfRelationship, AttachTarget, Document, Op, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let op = doc
    ///     .attach_file("readings.csv", b"day,reading\n1,12.5\n".to_vec(), "text/csv")
    ///     .with_target(AttachTarget::Document, AfRelationship::Data);
    /// let edited = doc.apply(vec![Op::from(op)])?;
    /// assert_eq!(doc.attachments()?.len(), edited.created_attachments.len());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Added in 0.6.0.
    pub fn attach_file(&mut self, name: impl Into<String>, bytes: Vec<u8>, mime: impl Into<String>) -> AttachFile {
        let file = self.add_blob(bytes);
        AttachFile::new(name, file, mime)
    }

    /// Keeps `bytes` for the session (an attached file, a replacement image's JPEG or PNG) and
    /// returns their ID, which the ops name them by ([`AttachFile::file`],
    /// `ContentEdit::ReplaceImage`): the SHA-256 of the bytes, so the same bytes get the same ID
    /// on every device and are kept once. Adding bytes changes nothing to save or undo.
    ///
    /// Added in 0.5.0.
    pub fn add_blob(&mut self, bytes: Vec<u8>) -> BlobId {
        self.ops_mut().add_blob(bytes)
    }
}
