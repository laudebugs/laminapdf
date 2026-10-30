//! Stable, enumerated warning codes (ADR 0003 §2) and the per-document warning log.

use std::collections::BTreeSet;

use lamina_syntax::{ErrorKind, Limits, Location, Position};

use crate::error::{Error, Result};

/// A recoverable problem the tolerant reader repaired or worked around (ADR 0003 §2).
///
/// Each variant has a stable kebab-case [`code`](Warning::code) that is part of the public
/// API in Rust, WASM and TypeScript; codes are never renamed or reused. Variants carry the
/// byte offset or object number where one exists. With
/// [`OpenOptions::with_strict`](crate::OpenOptions::with_strict), every warning is an
/// [`Error::Strict`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Warning {
    /// `missing-header`: no `%PDF-x.y` in the first 1,024 bytes; parsing went ahead anyway.
    MissingHeader,
    /// `xref-repaired`: the stored cross-reference data was unusable and was rebuilt by scanning.
    XrefRepaired,
    /// `xrefstm-ignored`: a hybrid file's `/XRefStm` could not be read.
    XrefStmIgnored {
        /// Byte offset the `/XRefStm` entry points at.
        offset: u64,
    },
    /// `stream-length-recovered`: a stream's `/Length` was wrong or unresolvable, so its
    /// extent was found by scanning for `endstream`.
    StreamLengthRecovered {
        /// The stream's object number.
        object: u32,
    },
    /// `stale-generation`: a reference's generation did not match its xref entry, so it
    /// resolved to `null` (§7.3.10).
    StaleGeneration {
        /// The referenced object number.
        object: u32,
        /// The generation the reference asked for.
        generation: u16,
    },
    /// `page-tree-node-skipped`: a page-tree kid or `/Kids` was missing, unparsable, of the
    /// wrong type, or deeper than [`Limits::max_page_tree_depth`].
    PageTreeNodeSkipped {
        /// The skipped node's object number (`None` for a direct object).
        object: Option<u32>,
    },
    /// `page-count-mismatch`: the page-tree root's `/Count` differs from the walked page count.
    PageCountMismatch {
        /// The root node's object number (`None` for a direct object).
        object: Option<u32>,
        /// The `/Count` the file declares.
        declared: i64,
        /// The number of pages the walk found.
        walked: u64,
    },
    /// `flate-truncated`: Flate output was kept despite truncation or a bad checksum.
    FlateTruncated {
        /// The stream's object number, when known.
        object: Option<u32>,
    },
    /// `objstm-index-mismatch`: an object-stream index was stale and a search found the object.
    ObjStmIndexMismatch {
        /// The object found by searching.
        object: u32,
    },
    /// `object-number-out-of-range`: an xref entry above [`Limits::max_object_number`] was dropped.
    ObjectNumberOutOfRange {
        /// The dropped object number.
        object: u32,
    },
    /// `aes-padding-invalid`: an AES-encrypted string or stream was not `IV ‖ whole blocks`
    /// (kept as stored) or had invalid PKCS#5 padding (decrypted, padding kept). A plaintext
    /// string in an encrypted document (PDFex-style mixing) usually shows up this way.
    AesPaddingInvalid {
        /// The object holding the data.
        object: u32,
    },
    /// `perms-mismatch`: an R5/R6 document's `/Perms` does not decrypt to its `/P` and
    /// `/EncryptMetadata` (ISO 32000-2 Algorithm 13): `/P` may have been tampered with.
    PermsMismatch,
    /// `warnings-truncated`: more than [`Limits::max_warnings`] warnings were raised; the rest
    /// were dropped.
    WarningsTruncated,
    /// `invalid-box`: a page boundary box (`/MediaBox`, `/CropBox`, `/BleedBox`, `/TrimBox`,
    /// `/ArtBox`) is not an array of four numbers; the view reads it as absent.
    InvalidBox {
        /// The page's object number (`None` for a direct page dictionary).
        object: Option<u32>,
        /// The page's index (tells direct pages apart).
        page: u64,
        /// Which box, e.g. `"MediaBox"`.
        key: &'static str,
    },
    /// `invalid-rotate`: a page's `/Rotate` is not an integer multiple of 90; the view reads 0.
    InvalidRotate {
        /// The page's object number (`None` for a direct page dictionary).
        object: Option<u32>,
        /// The page's index (tells direct pages apart).
        page: u64,
    },
    /// `name-tree-node-skipped`: a name- or number-tree node, kid or entry was missing,
    /// malformed, reached twice, or deeper than [`Limits::max_tree_depth`]; it was skipped.
    NameTreeNodeSkipped {
        /// The skipped node's object number (`None` for a direct object).
        object: Option<u32>,
    },
    /// `outline-cycle`: an outline item was reached twice; the walk stopped that branch.
    OutlineCycle {
        /// The item reached again.
        object: u32,
    },
    /// `outline-item-skipped`: an outline item was missing, not a dictionary, or deeper than
    /// [`Limits::max_tree_depth`]; it and its subtree were skipped.
    OutlineItemSkipped {
        /// The skipped item's object number (`None` for a direct object).
        object: Option<u32>,
    },
    /// `invalid-date`: an Info date is not a valid PDF date (§7.9.4); the raw string is kept.
    InvalidDate {
        /// The Info dictionary's object number (`None` for a direct one).
        object: Option<u32>,
        /// Which entry, e.g. `"ModDate"` (`"other"` for a key outside the standard ones).
        key: &'static str,
    },
    /// `invalid-page-label`: a `/PageLabels` entry is malformed: not a dictionary, a negative
    /// start, an unknown `/S` style (no numeric part), a `/P` that is not a string (no prefix)
    /// or `/St` below 1 (numbering from 1). The rest of the entry is kept.
    InvalidPageLabel {
        /// The page index the range starts at, as stored (it may be negative).
        start: i64,
    },
    /// `invalid-entry`: an entry a view reads (`/Info`, `/Names`, `/Outlines`, `/Dests`,
    /// `/UserUnit`, …) is unreadable or of the wrong type; the view reads it as absent (or its
    /// default).
    InvalidEntry {
        /// The object holding the entry (`None` for a direct object or the trailer).
        object: Option<u32>,
        /// The entry's key, e.g. `"Info"`.
        key: &'static str,
    },
    /// `content-unknown-operator`: a content stream used an operator ISO 32000-2 Annex A does
    /// not define, outside a `BX`/`EX` compatibility section; it was skipped (ADR 0023).
    ContentUnknownOperator {
        /// The content's object: the page for page content, the Form XObject for a form;
        /// `None` for content interpreted on its own (`interpret_content`) or a direct page.
        object: Option<u32>,
        /// Byte offset of the operator in the decoded content.
        offset: u64,
    },
    /// `content-bad-operands`: a content-stream operator had too few operands or operands of
    /// the wrong type (it was skipped), or extra operands (the last ones were used) (ADR 0023).
    ContentBadOperands {
        /// The content's object (as for `content-unknown-operator`).
        object: Option<u32>,
        /// Byte offset of the operator in the decoded content.
        offset: u64,
    },
    /// `content-unbalanced-q`: a `Q` without a matching `q` (ignored), or a content stream
    /// that ended with saved graphics states still open (restored) (ADR 0023).
    ContentUnbalancedQ {
        /// The content's object (as for `content-unknown-operator`).
        object: Option<u32>,
        /// Byte offset of the `Q`, or the end of the content.
        offset: u64,
    },
    /// `content-unbalanced-bt`: a `BT` inside a text object, an `ET` outside one, a text
    /// operator outside one, or a content stream that ended inside one (ADR 0023).
    ContentUnbalancedBt {
        /// The content's object (as for `content-unknown-operator`).
        object: Option<u32>,
        /// Byte offset of the operator, or the end of the content.
        offset: u64,
    },
    /// `content-unbalanced-marked`: an `EMC` without a matching `BMC`/`BDC` (ignored), or a
    /// content stream that ended with marked-content sequences open (closed) (ADR 0023).
    ContentUnbalancedMarked {
        /// The content's object (as for `content-unknown-operator`).
        object: Option<u32>,
        /// Byte offset of the `EMC`, or the end of the content.
        offset: u64,
    },
    /// `content-syntax`: a content stream held bytes that are not a token (an unbalanced `)`
    /// or `>`, a bad hex digit, an unterminated string or array) or a stray `ID`/`EI`; they
    /// were skipped with the operands read so far (ADR 0023).
    ContentSyntax {
        /// The content's object (as for `content-unknown-operator`).
        object: Option<u32>,
        /// Byte offset of the problem in the decoded content.
        offset: u64,
    },
    /// `content-resource-missing`: an operator named a resource (font, XObject, graphics
    /// state, colour space, shading, property list) that is missing, of the wrong type or
    /// unreadable, or a Form XObject's content could not be decoded; the operator was
    /// skipped, or ran without the resource (ADR 0023).
    ContentResourceMissing {
        /// The content's object (as for `content-unknown-operator`).
        object: Option<u32>,
        /// Byte offset of the operator in the decoded content.
        offset: u64,
        /// The resource category, e.g. `"Font"`.
        category: &'static str,
    },
    /// `inline-image-unterminated`: no `EI` ends an inline image (neither at its declared or
    /// computed length nor by the heuristic of ADR 0023); the rest of the content was
    /// skipped.
    InlineImageUnterminated {
        /// The content's object (as for `content-unknown-operator`).
        object: Option<u32>,
        /// Byte offset of the `BI` in the decoded content.
        offset: u64,
    },
    /// `content-path-unpainted`: a content stream ended with a path that no painting operator
    /// ended; it was dropped (ADR 0023).
    ContentPathUnpainted {
        /// The content's object (as for `content-unknown-operator`).
        object: Option<u32>,
        /// Byte offset of the path's first segment in the decoded content.
        offset: u64,
    },
    /// `xobject-cycle`: a Form XObject draws itself, directly or through other forms; the
    /// inner `Do` was skipped (ADR 0023).
    XObjectCycle {
        /// The form reached again.
        object: u32,
    },
    /// `font-invalid`: a font dictionary read for text extraction has a missing, unreadable
    /// or malformed entry (descendant font, encoding, `/Differences`, widths, font program);
    /// its text was decoded with defaults (`lamina-text`, todo B9b).
    FontInvalid {
        /// The font dictionary's object (`None` for a direct one).
        object: Option<u32>,
    },
    /// `cmap-invalid`: a font's CMap or ToUnicode CMap had malformed entries (skipped), names
    /// a predefined CMap lamina has no table for (read as Identity), or a `/UseCMap` chain
    /// too deep (cut) (`lamina-text`, todo B9b).
    CMapInvalid {
        /// The font dictionary's object (`None` for a direct one).
        object: Option<u32>,
    },
    /// `unmapped-glyph`: a font showed glyphs that neither its ToUnicode CMap, its encoding's
    /// glyph names nor its font program map to text; they extract as U+FFFD (`lamina-text`,
    /// todo B9b).
    UnmappedGlyph {
        /// The font dictionary's object (`None` for a direct one).
        object: Option<u32>,
    },
}

