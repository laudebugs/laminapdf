//! `SetFieldValue`: give a form field a value (ISO 32000-2 §12.7.5): `/V` on the field and
//! the state each widget shows, kept in step.

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{FieldFlags, FieldId, FieldKind, FieldValue, PageId, TerminalField};
use laminapdf_syntax::text::encode_text_string;

use serde_json::Value;

mod appearance;
mod calculate;
mod hidden;

use appearance::Redraw;
pub use calculate::FieldLocks;
pub(crate) use calculate::Locks;
pub use hidden::SetFieldHidden;

use crate::blob::{BlobId, BlobStore};
use crate::error::{Error, Result};
use crate::wire;
use crate::wire::json::{entries, kind_of, quoted};

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
/// rich text is removed, as it would no longer match.
///
/// A text field's, combo box's or list box's widgets are drawn again with the new value, in the
/// font, size and colour its `/DA` sets (ISO 32000-2 §12.7.4.3), through the document's
/// [`Appearances`](crate::Appearances); check boxes and radio buttons switch between the states
/// they already have. A character the standard 14 fonts lack is drawn in the first of `fonts`
/// that has it, a font given by its bytes ([`Document::add_blob`](crate::Document::add_blob)),
/// embedded and added to the form's `/DR`; a value no font draws fails with
/// `annotation-glyph-missing`, and a document without a generator fails with
/// `appearance-unavailable`. Only the changed field's widgets are drawn: the op never sets or
/// clears `/NeedAppearances`.
///
/// The `ReadOnly` flag binds the interface, not the library: calculated fields are usually
/// read-only, and calculation writes them. Read [`TerminalField::read_only`] to follow the
/// interface's rule. `NoToggleToOff` is the interface's too: the op turns such a radio group
/// off when asked, as resetting a form without a default value does (§12.7.6.3).
///
/// On the wire:
/// `{ "op": "setFieldValue", "args": { "field": "f12.0", "value": { "type": "text", "value": "Ada" } } }`;
/// the other values are `{ "type": "button", "value": "Yes" }` (`null` for off),
/// `{ "type": "choice", "value": ["UK"] }` and `{ "type": "clear" }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetFieldValue {
    /// The field: a terminal field of the form, by its ID.
    #[serde(serialize_with = "wire::field_id::serialize")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub field: FieldId,
    /// The value.
    #[serde(serialize_with = "wire::field_value::serialize")]
    #[cfg_attr(feature = "tsify", tsify(type = "FieldValue"))]
    pub value: FieldValue,
    /// Fonts by their bytes' IDs for characters the field's font lacks, tried in order.
    ///
    /// Added in 0.6.0.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
    pub fonts: Vec<BlobId>,
}

impl SetFieldValue {
    /// The op that gives `field` the value `value`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(field: FieldId, value: FieldValue) -> Self {
        Self {
            field,
            value,
            fonts: Vec::new(),
        }
    }

    /// The same op with `fonts` for characters the field's font lacks, by their bytes' IDs.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn with_fonts(mut self, fonts: Vec<BlobId>) -> Self {
        self.fonts = fonts;
        self
    }

    /// Applies it; returns the pages whose widgets show the field.
    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn crate::Appearances>,
        blobs: &BlobStore,
    ) -> Result<Vec<PageId>> {
        let form = doc.form()?;
        let field = form
            .field(&self.field)
            .ok_or_else(|| Error::UnknownField(self.field.clone()))?;
        if field.kind == FieldKind::Signature {
            return Err(Error::FieldIsSignature(self.field.clone()));
        }
        let value = committed(doc, field, &self.value)?;
        let writes = plan(field, &value)?;
        for id in &self.fonts {
            if blobs.get(id).is_none() {
                return Err(Error::UnknownBlob(*id));
            }
        }
        // A value the field already holds changes nothing, so nothing is drawn again either.
        let unchanged = match &value {
            FieldValue::Clear => field.value.is_none(),
            value => field.value.as_ref() == Some(value),
        };
        let redraw = if unchanged {
            None
        } else {
            Redraw::plan(doc, field, &value, &self.fonts, appearances, blobs)?
        };
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
        if let Some(redraw) = redraw {
            redraw.write(doc, &self.fonts, appearances, blobs)?;
        }
        let mut pages: Vec<PageId> = Vec::new();
        for page in field.widgets.iter().filter_map(|w| w.page.as_ref()) {
            if !pages.contains(page) {
                pages.push(page.clone());
            }
        }
        Ok(pages)
    }
}

