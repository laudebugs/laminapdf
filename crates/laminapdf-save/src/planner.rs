//! [`SavePlanner`] and [`SavePlan`] ([ADR 0007] §1–§3): what restricts a save, and how it
//! goes.
//!
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use laminapdf_ops::{ConformanceWarning, ModificationClass, Op};

use crate::constraint::{Restriction, RewriteReason, SaveConstraint, SignatureInfo, Violation};
use crate::detect::{self, Patches, Signatures};
use crate::error::{Result, SaveError};
use crate::options::{SaveMode, SaveOptions};
use crate::resolve::resolve;
#[cfg(feature = "page-checks")]
use crate::widgets::WidgetIndex;

/// Inspects a document for everything that restricts saving it ([ADR 0007] §2): its
/// signatures and their `DocMDP`, `FieldMDP` and UR3 restrictions, why only a full rewrite would
/// be safe, and which pending changes the restrictions forbid. Planning is separate from
/// writing, so the UI can show every restriction and every signature at risk first;
/// [`Save::save`](crate::Save::save) plans again itself.
///
/// ```
/// use laminapdf_ops::{Document, MetadataPatch, Origin, SetMetadata};
/// use laminapdf_save::{ConstraintKind, SaveMode, SaveOptions, SavePlanner};
/// # use laminapdf_cos::{Dict, Object};
/// # use laminapdf_save::test_support::Writer;
/// # let mut w = Writer::new(1, 7);
/// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
/// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
///
/// let mut doc = Document::open(bytes, Default::default())?;
/// doc.apply_all(Origin::Local, [SetMetadata::new(MetadataPatch::new().title(Some("Report")), None).into()])?;
///
/// let planner = SavePlanner::new(&doc)?;
/// assert_eq!(planner.constraint().kind(), ConstraintKind::Free);
/// let plan = planner.plan(&SaveOptions::new())?;
/// assert_eq!((plan.mode(), plan.would_invalidate().len()), (SaveMode::Incremental, 0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
pub struct SavePlanner {
    signatures: Signatures,
    /// Which fields have widgets on which pages: read only when a signature has a field or a
    /// field lock, for the field-level checks of page ops.
    #[cfg(feature = "page-checks")]
    widgets: Option<WidgetIndex>,
    constraint: SaveConstraint,
    conformance: Vec<ConformanceWarning>,
    packet_rewritten: bool,
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
    ///
    /// Added in 0.2.0.
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
        let mut violations: Vec<Violation> = Vec::new();
        if dirty {
            violations.extend(pending.classes().iter().flat_map(|&class| signatures.violations(class)));
        }
        #[cfg(feature = "page-checks")]
        let widgets = if signatures.has_field_rules() {
            if dirty {
                pending_widget_violations(doc, &signatures, pending, &mut violations)?;
            }
            Some(WidgetIndex::new(doc)?)
        } else {
            None
        };
        let constraint = SaveConstraint {
            signatures: signatures.infos(),
            doc_mdp: signatures.doc_mdp(),
            rewrite_reasons,
            violations,
            signatures_unknown: signatures.unknown,
            unchanged: !dirty,
        };
        // Warnings only: a packet that can't be read (or read yet) claims nothing to break,
        // and the save itself reports any error reading the document.
        let conformance = if dirty {
            laminapdf_ops::pending_conformance_warnings(doc, pending).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(Self {
            signatures,
            #[cfg(feature = "page-checks")]
            widgets,
            constraint,
            conformance,
            packet_rewritten: dirty && pending.packet_rewritten(),
        })
    }

    /// The PDF/A and PDF/UA claims the document's metadata, as edited, no longer meets (E1 plan
    /// §3): for the UI to warn before the save. They never block it, since the user may mean to
    /// drop the claim. Empty for a document with no changes.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn conformance_warnings(&self) -> &[ConformanceWarning] {
        &self.conformance
    }

    /// Everything that restricts saving the document.
    ///
    /// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
    pub fn plan(&self, options: &SaveOptions) -> Result<SavePlan> {
        let mut plan = SavePlan::resolve(self.constraint.clone(), options)?;
        if plan.mode == SaveMode::Incremental && !plan.would_invalidate.is_empty() {
            for signature in self.signatures.restricting() {
                if !plan.would_invalidate.contains(signature) {
                    plan.would_invalidate.push(signature.clone());
                }
            }
        }
        plan.metadata = self.metadata_update(options, &plan.would_invalidate);
        plan.packet_rewritten = self.packet_rewritten;
        Ok(plan)
    }

    /// Whether a save with `options` stamps the save-time metadata ([E1 plan §2]): only when
    /// the options ask for it and there are changes to save, and only when the stamp (class
    /// `Metadata`) adds no violation the pending changes don't already have, besides those of
    /// signatures the save invalidates anyway. Such a stamp also leaves the constraint's kind
    /// as it was.
    ///
    /// [E1 plan §2]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e1-editing-foundation.md#2-the-operations
    fn metadata_update(&self, options: &SaveOptions, invalidated: &[SignatureInfo]) -> MetadataUpdate {
        if !options.stamps_metadata() || self.constraint.unchanged {
            return MetadataUpdate::None;
        }
        let added: Vec<Violation> = self
            .would_violate_class(ModificationClass::Metadata)
            .into_iter()
            .filter(|v| !self.constraint.violations.contains(v) && !invalidated.contains(&v.signature))
            .collect();
        if added.is_empty() {
            MetadataUpdate::Stamp
        } else {
            MetadataUpdate::Skipped { violations: added }
        }
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
    /// or warn before the edit ([ADR 0007] §2): those of its class; for an op that needs a
    /// full rewrite ([`Op::requires_rewrite`]), every signature ([`Restriction::SignedBytes`]);
    /// and for a page op ([`Op::page_changes`]) that deletes or reshows a page, those of the
    /// widgets on it ([`Restriction::SignedWidget`], [`Restriction::FieldMdp`]). The planner
    /// stays the authority at save time.
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    ///
    /// Added in 0.2.0.
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
        #[cfg(feature = "page-checks")]
        if let Some(index) = &self.widgets {
            for change in op.page_changes() {
                for field in index.on_page(&change.page) {
                    let name = index.name(field);
                    for v in self
                        .signatures
                        .widget_violations(field, name, change.change, op.class())
                    {
                        if !out.contains(&v) {
                            out.push(v);
                        }
                    }
                }
            }
        }
        out
    }

    /// [`SavePlanner::would_violate`] for any change of `class`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn would_violate_class(&self, class: ModificationClass) -> Vec<Violation> {
        self.signatures.violations(class).collect()
    }
}