impl Warning {
    /// Every code [`Warning::code`] can return (listed in `fixtures/codes.json`).
    pub const CODES: &'static [&'static str] = &[
        "missing-header",
        "xref-repaired",
        "xrefstm-ignored",
        "stream-length-recovered",
        "stale-generation",
        "page-tree-node-skipped",
        "page-count-mismatch",
        "flate-truncated",
        "objstm-index-mismatch",
        "object-number-out-of-range",
        "aes-padding-invalid",
        "perms-mismatch",
        "warnings-truncated",
        "invalid-box",
        "invalid-rotate",
        "name-tree-node-skipped",
        "outline-cycle",
        "outline-item-skipped",
        "invalid-date",
        "invalid-page-label",
        "invalid-entry",
        "content-unknown-operator",
        "content-bad-operands",
        "content-unbalanced-q",
        "content-unbalanced-bt",
        "content-unbalanced-marked",
        "content-syntax",
        "content-resource-missing",
        "inline-image-unterminated",
        "xobject-cycle",
        "content-path-unpainted",
        "font-invalid",
        "cmap-invalid",
        "unmapped-glyph",
    ];

    /// The stable kebab-case code, e.g. `"xref-repaired"`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::MissingHeader => "missing-header",
            Self::XrefRepaired => "xref-repaired",
            Self::XrefStmIgnored { .. } => "xrefstm-ignored",
            Self::StreamLengthRecovered { .. } => "stream-length-recovered",
            Self::StaleGeneration { .. } => "stale-generation",
            Self::PageTreeNodeSkipped { .. } => "page-tree-node-skipped",
            Self::PageCountMismatch { .. } => "page-count-mismatch",
            Self::FlateTruncated { .. } => "flate-truncated",
            Self::ObjStmIndexMismatch { .. } => "objstm-index-mismatch",
            Self::ObjectNumberOutOfRange { .. } => "object-number-out-of-range",
            Self::AesPaddingInvalid { .. } => "aes-padding-invalid",
            Self::PermsMismatch => "perms-mismatch",
            Self::WarningsTruncated => "warnings-truncated",
            Self::InvalidBox { .. } => "invalid-box",
            Self::InvalidRotate { .. } => "invalid-rotate",
            Self::NameTreeNodeSkipped { .. } => "name-tree-node-skipped",
            Self::OutlineCycle { .. } => "outline-cycle",
            Self::OutlineItemSkipped { .. } => "outline-item-skipped",
            Self::InvalidDate { .. } => "invalid-date",
            Self::InvalidPageLabel { .. } => "invalid-page-label",
            Self::InvalidEntry { .. } => "invalid-entry",
            Self::ContentUnknownOperator { .. } => "content-unknown-operator",
            Self::ContentBadOperands { .. } => "content-bad-operands",
            Self::ContentUnbalancedQ { .. } => "content-unbalanced-q",
            Self::ContentUnbalancedBt { .. } => "content-unbalanced-bt",
            Self::ContentUnbalancedMarked { .. } => "content-unbalanced-marked",
            Self::ContentSyntax { .. } => "content-syntax",
            Self::ContentResourceMissing { .. } => "content-resource-missing",
            Self::InlineImageUnterminated { .. } => "inline-image-unterminated",
            Self::XObjectCycle { .. } => "xobject-cycle",
            Self::ContentPathUnpainted { .. } => "content-path-unpainted",
            Self::FontInvalid { .. } => "font-invalid",
            Self::CMapInvalid { .. } => "cmap-invalid",
            Self::UnmappedGlyph { .. } => "unmapped-glyph",
        }
    }

    /// The object number the warning is about, if any.
    #[must_use]
    pub const fn object(&self) -> Option<u32> {
        match *self {
            Self::StreamLengthRecovered { object }
            | Self::StaleGeneration { object, .. }
            | Self::ObjStmIndexMismatch { object }
            | Self::ObjectNumberOutOfRange { object }
            | Self::OutlineCycle { object }
            | Self::AesPaddingInvalid { object }
            | Self::XObjectCycle { object } => Some(object),
            Self::PageTreeNodeSkipped { object }
            | Self::PageCountMismatch { object, .. }
            | Self::FlateTruncated { object }
            | Self::InvalidBox { object, .. }
            | Self::InvalidRotate { object, .. }
            | Self::NameTreeNodeSkipped { object }
            | Self::OutlineItemSkipped { object }
            | Self::InvalidDate { object, .. }
            | Self::InvalidEntry { object, .. }
            | Self::ContentUnknownOperator { object, .. }
            | Self::ContentBadOperands { object, .. }
            | Self::ContentUnbalancedQ { object, .. }
            | Self::ContentUnbalancedBt { object, .. }
            | Self::ContentUnbalancedMarked { object, .. }
            | Self::ContentSyntax { object, .. }
            | Self::ContentResourceMissing { object, .. }
            | Self::InlineImageUnterminated { object, .. }
            | Self::ContentPathUnpainted { object, .. }
            | Self::FontInvalid { object }
            | Self::CMapInvalid { object }
            | Self::UnmappedGlyph { object } => object,
            Self::MissingHeader
            | Self::XrefRepaired
            | Self::XrefStmIgnored { .. }
            | Self::PermsMismatch
            | Self::WarningsTruncated
            | Self::InvalidPageLabel { .. } => None,
        }
    }

    /// The kind of this warning ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`Warning::code`] (a test checks every variant).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::MissingHeader
            | Self::XrefRepaired
            | Self::XrefStmIgnored { .. }
            | Self::StreamLengthRecovered { .. }
            | Self::StaleGeneration { .. }
            | Self::PageTreeNodeSkipped { .. }
            | Self::PageCountMismatch { .. }
            | Self::FlateTruncated { .. }
            | Self::ObjStmIndexMismatch { .. }
            | Self::InvalidBox { .. }
            | Self::InvalidRotate { .. }
            | Self::NameTreeNodeSkipped { .. }
            | Self::OutlineCycle { .. }
            | Self::OutlineItemSkipped { .. }
            | Self::InvalidDate { .. }
            | Self::InvalidPageLabel { .. }
            | Self::InvalidEntry { .. }
            | Self::ContentUnknownOperator { .. }
            | Self::ContentBadOperands { .. }
            | Self::ContentUnbalancedQ { .. }
            | Self::ContentUnbalancedBt { .. }
            | Self::ContentUnbalancedMarked { .. }
            | Self::ContentSyntax { .. }
            | Self::ContentResourceMissing { .. }
            | Self::InlineImageUnterminated { .. }
            | Self::XObjectCycle { .. }
            | Self::ContentPathUnpainted { .. }
            | Self::FontInvalid { .. }
            | Self::CMapInvalid { .. }
            | Self::UnmappedGlyph { .. } => ErrorKind::Input,
            Self::ObjectNumberOutOfRange { .. } | Self::WarningsTruncated => ErrorKind::Limit,
            Self::AesPaddingInvalid { .. } | Self::PermsMismatch => ErrorKind::Security,
        }
    }

    /// Where the problem is (ADR 0026 §1.1): the object, a file offset, a page index, or for
    /// content warnings the content's object and the offset in its decoded data (no offset
    /// when the content has no object: content interpreted on its own or a direct page).
    #[must_use]
    pub fn location(&self) -> Location {
        let page_index = match *self {
            Self::XrefStmIgnored { offset } => return Location::from(Position::FileOffset(offset)),
            Self::ContentUnknownOperator { object, offset }
            | Self::ContentBadOperands { object, offset }
            | Self::ContentUnbalancedQ { object, offset }
            | Self::ContentUnbalancedBt { object, offset }
            | Self::ContentUnbalancedMarked { object, offset }
            | Self::ContentSyntax { object, offset }
            | Self::ContentResourceMissing { object, offset, .. }
            | Self::InlineImageUnterminated { object, offset }
            | Self::ContentPathUnpainted { object, offset } => {
                return object.map_or(Location::NONE, |object| {
                    Location::from(Position::InStream { object, offset })
                });
            }
            Self::InvalidBox { page, .. } | Self::InvalidRotate { page, .. } => Some(page),
            Self::InvalidPageLabel { start } => u64::try_from(start).ok(),
            _ => None,
        };
        Location {
            page_index,
            ..Location::object(self.object())
        }
    }

    /// Deduplication key: (code, object, a position — byte offset, page index or label
    /// start — and an entry key), so two different problems on one object are both kept.
    const fn key(&self) -> WarningKey {
        let position = match *self {
            Self::InvalidPageLabel { start } => Some(start as i128),
            Self::InvalidBox { page, .. } | Self::InvalidRotate { page, .. } => Some(page as i128),
            Self::XrefStmIgnored { offset }
            | Self::ContentUnknownOperator { offset, .. }
            | Self::ContentBadOperands { offset, .. }
            | Self::ContentUnbalancedQ { offset, .. }
            | Self::ContentUnbalancedBt { offset, .. }
            | Self::ContentUnbalancedMarked { offset, .. }
            | Self::ContentSyntax { offset, .. }
            | Self::ContentResourceMissing { offset, .. }
            | Self::InlineImageUnterminated { offset, .. }
            | Self::ContentPathUnpainted { offset, .. } => Some(offset as i128),
            _ => None,
        };
        let detail = match *self {
            Self::InvalidBox { key, .. } | Self::InvalidDate { key, .. } | Self::InvalidEntry { key, .. } => Some(key),
            Self::ContentResourceMissing { category, .. } => Some(category),
            _ => None,
        };
        (self.code(), self.object(), position, detail)
    }
}

