//! Stable, enumerated warning codes (ADR 0003 §2) and the per-document warning log.

use std::collections::BTreeSet;

use lamina_syntax::Limits;

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
    /// `warnings-truncated`: more than [`Limits::max_warnings`] warnings were raised; the rest
    /// were dropped.
    WarningsTruncated,
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
        "warnings-truncated",
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
            Self::WarningsTruncated => "warnings-truncated",
        }
    }

    /// The object number the warning is about, if any.
    #[must_use]
    pub const fn object(&self) -> Option<u32> {
        match *self {
            Self::StreamLengthRecovered { object }
            | Self::StaleGeneration { object, .. }
            | Self::ObjStmIndexMismatch { object }
            | Self::ObjectNumberOutOfRange { object } => Some(object),
            Self::PageTreeNodeSkipped { object }
            | Self::PageCountMismatch { object, .. }
            | Self::FlateTruncated { object } => object,
            Self::MissingHeader | Self::XrefRepaired | Self::XrefStmIgnored { .. } | Self::WarningsTruncated => None,
        }
    }

    /// Deduplication key: (code, object, offset).
    const fn key(&self) -> WarningKey {
        let offset = match *self {
            Self::XrefStmIgnored { offset } => Some(offset),
            _ => None,
        };
        (self.code(), self.object(), offset)
    }
}

type WarningKey = (&'static str, Option<u32>, Option<u64>);

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
            Warning::WarningsTruncated,
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
                | Warning::WarningsTruncated => {}
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