/// Adds to `violations` those of the widgets the pending page ops touched. Names come from
/// each field's own `/Parent` chain, not the pages: a deleted page's widgets are on no page any
/// more.
#[cfg(feature = "page-checks")]
fn pending_widget_violations(
    doc: &laminapdf_ops::Document,
    signatures: &Signatures,
    pending: &laminapdf_ops::PendingChanges,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    for w in pending.widgets() {
        let name = match w.field.as_lamina_id().obj_ref() {
            Some(r) => doc.field_name(r)?,
            None => None,
        };
        for v in signatures.widget_violations(&w.field, name.as_deref(), w.change, w.class) {
            if !violations.contains(&v) {
                violations.push(v);
            }
        }
    }
    Ok(())
}

/// What a save does with the save-time metadata ([`SaveOptions::with_metadata_date`]).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MetadataUpdate {
    /// Nothing to stamp: the options gave no date or ID, or the save writes no changes.
    #[default]
    None,
    /// The save stamps `xmp:MetadataDate`, the modification dates and `xmpMM:InstanceID`.
    Stamp,
    /// The stamp would break these restrictions, which the changes saved keep, so the save
    /// leaves it out ([`SaveReport::metadata_not_updated`](crate::SaveReport::metadata_not_updated)).
    Skipped {
        /// What the stamp would have broken.
        violations: Vec<Violation>,
    },
}

/// A resolved save: the constraint, the mode and the signatures it would invalidate.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavePlan {
    constraint: SaveConstraint,
    mode: SaveMode,
    would_invalidate: Vec<SignatureInfo>,
    removes_encryption: bool,
    metadata: MetadataUpdate,
    packet_rewritten: bool,
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
    ///
    /// Added in 0.2.0.
    pub fn resolve(constraint: SaveConstraint, options: &SaveOptions) -> Result<Self> {
        let (mode, would_invalidate) = resolve(&constraint, *options)?;
        let removes_encryption =
            mode == SaveMode::FullRewrite && constraint.rewrite_reasons.contains(&RewriteReason::EncryptedBase);
        Ok(Self {
            constraint,
            mode,
            would_invalidate,
            removes_encryption,
            metadata: MetadataUpdate::None,
            packet_rewritten: false,
        })
    }

    /// Everything that restricts the save.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn constraint(&self) -> &SaveConstraint {
        &self.constraint
    }

    /// The resolved mode: [`SaveMode::Incremental`] or [`SaveMode::FullRewrite`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn mode(&self) -> SaveMode {
        self.mode
    }

    /// The signatures this save would invalidate: empty unless the options override.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn would_invalidate(&self) -> &[SignatureInfo] {
        &self.would_invalidate
    }

    /// Whether the save writes the encrypted document as plaintext.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn removes_encryption(&self) -> bool {
        self.removes_encryption
    }

    /// What the save does with the save-time metadata ([`SavePlanner::plan`] fills it in).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn metadata_update(&self) -> &MetadataUpdate {
        &self.metadata
    }

    /// Whether an edit to save rewrote the XMP packet whole (UTF-16, or `end="r"`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn packet_rewritten(&self) -> bool {
        self.packet_rewritten
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::OpenOptions;
    use laminapdf_test_fixtures::fixture;

    use super::*;
    use crate::constraint::ConstraintKind;

    /// The planner of a `DocMDP` P2 file whose pending changes break nothing: what a form fill
    /// leaves (no form-filling op exists yet, so the changes are stood in for by marking the
    /// constraint changed).
    fn p2_after_an_allowed_change() -> SavePlanner {
        let doc = laminapdf_ops::Document::open(fixture("signed/certified-p2.pdf"), OpenOptions::default()).unwrap();
        let mut planner = SavePlanner::new(&doc).unwrap();
        planner.constraint.unchanged = false;
        planner
    }

    #[test]
    fn a_form_fill_on_a_p2_file_saves_append_only_without_the_stamp() {
        let planner = p2_after_an_allowed_change();
        let options = SaveOptions::new().with_metadata_date("D:20261110093000Z".parse().unwrap());
        let plan = planner.plan(&options).unwrap();
        let MetadataUpdate::Skipped { violations } = plan.metadata_update() else {
            panic!("stamped: {:?}", plan.metadata_update());
        };
        assert_eq!(
            (plan.constraint().kind(), plan.mode(), violations.len()),
            (ConstraintKind::AppendOnly, SaveMode::Incremental, 1)
        );
    }
}
