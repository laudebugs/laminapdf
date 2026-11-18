//! Calculated fields (E8 plan §2.3): after a field's value changes, each `/CO` calculation runs
//! once, in order (§12.7.3), and every total whose value changes gets a
//! [`SetFieldValue`](super::SetFieldValue) of its own in the same transaction, so undo reverts
//! the change and its totals together and the save planner checks each one.

use std::collections::BTreeSet;
use std::fmt;

use laminapdf_cos::{ObjRef, Object, Result as CosResult};
use laminapdf_model::{Behaviour, FieldId, FieldValue, FormValues, TerminalField};

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
    /// The fields of `fields` a lock covers, read in one pass over the signatures.
    ///
    /// # Errors
    /// COS errors reading the signatures.
    fn locked(&self, doc: &laminapdf_model::Document, fields: &[&TerminalField]) -> CosResult<BTreeSet<FieldId>>;
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
    let Some(acro) = super::appearance::acro_form(doc)? else {
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
    /// listed. A remote change carries its totals already, so it calculates nothing. The form
    /// is read once; each total reads the values the ones before it gave.
    pub(crate) fn calculate(&mut self) -> Result<()> {
        if self.state.origin != Origin::Local {
            return Ok(());
        }
        let form = self.doc.form()?;
        let mut seen = BTreeSet::new();
        let mut totals: Vec<(TerminalField, Behaviour)> = Vec::new();
        for object in calculation_order(self.doc)? {
            // A field `/CO` lists twice calculates once.
            if !seen.insert(object) {
                continue;
            }
            let Some(field) = form.field(&FieldId::object(object)) else {
                continue;
            };
            let behaviour = self.doc.field_behaviour(field)?;
            if behaviour.calculate.is_some() {
                totals.push((field.clone(), behaviour));
            }
        }
        if totals.is_empty() {
            return Ok(());
        }
        let locked = match self.locks {
            Some(locks) => locks.locked(self.doc, &totals.iter().map(|(f, _)| f).collect::<Vec<_>>())?,
            None => BTreeSet::new(),
        };
        let mut values = FormValues::of(&form);
        for (field, behaviour) in &totals {
            let Some(calculation) = &behaviour.calculate else {
                continue;
            };
            let value = calculation.compute(&values);
            if values.get(&field.name) == Some(value.as_str()) {
                continue;
            }
            // pdf.js checks a calculated value with its validate helper alone, never the
            // keystroke check, which takes typed text; a total it refuses is left as it is, so a
            // form can never make the change to its inputs fail.
            if behaviour.validate.as_ref().is_some_and(|v| v.commit(&value).is_err()) {
                continue;
            }
            if locked.contains(&field.id) {
                if !self.state.skipped_calculations.contains(&field.id) {
                    self.state.skipped_calculations.push(field.id.clone());
                }
                continue;
            }
            // Run as itself: a total's change starts no calculation of its own.
            let mut op = SetFieldValue::new(field.id.clone(), FieldValue::Text(value.clone()));
            op.calculated = true;
            let op = Op::from(op);
            self.run_op(&op)?;
            values.set(&field.name, value);
        }
        Ok(())
    }
}
