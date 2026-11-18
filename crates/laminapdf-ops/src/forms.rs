//! `SetFieldValue`: give a form field a value (ISO 32000-2 §12.7.5): `/V` on the field and
//! the state each widget shows, kept in step.

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{Field, FieldFlags, FieldId, FieldKind, FieldValue, PageId};
use laminapdf_syntax::text::encode_text_string;

use crate::error::{Error, Result};
use crate::wire;

/// Sets a form field's value, or clears it ([`FieldValue::Clear`]).
///
/// The value is checked against the field: text for a text field, no longer than `/MaxLen`;
/// for a check box or radio group, an on-state one of its widgets has, or `None` for off (a
/// button with `/Opt` also takes an export value, mapped to the widget at that index,
/// §12.7.5.2.4); for a list or combo box, export values from `/Opt` (an editable combo box takes
/// any text), several only for a multi-select list box. A value that doesn't fit fails with
/// `field-value-invalid`, a push button always does, and a signature field fails with
/// `field-is-signature`.
///
/// The op writes `/V` on the field (on each object when several share its name) and keeps the
/// rest in step: each button widget's `/AS` (with `RadiosInUnison`, every widget of the chosen
/// on-state turns on; without it, only the first), a list box's `/I`, and a text field's `/RV`
/// rich text is removed, as it would no longer match. It doesn't draw the widgets again.
///
/// The `ReadOnly` flag binds the interface, not the library: calculated fields are usually
/// read-only, and calculation writes them. Read [`Field::read_only`] to follow the
/// interface's rule.
///
/// On the wire:
/// `{ "op": "setFieldValue", "args": { "field": "f12.0", "value": { "type": "text", "value": "Ada" } } }`;
/// the other values are `{ "type": "button", "value": "Yes" }` (`null` for off),
/// `{ "type": "choice", "value": ["UK"] }` and `{ "type": "clear" }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetFieldValue {
    /// The field: a terminal field of the form, by its ID.
    #[serde(with = "wire::field_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub field: FieldId,
    /// The value.
    #[serde(with = "wire::field_value")]
    #[cfg_attr(feature = "tsify", tsify(type = "FieldValue"))]
    pub value: FieldValue,
}

impl SetFieldValue {
    /// The op that gives `field` the value `value`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(field: FieldId, value: FieldValue) -> Self {
        Self { field, value }
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
        let writes = plan(field, &self.value)?;
        let objects = std::iter::once(&field.id).chain(&field.shared);
        let mut edits: Vec<(ObjRef, Edit)> = Vec::new();
        for id in objects {
            if let Some(r) = id.as_lamina_id().obj_ref() {
                edits.push((r, Edit::Field(writes.field.clone())));
            }
        }
        for (widget, state) in &writes.states {
            if let Some(r) = widget.as_lamina_id().obj_ref() {
                edits.push((r, Edit::State(state.clone())));
            }
        }
        for (r, edit) in edits {
            let Some(mut dict) =
                laminapdf_cos::tolerant(doc.deref(&Object::Ref(r)))?.and_then(|o| o.as_dict().cloned())
            else {
                continue;
            };
            let before = dict.clone();
            edit.apply(&mut dict);
            if dict != before {
                doc.cos_mut().edit().set(r, Object::Dict(dict))?;
            }
        }
        let mut pages: Vec<PageId> = field.widgets.iter().filter_map(|w| w.page.clone()).collect();
        pages.sort();
        pages.dedup();
        Ok(pages)
    }
}

/// What to write in each field object.
#[derive(Debug, Clone, PartialEq)]
struct FieldWrite {
    /// `/V`, or `None` to remove it.
    v: Option<Object>,
    /// `/I`.
    i: Indices,
    /// Whether `/RV` goes.
    drop_rv: bool,
}

/// What happens to a list box's `/I`, the indices of the chosen options.
#[derive(Debug, Clone, PartialEq)]
enum Indices {
    /// Left as it is (not a list box).
    Keep,
    /// Written.
    Set(Object),
    /// Removed.
    Remove,
}

/// One object's change.
enum Edit {
    Field(FieldWrite),
    /// A widget's `/AS`.
    State(String),
}

impl Edit {
    fn apply(self, dict: &mut Dict) {
        match self {
            Self::Field(w) => {
                match w.v {
                    Some(v) => {
                        dict.insert("V", v);
                    }
                    None => {
                        dict.remove("V");
                    }
                }
                match w.i {
                    Indices::Set(i) => {
                        dict.insert("I", i);
                    }
                    Indices::Remove => {
                        dict.remove("I");
                    }
                    Indices::Keep => {}
                }
                if w.drop_rv {
                    dict.remove("RV");
                }
            }
            Self::State(state) => {
                dict.insert("AS", Object::name(&state));
            }
        }
    }
}

