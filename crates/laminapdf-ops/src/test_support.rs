//! Test helpers for the crates that judge ops (feature `test-support`, off by default): an op
//! that declares the widgets it touches, so `laminapdf-save`'s planner tests can check its
//! field-level rules before the page ops that report real widgets exist.

use laminapdf_model::FieldId;

use crate::class::ModificationClass;
use crate::error::Result;
use crate::op::{Effect, Op};
use crate::widgets::{PageChange, TouchedWidget, WidgetChange};

/// Applies `op` and reports `widgets` as touched and `pages` as about to change, whatever `op`
/// itself does. Its class and rewrite need are `op`'s.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Declared {
    /// The op that makes the change.
    pub op: Box<Op>,
    /// The fields whose widgets it reports, with what happened to each.
    pub widgets: Vec<(FieldId, WidgetChange)>,
    /// What [`Op::page_changes`] reports.
    pub pages: Vec<PageChange>,
}

impl Declared {
    /// `op`, declaring `widgets` touched and `pages` changed.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(op: impl Into<Op>, widgets: Vec<(FieldId, WidgetChange)>, pages: Vec<PageChange>) -> Self {
        Self {
            op: Box::new(op.into()),
            widgets,
            pages,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document, class: ModificationClass) -> Result<Effect> {
        let mut effect = self.op.apply_to(doc, None)?;
        effect.widgets.extend(
            self.widgets
                .iter()
                .map(|(field, change)| TouchedWidget::new(field.clone(), *change, class)),
        );
        Ok(effect)
    }
}

impl From<Declared> for Op {
    fn from(op: Declared) -> Self {
        Self::Declared(op)
    }
}
