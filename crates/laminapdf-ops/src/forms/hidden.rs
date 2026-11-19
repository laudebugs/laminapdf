//! [`SetFieldHidden`]: what a Hide action does to a field (§12.6.4.10), setting or clearing the
//! Hidden flag of each of its widgets (§12.5.3, Table 167).

use laminapdf_cos::{Object, tolerant};
use laminapdf_model::{FieldId, FieldKind, PageId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::wire;
use crate::wire::json::{entries, kind_of};

/// The Hidden bit of an annotation's `/F` (bit 2).
const HIDDEN: i64 = 1 << 1;

/// Hides a form field, or shows it again: each of its widgets' Hidden flag set or cleared, as
/// a Hide action does (§12.6.4.10). Its class is `Annotation` ([E8 plan §5]): a widget's flags
/// aren't among what `DocMDP` P2 lets change, so the planner allows it under P3 and approval
/// signatures, reports it under P1 and P2, and holds it to UR3's `/Annots [/Modify]`.
///
/// ```
/// use laminapdf_ops::{Op, SetFieldHidden};
/// let op: Op = serde_json::from_str(r#"{ "op": "setFieldHidden", "args": { "field": "f12.0", "hidden": true } }"#)?;
/// assert!(matches!(op, Op::SetFieldHidden(SetFieldHidden { hidden: true, .. })));
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// [E8 plan §5]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetFieldHidden {
    /// The field: a terminal field of the form, by its ID.
    #[serde(serialize_with = "wire::field_id::serialize")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub field: FieldId,
    /// Whether its widgets are hidden.
    pub hidden: bool,
}

impl<'de> Deserialize<'de> for SetFieldHidden {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        Self::from_value(Value::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl SetFieldHidden {
    /// The op that hides `field` (`hidden` true) or shows it.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(field: FieldId, hidden: bool) -> Self {
        Self { field, hidden }
    }

    /// The op from its wire arguments.
    fn from_value(value: Value) -> core::result::Result<Self, String> {
        let mut map = entries(value, "the arguments of setFieldHidden", &["field", "hidden"])?;
        let field = match map.remove("field") {
            Some(Value::String(id)) => id.parse().map_err(|e: laminapdf_model::ParseIdError| e.to_string())?,
            Some(other) => return Err(format!("invalid type: {}, expected a field ID", kind_of(&other))),
            None => return Err("missing field `field`".to_owned()),
        };
        let hidden = match map.remove("hidden") {
            Some(Value::Bool(b)) => b,
            Some(other) => return Err(format!("invalid type: {}, expected a boolean", kind_of(&other))),
            None => return Err("missing field `hidden`".to_owned()),
        };
        Ok(Self::new(field, hidden))
    }

    /// Applies it; returns the pages whose widgets show the field.
    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Vec<PageId>> {
        let form = doc.form()?;
        let field = form
            .field(&self.field)
            .ok_or_else(|| Error::UnknownField(self.field.clone()))?;
        if field.kind == FieldKind::Signature {
            return Err(Error::FieldIsSignature(self.field.clone()));
        }
        let mut pages = Vec::new();
        for widget in &field.widgets {
            let Some(r) = widget.id.as_lamina_id().obj_ref() else {
                continue;
            };
            let Some(mut dict) = tolerant(doc.deref(&Object::Ref(r)))?.and_then(|o| o.as_dict().cloned()) else {
                continue;
            };
            let flags = match dict.get("F") {
                Some(f) => tolerant(doc.deref(f))?.and_then(|o| o.as_int()).unwrap_or(0),
                None => 0,
            };
            let set = if self.hidden { flags | HIDDEN } else { flags & !HIDDEN };
            if set != flags {
                dict.insert("F", Object::Integer(set));
                doc.cos_mut().edit().set(r, Object::Dict(dict))?;
            }
            if let Some(page) = &widget.page
                && !pages.contains(page)
            {
                pages.push(page.clone());
            }
        }
        Ok(pages)
    }
}
