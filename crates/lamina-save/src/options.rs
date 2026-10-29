//! [`SaveOptions`] and [`SaveReport`] ([ADR 0007] §1, §5): the subset B3b's writers need.
//! B3c adds the mode (`Auto`), signature overrides and encryption removal.
//!
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use std::collections::BTreeMap;

use crate::legacy::XrefEol;

/// Which writer produced the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SaveMode {
    /// The base bytes unchanged, then an appended revision (§7.5.6).
    Incremental,
    /// A new file holding only what is reachable from the trailer.
    FullRewrite,
}

/// What an incremental save writes to the sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum IncrementalOutput {
    /// The whole file: the base bytes, then the appended revision.
    #[default]
    Whole,
    /// Only the appended revision, for a shell that appends to the original file in place.
    /// Its offsets assume it follows the base bytes ([`SaveReport::append_offset`]).
    AppendedOnly,
}

/// Object numbering in a full rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Renumber {
    /// Keep every object's number and generation, so `LaminaId`s stay valid (ADR 0005 §5).
    #[default]
    Keep,
    /// Number the written objects `1..=n` in their old order, generation 0; the report
    /// carries the old → new map ([`SaveReport::renumbered`]).
    Compact,
}

/// The cross-reference form of a full rewrite. An incremental save always appends the same
/// kind as the base's newest section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum XrefKind {
    /// Object streams plus an xref stream (PDF 1.5+; ch. 01 §14).
    #[default]
    Stream,
    /// Every object top-level and a classic `xref` table.
    Classic,
}

/// How to save. Every field has a default; set them with the `with_*` builders.
///
/// ```
/// use lamina_save::{Renumber, SaveOptions, XrefKind};
///
/// let options = SaveOptions::new().with_renumber(Renumber::Compact).with_xref(XrefKind::Classic);
/// assert_eq!((options.renumber(), options.xref()), (Renumber::Compact, XrefKind::Classic));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SaveOptions {
    output: IncrementalOutput,
    renumber: Renumber,
    xref: XrefKind,
    eol: XrefEol,
}

impl SaveOptions {
    /// The defaults: whole output, numbers kept, xref stream, `\r\n` entry EOLs.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What an incremental save writes.
    #[must_use]
    pub const fn with_output(mut self, output: IncrementalOutput) -> Self {
        self.output = output;
        self
    }

    /// Numbering of a full rewrite.
    #[must_use]
    pub const fn with_renumber(mut self, renumber: Renumber) -> Self {
        self.renumber = renumber;
        self
    }

    /// Xref form of a full rewrite.
    #[must_use]
    pub const fn with_xref(mut self, xref: XrefKind) -> Self {
        self.xref = xref;
        self
    }

    /// The two-byte EOL of classic xref entries (§7.5.4).
    #[must_use]
    pub const fn with_eol(mut self, eol: XrefEol) -> Self {
        self.eol = eol;
        self
    }

    /// See [`SaveOptions::with_output`].
    #[must_use]
    pub const fn output(&self) -> IncrementalOutput {
        self.output
    }

    /// See [`SaveOptions::with_renumber`].
    #[must_use]
    pub const fn renumber(&self) -> Renumber {
        self.renumber
    }

    /// See [`SaveOptions::with_xref`].
    #[must_use]
    pub const fn xref(&self) -> XrefKind {
        self.xref
    }

    /// See [`SaveOptions::with_eol`].
    #[must_use]
    pub const fn eol(&self) -> XrefEol {
        self.eol
    }
}

/// What a save wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveReport {
    pub(crate) bytes_written: u64,
    pub(crate) mode: SaveMode,
    pub(crate) renumbered: Option<BTreeMap<u32, u32>>,
    pub(crate) append_offset: Option<u64>,
}

impl SaveReport {
    /// Bytes written to the sink.
    #[must_use]
    pub const fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// The writer that ran.
    #[must_use]
    pub const fn mode(&self) -> SaveMode {
        self.mode
    }

    /// For [`Renumber::Compact`]: every written object's old number → new number.
    #[must_use]
    pub const fn renumbered(&self) -> Option<&BTreeMap<u32, u32>> {
        self.renumbered.as_ref()
    }

    /// For an incremental save: the offset in the complete file where the appended revision
    /// starts (the base's length).
    #[must_use]
    pub const fn append_offset(&self) -> Option<u64> {
        self.append_offset
    }
}
