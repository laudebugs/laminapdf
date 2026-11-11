//! [`ConformanceWarning`]: what the document's metadata, as edited, breaks of the PDF/A or
//! PDF/UA level its packet claims (E1 plan §3). Warnings never block a save: the user may mean
//! to drop the claim.

use laminapdf_cos::Location;
use laminapdf_model::xmp::PacketState;
use serde::Serialize;

use super::MetadataKey;
use super::read::info_without_equal_xmp;
use crate::error::Result;

/// A conformance claim the document's metadata no longer meets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "code", rename_all = "kebab-case")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum ConformanceWarning {
    /// PDF/A-1 allows `dc:creator` exactly one entry (ISO 19005-1 §6.7.3; veraPDF's 6.7.3
    /// rules), and the packet lists more (`pdfa-one-author`).
    PdfaOneAuthor {
        /// How many authors the packet lists.
        authors: u32,
    },
    /// PDF/UA-1 requires `dc:title` (ISO 14289-1 §7.1, veraPDF rule 7.1-9), and the packet has
    /// none (`pdfua-title-missing`).
    PdfuaTitleMissing,
    /// PDF/A requires every Info entry to have an equal XMP property (ISO 19005-2 §6.6.2.3.1),
    /// and these differ (`pdfa-info-mismatch`).
    PdfaInfoMismatch {
        /// The fields that differ.
        keys: Vec<MetadataKey>,
    },
}

impl ConformanceWarning {
    /// Every code, in the order `fixtures/codes.json`'s `opWarnings` lists them.
    pub const CODES: &'static [&'static str] = &["pdfa-one-author", "pdfua-title-missing", "pdfa-info-mismatch"];

    /// The stable kebab-case code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::PdfaOneAuthor { .. } => "pdfa-one-author",
            Self::PdfuaTitleMissing => "pdfua-title-missing",
            Self::PdfaInfoMismatch { .. } => "pdfa-info-mismatch",
        }
    }

    /// Where the problem is: the document as a whole.
    #[must_use]
    pub const fn location(&self) -> Location {
        Location::NONE
    }
}

/// The claims the document's metadata breaks as it is now (base plus overlay): what the save
/// planner reports before a save.
///
/// # Errors
/// Reading errors (Info, the catalog, the packet's stream).
pub fn conformance_warnings(doc: &laminapdf_model::Document) -> Result<Vec<ConformanceWarning>> {
    let PacketState::Parsed(packet) = doc.xmp()? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let authors = packet.authors().len();
    if packet.pdfa_part() == Some(1) && authors > 1 {
        out.push(ConformanceWarning::PdfaOneAuthor {
            authors: u32::try_from(authors).unwrap_or(u32::MAX),
        });
    }
    if packet.pdfua_part() == Some(1) && packet.title().is_none() {
        out.push(ConformanceWarning::PdfuaTitleMissing);
    }
    if packet.pdfa_part().is_some() {
        let keys = info_without_equal_xmp(doc, &packet)?;
        if !keys.is_empty() {
            out.push(ConformanceWarning::PdfaInfoMismatch { keys });
        }
    }
    Ok(out)
}
