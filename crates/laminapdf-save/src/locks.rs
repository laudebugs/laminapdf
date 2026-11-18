//! The fields a document's signatures lock ([E8 plan §5]), for the ops crate's calculations:
//! the same rule the planner applies to a `SetFieldValue`, so a total the planner would refuse
//! is the total a calculation skips.
//!
//! [E8 plan §5]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint

use std::collections::BTreeSet;

use laminapdf_model::{FieldId, TerminalField};
use laminapdf_ops::{FieldLocks, ModificationClass};

use crate::detect::signatures;
use crate::error::SaveError;

/// Reads a document's `FieldMDP` locks and signature fields' `/Lock` (§12.8.2.4, §12.7.5.5):
/// a field is locked when a change to its value would break one. Set it with
/// [`laminapdf_ops::Document::with_field_locks`].
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, Default)]
pub struct SignatureLocks;

impl FieldLocks for SignatureLocks {
    fn locked(
        &self,
        doc: &laminapdf_model::Document,
        fields: &[&TerminalField],
    ) -> laminapdf_cos::Result<BTreeSet<FieldId>> {
        let found = match signatures(doc.cos()) {
            Ok(found) => found,
            Err(SaveError::Cos(e)) => return Err(e),
            // Signatures that can't be read lock every field, as ADR 0007 reads lock names it
            // can't read.
            Err(_) => return Ok(fields.iter().map(|f| f.id.clone()).collect()),
        };
        Ok(fields
            .iter()
            .filter(|f| {
                found
                    .field_violations(&f.id, Some(&f.name), ModificationClass::FormFill)
                    .next()
                    .is_some()
            })
            .map(|f| f.id.clone())
            .collect())
    }
}
