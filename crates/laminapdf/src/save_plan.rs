//! Planning a save before it happens: [`Document::plan`] gives the [`PlannedSave`] record, and
//! [`Document::would_violate`] checks one edit against the signatures (E1 plan §4, §7).

use laminapdf_save::{
    ConstraintKind, DocMdpPermission, MetadataUpdate, RewriteReason, SaveMode, SaveOptions, SavePlanner, SignatureInfo,
    Violation,
};

use crate::document::Document;
use crate::error::{Error, Result};
use crate::warning::Warning;
use crate::{ErrorCode, Op};

/// How a save would go, before it happens ([ADR 0007] §1–§3): what restricts it, the mode it
/// resolves to or the reason it is refused, the signatures it would invalidate, what it does
/// with the save-time metadata, and the conformance claims the edits break. An interface shows
/// it before the user saves.
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
///
/// Added in 0.2.0.
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

/// What the planner resolved for the options: the parts of [`PlannedSave`] a refused save
/// leaves at their defaults.
#[derive(Default)]
struct Resolved {
    mode: Option<SaveMode>,
    refusal: Option<ErrorCode>,
    would_invalidate: Vec<SignatureInfo>,
    removes_encryption: bool,
    metadata: MetadataUpdate,
    packet_rewritten: bool,
}

impl Document {
    /// How a save with `options` would go ([`PlannedSave`]), without saving. A save that would
    /// be refused is a plan with its [`refusal`](PlannedSave::refusal), not an error.
    ///
    /// # Errors
    /// Reading errors (the catalog, the form, a signature dictionary).
    ///
    /// Added in 0.2.0.
    pub fn plan(&self, options: &SaveOptions) -> Result<PlannedSave> {
        let planner = SavePlanner::new(self.ops())?;
        let constraint = planner.constraint();
        let resolved = match planner.plan(options) {
            Ok(plan) => Resolved {
                mode: Some(plan.mode()),
                refusal: None,
                would_invalidate: plan.would_invalidate().to_vec(),
                removes_encryption: plan.removes_encryption(),
                metadata: plan.metadata_update().clone(),
                packet_rewritten: plan.packet_rewritten(),
            },
            Err(e) => Resolved {
                refusal: Some(Error::from(e).error_code()),
                ..Resolved::default()
            },
        };
        Ok(PlannedSave {
            mode: resolved.mode,
            refusal: resolved.refusal,
            constraint: constraint.kind(),
            signatures: constraint.signatures.clone(),
            doc_mdp: constraint.doc_mdp.as_ref().map(|d| d.permission),
            rewrite_reasons: constraint.rewrite_reasons.clone(),
            violations: constraint.violations.clone(),
            would_invalidate: resolved.would_invalidate,
            removes_encryption: resolved.removes_encryption,
            metadata: resolved.metadata,
            packet_rewritten: resolved.packet_rewritten,
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
    ///
    /// Added in 0.2.0.
    pub fn would_violate(&self, op: &Op) -> Result<Vec<Violation>> {
        Ok(SavePlanner::new(self.ops())?.would_violate(op))
    }
}