impl<'de> serde::Deserialize<'de> for SetFieldValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        Self::from_value(Value::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl SetFieldValue {
    const FIELDS: [&'static str; 3] = ["field", "value", "fonts"];
    const VALUE_FIELDS: [&'static str; 2] = ["type", "value"];
    const TYPES: [&'static str; 4] = ["text", "button", "choice", "clear"];

    /// The op from its wire arguments, as serde's derived reader would read them.
    fn from_value(value: Value) -> core::result::Result<Self, String> {
        let mut map = entries(value, "the arguments of setFieldValue", &Self::FIELDS)?;
        let field = match map.remove("field") {
            Some(Value::String(id)) => id.parse().map_err(|e: laminapdf_model::ParseIdError| e.to_string())?,
            Some(other) => return Err(format!("invalid type: {}, expected a field ID", kind_of(&other))),
            None => return Err("missing field `field`".to_owned()),
        };
        let value = map.remove("value").ok_or("missing field `value`")?;
        let fonts = match map.remove("fonts") {
            None => Vec::new(),
            Some(Value::Array(items)) => items
                .into_iter()
                .map(|item| match item {
                    Value::String(id) => id.parse().map_err(|_| "a font must be a blob ID".to_owned()),
                    other => Err(format!("invalid type: {}, expected a blob ID", kind_of(&other))),
                })
                .collect::<core::result::Result<_, _>>()?,
            Some(other) => return Err(format!("invalid type: {}, expected a sequence", kind_of(&other))),
        };
        Ok(Self::new(field, Self::value_from(value)?).with_fonts(fonts))
    }

    /// A [`FieldValue`] from `{ "type": …, "value": … }`.
    fn value_from(value: Value) -> core::result::Result<FieldValue, String> {
        let mut map = entries(value, "a field value", &Self::VALUE_FIELDS)?;
        let content = map.remove("value");
        let text = |v: Value| match v {
            Value::String(s) => Ok(s),
            other => Err(format!("invalid type: {}, expected a string", kind_of(&other))),
        };
        let missing = || "missing field `value`".to_owned();
        match map.remove("type") {
            Some(Value::String(t)) if t == "text" => Ok(FieldValue::Text(text(content.ok_or_else(missing)?)?)),
            Some(Value::String(t)) if t == "button" => match content.ok_or_else(missing)? {
                Value::Null => Ok(FieldValue::Button(None)),
                other => Ok(FieldValue::Button(Some(text(other)?))),
            },
            Some(Value::String(t)) if t == "choice" => match content.ok_or_else(missing)? {
                Value::Array(items) => Ok(FieldValue::Choice(
                    items.into_iter().map(text).collect::<core::result::Result<_, _>>()?,
                )),
                other => Err(format!("invalid type: {}, expected a sequence", kind_of(&other))),
            },
            Some(Value::String(t)) if t == "clear" && content.is_none() => Ok(FieldValue::Clear),
            Some(Value::String(t)) if t == "clear" => Err("unknown field `value`: a clear value has none".to_owned()),
            Some(Value::String(t)) => Err(format!(
                "unknown variant `{t}`, expected one of {}",
                quoted(&Self::TYPES)
            )),
            Some(other) => Err(format!("invalid type: {}, expected a string", kind_of(&other))),
            None => Err("missing field `type`".to_owned()),
        }
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
fn plan(field: &TerminalField, value: &FieldValue) -> Result<Writes> {
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

/// The error for a value that breaks `reason`'s rule.
const fn invalid(reason: &'static str) -> Error {
    Error::FieldValueInvalid { reason }
}

/// The on-state `value` names: one a widget has, else, for a button with `/Opt`, the on-state
/// of the widget at the index of that export value.
fn on_state(field: &TerminalField, value: &str) -> Option<String> {
    // `Off` is the off state, never an on-state (§12.7.5.2.3).
    if value == "Off" {
        return None;
    }
    // A check box whose widgets have no appearance yet gains the on-state the value names. A
    // radio group's widgets need an on-state each to be told apart, which a value can't give.
    let fresh = field.on_states().is_empty() && field.kind == FieldKind::CheckBox;
    if fresh || field.on_states().contains(&value) {
        return Some(value.to_owned());
    }
    let index = field.options.iter().position(|o| o.export == value)?;
    field.widgets.get(index)?.on_state.clone()
}

/// Each widget's state for `on` (`None` for off): a check box turns on every widget with that
/// on-state, a radio group every one with `RadiosInUnison` and only the first without.
fn button_states(field: &TerminalField, on: Option<&str>) -> Vec<(laminapdf_model::AnnotId, String)> {
    if !matches!(field.kind, FieldKind::CheckBox | FieldKind::RadioGroup) {
        return Vec::new();
    }
    let only_first = field.kind == FieldKind::RadioGroup && !field.flags.has(FieldFlags::RADIOS_IN_UNISON);
    // A field whose widgets have no appearance yet: each gains the on-state the value names.
    let fresh = field.on_states().is_empty();
    let mut turned_on = false;
    field
        .widgets
        .iter()
        .map(|w| {
            let on_here = on.is_some() && (fresh || w.on_state.as_deref() == on) && !(only_first && turned_on);
            turned_on |= on_here;
            let state = if on_here { on.unwrap_or("Off") } else { "Off" };
            (w.id.clone(), state.to_owned())
        })
        .collect()
}

/// The writes for a list or combo box's choice of `values`.
fn choice(field: &TerminalField, values: &[String]) -> Result<Writes> {
    let multi = field.kind == FieldKind::ListBox && field.flags.has(FieldFlags::MULTI_SELECT);
    let free_text = field.kind == FieldKind::ComboBox && field.flags.has(FieldFlags::EDIT);
    if (values.len() > 1 && !multi) || (values.is_empty() && field.kind == FieldKind::ComboBox) {
        return Err(invalid("count"));
    }
    if !free_text && values.iter().any(|v| !field.options.iter().any(|o| &o.export == v)) {
        return Err(invalid("option"));
    }
    // The chosen options' indices, in `/Opt` order (§12.7.5.4: sorted ascending).
    let indices: Vec<usize> = (0..field.options.len())
        .filter(|&i| field.options.get(i).is_some_and(|o| values.contains(&o.export)))
        .collect();
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

/// The value a text field stores for `value`: what its keystroke and validate helpers commit
/// (a comma-point number with a point), or a refusal; other values as they are.
fn committed(doc: &laminapdf_model::Document, field: &TerminalField, value: &FieldValue) -> Result<FieldValue> {
    let FieldValue::Text(text) = value else {
        return Ok(value.clone());
    };
    if !matches!(field.kind, FieldKind::Text | FieldKind::ComboBox) {
        return Ok(value.clone());
    }
    match doc.field_behaviour(field)?.commit(text) {
        Ok(v) => Ok(FieldValue::Text(v)),
        Err(laminapdf_model::InvalidValue::OutOfRange(_)) => Err(Error::FieldValueInvalid { reason: "range" }),
        Err(_) => Err(Error::FieldValueInvalid { reason: "format" }),
    }
}