type WarningKey = (&'static str, Option<u32>, Option<i128>, Option<&'static str>);

/// The warnings a document has raised so far: deduplicated by (code, object) and capped at
/// [`Limits::max_warnings`] (soft limit). Raising is O(log n): resolving a stale reference
/// raises on every call, so dedup must not scan the list.
#[derive(Debug, Clone, Default)]
pub(crate) struct WarningLog {
    list: Vec<Warning>,
    seen: BTreeSet<WarningKey>,
    truncated: bool,
}

impl WarningLog {
    /// Records `warning`, or fails with [`Error::Strict`] in strict mode.
    pub(crate) fn raise(&mut self, warning: Warning, limits: &Limits, strict: bool) -> Result<()> {
        if strict {
            return Err(Error::Strict { warning });
        }
        if self.truncated || !self.seen.insert(warning.key()) {
            return Ok(());
        }
        if self.list.len() >= limits.max_warnings() {
            self.truncated = true;
            self.list.push(Warning::WarningsTruncated);
        } else {
            self.list.push(warning);
        }
        Ok(())
    }

    pub(crate) fn list(&self) -> &[Warning] {
        &self.list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One value of every variant. The exhaustive match makes a new variant a compile error
    /// here until it is added to the samples (and to [`Warning::CODES`]).
    #[expect(clippy::too_many_lines, reason = "one sample per variant, then the exhaustive match")]
    fn samples() -> Vec<Warning> {
        let all = vec![
            Warning::MissingHeader,
            Warning::XrefRepaired,
            Warning::XrefStmIgnored { offset: 1 },
            Warning::StreamLengthRecovered { object: 1 },
            Warning::StaleGeneration {
                object: 1,
                generation: 1,
            },
            Warning::PageTreeNodeSkipped { object: None },
            Warning::PageCountMismatch {
                object: None,
                declared: 1,
                walked: 0,
            },
            Warning::FlateTruncated { object: None },
            Warning::ObjStmIndexMismatch { object: 1 },
            Warning::ObjectNumberOutOfRange { object: 1 },
            Warning::AesPaddingInvalid { object: 1 },
            Warning::PermsMismatch,
            Warning::WarningsTruncated,
            Warning::InvalidBox {
                object: None,
                page: 0,
                key: "MediaBox",
            },
            Warning::InvalidRotate { object: None, page: 0 },
            Warning::NameTreeNodeSkipped { object: None },
            Warning::OutlineCycle { object: 1 },
            Warning::OutlineItemSkipped { object: None },
            Warning::InvalidDate {
                object: None,
                key: "ModDate",
            },
            Warning::InvalidPageLabel { start: 0 },
            Warning::InvalidEntry {
                object: None,
                key: "Info",
            },
            Warning::ContentUnknownOperator {
                object: None,
                offset: 0,
            },
            Warning::ContentBadOperands {
                object: None,
                offset: 0,
            },
            Warning::ContentUnbalancedQ {
                object: None,
                offset: 0,
            },
            Warning::ContentUnbalancedBt {
                object: None,
                offset: 0,
            },
            Warning::ContentUnbalancedMarked {
                object: None,
                offset: 0,
            },
            Warning::ContentSyntax {
                object: None,
                offset: 0,
            },
            Warning::ContentResourceMissing {
                object: None,
                offset: 0,
                category: "Font",
            },
            Warning::InlineImageUnterminated {
                object: None,
                offset: 0,
            },
            Warning::XObjectCycle { object: 1 },
            Warning::ContentPathUnpainted {
                object: None,
                offset: 0,
            },
            Warning::FontInvalid { object: None },
            Warning::CMapInvalid { object: None },
            Warning::UnmappedGlyph { object: None },
        ];
        for w in &all {
            match w {
                Warning::MissingHeader
                | Warning::XrefRepaired
                | Warning::XrefStmIgnored { .. }
                | Warning::StreamLengthRecovered { .. }
                | Warning::StaleGeneration { .. }
                | Warning::PageTreeNodeSkipped { .. }
                | Warning::PageCountMismatch { .. }
                | Warning::FlateTruncated { .. }
                | Warning::ObjStmIndexMismatch { .. }
                | Warning::ObjectNumberOutOfRange { .. }
                | Warning::AesPaddingInvalid { .. }
                | Warning::PermsMismatch
                | Warning::WarningsTruncated
                | Warning::InvalidBox { .. }
                | Warning::InvalidRotate { .. }
                | Warning::NameTreeNodeSkipped { .. }
                | Warning::OutlineCycle { .. }
                | Warning::OutlineItemSkipped { .. }
                | Warning::InvalidDate { .. }
                | Warning::InvalidPageLabel { .. }
                | Warning::InvalidEntry { .. }
                | Warning::ContentUnknownOperator { .. }
                | Warning::ContentBadOperands { .. }
                | Warning::ContentUnbalancedQ { .. }
                | Warning::ContentUnbalancedBt { .. }
                | Warning::ContentUnbalancedMarked { .. }
                | Warning::ContentSyntax { .. }
                | Warning::ContentResourceMissing { .. }
                | Warning::InlineImageUnterminated { .. }
                | Warning::XObjectCycle { .. }
                | Warning::ContentPathUnpainted { .. }
                | Warning::FontInvalid { .. }
                | Warning::CMapInvalid { .. }
                | Warning::UnmappedGlyph { .. } => {}
            }
        }
        all
    }

    #[test]
    fn codes_lists_the_code_of_every_variant_in_order() {
        let codes: Vec<_> = samples().iter().map(Warning::code).collect();
        assert_eq!(codes, Warning::CODES);
    }

    #[test]
    fn every_variant_has_the_kind_codes_json_lists() {
        crate::test_codes::assert_kinds_match(samples().iter().map(|w| (w.code(), w.kind().as_str())));
    }

    #[test]
    fn location_names_the_object_and_the_page_index() {
        let warning = Warning::InvalidRotate {
            object: Some(3),
            page: 2,
        };
        assert_eq!(
            warning.location(),
            Location {
                object: Some(3),
                page_index: Some(2),
                ..Location::NONE
            }
        );
    }

    #[test]
    fn location_of_a_content_warning_is_its_object_and_stream_offset() {
        let warning = Warning::ContentPathUnpainted {
            object: Some(5),
            offset: 40,
        };
        assert_eq!(
            warning.location(),
            Location {
                object: Some(5),
                stream_offset: Some(40),
                ..Location::NONE
            }
        );
    }

    #[test]
    fn location_of_a_content_warning_without_an_object_is_empty() {
        let warning = Warning::ContentSyntax {
            object: None,
            offset: 40,
        };
        assert_eq!(warning.location(), Location::NONE);
    }

    #[test]
    fn location_of_an_ignored_xref_stream_is_a_file_offset() {
        assert_eq!(Warning::XrefStmIgnored { offset: 9 }.location().offset, Some(9));
    }

    #[test]
    fn raise_deduplicates_by_code_and_object() {
        let mut log = WarningLog::default();
        let limits = Limits::default();
        for _ in 0..3 {
            log.raise(Warning::StreamLengthRecovered { object: 4 }, &limits, false)
                .unwrap();
        }
        assert_eq!(log.list(), [Warning::StreamLengthRecovered { object: 4 }]);
    }

    #[test]
    fn raise_keeps_two_problems_on_one_object_apart() {
        let mut log = WarningLog::default();
        let limits = Limits::default();
        for key in ["MediaBox", "CropBox"] {
            let warning = Warning::InvalidBox {
                object: Some(3),
                page: 0,
                key,
            };
            log.raise(warning, &limits, false).unwrap();
        }
        assert_eq!(log.list().len(), 2);
    }

    #[test]
    fn raise_keeps_at_most_max_warnings_then_one_truncation_marker() {
        let mut log = WarningLog::default();
        let limits = Limits::default().with_max_warnings(2);
        for object in 0..5 {
            log.raise(Warning::ObjStmIndexMismatch { object }, &limits, false)
                .unwrap();
        }
        assert_eq!(
            log.list(),
            [
                Warning::ObjStmIndexMismatch { object: 0 },
                Warning::ObjStmIndexMismatch { object: 1 },
                Warning::WarningsTruncated
            ]
        );
    }

    #[test]
    fn raise_in_strict_mode_is_an_error() {
        let mut log = WarningLog::default();
        assert_eq!(
            log.raise(Warning::XrefRepaired, &Limits::default(), true),
            Err(Error::Strict {
                warning: Warning::XrefRepaired
            })
        );
    }
}
