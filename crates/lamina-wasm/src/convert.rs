//! Records across the boundary as JSON (ADR 0016 §7): a record from JavaScript is
//! `JSON.stringify`'d and read with `serde_json`, a record to JavaScript is written with
//! `serde_json` and `JSON.parse`'d. The core already links `serde_json` (op logs, the
//! ADR 0026 object), so this costs a fraction of `serde-wasm-bindgen`'s generic
//! deserialiser (measured in ADR 0016 §5). Records stay plain objects on the JS side.

use std::cell::Cell;

use serde::Serialize;
use serde::de::{DeserializeOwned, Deserializer, Visitor};
use wasm_bindgen::{JsCast as _, JsValue};

use crate::error::{JsResult, invalid_argument, unknown_option};

/// `value` as a plain JS value. Our records always serialise; `undefined` stands in should
/// one ever fail, rather than a panic across the boundary (F11).
pub(crate) fn to_js<T: Serialize + ?Sized>(value: &T) -> JsValue {
    serde_json::to_string(value)
        .ok()
        .and_then(|json| js_sys::JSON::parse(&json).ok())
        .unwrap_or(JsValue::UNDEFINED)
}

/// `value` read as `T`; `argument` names the parameter in the error.
///
/// # Errors
/// `unknown-option` for a key the record doesn't have, `invalid-argument` for anything else
/// (including values JSON can't hold: functions, `BigInt`s, cycles).
pub(crate) fn from_js<T: DeserializeOwned>(value: &JsValue, argument: &'static str) -> JsResult<T> {
    let json = js_sys::JSON::stringify(value)
        .ok()
        .and_then(|s| s.as_string())
        .ok_or_else(|| invalid_argument(argument, "the value can't be written as JSON".to_owned(), Vec::new()))?;
    serde_json::from_str(&json).map_err(|e| conversion_error::<T>(value, argument, &e))
}

/// [`from_js`], with `undefined` and `null` meaning the record's defaults.
///
/// # Errors
/// As [`from_js`].
pub(crate) fn from_js_or_default<T: DeserializeOwned + Default>(
    value: &JsValue,
    argument: &'static str,
) -> JsResult<T> {
    if value.is_undefined() || value.is_null() {
        Ok(T::default())
    } else {
        from_js(value, argument)
    }
}

/// The binding error for a value `T` refused. An unknown key is `unknown-option`; the key and
/// the suggestions come from the value's own keys and `T`'s field list, never from serde's
/// message (which quotes the caller's key: ADR 0026 §4 keeps caller text out of `didYouMean`).
fn conversion_error<T: DeserializeOwned>(value: &JsValue, argument: &'static str, e: &serde_json::Error) -> JsValue {
    let message = e.to_string();
    if !message.starts_with("unknown field") {
        return invalid_argument(argument, message, Vec::new());
    }
    let known = fields_of::<T>();
    let keys = if value.is_object() {
        js_sys::Object::keys(value.unchecked_ref::<js_sys::Object>()).to_vec()
    } else {
        Vec::new()
    };
    let unknown = keys
        .iter()
        .filter_map(JsValue::as_string)
        .find(|k| !known.contains(&k.as_str()));
    // A key unknown to a nested record (the top-level keys are all known): no suggestion.
    let known: Vec<String> = if unknown.is_some() {
        known.iter().map(|k| (*k).to_owned()).collect()
    } else {
        Vec::new()
    };
    unknown_option(argument, unknown, &known)
}

/// The field names a struct's `Deserialize` impl declares (serde passes them to
/// `deserialize_struct`); empty for anything that isn't a struct.
pub(crate) fn fields_of<T: DeserializeOwned>() -> &'static [&'static str] {
    let probe = FieldProbe(Cell::new(&[]));
    let _ = T::deserialize(&probe);
    probe.0.get()
}

/// A deserialiser that only records the field list serde hands it.
struct FieldProbe(Cell<&'static [&'static str]>);

impl<'de> Deserializer<'de> for &FieldProbe {
    type Error = serde::de::value::Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::custom("field probe"))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.set(fields);
        Err(serde::de::Error::custom("field probe"))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option
        unit unit_struct newtype_struct seq tuple tuple_struct map enum identifier ignored_any
    }
}

#[cfg(test)]
mod tests {
    use super::fields_of;
    use crate::dto::OpenOptionsInit;

    #[test]
    fn fields_of_reads_a_records_field_list() {
        assert_eq!(fields_of::<OpenOptionsInit>(), ["password", "strict", "limits"]);
    }

    #[test]
    fn fields_of_is_empty_for_a_list() {
        assert_eq!(fields_of::<Vec<u32>>(), [""; 0]);
    }
}
