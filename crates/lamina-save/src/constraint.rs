//! [`SaveConstraint`] ([ADR 0007] §2): every condition that restricts a save, not one of them.
//!
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use lamina_cos::ObjRef;
use lamina_model::FieldId;
use lamina_ops::ModificationClass;

/// What a signature is (ISO 32000-2 §12.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SignatureKind {
    /// An approval signature: a signed field with no `DocMDP` or UR3 transform.
    Approval,
    /// A certification signature (§12.8.2.2): referenced from `/Perms /DocMDP`, or with a
    /// `/DocMDP` transform.
    Certification,
    /// A document timestamp (§12.8.5, `/Type /DocTimeStamp`).
    DocTimeStamp,
    /// A usage-rights signature (§12.8.2.3): referenced from `/Perms /UR3` (or `/UR`), or with
    /// a `/UR3` transform.
    UsageRights,
}

/// A signature found in the document: one per signature dictionary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SignatureInfo {
    /// The signature field whose `/V` holds it; `None` for a signature dictionary found only by
    /// scanning a repaired file, or referenced only from `/Perms`.
    pub field: Option<FieldId>,
    /// What it is.
    pub kind: SignatureKind,
    /// The signature dictionary, when it is an indirect object.
    pub value: Option<ObjRef>,
    /// The 1-based revision the signature closes: the number of `%%EOF` markers in the bytes
    /// its `/ByteRange` covers. `None` when the range is missing or malformed, or the base is
    /// not in memory.
    pub revision: Option<usize>,
    /// Where the bytes the signature covers end (the end of its `/ByteRange`), when it has one.
    pub covers: Option<u64>,
}

impl SignatureInfo {
    /// A signature of `kind` with nothing else known.
    #[must_use]
    pub const fn new(kind: SignatureKind) -> Self {
        Self {
            field: None,
            kind,
            value: None,
            revision: None,
            covers: None,
        }
    }

    /// With the field that holds it.
    #[must_use]
    pub fn with_field(mut self, field: FieldId) -> Self {
        self.field = Some(field);
        self
    }

    /// With its signature dictionary.
    #[must_use]
    pub const fn with_value(mut self, value: ObjRef) -> Self {
        self.value = Some(value);
        self
    }
}

/// The `DocMDP` access permissions of a certification signature (§12.8.2.2, `/P` in the
/// transform parameters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum DocMdpPermission {
    /// `/P 1`: no changes at all.
    NoChanges,
    /// `/P 2` (the default): filling in forms, instantiating page templates and signing.
    FormFill,
    /// `/P 3`: as 2, plus annotations (creation, deletion and modification).
    Annotate,
}

impl DocMdpPermission {
    /// The permission for a `/P` value (§12.8.2.2, Table 257); `None` for anything but 1, 2
    /// and 3. An absent `/P` means 2; the planner takes an invalid one as 1, the strictest.
    #[must_use]
    pub const fn from_p(p: i64) -> Option<Self> {
        match p {
            1 => Some(Self::NoChanges),
            2 => Some(Self::FormFill),
            3 => Some(Self::Annotate),
            _ => None,
        }
    }

    /// The `/P` value.
    #[must_use]
    pub const fn p(self) -> u8 {
        match self {
            Self::NoChanges => 1,
            Self::FormFill => 2,
            Self::Annotate => 3,
        }
    }

    /// Whether a change of `class` keeps a certification with this permission valid.
    #[must_use]
    pub const fn allows(self, class: ModificationClass) -> bool {
        match self {
            Self::NoChanges => false,
            Self::FormFill => matches!(class, ModificationClass::FormFill | ModificationClass::Signature),
            Self::Annotate => matches!(
                class,
                ModificationClass::FormFill | ModificationClass::Signature | ModificationClass::Annotation
            ),
        }
    }
}

/// The document's certification (`DocMDP`) restriction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DocMdp {
    /// What it permits.
    pub permission: DocMdpPermission,
    /// The permission couldn't be read (no readable `DocMDP` transform, or a `/P` that isn't 1,
    /// 2 or 3), so the strictest, [`DocMdpPermission::NoChanges`], is assumed.
    pub assumed: bool,
    /// The certification signature.
    pub signature: SignatureInfo,
}

