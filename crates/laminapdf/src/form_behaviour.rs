//! Form behaviour without JavaScript ([ADR 0021]): what a field's helper calls do, a value
//! checked and shown as its field would show it, the form's calculations, and every script in
//! the document with whether Lamina PDF handles it.
//!
//! [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md

use laminapdf_model::{Behaviour, FieldId, FormScript, InvalidValue, ShownValue};

use crate::document::{Document, Edited};
use crate::error::{Error, Result};

/// A value checked as its field would commit it ([`Document::validate`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FieldCheck {
    /// What the field would store (the text, or a comma-point number with a point), or why it
    /// refuses it.
    pub committed: Result<String, InvalidValue>,
    /// What the field would show: its format's text for the value, else the text as typed.
    pub shown: ShownValue,
}

impl Document {
    /// What the field `field` does by its keystroke, format, validate and calculate
    /// scripts, as far as each is one of Acrobat's helper calls; the others are listed in
    /// [`Behaviour::not_handled`]. Lamina PDF runs no script (ADR 0021).
    ///
    /// # Errors
    /// `unknown-field`; reading errors.
    ///
    /// Added in 0.6.0.
    pub fn field_behaviour(&self, field: &FieldId) -> Result<Behaviour> {
        let doc = self.ops();
        let form = doc.form()?;
        let field = form
            .field(field)
            .ok_or_else(|| Error::from(laminapdf_ops::Error::UnknownField(field.clone())))?;
        Ok(doc.field_behaviour(field)?)
    }

    /// `text` checked as the field `field` would commit it, by its keystroke and validate
    /// helpers, and what the field would show for it. Nothing changes.
    ///
    /// # Errors
    /// As [`Document::field_behaviour`].
    ///
    /// Added in 0.6.0.
    pub fn validate(&self, field: &FieldId, text: &str) -> Result<FieldCheck> {
        let behaviour = self.field_behaviour(field)?;
        let committed = behaviour.commit(text);
        let shown = match &committed {
            Ok(value) => behaviour.show(value),
            Err(_) => ShownValue::plain(text.to_owned()),
        };
        Ok(FieldCheck { committed, shown })
    }

    /// Whether field `field`'s keystroke helper takes `text` as the field's text while it is
    /// typed (a number field takes `12.` but not `12a`); every text for a field without one.
    ///
    /// # Errors
    /// As [`Document::field_behaviour`].
    ///
    /// Added in 0.6.0.
    pub fn field_accepts(&self, field: &FieldId, text: &str) -> Result<bool> {
        Ok(self
            .field_behaviour(field)?
            .keystroke
            .as_ref()
            .is_none_or(|k| k.accepts(text)))
    }

    /// Runs the form's calculations once, in `/CO` order, as one edit: each total whose value
    /// changes gets a `SetFieldValue`, and totals a signature's lock covers are left as they
    /// are. A change to a value does this already; call it for a form whose stored totals are
    /// stale.
    ///
    /// # Errors
    /// `saved-not-rebased`; reading errors.
    ///
    /// Added in 0.6.0.
    pub fn calculate(&mut self) -> Result<Edited> {
        let applied = self.ops_mut().calculate()?;
        Ok(self.edited(applied))
    }

    /// Every script in the document ([ADR 0021] §3): the document's, the pages', the
    /// annotations' and the fields' JavaScript actions, and an XFA form's `<script>` elements,
    /// each with where it is, its size and whether it is handled. Script text is never
    /// returned.
    ///
    /// [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md
    ///
    /// # Errors
    /// Reading errors.
    ///
    /// Added in 0.6.0.
    pub fn form_scripts(&self) -> Result<Vec<FormScript>> {
        Ok(self.ops().form_scripts()?)
    }
}
