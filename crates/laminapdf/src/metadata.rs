//! The document's metadata and the save plan: [`Document::metadata`], [`Document::plan`] and
//! [`Document::would_violate`] (E1 plan §2, §4, §7).

use laminapdf_save::{
    ConstraintKind, DocMdpPermission, MetadataUpdate, RewriteReason, SaveMode, SaveOptions, SavePlanner, SignatureInfo,
    Violation,
};

use crate::document::Document;
use crate::error::{Error, Result};
use crate::warning::Warning;
use crate::{DocumentMetadata, ErrorCode, Op};

/// How a save would go, before it happens ([ADR 0007] §1–§3): what restricts it, the mode it
/// resolves to or the reason it is refused, the signatures it would invalidate, what it does
/// with the save-time metadata, and the conformance claims the edits break. An interface shows
/// it before the user saves.
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlannedSave {
    /// The mode the save resolves to; `None` when it would be refused ([`PlannedSave::refusal`]).
    pub mode: Option<SaveMode>,
    /// Why the save would be refused (`would-invalidate-signatures`, `requires-rewrite`,
    /// `encrypted-base`); `None` when it would go ahead.
    pub refusal: Option<ErrorCode>,
    /// The summary for the save strip: `Free`, `AppendOnly`, `RequiresRewrite` or `Conflict`.
    pub constraint: ConstraintKind,
    /// Every signature, in the order found.
    pub signatures: Vec<SignatureInfo>,
    /// The certification's permission, when the document is certified.
    pub doc_mdp: Option<DocMdpPermission>,
    /// Why only a full rewrite is safe.
    pub rewrite_reasons: Vec<RewriteReason>,
    /// The pending changes a signature's restriction forbids, each naming its signature.
    pub violations: Vec<Violation>,
    /// The signatures the save would invalidate (only with the override).
    pub would_invalidate: Vec<SignatureInfo>,
    /// Whether the save writes an encrypted document as plaintext.
    pub removes_encryption: bool,
    /// What the save does with the save-time metadata.
    pub metadata: MetadataUpdate,
    /// Whether an edit to save rewrote the XMP packet whole (UTF-16, or marked read-only).
    pub packet_rewritten: bool,
    /// The PDF/A and PDF/UA claims the edited metadata breaks: warnings that never block.
    pub conformance_warnings: Vec<Warning>,
    /// The document has no changes: a save writes the file as it was.
    pub unchanged: bool,
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
    pub fn metadata(&self) -> Result<DocumentMetadata> {
        Ok(laminapdf_ops::read_metadata(self.ops())?)
    }

    /// How a save with `options` would go ([`PlannedSave`]), without saving. A save that would
    /// be refused is a plan with its [`refusal`](PlannedSave::refusal), not an error.
    ///
    /// # Errors
    /// Reading errors (the catalog, the form, a signature dictionary).
    pub fn plan(&self, options: &SaveOptions) -> Result<PlannedSave> {
        let planner = SavePlanner::new(self.ops())?;
        let constraint = planner.constraint();
        let (mode, refusal, would_invalidate, removes_encryption, metadata, packet_rewritten) =
            match planner.plan(options) {
                Ok(plan) => (
                    Some(plan.mode()),
                    None,
                    plan.would_invalidate().to_vec(),
                    plan.removes_encryption(),
                    plan.metadata_update().clone(),
                    plan.packet_rewritten(),
                ),
                Err(e) => (
                    None,
                    Some(Error::from(e).error_code()),
                    Vec::new(),
                    false,
                    MetadataUpdate::None,
                    false,
                ),
            };
        Ok(PlannedSave {
            mode,
            refusal,
            constraint: constraint.kind(),
            signatures: constraint.signatures.clone(),
            doc_mdp: constraint.doc_mdp.as_ref().map(|d| d.permission),
            rewrite_reasons: constraint.rewrite_reasons.clone(),
            violations: constraint.violations.clone(),
            would_invalidate,
            removes_encryption,
            metadata,
            packet_rewritten,
            conformance_warnings: planner
                .conformance_warnings()
                .iter()
                .cloned()
                .map(Warning::Conformance)
                .collect(),
            unchanged: constraint.unchanged,
        })
    }

    /// The signature restrictions `op` would break if it were applied and saved, for an
    /// interface to warn before the edit ([ADR 0007] §2); the save planner stays the authority
    /// at save time.
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    ///
    /// # Errors
    /// As [`Document::plan`].
    pub fn would_violate(&self, op: &Op) -> Result<Vec<Violation>> {
        Ok(SavePlanner::new(self.ops())?.would_violate(op))
    }
}
