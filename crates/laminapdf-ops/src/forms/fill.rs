//! Filling many fields at once (E8 plan §2.1): `fill_form`, `import_form_data` and
//! `reset_fields` are one transaction of [`SetFieldValue`] ops, one undo step, with the form's
//! calculations run once at the end. A field a signature locks is skipped, and a value its field
//! refuses is rolled back alone, so one bad answer in a file from another app fills the rest.

use std::collections::BTreeSet;

use laminapdf_model::{FieldId, TerminalField};

use super::SetFieldValue;
use crate::document::Document;
use crate::error::{Error, Result};
use crate::op::Op;
use crate::tx::{Applied, Origin};

/// What [`Document::fill`] did: the transaction, and the fields it left as they were.
///
/// Added in 0.7.0.
#[derive(Debug)]
#[non_exhaustive]
pub struct Filled {
    /// The transaction: one undo step for every value set and every total calculated.
    pub applied: Applied,
    /// The fields a signature's lock covers, left as they are (§12.8.2.4, §12.7.5.5).
    pub locked: Vec<FieldId>,
    /// The fields whose value was refused, each with why: `unknown-field`, `field-is-signature`,
    /// `field-value-invalid` or `annotation-glyph-missing`.
    pub refused: Vec<(FieldId, Error)>,
}

impl Document {
    /// Sets each value of `values` as one transaction, then, when `calculate` is true, runs the
    /// form's calculations once ([E8 plan §2.1]). A field a signature's lock covers is skipped
    /// and listed, when the document reads locks ([`Document::with_field_locks`], which the
    /// `laminapdf` façade always sets); so is one whose value its field refuses: that op alone
    /// is rolled back.
    ///
    /// [E8 plan §2.1]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e8-form-filling.md#21-ops
    ///
    /// # Errors
    /// Any other error of an op (`appearance-unavailable`, reading errors), which rolls back
    /// the whole transaction.
    ///
    /// Added in 0.7.0.
    pub fn fill(&mut self, values: Vec<SetFieldValue>, calculate: bool) -> Result<Filled> {
        let ((locked, refused), applied) = self.transact(Origin::Local, |tx| {
            let form = tx.doc.form()?;
            let named: Vec<&TerminalField> = values.iter().filter_map(|v| form.field(&v.field)).collect();
            let locks = match tx.locks {
                Some(locks) => locks.locked(tx.doc, &named)?,
                None => BTreeSet::new(),
            };
            let mut locked = Vec::new();
            let mut refused = Vec::new();
            for value in values {
                if locks.contains(&value.field) {
                    locked.push(value.field);
                    continue;
                }
                let field = value.field.clone();
                match tx.apply_quietly(&Op::from(value)) {
                    Ok(()) => {}
                    Err(e) if refuses_value(&e) => refused.push((field, e)),
                    Err(e) => return Err(e),
                }
            }
            if calculate {
                tx.calculate()?;
            }
            Ok((locked, refused))
        })?;
        Ok(Filled {
            applied,
            locked,
            refused,
        })
    }
}

/// Whether `error` is about one field's value, so the fill goes on without it.
const fn refuses_value(error: &Error) -> bool {
    matches!(
        error,
        Error::UnknownField(_)
            | Error::FieldIsSignature(_)
            | Error::FieldValueInvalid { .. }
            | Error::AnnotationGlyphMissing
    )
}
