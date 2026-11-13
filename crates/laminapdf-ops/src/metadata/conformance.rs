//! [`ConformanceWarning`]: what the document's edits break of the PDF/A or PDF/UA level its
//! packet claims: its metadata as edited (E1 plan §3), and a page duplication that couldn't
//! clone all of a page's tags (E2 plan §3). Warnings never block a save: the user may mean to
//! drop the claim.

use laminapdf_cos::{ErrorKind, Location};
use laminapdf_model::xmp::PacketState;
use serde::Serialize;

use super::MetadataKey;
use super::read::info_without_equal_xmp;
use crate::error::Result;

/// A conformance claim the document's metadata no longer meets.
///
/// Added in 0.2.0.
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
    /// PDF/UA-1 requires all real content to be tagged (ISO 14289-1 §7.1), and a duplicated
    /// page's content is partly untagged: it belonged to elements that also hold content on
    /// other pages, which a copy can't share (`duplicate-tags-incomplete`).
    DuplicateTagsIncomplete,
}

impl ConformanceWarning {
    /// Every code, in the order `fixtures/codes.json`'s `opWarnings` lists them.
    ///
    /// Added in 0.2.0.
    pub const CODES: &'static [&'static str] = &[
        "pdfa-one-author",
        "pdfua-title-missing",
        "pdfa-info-mismatch",
        "duplicate-tags-incomplete",
    ];

    /// The stable kebab-case code.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::PdfaOneAuthor { .. } => "pdfa-one-author",
            Self::PdfuaTitleMissing => "pdfua-title-missing",
            Self::PdfaInfoMismatch { .. } => "pdfa-info-mismatch",
            Self::DuplicateTagsIncomplete => "duplicate-tags-incomplete",
        }
    }

    /// The kind ([ADR 0026] §2), as `fixtures/codes.json` lists it for [`code`](Self::code) (a
    /// unit test checks every variant): `usage` for what an edit did to the claim, `input` for
    /// Info and XMP disagreeing, which comes from the file (or another tool), since Lamina PDF's
    /// edits write both.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::PdfaOneAuthor { .. } | Self::PdfuaTitleMissing | Self::DuplicateTagsIncomplete => ErrorKind::Usage,
            Self::PdfaInfoMismatch { .. } => ErrorKind::Input,
        }
    }

    /// Where the problem is: the document as a whole.
    ///
    /// Added in 0.2.0.
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
///
/// Added in 0.2.0.
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

/// [`conformance_warnings`], plus what `pending` (the changes not yet saved) breaks: a page
/// duplication that left a copy, still in the document, partly untagged, on a file whose
/// packet claims PDF/UA-1.
///
/// # Errors
/// As [`conformance_warnings`].
///
/// Added in 0.2.0.
pub fn pending_conformance_warnings(
    doc: &laminapdf_model::Document,
    pending: &crate::PendingChanges,
) -> Result<Vec<ConformanceWarning>> {
    let mut out = conformance_warnings(doc)?;
    if !pending.tags_incomplete().is_empty()
        // A page tree that can't be read can't show the copy gone: keep the warning.
        && doc
            .pages()
            .map_or(true, |tree| tree.iter().any(|p| pending.tags_incomplete().contains(p.id())))
        && let PacketState::Parsed(packet) = doc.xmp()?
        && packet.pdfua_part() == Some(1)
    {
        out.push(ConformanceWarning::DuplicateTagsIncomplete);
    }
    Ok(out)
}
