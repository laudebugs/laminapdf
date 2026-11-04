//! Reading a font's dictionaries tolerantly: every problem becomes an issue flag, never an error.

use std::sync::Arc;

use laminapdf_cos::{Document, Result, absent_if_malformed};
use laminapdf_syntax::{Dict, Object};

use super::Issues;

/// Reads entries, resolving references; a malformed object reads as absent and is noted, and
/// any other error ends the load ([`absent_if_malformed`]).
pub(super) struct Reader<'d> {
    pub(super) doc: &'d Document,
    pub(super) issues: Issues,
}

impl Reader<'_> {
    pub(super) fn get(&mut self, dict: &Dict, key: &str) -> Result<Option<Arc<Object>>> {
        let Some(value) = dict.get(key) else { return Ok(None) };
        Ok(self
            .noted(self.doc.deref(value))?
            .map(laminapdf_cos::Resolved::into_arc))
    }

    pub(super) fn dict(&mut self, dict: &Dict, key: &str) -> Result<Option<Dict>> {
        Ok(self.get(dict, key)?.and_then(|o| match &*o {
            Object::Dict(d) => Some(d.clone()),
            Object::Stream(s) => Some(s.dict.clone()),
            _ => None,
        }))
    }

    pub(super) fn number(&mut self, dict: &Dict, key: &str) -> Result<Option<f64>> {
        Ok(self.get(dict, key)?.and_then(|o| o.as_f64()))
    }

    /// A stream's decoded bytes; a stream that can't be decoded reads as absent.
    pub(super) fn stream(&mut self, object: &Object) -> Result<Option<Vec<u8>>> {
        let Object::Stream(stream) = object else {
            return Ok(None);
        };
        self.noted(self.doc.decode_stream(stream))
    }

    /// `result` read with [`absent_if_malformed`], a malformed object noted as a font issue.
    pub(super) fn noted<T>(&mut self, result: Result<T>) -> Result<Option<T>> {
        let value = absent_if_malformed(result)?;
        self.issues.font |= value.is_none();
        Ok(value)
    }

    /// The numbers of an array (non-numbers dropped, and noted; an error
    /// [`absent_if_malformed`] returns while resolving an element ends the load).
    pub(super) fn numbers(&mut self, object: &Object) -> Result<Vec<f64>> {
        let Some(items) = object.as_array() else {
            self.issues.font = true;
            return Ok(Vec::new());
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let value = absent_if_malformed(self.doc.deref(item))?.and_then(|resolved| resolved.as_f64());
            match value {
                Some(n) if n.is_finite() => out.push(n),
                _ => self.issues.font = true,
            }
        }
        Ok(out)
    }

    /// The embedded font program of a descriptor, if TrueType or OpenType (skrifa reads
    /// those; bare CFF and Type 1 programs are not read).
    pub(super) fn program(&mut self, descriptor: Option<&Dict>) -> Result<Option<Vec<u8>>> {
        let Some(descriptor) = descriptor else { return Ok(None) };
        for key in ["FontFile2", "FontFile3"] {
            if let Some(object) = self.get(descriptor, key)? {
                let open_type = key == "FontFile2"
                    || matches!(&*object, Object::Stream(s) if s.dict.get("Subtype").is_some_and(|n| matches!(n, Object::Name(n) if n.0 == b"OpenType")));
                if open_type {
                    return self.stream(&object);
                }
            }
        }
        Ok(None)
    }
}

/// A non-negative whole number that fits a `u32` (CIDs, codes, flags), else `None`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "checked: finite, whole, within 0..=u32::MAX"
)]
pub(super) fn whole(n: f64) -> Option<u32> {
    (n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX)).then_some(n as u32)
}

pub(super) fn name_of(object: Option<&Object>) -> Option<&[u8]> {
    match object {
        Some(Object::Name(name)) => Some(&name.0),
        _ => None,
    }
}
