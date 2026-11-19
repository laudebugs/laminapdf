//! What ops do to form fields, for the save planner's field-level checks: a signed signature's
//! widget or a locked field's widget on a page that an op deletes, replaces, rotates, crops or
//! resizes changes how that field is shown ([E2 plan §6]), and an authoring op changes the
//! fields themselves ([E9 plan §5]).
//!
//! [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint
//! [E9 plan §5]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint

use laminapdf_model::{FieldId, PageId};

use crate::class::ModificationClass;

/// What an op did to a widget's page, as the save planner judges it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum WidgetChange {
    /// The page was deleted or replaced: the widget is no longer shown.
    Deleted,
    /// The page was rotated, cropped or resized, or its boxes changed: the widget is shown
    /// differently.
    Reshown,
    /// The page moved in the page order, or was reversed or duplicated: the widget is shown as
    /// it was.
    Moved,
}

impl WidgetChange {
    /// Whether the change alters how the widget is shown ([`WidgetChange::Deleted`] or
    /// [`WidgetChange::Reshown`]): the change the planner checks against signed signatures
    /// and field locks.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn alters_appearance(self) -> bool {
        matches!(self, Self::Deleted | Self::Reshown)
    }
}

/// A form field whose widget an op changed, recorded in
/// [`PendingChanges::widgets`](crate::PendingChanges::widgets).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct TouchedWidget {
    /// The terminal field the widget belongs to.
    pub field: FieldId,
    /// What happened to the widget's page.
    pub change: WidgetChange,
    /// The class of the op that changed it.
    pub class: ModificationClass,
}

impl TouchedWidget {
    /// `field`'s widget changed by `change`, by an op of `class`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(field: FieldId, change: WidgetChange, class: ModificationClass) -> Self {
        Self { field, change, class }
    }
}

/// A page an op is about to change, from [`Op::page_changes`](crate::Op::page_changes): what
/// the save planner checks before the op applies, against the widgets on the page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct PageChange {
    /// The page.
    pub page: PageId,
    /// What happens to it.
    pub change: WidgetChange,
}

impl PageChange {
    /// `page`, changed by `change`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(page: PageId, change: WidgetChange) -> Self {
        Self { page, change }
    }
}

/// A form field an authoring op creates, renames, changes or deletes, from
/// [`Op::authored_fields`](crate::Op::authored_fields) and kept in
/// [`PendingChanges::authored_fields`](crate::PendingChanges::authored_fields): what the save
/// planner holds to the field locks that cover any name it has or gets, and to signed
/// signature fields ([ADR 0007] §13).
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct AuthoredField {
    /// The field, when it exists before the op: `None` for a field the op creates.
    pub field: Option<FieldId>,
    /// Every fully qualified name the op gives the field or takes away from it: the new name of
    /// a created field, both the old and the new name of a renamed one (whose fields below it
    /// are renamed too), and the name of a deleted one. Empty for an op that keeps the name,
    /// which the planner then reads from the document.
    pub names: Vec<String>,
    /// The class of the op: [`PendingChanges`](crate::PendingChanges) keeps the field after
    /// the op is gone, and the planner reports its violations under this class, as
    /// [`TouchedWidget::class`] does for widgets.
    pub class: ModificationClass,
}

impl AuthoredField {
    /// `field` (when it exists), given or losing `names`, by an op of `class`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(field: Option<FieldId>, names: Vec<String>, class: ModificationClass) -> Self {
        Self { field, names, class }
    }
}
