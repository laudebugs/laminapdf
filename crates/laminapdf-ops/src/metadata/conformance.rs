//! [`ConformanceWarning`]: what the document's edits break of the PDF/A or PDF/UA level its
//! packet claims: its metadata as edited (E1 plan §3), a page duplication that couldn't
//! clone all of a page's tags (E2 plan §3), and the annotations the edits wrote (E4 plan §3). Warnings never block a save: the user may mean to
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
    /// PDF/UA-1 requires all real content to be tagged (ISO 14289-1 §7.1), and pages copied in
    /// from another document came without the tags their source gave them
    /// (`imported-tags-dropped`).
    ImportedTagsDropped,
    /// The file claims PDF/A, and pages were copied in from a document that doesn't claim the
    /// same level: nothing checks the fonts, colour spaces and transparency they bring against
    /// the claim (ISO 19005-2 §6) (`imported-pdfa-unchecked`).
    ImportedPdfaUnchecked,
    /// The file claims PDF/UA-1, and pages were copied in from a document that doesn't claim
    /// it: nothing checks their tags, alternate text and reading order against the claim
    /// (ISO 14289-1 §7) (`imported-pdfua-unchecked`).
    ///
    /// Added in 0.3.0.
    ImportedPdfuaUnchecked,
    /// PDF/UA-1 requires an annotation's `/Contents`, its alternative description (ISO 14289-1
    /// §7.18.1), and these new or changed annotations have none (`annot-without-contents`).
    ///
    /// Added in 0.3.0.
    AnnotWithoutContents {
        /// The annotations' IDs.
        annotations: Vec<String>,
    },
    /// PDF/UA-1 requires every annotation to be tagged (ISO 14289-1 §7.18.1), and these new
    /// annotations have no place in the structure tree: the file has none
    /// (`annot-not-tagged`).
    ///
    /// Added in 0.3.0.
    AnnotNotTagged {
        /// The annotations' IDs.
        annotations: Vec<String>,
    },
    /// PDF/A requires every annotation to print and none to be hidden (ISO 19005-2 §6.3.2), and
    /// these annotations' flags break that (`pdfa-annot-flags`).
    ///
    /// Added in 0.3.0.
    PdfaAnnotFlags {
        /// The annotations' IDs.
        annotations: Vec<String>,
    },
    /// PDF/A-1 forbids transparency (ISO 19005-1 §6.4), and these annotations are translucent or
    /// blend, as a highlight does (`pdfa-transparency`).
    ///
    /// Added in 0.3.0.
    PdfaTransparency {
        /// The annotations' IDs.
        annotations: Vec<String>,
    },
    /// The file claims PDF/A, and an annotation the edits added, changed or flattened uses a
    /// font that isn't embedded (ISO 19005-2 §6.2.11.4.1): a text box drawn in a standard 14
    /// font, which wants a font by its bytes, or a flattened appearance that names one
    /// (`pdfa-font-not-embedded`).
    ///
    /// Added in 0.5.0.
    PdfaFontNotEmbedded {
        /// The annotations' IDs: the text boxes, or the flattened annotations, which no page
        /// lists any more.
        annotations: Vec<String>,
        /// The pages whose edited content uses a font the edits added that isn't embedded; empty
        /// when only annotations do (left out of the JSON then). Added in 0.6.0.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "tsify", tsify(optional))]
        pages: Vec<String>,
    },
    /// The file claims PDF/UA-1, and flattening drew comments into the page as artifacts and
    /// took them out of the structure tree, so their text is no longer read out (ISO 14289-1
    /// §7.18.1) (`flatten-untagged`).
    ///
    /// Added in 0.6.0.
    FlattenUntagged {
        /// The flattened annotations' IDs.
        annotations: Vec<String>,
    },
    /// The file claims PDF/UA, and an edit replaced the image of a figure, whose alternative
    /// text described the old image and was removed; it has none now (ISO 14289-1 §7.3)
    /// (`figure-without-alt`).
    ///
    /// Added in 0.6.0.
    FigureWithoutAlt {
        /// The pages the figures are on.
        pages: Vec<String>,
    },
    /// The file claims PDF/UA, and pages an edit changed draw content their tags don't cover: marked
    /// content the structure tree doesn't map to an element, real content outside any marked
    /// content, or a tagged form or image an edit took off the page (ISO 14289-1 §7.1)
    /// (`content-tags-broken`). The tags were often broken before the edit; kind `input`.
    ///
    /// Added in 0.6.0.
    ContentTagsBroken {
        /// The pages.
        pages: Vec<String>,
    },
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
        "imported-tags-dropped",
        "imported-pdfa-unchecked",
        "imported-pdfua-unchecked",
        "annot-without-contents",
        "annot-not-tagged",
        "pdfa-annot-flags",
        "pdfa-transparency",
        "pdfa-font-not-embedded",
        "flatten-untagged",
        "figure-without-alt",
        "content-tags-broken",
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
            Self::ImportedTagsDropped => "imported-tags-dropped",
            Self::ImportedPdfaUnchecked => "imported-pdfa-unchecked",
            Self::ImportedPdfuaUnchecked => "imported-pdfua-unchecked",
            Self::AnnotWithoutContents { .. } => "annot-without-contents",
            Self::AnnotNotTagged { .. } => "annot-not-tagged",
            Self::PdfaAnnotFlags { .. } => "pdfa-annot-flags",
            Self::PdfaTransparency { .. } => "pdfa-transparency",
            Self::PdfaFontNotEmbedded { .. } => "pdfa-font-not-embedded",
            Self::FlattenUntagged { .. } => "flatten-untagged",
            Self::FigureWithoutAlt { .. } => "figure-without-alt",
            Self::ContentTagsBroken { .. } => "content-tags-broken",
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
            Self::PdfaOneAuthor { .. }
            | Self::PdfuaTitleMissing
            | Self::DuplicateTagsIncomplete
            | Self::ImportedTagsDropped
            | Self::ImportedPdfaUnchecked
            | Self::ImportedPdfuaUnchecked
            | Self::AnnotWithoutContents { .. }
            | Self::AnnotNotTagged { .. }
            | Self::PdfaAnnotFlags { .. }
            | Self::PdfaTransparency { .. }
            | Self::PdfaFontNotEmbedded { .. }
            | Self::FlattenUntagged { .. }
            | Self::FigureWithoutAlt { .. } => ErrorKind::Usage,
            // Mostly the file's own tags were broken before the edit touched the page.
            Self::PdfaInfoMismatch { .. } | Self::ContentTagsBroken { .. } => ErrorKind::Input,
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

