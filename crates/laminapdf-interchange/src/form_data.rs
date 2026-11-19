//! Form data against a document's form (E8 plan §2.1): the values a form data file gives, matched
//! to fields by fully qualified name and turned into [`SetFieldValue`] ops; the form's values
//! written out in each format; and the defaults a reset restores.
//!
//! Each format writes a value its own way. A check box with one on-state is `true` or `false`
//! in JSON; XFDF writes the on-state's name, or `Off`; FDF writes a name (`/Yes`, `/Off`). A
//! multi-select list box is always an array in JSON, so a form filled from an export reads the
//! same, and a list with one selection stays a list.

use laminapdf_model::{FieldFlags, FieldId, FieldKind, FieldValue, Form, TerminalField};
use laminapdf_ops::SetFieldValue;

use crate::fields::{DataValue, FieldData, FormDataFormat};
use crate::{InterchangeError, Result, write_form_data};

/// What a form data file would do to a form: the ops that set its values, and the names it
/// can't set.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FormPlan {
    /// One op for each value, in the file's order.
    pub values: Vec<SetFieldValue>,
    /// The names in the file that no field of the form has.
    pub unknown: Vec<String>,
    /// The fields whose value is of a kind they don't take, each with its code:
    /// `field-is-signature` for a signature field, `field-value-invalid` for the rest (a list
    /// for a text field, `true` for a radio group, any value for a push button).
    pub refused: Vec<(FieldId, &'static str)>,
}

/// Matches `fields` to `form`'s fields by fully qualified name.
///
/// Added in 0.7.0.
#[must_use]
pub fn plan_form_data(form: &Form, fields: &[FieldData]) -> FormPlan {
    let mut plan = FormPlan::default();
    for data in fields {
        let Some(field) = form.field_named(&data.name) else {
            plan.unknown.push(data.name.clone());
            continue;
        };
        if field.kind == FieldKind::Signature {
            plan.refused.push((field.id.clone(), "field-is-signature"));
            continue;
        }
        match field_value(field, &data.value) {
            Some(value) => plan.values.push(SetFieldValue::new(field.id.clone(), value)),
            None => plan.refused.push((field.id.clone(), "field-value-invalid")),
        }
    }
    plan
}

/// The value `data` gives `field`, or `None` when it is of a kind the field doesn't take.
fn field_value(field: &TerminalField, data: &DataValue) -> Option<FieldValue> {
    let off = |s: &str| s.is_empty() || s == "Off";
    Some(match (field.kind, data) {
        (_, DataValue::Empty) if is_button(field.kind) => FieldValue::Button(None),
        (_, DataValue::Empty) => FieldValue::Clear,
        (FieldKind::Text, DataValue::Text(t) | DataValue::State(t)) => FieldValue::Text(t.clone()),
        (FieldKind::Text, DataValue::Choices(v)) => match v.as_slice() {
            [one] => FieldValue::Text(one.clone()),
            _ => return None,
        },
        (FieldKind::CheckBox, DataValue::Checked(on)) => match (on, field.on_states().as_slice()) {
            (false, _) => FieldValue::Button(None),
            (true, [state]) => FieldValue::Button(Some((*state).to_owned())),
            (true, _) => return None,
        },
        (FieldKind::CheckBox | FieldKind::RadioGroup, DataValue::Text(s) | DataValue::State(s)) => {
            FieldValue::Button((!off(s)).then(|| s.clone()))
        }
        (FieldKind::RadioGroup, DataValue::Checked(false)) => FieldValue::Button(None),
        (FieldKind::ComboBox | FieldKind::ListBox, DataValue::Text(t) | DataValue::State(t)) => {
            FieldValue::Choice(vec![t.clone()])
        }
        (FieldKind::ComboBox | FieldKind::ListBox, DataValue::Choices(v)) => FieldValue::Choice(v.clone()),
        _ => return None,
    })
}

const fn is_button(kind: FieldKind) -> bool {
    matches!(kind, FieldKind::CheckBox | FieldKind::RadioGroup)
}

