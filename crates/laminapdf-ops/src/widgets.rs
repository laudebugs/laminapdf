//! What a page op does to form fields' widgets, for the save planner's field-level checks
//! ([E2 plan §6]): a signed signature's widget or a locked field's widget on a page that an op
//! deletes, replaces, rotates, crops or resizes changes how that field is shown.
//!
//! [E2 plan §6]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint

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