/// Why only a full rewrite is a safe save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RewriteReason {
    /// The xref was rebuilt by scanning (ADR 0003): a revision appended to it would point into
    /// a file whose own cross-reference data is unusable.
    RepairedBase,
    /// The base is encrypted and encryption on write is missing (full-editor E11): it can only
    /// be saved by removing the encryption ([`SaveOptions::with_remove_encryption`]).
    ///
    /// [`SaveOptions::with_remove_encryption`]: crate::SaveOptions::with_remove_encryption
    EncryptedBase,
    /// There are no base bytes to append to (a document built from scratch): only a full
    /// rewrite can write it.
    NoBase,
    /// A pending op needs a full rewrite ([`lamina_ops::Op::requires_rewrite`]: redaction,
    /// sanitising), so that no earlier revision keeps what it removed.
    OpRequiresRewrite,
}

/// Which restriction a pending change breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Restriction {
    /// The certification's `DocMDP` permission (§12.8.2.2).
    DocMdp(DocMdpPermission),
    /// A `FieldMDP` lock (§12.8.2.4, a field's `/Lock` or a `/FieldMDP` transform): it covers
    /// fields, and op classes don't name fields, so any form change counts.
    FieldMdp,
    /// A usage-rights signature (§12.8.2.3): a change outside the rights it grants.
    UsageRights,
    /// The signed bytes themselves: a change that needs a full rewrite moves them, which
    /// breaks every signature ([`SavePlanner::would_violate`](crate::SavePlanner::would_violate)).
    SignedBytes,
}

/// A pending change that a signature's restriction forbids: saving it, even incrementally,
/// invalidates that signature.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Violation {
    /// The kind of change.
    pub class: ModificationClass,
    /// The restriction it breaks.
    pub restriction: Restriction,
    /// The signature that carries the restriction.
    pub signature: SignatureInfo,
}

impl Violation {
    /// A change of `class` that breaks `restriction`, carried by `signature`.
    #[must_use]
    pub const fn new(class: ModificationClass, restriction: Restriction, signature: SignatureInfo) -> Self {
        Self {
            class,
            restriction,
            signature,
        }
    }
}

/// What [`SaveConstraint::kind`] says, for the UI's banner ([ADR 0007] §2).
///
/// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConstraintKind {
    /// No signatures, no rewrite reasons: any mode.
    Free,
    /// Signatures, and nothing that conflicts with them: incremental saves only.
    AppendOnly,
    /// Rewrite reasons and no signatures: full rewrites only.
    RequiresRewrite,
    /// Signatures plus rewrite reasons or violations: no save keeps every signature.
    Conflict,
}

/// Everything that restricts a save ([ADR 0007] §2): the conditions overlap, so all of them
/// are reported. [`SavePlanner`](crate::SavePlanner) fills it from the base and the overlay.
///
/// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SaveConstraint {
    /// Every signature, in the order found (fields first, then `/Perms`, then a repaired
    /// file's scan).
    pub signatures: Vec<SignatureInfo>,
    /// The certification signature's `DocMDP` restriction, if the document is certified.
    pub doc_mdp: Option<DocMdp>,
    /// Why only a full rewrite is safe.
    pub rewrite_reasons: Vec<RewriteReason>,
    /// Pending changes a signature's restriction forbids.
    pub violations: Vec<Violation>,
    /// Part of the form couldn't be followed (a branch too deep, or not an array or
    /// dictionary where one belongs): there may be signatures nobody saw, so the document
    /// counts as signed.
    pub signatures_unknown: bool,
    /// The overlay is empty: an incremental save writes the base bytes unchanged, which
    /// invalidates nothing and keeps any encryption.
    pub unchanged: bool,
}

impl SaveConstraint {
    /// The summary for the UI's banner. Unknown signatures
    /// ([`SaveConstraint::signatures_unknown`]) count as signatures.
    ///
    /// ```
    /// use lamina_save::{ConstraintKind, RewriteReason, SaveConstraint, SignatureInfo, SignatureKind};
    ///
    /// let mut constraint = SaveConstraint::default();
    /// assert_eq!(constraint.kind(), ConstraintKind::Free);
    /// constraint.signatures.push(SignatureInfo::new(SignatureKind::Approval));
    /// assert_eq!(constraint.kind(), ConstraintKind::AppendOnly);
    /// constraint.rewrite_reasons.push(RewriteReason::RepairedBase);
    /// assert_eq!(constraint.kind(), ConstraintKind::Conflict);
    /// ```
    #[must_use]
    pub fn kind(&self) -> ConstraintKind {
        let signed = !self.signatures.is_empty() || self.signatures_unknown;
        let rewrite = !self.rewrite_reasons.is_empty();
        match (signed, rewrite || !self.violations.is_empty()) {
            (true, true) => ConstraintKind::Conflict,
            (true, false) => ConstraintKind::AppendOnly,
            (false, _) if rewrite => ConstraintKind::RequiresRewrite,
            (false, _) => ConstraintKind::Free,
        }
    }
}
