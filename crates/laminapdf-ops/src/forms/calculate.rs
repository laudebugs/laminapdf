//! Calculated fields (E8 plan §2.3): after a field's value changes, each `/CO` calculation runs
//! once, in order (§12.7.3), and every total whose value changes gets a
//! [`SetFieldValue`](super::SetFieldValue) of its own in the same transaction, so undo reverts
//! the change and its totals together and the save planner checks each one.

use std::fmt;

use laminapdf_cos::{ObjRef, Object, Result as CosResult};
use laminapdf_model::{FieldId, FieldValue, TerminalField};

use super::SetFieldValue;
use crate::error::Result;
use crate::op::Op;
use crate::tx::{Origin, Tx};

/// Which fields a signature's lock keeps from changing (§12.8.2.4, §12.7.5.5): a calculation
/// skips them, so filling an unlocked field never conflicts with a lock through a total.
/// `laminapdf-save`'s `SignatureLocks` reads them from the document's signatures; the
/// `laminapdf` façade always sets it.
///
/// Added in 0.6.0.
pub trait FieldLocks {
    /// Whether a lock covers `field`.
    ///
    /// # Errors
    /// COS errors reading the signatures.
    fn locked(&self, doc: &laminapdf_model::Document, field: &TerminalField) -> CosResult<bool>;
}

/// The document's lock reader.
pub(crate) struct Locks(pub(crate) Box<dyn FieldLocks + Send>);

impl fmt::Debug for Locks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FieldLocks")
    }
}

/// The objects `/AcroForm /CO` lists, in order.
fn calculation_order(doc: &laminapdf_model::Document) -> Result<Vec<ObjRef>> {
    let Some(acro) = doc.catalog()?.get("AcroForm").cloned() else {
        return Ok(Vec::new());
    };
    let Some(acro) = laminapdf_cos::tolerant(doc.deref(&acro))?.and_then(|o| o.as_dict().cloned()) else {
        return Ok(Vec::new());
    };
    let Some(order) = acro.get("CO") else {
        return Ok(Vec::new());
    };
    let order = laminapdf_cos::tolerant(doc.deref(order))?;
    Ok(order
        .as_deref()
        .and_then(Object::as_array)
        .unwrap_or_default()
        .iter()
        .filter_map(Object::as_ref)
        .collect())
}

impl Tx<'_> {
    /// Runs the form's calculations once, in `/CO` order, after a local change to a value: each
    /// total whose value changes gets a `SetFieldValue`, and each a lock covers is skipped and
    /// listed. A remote change carries its totals already, so it calculates nothing.
    pub(crate) fn calculate(&mut self) -> Result<()> {
        if self.state.origin != Origin::Local {
            return Ok(());
        }
        for object in calculation_order(self.doc)? {
            // Each total reads the values the ones before it gave.
            let form = self.doc.form()?;
            let Some(field) = form.field(&FieldId::object(object)) else {
                continue;
            };
            let behaviour = self.doc.field_behaviour(field)?;
            let Some(calculation) = &behaviour.calculate else {
                continue;
            };
            let value = calculation.compute(&form);
            if matches!(&field.value, Some(FieldValue::Text(t)) if *t == value) {
                continue;
            }
            // A total its own checks refuse is left as it is.
            let Ok(value) = behaviour.commit(&value) else {
                continue;
            };
            if let Some(locks) = self.locks
                && locks.locked(self.doc, field)?
            {
                if !self.state.skipped_calculations.contains(&field.id) {
                    self.state.skipped_calculations.push(field.id.clone());
                }
                continue;
            }
            // Run as itself: a total's change starts no calculation of its own.
            let op = Op::from(SetFieldValue::new(field.id.clone(), FieldValue::Text(value)));
            self.run_op(&op)?;
        }
        Ok(())
    }
}
