//! [`SaveOptions`] and [`SaveReport`] ([ADR 0007] §1, §3, §5): the mode, the explicit
//! overrides (invalidating signatures, removing encryption) and the writers' settings.
//!
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use std::collections::BTreeMap;

use crate::constraint::SignatureInfo;
use crate::legacy::XrefEol;

/// How to save: the mode requested in [`SaveOptions`], and the one a plan resolves to
/// ([`SavePlan::mode`](crate::SavePlan::mode), [`SaveReport::mode`]: never `Auto`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SaveMode {
    /// The base bytes unchanged, then an appended revision (§7.5.6).
    Incremental,
    /// A new file holding only what is reachable from the trailer.
    FullRewrite,
    /// Let the planner choose ([ADR 0007] §3): incremental unless only a full rewrite is safe.
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    #[default]
    Auto,
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
/// The two overrides are off by default and never set implicitly: invalidating signatures
/// ([`SaveOptions::with_invalidate_signatures`]) and removing encryption
/// ([`SaveOptions::with_remove_encryption`]) each need the caller (usually the user, asked by
/// the UI) to opt in.
///
/// ```
/// use laminapdf_save::{Renumber, SaveMode, SaveOptions, XrefKind};
///
/// let options = SaveOptions::new().with_renumber(Renumber::Compact).with_xref(XrefKind::Classic);
/// assert_eq!((options.renumber(), options.xref()), (Renumber::Compact, XrefKind::Classic));
/// assert_eq!((options.mode(), options.invalidate_signatures()), (SaveMode::Auto, false));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SaveOptions {
    mode: SaveMode,
    invalidate_signatures: bool,
    remove_encryption: bool,
    output: IncrementalOutput,
    renumber: Renumber,
    xref: XrefKind,
    eol: XrefEol,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveOptions {
    /// The defaults: `Auto`, no overrides, whole output, numbers kept, xref stream, `\r\n`
    /// entry EOLs.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mode: SaveMode::Auto,
            invalidate_signatures: false,
            remove_encryption: false,
            output: IncrementalOutput::Whole,
            renumber: Renumber::Keep,
            xref: XrefKind::Stream,
            eol: XrefEol::CrLf,
        }
    }

    /// The mode to save in ([ADR 0007] §3; default [`SaveMode::Auto`]).
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    #[must_use]
    pub const fn with_mode(mut self, mode: SaveMode) -> Self {
        self.mode = mode;
        self
    }

    /// The explicit override ([ADR 0007] §3): save even though signatures break. The values
    /// (`/V`) of the signatures the save invalidates and their `/Perms` entries are removed, so
    /// the output carries no signature a viewer would show as broken; the fields stay,
    /// unsigned, and [`SaveReport::invalidated`] lists them. Ask the user first, naming
    /// [`SavePlan::would_invalidate`](crate::SavePlan::would_invalidate).
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    #[must_use]
    pub const fn with_invalidate_signatures(mut self, invalidate: bool) -> Self {
        self.invalidate_signatures = invalidate;
        self
    }

    /// Save an encrypted document as a plaintext full rewrite: every string and stream
    /// decrypted, no `/Encrypt` (`/ID` kept). Without it an encrypted document can't be saved
    /// at all ([`SaveError::EncryptedBase`](crate::SaveError::EncryptedBase)), since there is no
    /// encryption on write yet (full-editor E11). Anyone who could open the document can
    /// already read it decrypted; whether the editor asks for the owner password first is
    /// E11's decision. No effect on an unencrypted document.
    #[must_use]
    pub const fn with_remove_encryption(mut self, remove: bool) -> Self {
        self.remove_encryption = remove;
        self
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

    /// See [`SaveOptions::with_mode`].
    #[must_use]
    pub const fn mode(&self) -> SaveMode {
        self.mode
    }

    /// See [`SaveOptions::with_invalidate_signatures`].
    #[must_use]
    pub const fn invalidate_signatures(&self) -> bool {
        self.invalidate_signatures
    }

    /// See [`SaveOptions::with_remove_encryption`].
    #[must_use]
    pub const fn remove_encryption(&self) -> bool {
        self.remove_encryption
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
    pub(crate) invalidated: Vec<SignatureInfo>,
    pub(crate) encryption_removed: bool,
}

impl SaveReport {
    /// Bytes written to the sink.
    #[must_use]
    pub const fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// The writer that ran ([`SaveMode::Incremental`] or [`SaveMode::FullRewrite`]).
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

    /// The signatures the save invalidated under the explicit override (their values and
    /// `/Perms` entries removed); empty otherwise.
    #[must_use]
    pub fn invalidated(&self) -> &[SignatureInfo] {
        &self.invalidated
    }

    /// Whether the save removed the document's encryption
    /// ([`SaveOptions::with_remove_encryption`]).
    #[must_use]
    pub const fn encryption_removed(&self) -> bool {
        self.encryption_removed
    }
}
