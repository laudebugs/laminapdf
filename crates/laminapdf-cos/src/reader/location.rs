//! Where an object is stored (ISO 32000-2 §7.5.4, §7.5.7, §7.5.8): at a byte offset in the
//! file, inside an object stream, or nowhere yet because this session created it. An inspector
//! shows it; nothing else needs it, since resolution hides it.

use super::Document;
use super::xref::XrefEntry;
use crate::overlay::OverlayEntry;

/// Where object `num` is stored ([`Document::object_location`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObjectLocation {
    /// At this byte offset in the file, with its generation number (a cross-reference table's
    /// `n` entry, or a type 1 entry of a cross-reference stream).
    Offset {
        /// The byte offset of `num gen obj`.
        offset: u64,
        /// The generation number.
        generation: u16,
    },
    /// The `index`th object of object stream `stream` (a type 2 entry; generation 0).
    ObjectStream {
        /// The object stream's number.
        stream: u32,
        /// The object's index within it.
        index: u32,
    },
    /// Created in this session: in no revision of the file yet.
    Created,
}

impl Document {
    /// Where object `num` is stored now: as the newest cross-reference section lists it, or
    /// [`ObjectLocation::Created`] for one this session made. `None` for a free, deleted or
    /// unknown object. A changed base object keeps its place in the file until saved.
    ///
    /// ```
    /// use laminapdf_cos::{Document, ObjectLocation, OpenOptions};
    ///
    /// let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n";
    /// let doc = Document::open(bytes.to_vec(), OpenOptions::default())?;
    /// assert_eq!(doc.object_location(1), Some(ObjectLocation::Offset { offset: 9, generation: 0 }));
    /// assert_eq!(doc.object_location(2), None);
    /// # Ok::<(), laminapdf_cos::Error>(())
    /// ```
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn object_location(&self, num: u32) -> Option<ObjectLocation> {
        match self.overlay_ref().get(num) {
            Some(OverlayEntry::Deleted) => return None,
            Some(OverlayEntry::New(_)) => return Some(ObjectLocation::Created),
            _ => {}
        }
        match self.entries.get(&num)? {
            XrefEntry::Free => None,
            XrefEntry::InFile { offset, gen_nr } => Some(ObjectLocation::Offset {
                offset: u64::try_from(*offset).unwrap_or(u64::MAX),
                generation: *gen_nr,
            }),
            XrefEntry::InStream { stream, index } => Some(ObjectLocation::ObjectStream {
                stream: *stream,
                index: *index,
            }),
        }
    }
}
