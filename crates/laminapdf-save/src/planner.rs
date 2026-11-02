//! [`SavePlanner`] and [`SavePlan`] ([ADR 0007] §1–§3): what restricts a save, and how it
//! goes.
//!
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use laminapdf_ops::{ModificationClass, Op};

use crate::constraint::{Restriction, RewriteReason, SaveConstraint, SignatureInfo, Violation};
use crate::detect::{self, Patches, Signatures};
use crate::error::{Result, SaveError};
use crate::options::{SaveMode, SaveOptions};
use crate::resolve::resolve;

/// Inspects a document for everything that restricts saving it ([ADR 0007] §2): its
/// signatures and their `DocMDP`, `FieldMDP` and UR3 restrictions, why only a full rewrite would
/// be safe, and which pending changes the restrictions forbid. Planning is separate from
/// writing, so the UI can show every restriction and every signature at risk first;
/// [`Save::save`](crate::Save::save) plans again itself.
///
/// ```
/// use laminapdf_ops::{Document, InfoPatch, Origin, SetInfo};
/// use laminapdf_save::{ConstraintKind, SaveMode, SaveOptions, SavePlanner};
/// # use laminapdf_cos::{Dict, Object};
/// # use laminapdf_save::Writer;
/// # let mut w = Writer::new(1, 7);
/// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
/// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
///
/// let mut doc = Document::open(bytes, Default::default())?;
/// doc.apply_all(Origin::Local, [SetInfo::patch(InfoPatch::new().title(Some("Report"))).into()])?;
///
/// let planner = SavePlanner::new(&doc)?;
/// assert_eq!(planner.constraint().kind(), ConstraintKind::Free);
/// let plan = planner.plan(&SaveOptions::new())?;
/// assert_eq!((plan.mode(), plan.would_invalidate().len()), (SaveMode::Incremental, 0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
#[derive(Debug, Clone)]
pub struct SavePlanner {
    signatures: Signatures,
    constraint: SaveConstraint,
}

impl SavePlanner {
    /// Inspects `doc` as it is now (base plus overlay, and the classes of its pending
    /// changes, [`laminapdf_ops::Document::pending_changes`]).
    ///
    /// # Errors
    /// [`SaveError::Ops`](crate::SaveError::Ops) with
    /// [`NestedTransaction`](laminapdf_ops::Error::NestedTransaction) while a transaction is open
    /// (its changes are in the overlay but not yet in the pending classes);
    /// [`SaveError::Cos`](crate::SaveError::Cos) when the catalog, the form or a signature
    /// dictionary can't be read: a signature that can't be read is not assumed absent.
    pub fn new(doc: &laminapdf_ops::Document) -> Result<Self> {
        if doc.in_transaction() {
            return Err(SaveError::Ops(laminapdf_ops::Error::NestedTransaction));
        }
        let cos = doc.cos();
        let signatures = detect::signatures(cos)?;
        let mut rewrite_reasons = Vec::new();
        if cos.source().is_empty() {
            rewrite_reasons.push(RewriteReason::NoBase);
        }
        if cos.was_repaired() {
            rewrite_reasons.push(RewriteReason::RepairedBase);
        }
        if cos.is_encrypted() {
            rewrite_reasons.push(RewriteReason::EncryptedBase);
        }
        let pending = doc.pending_changes();
        let dirty = cos.is_dirty();
        if dirty && pending.requires_rewrite() {
            rewrite_reasons.push(RewriteReason::OpRequiresRewrite);
        }
        let violations = if dirty {
            pending
                .classes()
                .iter()
                .flat_map(|&class| signatures.violations(class))
                .collect()
        } else {
            Vec::new()
        };
        let constraint = SaveConstraint {
            signatures: signatures.infos(),
            doc_mdp: signatures.doc_mdp(),
            rewrite_reasons,
            violations,
            signatures_unknown: signatures.unknown,
            unchanged: !dirty,
        };
        Ok(Self { signatures, constraint })
    }

    /// Everything that restricts saving the document.
    #[must_use]
    pub const fn constraint(&self) -> &SaveConstraint {
        &self.constraint
    }