/// What flattening broke: on a file claiming PDF/UA-1, comments that left the structure tree; on
/// one claiming PDF/A, fonts that aren't embedded. Only annotations no page lists any more count,
/// so an undone flattening warns of nothing.
fn flatten_warnings(
    doc: &laminapdf_model::Document,
    pending: &crate::PendingChanges,
) -> Result<Vec<ConformanceWarning>> {
    if pending.flattened().is_empty() {
        return Ok(Vec::new());
    }
    let PacketState::Parsed(packet) = doc.xmp()? else {
        return Ok(Vec::new());
    };
    let listed: std::collections::BTreeSet<laminapdf_model::AnnotId> = crate::annot::every_annotation(doc)?
        .into_iter()
        .flat_map(|(_, annots)| annots.into_iter().map(|a| a.id))
        .collect();
    let gone = |ids: &std::collections::BTreeSet<laminapdf_model::AnnotId>| -> Vec<String> {
        ids.iter()
            .filter(|id| !listed.contains(id))
            .map(ToString::to_string)
            .collect()
    };
    let mut out = Vec::new();
    let untagged = gone(pending.flattened());
    if packet.pdfua_part() == Some(1) && !untagged.is_empty() {
        out.push(ConformanceWarning::FlattenUntagged { annotations: untagged });
    }
    let unembedded = gone(pending.flattened_unembedded());
    if packet.pdfa_part().is_some() && !unembedded.is_empty() {
        out.push(ConformanceWarning::PdfaFontNotEmbedded {
            annotations: unembedded,
            pages: Vec::new(),
        });
    }
    Ok(out)
}

/// [`conformance_warnings`], plus what `pending` (the changes not yet saved) breaks: annotations
/// the edits wrote that break the file's PDF/A or PDF/UA claim; content edits that added a font
/// that isn't embedded, left a figure without alternative text or broke the tags; on a file whose packet claims
/// PDF/UA-1, a page duplication that left a copy, still in the document, partly untagged, tagged
/// pages copied in from another document without their tags, and pages copied with their tags
/// from a document without the claim; on a file claiming PDF/A,
/// pages copied in from a document without the same claim.
///
/// # Errors
/// As [`conformance_warnings`].
///
/// Added in 0.3.0.
pub fn pending_conformance_warnings(
    doc: &laminapdf_model::Document,
    pending: &crate::PendingChanges,
) -> Result<Vec<ConformanceWarning>> {
    let mut out = conformance_warnings(doc)?;
    if pending.classes().contains(&crate::ModificationClass::Annotation) {
        out.extend(crate::annot::conformance_warnings(doc)?);
    }
    out.extend(flatten_warnings(doc, pending)?);
    #[cfg(feature = "content-edit")]
    super::content_conformance::warnings(doc, pending, &mut out)?;
    let (incomplete, dropped, unchecked, ua_unchecked) = (
        pending.tags_incomplete(),
        pending.tags_dropped(),
        pending.pdfa_unchecked(),
        pending.pdfua_unchecked(),
    );
    if incomplete.is_empty() && dropped.is_empty() && unchecked.is_empty() && ua_unchecked.is_empty() {
        return Ok(out);
    }
    let PacketState::Parsed(packet) = doc.xmp()? else {
        return Ok(out);
    };
    // A page tree that can't be read can't show the pages gone: keep the warnings.
    let still = |pages: &std::collections::BTreeSet<laminapdf_model::PageId>| {
        !pages.is_empty()
            && doc
                .pages()
                .map_or(true, |tree| tree.iter().any(|p| pages.contains(p.id())))
    };
    if packet.pdfua_part() == Some(1) {
        if still(incomplete) {
            out.push(ConformanceWarning::DuplicateTagsIncomplete);
        }
        if still(dropped) {
            out.push(ConformanceWarning::ImportedTagsDropped);
        }
        if still(ua_unchecked) {
            out.push(ConformanceWarning::ImportedPdfuaUnchecked);
        }
    }
    if packet.pdfa_part().is_some() && still(unchecked) {
        out.push(ConformanceWarning::ImportedPdfaUnchecked);
    }
    Ok(out)
}
