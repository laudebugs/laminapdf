//! [`PendingChanges`]: what the changes not yet saved are, for the save planner (ADR 0006 §1,
//! ADR 0007 §2).

use std::collections::BTreeSet;

use crate::class::ModificationClass;
use crate::op::{Effect, Op};
use crate::widgets::TouchedWidget;

/// The kinds of change the document holds since it was opened or last rebased onto a save:
/// what the save planner checks against `DocMDP`, `FieldMDP` and UR3, and whether any of them
/// needs a full rewrite ([ADR 0007] §2).
///
/// It errs on the side of reporting: every committed transaction's op classes are added (an
/// undo or redo adds those of the transaction it reverts), and only an empty overlay (nothing
/// left to save) or a rebase clears it. A class that is reported but whose change was undone
/// can make the planner refuse a save it could allow; a class that is missing would let a save
/// break a signature without saying so.
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingChanges {
    classes: BTreeSet<ModificationClass>,
    requires_rewrite: bool,
    unknown: bool,
    packet_rewritten: bool,
    widgets: BTreeSet<TouchedWidget>,
}

impl PendingChanges {
    /// Changes of unknown kind (an overlay changed without ops, such as a model document
    /// wrapped while dirty): every class, and a full rewrite needed, since nothing tells
    /// otherwise.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            classes: [
                ModificationClass::Metadata,
                ModificationClass::FormFill,
                ModificationClass::Annotation,
                ModificationClass::Signature,
                ModificationClass::PageStructure,
                ModificationClass::Content,
                ModificationClass::Document,
            ]
            .into_iter()
            .collect(),
            requires_rewrite: true,
            unknown: true,
            packet_rewritten: false,
            widgets: BTreeSet::new(),
        }
    }

    /// Whether the changes are of unknown kind ([`PendingChanges::unknown`]).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        self.unknown
    }

    /// The op classes of the changes not yet saved.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn classes(&self) -> &BTreeSet<ModificationClass> {
        &self.classes
    }

    /// Whether any of them needs a full rewrite ([`Op::requires_rewrite`]).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn requires_rewrite(&self) -> bool {
        self.requires_rewrite
    }

    /// Whether a metadata op rewrote the XMP packet whole (UTF-16, or read-only by its
    /// `end="r"` trailer), so the save can say the packet lost its layout and comments.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn packet_rewritten(&self) -> bool {
        self.packet_rewritten
    }

    /// The form fields whose widgets the changes deleted, reshowed or moved: what the save
    /// planner checks against signed signature fields and field locks, field by field
    /// ([E2 plan §6]). Changes of unknown kind ([`PendingChanges::unknown`]) name no field;
    /// they hold every class, which the planner already judges.
    ///
    /// [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn widgets(&self) -> &BTreeSet<TouchedWidget> {
        &self.widgets
    }

    /// Whether there are none.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty() && !self.requires_rewrite
    }

    /// Adds `op`'s class and rewrite need, and what applying it did: whether it rewrote the
    /// packet whole, and the widgets it touched.
    pub(crate) fn record(&mut self, op: &Op, effect: &Effect) {
        self.classes.insert(op.class());
        self.requires_rewrite |= op.requires_rewrite();
        self.packet_rewritten |= effect.packet_rewritten;
        self.widgets.extend(effect.widgets.iter().cloned());
    }

    /// Adds everything `other` holds.
    pub(crate) fn merge(&mut self, other: &Self) {
        self.classes.extend(other.classes.iter().copied());
        self.requires_rewrite |= other.requires_rewrite;
        self.unknown |= other.unknown;
        self.packet_rewritten |= other.packet_rewritten;
        self.widgets.extend(other.widgets.iter().cloned());
    }

    /// Forgets everything (after a rebase, or when the overlay is empty again).
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}