/// `form`'s values as a form data file in `format`, naming the PDF (`file`, and `ids`, the
/// file's `/ID`) in XFDF and FDF. Push buttons and signature fields have no value to carry and
/// are left out; every other field is written, an empty one too, so importing the file into a
/// filled copy empties what the original left empty.
///
/// Added in 0.7.0.
#[must_use]
pub fn form_data_of(form: &Form, format: FormDataFormat, file: Option<&str>, ids: Option<[Vec<u8>; 2]>) -> Vec<u8> {
    let fields = form
        .fields
        .iter()
        .filter(|f| !matches!(f.kind, FieldKind::PushButton | FieldKind::Signature))
        .map(|f| FieldData::new(f.name.clone(), data_value(f, format)))
        .collect();
    write_form_data(format, fields, file.map(str::to_owned), ids)
}

/// `field`'s value as `format` writes it.
fn data_value(field: &TerminalField, format: FormDataFormat) -> DataValue {
    let state = |s: Option<&str>| {
        let s = s.unwrap_or("Off").to_owned();
        if format == FormDataFormat::Fdf {
            DataValue::State(s)
        } else {
            DataValue::Text(s)
        }
    };
    let multi = field.kind == FieldKind::ListBox && field.flags.has(FieldFlags::MULTI_SELECT);
    match (&field.value, field.kind) {
        (Some(FieldValue::Button(on)), FieldKind::CheckBox)
            if format == FormDataFormat::Json && field.on_states().len() == 1 =>
        {
            DataValue::Checked(on.is_some())
        }
        (None, FieldKind::CheckBox) if format == FormDataFormat::Json && field.on_states().len() == 1 => {
            DataValue::Checked(false)
        }
        (Some(FieldValue::Button(on)), _) => match on {
            None if format == FormDataFormat::Json => DataValue::Empty,
            on => state(on.as_deref()),
        },
        (None, FieldKind::CheckBox | FieldKind::RadioGroup) if format != FormDataFormat::Json => state(None),
        (Some(FieldValue::Text(t)), _) => DataValue::Text(t.clone()),
        (Some(FieldValue::Choice(v)), _) if multi => DataValue::Choices(v.clone()),
        (Some(FieldValue::Choice(v)), _) => match v.as_slice() {
            [one] => DataValue::Text(one.clone()),
            [] => DataValue::Empty,
            _ => DataValue::Choices(v.clone()),
        },
        _ => DataValue::Empty,
    }
}

/// The ops that give each of `ids` (every field but push buttons and signature fields when
/// `None`) its default value (`/DV`), or none where it has no default (ISO 32000-2 §12.7.6.3).
///
/// # Errors
/// `unknown-field` for an ID the form doesn't have; `field-is-signature` for a signature
/// field, which a reset never touches.
///
/// Added in 0.7.0.
pub fn reset_values(form: &Form, ids: Option<&[FieldId]>) -> Result<Vec<SetFieldValue>> {
    let fields: Vec<&TerminalField> = match ids {
        None => form
            .fields
            .iter()
            .filter(|f| !matches!(f.kind, FieldKind::PushButton | FieldKind::Signature))
            .collect(),
        Some(ids) => ids
            .iter()
            .map(|id| {
                let field = form
                    .field(id)
                    .ok_or_else(|| laminapdf_ops::Error::UnknownField(id.clone()))?;
                if field.kind == FieldKind::Signature {
                    return Err(laminapdf_ops::Error::FieldIsSignature(id.clone()).into());
                }
                Ok::<_, InterchangeError>(field)
            })
            .collect::<Result<_>>()?,
    };
    Ok(fields
        .into_iter()
        .map(|f| {
            let value = match (&f.default_value, is_button(f.kind)) {
                (Some(v), _) => v.clone(),
                (None, true) => FieldValue::Button(None),
                (None, false) => FieldValue::Clear,
            };
            SetFieldValue::new(f.id.clone(), value)
        })
        .collect())
}
