//! The document's metadata, read from the Info dictionary and the XMP packet together:
//! [`Document::metadata`] (E1 plan §2); and the XMP packet itself, [`Document::xmp_packet`].

use laminapdf_model::xmp::PacketState;

use crate::document::Document;
use crate::error::Result;
use crate::{DocumentMetadata, PacketStatus};

/// The document's XMP packet as text, for an editor that shows or replaces it whole.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct XmpText {
    /// Whether there is a packet, and whether it was read.
    pub status: PacketStatus,
    /// The packet's XML as UTF-8 (a UTF-16 packet decoded), with its edits; for a packet that
    /// can't be read, its bytes with anything that isn't UTF-8 replaced. `None` without one.
    pub xml: Option<String>,
    /// The PDF/A part and conformance the packet claims, e.g. `"2B"`: a property an edit adds
    /// is then declared in its extension schemas.
    pub pdfa: Option<String>,
}

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
    ///
    /// Added in 0.2.0.
    pub fn metadata(&self) -> Result<DocumentMetadata> {
        Ok(laminapdf_ops::read_metadata(self.ops())?)
    }

    /// The document's XMP packet as text, with the edits made so far.
    ///
    /// ```
    /// # let bytes = include_bytes!("../../../fixtures/golden/metadata.pdf").to_vec();
    /// let doc = laminapdf::Document::open_bytes(bytes, Default::default())?;
    /// let packet = doc.xmp_packet()?;
    /// assert!(packet.xml.is_some_and(|xml| xml.contains("<rdf:RDF")));
    /// # Ok::<(), laminapdf::Error>(())
    /// ```
    ///
    /// # Errors
    /// Reading errors (the catalog, the packet's stream).
    ///
    /// Added in 0.2.0.
    pub fn xmp_packet(&self) -> Result<XmpText> {
        Ok(match self.ops().xmp()? {
            PacketState::Parsed(p) => XmpText {
                status: PacketStatus::Parsed,
                xml: Some(p.xml().to_owned()),
                pdfa: p
                    .pdfa_part()
                    .map(|part| format!("{part}{}", p.pdfa_conformance().unwrap_or_default())),
            },
            PacketState::Malformed { bytes, .. } => XmpText {
                status: PacketStatus::Malformed,
                xml: Some(String::from_utf8_lossy(&bytes).into_owned()),
                pdfa: None,
            },
            // No packet, or a later state with none to show.
            _ => XmpText::default(),
        })
    }
}