    /// How a save with `options` would go: the mode resolved per [ADR 0007] §3 and the
    /// signatures it would invalidate. An incremental update that removes any signature's
    /// value changes the document in a way no certification, usage-rights signature or field
    /// lock permits, so those signatures are invalidated (and removed) too.
    ///
    /// # Errors
    /// [`SaveError::EncryptedBase`](crate::SaveError::EncryptedBase) for an encrypted document
    /// without `remove_encryption`;
    /// [`SaveError::WouldInvalidateSignatures`](crate::SaveError::WouldInvalidateSignatures)
    /// and [`SaveError::RequiresRewrite`](crate::SaveError::RequiresRewrite) as the table says.
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    pub fn plan(&self, options: &SaveOptions) -> Result<SavePlan> {
        let mut plan = SavePlan::resolve(self.constraint.clone(), options)?;
        if plan.mode == SaveMode::Incremental && !plan.would_invalidate.is_empty() {
            for signature in self.signatures.restricting() {
                if !plan.would_invalidate.contains(signature) {
                    plan.would_invalidate.push(signature.clone());
                }
            }
        }
        Ok(plan)
    }

    /// The changes that remove `plan`'s invalidated signatures from `doc` (the document this
    /// planner inspected).
    pub(crate) fn patches(&self, doc: &laminapdf_cos::Document, plan: &SavePlan) -> Result<Patches> {
        if plan.would_invalidate.is_empty() {
            return Ok(Patches::default());
        }
        self.signatures.patches(doc, &plan.would_invalidate)
    }

    /// The restrictions `op` would break if it were applied and saved, for the editor to block
    /// or warn before the edit ([ADR 0007] §2): those of its class, and, for an op that needs a
    /// full rewrite ([`Op::requires_rewrite`]), every signature ([`Restriction::SignedBytes`]).
    /// The planner stays the authority at save time.
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    #[must_use]
    pub fn would_violate(&self, op: &Op) -> Vec<Violation> {
        let mut out = self.would_violate_class(op.class());
        if op.requires_rewrite() {
            out.extend(
                self.constraint
                    .signatures
                    .iter()
                    .map(|s| Violation::new(op.class(), Restriction::SignedBytes, s.clone())),
            );
        }
        out
    }

    /// [`SavePlanner::would_violate`] for any change of `class`.
    #[must_use]
    pub fn would_violate_class(&self, class: ModificationClass) -> Vec<Violation> {
        self.signatures.violations(class).collect()
    }
}

/// A resolved save: the constraint, the mode and the signatures it would invalidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavePlan {
    constraint: SaveConstraint,
    mode: SaveMode,
    would_invalidate: Vec<SignatureInfo>,
    removes_encryption: bool,
}

impl SavePlan {
    /// Resolves `constraint` with `options` ([ADR 0007] §3): the table, for any constraint.
    /// (The planner's [`SavePlanner::plan`] also adds the restricting signatures an
    /// overridden incremental update breaks.)
    ///
    /// ```
    /// use laminapdf_save::{SaveError, SaveMode, SaveOptions, SavePlan, SaveConstraint, SignatureInfo, SignatureKind};
    ///
    /// let mut signed = SaveConstraint::default();
    /// signed.signatures.push(SignatureInfo::new(SignatureKind::Approval));
    /// let full = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    /// assert_eq!(SavePlan::resolve(signed.clone(), &full).map_err(|e| e.code()), Err("would-invalidate-signatures"));
    /// let plan = SavePlan::resolve(signed, &full.with_invalidate_signatures(true))?;
    /// assert_eq!((plan.mode(), plan.would_invalidate().len()), (SaveMode::FullRewrite, 1));
    /// # Ok::<(), SaveError>(())
    /// ```
    ///
    /// # Errors
    /// As [`SavePlanner::plan`].
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    pub fn resolve(constraint: SaveConstraint, options: &SaveOptions) -> Result<Self> {
        let (mode, would_invalidate) = resolve(&constraint, *options)?;
        let removes_encryption =
            mode == SaveMode::FullRewrite && constraint.rewrite_reasons.contains(&RewriteReason::EncryptedBase);
        Ok(Self {
            constraint,
            mode,
            would_invalidate,
            removes_encryption,
        })
    }

    /// Everything that restricts the save.
    #[must_use]
    pub const fn constraint(&self) -> &SaveConstraint {
        &self.constraint
    }

    /// The resolved mode: [`SaveMode::Incremental`] or [`SaveMode::FullRewrite`].
    #[must_use]
    pub const fn mode(&self) -> SaveMode {
        self.mode
    }

    /// The signatures this save would invalidate: empty unless the options override.
    #[must_use]
    pub fn would_invalidate(&self) -> &[SignatureInfo] {
        &self.would_invalidate
    }

    /// Whether the save writes the encrypted document as plaintext.
    #[must_use]
    pub const fn removes_encryption(&self) -> bool {
        self.removes_encryption
    }
}
