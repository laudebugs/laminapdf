//! Test helpers for the crates that judge ops (feature `test-support`, off by default): an op
//! that declares the widgets it touches, or the fields it authors, so `laminapdf-save`'s
//! planner tests can check its field-level rules before the page ops that report real widgets
//! and the authoring ops exist.

use laminapdf_model::FieldId;

use crate::class::{FormRight, ModificationClass};
use crate::error::Result;
use crate::op::{Effect, Op};
use crate::widgets::{AuthoredField, PageChange, TouchedWidget, WidgetChange};

/// Applies `op` and reports `widgets` as touched and `pages` as about to change, whatever `op`
/// itself does; an authoring stand-in ([`Declared::authoring`]) also declares its class, the
/// fields it authors and the usage right it needs. Its rewrite need is `op`'s.
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
    /// The class [`Op::class`] reports, when not `op`'s.
    ///
    /// Added in 0.7.0.
    pub class: Option<ModificationClass>,
    /// What [`Op::authored_fields`] reports.
    ///
    /// Added in 0.7.0.
    pub authored: Vec<AuthoredField>,
    /// What [`Op::form_right`] reports.
    ///
    /// Added in 0.7.0.
    pub form_right: Option<FormRight>,
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
            class: None,
            authored: Vec::new(),
            form_right: None,
        }
    }

    /// `op`, standing in for an authoring op of `class` that authors `fields` (each the field,
    /// when it exists, and the names it gives or takes away) and needs `form_right`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn authoring(
        op: impl Into<Op>,
        class: ModificationClass,
        fields: Vec<(Option<FieldId>, Vec<String>)>,
        form_right: Option<FormRight>,
    ) -> Self {
        Self {
            class: Some(class),
            authored: fields
                .into_iter()
                .map(|(field, names)| AuthoredField::new(field, names, class))
                .collect(),
            form_right,
            ..Self::new(op, Vec::new(), Vec::new())
        }
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        sources: &crate::document::Sources,
        class: ModificationClass,
        content: (&crate::BlobStore, Option<&dyn crate::ImageXObjects>),
    ) -> Result<Effect> {
        let mut effect = self.op.apply_to(doc, sources, None, content)?;
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