/// The writes a value makes: the field's entries and each button widget's state.
struct Writes {
    field: FieldWrite,
    states: Vec<(laminapdf_model::AnnotId, String)>,
}

/// Checks `value` against `field` and works out the writes.
fn plan(field: &Field, value: &FieldValue) -> Result<Writes> {
    let invalid = |reason| Error::FieldValueInvalid { reason };
    let keep = |v| FieldWrite {
        v,
        i: Indices::Keep,
        drop_rv: false,
    };
    match (field.kind, value) {
        (FieldKind::PushButton, _) => Err(invalid("kind")),
        (FieldKind::Text, FieldValue::Text(text)) => {
            if field
                .max_len
                .is_some_and(|max| text.chars().count() > usize::try_from(max).unwrap_or(usize::MAX))
            {
                return Err(invalid("max-len"));
            }
            Ok(Writes {
                field: FieldWrite {
                    drop_rv: true,
                    ..keep(Some(text_string(text)))
                },
                states: Vec::new(),
            })
        }
        (FieldKind::CheckBox | FieldKind::RadioGroup, FieldValue::Button(state)) => {
            let state = match state {
                Some(s) => Some(on_state(field, s).ok_or_else(|| invalid("state"))?),
                None => None,
            };
            Ok(Writes {
                field: keep(Some(Object::name(state.as_deref().unwrap_or("Off")))),
                states: button_states(field, state.as_deref()),
            })
        }
        (FieldKind::ComboBox | FieldKind::ListBox, FieldValue::Choice(values)) => choice(field, values),
        (_, FieldValue::Clear) => Ok(Writes {
            field: FieldWrite {
                v: None,
                i: if field.kind == FieldKind::ListBox {
                    Indices::Remove
                } else {
                    Indices::Keep
                },
                drop_rv: field.kind == FieldKind::Text,
            },
            states: button_states(field, None),
        }),
        _ => Err(invalid("kind")),
    }
}

/// The on-state `value` names: one a widget has, else, for a button with `/Opt`, the on-state
/// of the widget at the index of that export value.
fn on_state(field: &Field, value: &str) -> Option<String> {
    if field.on_states().contains(&value) {
        return Some(value.to_owned());
    }
    let index = field.options.iter().position(|o| o.export == value)?;
    field.widgets.get(index)?.on_state.clone()
}

/// Each widget's state for `on` (`None` for off): a check box turns on every widget with that
/// on-state, a radio group every one with `RadiosInUnison` and only the first without.
fn button_states(field: &Field, on: Option<&str>) -> Vec<(laminapdf_model::AnnotId, String)> {
    if !matches!(field.kind, FieldKind::CheckBox | FieldKind::RadioGroup) {
        return Vec::new();
    }
    let only_first = field.kind == FieldKind::RadioGroup && !field.flags.has(FieldFlags::RADIOS_IN_UNISON);
    let mut turned_on = false;
    field
        .widgets
        .iter()
        .map(|w| {
            let on_here = on.is_some() && w.on_state.as_deref() == on && !(only_first && turned_on);
            turned_on |= on_here;
            let state = if on_here { on.unwrap_or("Off") } else { "Off" };
            (w.id.clone(), state.to_owned())
        })
        .collect()
}

/// The writes for a list or combo box's choice of `values`.
fn choice(field: &Field, values: &[String]) -> Result<Writes> {
    let invalid = |reason| Error::FieldValueInvalid { reason };
    let multi = field.kind == FieldKind::ListBox && field.flags.has(FieldFlags::MULTI_SELECT);
    let free_text = field.kind == FieldKind::ComboBox && field.flags.has(FieldFlags::EDIT);
    if (values.len() > 1 && !multi) || (values.is_empty() && field.kind == FieldKind::ComboBox) {
        return Err(invalid("count"));
    }
    let mut indices = Vec::with_capacity(values.len());
    for value in values {
        match field.options.iter().position(|o| &o.export == value) {
            Some(i) => indices.push(i),
            None if free_text => {}
            None => return Err(invalid("option")),
        }
    }
    indices.sort_unstable();
    indices.dedup();
    let v = match values {
        [] => None,
        [one] => Some(text_string(one)),
        many => Some(Object::Array(many.iter().map(|s| text_string(s)).collect())),
    };
    let i = if field.kind != FieldKind::ListBox {
        Indices::Keep
    } else if multi && !indices.is_empty() {
        Indices::Set(Object::Array(
            indices
                .iter()
                .map(|&i| Object::Integer(i64::try_from(i).unwrap_or(i64::MAX)))
                .collect(),
        ))
    } else {
        Indices::Remove
    };
    Ok(Writes {
        field: FieldWrite { v, i, drop_rv: false },
        states: Vec::new(),
    })
}

/// `text` as a text string (§7.9.2.2): PDFDocEncoding when it can hold it, else UTF-16BE.
fn text_string(text: &str) -> Object {
    Object::String(PdfString::literal(encode_text_string(text)))
}
