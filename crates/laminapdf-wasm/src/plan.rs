//! The save plan as JavaScript records (`laminapdf::PlannedSave`, `Violation`): what
//! `WasmDocument.plan` and `wouldViolate` return, so an interface can explain a save, and warn
//! before an edit, before either happens.

use laminapdf::{
    ConstraintKind, MetadataUpdate, ModificationClass, PlannedSave, Restriction, RewriteReason, SaveMode,
    SignatureInfo, SignatureKind, Violation,
};

use serde::Serialize;
use tsify::Tsify;

use crate::dto::SaveModeJs;

/// A signature the planner found.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SignatureJs {
    /// `approval`, `certification`, `docTimeStamp` or `usageRights`.
    pub kind: &'static str,
    /// The signature field's ID, when a field holds it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub field: Option<String>,
    /// The 1-based revision the signature closes, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub revision: Option<u32>,
}

impl From<&SignatureInfo> for SignatureJs {
    fn from(s: &SignatureInfo) -> Self {
        Self {
            kind: match s.kind {
                SignatureKind::Approval => "approval",
                SignatureKind::Certification => "certification",
                SignatureKind::DocTimeStamp => "docTimeStamp",
                _ => "usageRights",
            },
            field: s.field.as_ref().map(ToString::to_string),
            revision: s.revision.and_then(|r| u32::try_from(r).ok()),
        }
    }
}

/// A change a signature's restriction forbids.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ViolationJs {
    /// The kind of change.
    pub class: ModificationClass,
    /// `docMdp`, `fieldMdp`, `usageRights`, `signedBytes` or `signedWidget`.
    pub restriction: &'static str,
    /// The certification's `/P` (1, 2 or 3), for a `docMdp` restriction.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub permission: Option<u8>,
    /// The signature that carries the restriction.
    pub signature: SignatureJs,
    /// The form field whose widget a page edit deletes or reshows (its ID), for a
    /// `signedWidget` restriction or a `fieldMdp` one that a page edit breaks.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub field: Option<String>,
}

impl From<&Violation> for ViolationJs {
    fn from(v: &Violation) -> Self {
        let (restriction, permission) = match v.restriction {
            Restriction::DocMdp(p) => ("docMdp", Some(p.p())),
            Restriction::FieldMdp => ("fieldMdp", None),
            Restriction::UsageRights => ("usageRights", None),
            Restriction::SignedWidget => ("signedWidget", None),
            _ => ("signedBytes", None),
        };
        Self {
            class: v.class,
            restriction,
            permission,
            signature: SignatureJs::from(&v.signature),
            field: v.field.as_ref().map(ToString::to_string),
        }
    }
}

/// How a save would go (`laminapdf::PlannedSave`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlannedSaveJs {
    /// The mode the save resolves to; absent when it would be refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub mode: Option<SaveModeJs>,
    /// The code the save would be refused with.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub refusal: Option<&'static str>,
    /// `free`, `appendOnly`, `requiresRewrite` or `conflict`.
    pub constraint: &'static str,
    /// Every signature, in the order found.
    pub signatures: Vec<SignatureJs>,
    /// The certification's `/P` (1, 2 or 3), when the document is certified.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub doc_mdp: Option<u8>,
    /// Why only a full rewrite is safe: `repairedBase`, `encryptedBase`, `noBase`,
    /// `opRequiresRewrite`.
    pub rewrite_reasons: Vec<&'static str>,
    /// The pending changes a signature's restriction forbids.
    pub violations: Vec<ViolationJs>,
    /// The signatures the save would invalidate (only with `invalidateSignatures`).
    pub would_invalidate: Vec<SignatureJs>,
    /// Whether the save writes an encrypted document as plaintext.
    pub removes_encryption: bool,
    /// What the save does with the save-time metadata: `none`, `stamp` or `skipped`.
    pub metadata: &'static str,
    /// Whether an edit to save rewrote the XMP packet whole.
    pub packet_rewritten: bool,
    /// The PDF/A and PDF/UA claims the edited metadata breaks (warnings that never block).
    #[tsify(type = "Diagnostic[]")]
    pub conformance_warnings: Vec<laminapdf::Diagnostic>,
    /// The document has no changes.
    pub unchanged: bool,
}

impl From<&PlannedSave> for PlannedSaveJs {
    fn from(p: &PlannedSave) -> Self {
        Self {
            mode: p.mode.map(|m| {
                if m == SaveMode::Incremental {
                    SaveModeJs::Incremental
                } else {
                    SaveModeJs::FullRewrite
                }
            }),
            refusal: p.refusal.map(laminapdf::ErrorCode::as_str),
            constraint: match p.constraint {
                ConstraintKind::Free => "free",
                ConstraintKind::AppendOnly => "appendOnly",
                ConstraintKind::RequiresRewrite => "requiresRewrite",
                _ => "conflict",
            },
            signatures: p.signatures.iter().map(SignatureJs::from).collect(),
            doc_mdp: p.doc_mdp.map(laminapdf::DocMdpPermission::p),
            rewrite_reasons: p
                .rewrite_reasons
                .iter()
                .map(|r| match r {
                    RewriteReason::RepairedBase => "repairedBase",
                    RewriteReason::EncryptedBase => "encryptedBase",
                    RewriteReason::NoBase => "noBase",
                    _ => "opRequiresRewrite",
                })
                .collect(),
            violations: p.violations.iter().map(ViolationJs::from).collect(),
            would_invalidate: p.would_invalidate.iter().map(SignatureJs::from).collect(),
            removes_encryption: p.removes_encryption,
            metadata: match p.metadata {
                MetadataUpdate::None => "none",
                MetadataUpdate::Stamp => "stamp",
                _ => "skipped",
            },
            packet_rewritten: p.packet_rewritten,
            conformance_warnings: p
                .conformance_warnings
                .iter()
                .map(laminapdf::Warning::diagnostic)
                .collect(),
            unchanged: p.unchanged,
        }
    }
}
