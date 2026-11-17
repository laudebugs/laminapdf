//! The version a file declares and the standards it claims to meet: PDF/A (ISO 19005), PDF/UA
//! (ISO 14289) and PDF/X (ISO 15930). A claim is what the file says, never a verdict: nothing
//! here checks that the file meets the standard.

use laminapdf_cos::Result;

use crate::document::Document;
use crate::xmp::PacketState;

/// The version a file declares and the standards it claims ([`Document::conformance`]). Each
/// claim is claimed, not verified.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ConformanceClaims {
    /// The `%PDF-x.y` header's version; `None` without a header.
    pub header_version: Option<String>,
    /// The catalog's `/Version` (ISO 32000-2 §7.7.2), as written; `None` when absent.
    pub catalog_version: Option<String>,
    /// The PDF/A claim (`pdfaid:part` and `pdfaid:conformance` in the XMP metadata).
    pub pdfa: Option<PdfaClaim>,
    /// The PDF/UA part claimed (`pdfuaid:part`).
    pub pdfua: Option<u8>,
    /// The PDF/X version claimed (`GTS_PDFXVersion`: in the XMP metadata for PDF/X-4 and later,
    /// in the Info dictionary for earlier parts), as written (`PDF/X-4`, `PDF/X-1a:2003`).
    pub pdfx: Option<String>,
}

/// A PDF/A claim.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PdfaClaim {
    /// The part of ISO 19005 (1 to 4).
    pub part: u8,
    /// The conformance level (`A`, `B`, `U`, or `E` and `F` for PDF/A-4); `None` when the file
    /// gives none, as PDF/A-4 allows.
    pub conformance: Option<String>,
}

impl Document {
    /// The version the file declares and the standards it claims: the header's and the
    /// catalog's versions, and the PDF/A, PDF/UA and PDF/X claims. A packet that doesn't parse
    /// claims nothing.
    ///
    /// # Errors
    /// Reading errors (the catalog, the metadata stream).
    ///
    /// Added in 0.6.0.
    pub fn conformance(&self) -> Result<ConformanceClaims> {
        let catalog = self.catalog()?;
        let catalog_version = catalog
            .get_name("Version")
            .map(|name| String::from_utf8_lossy(name).into_owned());
        let mut out = ConformanceClaims {
            header_version: self.header_version().map(str::to_owned),
            catalog_version,
            ..ConformanceClaims::default()
        };
        if let PacketState::Parsed(packet) = self.xmp()? {
            out.pdfa = packet.pdfa_part().map(|part| PdfaClaim {
                part,
                conformance: packet.pdfa_conformance().filter(|c| !c.is_empty()),
            });
            out.pdfua = packet.pdfua_part();
            out.pdfx = packet.pdfx_version();
        }
        if out.pdfx.is_none()
            && let Some(info) = self.info()?
        {
            out.pdfx = info
                .text("GTS_PDFXVersion")?
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty());
        }
        Ok(out)
    }
}
