//! Records across the boundary as JSON (ADR 0016 §7): a record from JavaScript is
//! `JSON.stringify`'d and read with `serde_json`, a record to JavaScript is written with
//! `serde_json` and `JSON.parse`'d. The core already links `serde_json` (op logs, the
//! ADR 0026 object), so this costs a fraction of `serde-wasm-bindgen`'s generic
//! deserialiser (measured in ADR 0016 §5). Records stay plain objects on the JS side.

use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsValue;

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
    serde_json::from_str(&json).map_err(|e| conversion_error(argument, &e.to_string()))
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

/// Maps serde's message to the binding's codes. serde words unknown keys and variants as
/// "unknown field `x`, expected one of `a`, `b`" ("expected `a`" for one, "there are no
/// fields" for none).
pub(crate) fn conversion_error(argument: &'static str, message: &str) -> JsValue {
    if let Some((given, known)) = unknown_name(message, "unknown field `") {
        return unknown_option(argument, given, &known);
    }
    let did_you_mean = unknown_name(message, "unknown variant `")
        .map(|(given, known)| crate::error::did_you_mean(&given, &known))
        .unwrap_or_default();
    invalid_argument(argument, message.to_owned(), did_you_mean)
}

/// The name after `prefix` and the backquoted names serde says it expected.
fn unknown_name(message: &str, prefix: &str) -> Option<(String, Vec<String>)> {
    let rest = message.strip_prefix(prefix)?;
    let (given, expected) = rest.split_once('`')?;
    let known = expected.split('`').skip(1).step_by(2).map(str::to_owned).collect();
    Some((given.to_owned(), known))
}

#[cfg(test)]
mod tests {
    use super::unknown_name;

    #[test]
    fn unknown_name_reads_serdes_message() {
        assert_eq!(
            unknown_name(
                "unknown field `pasword`, expected one of `password`, `strict`",
                "unknown field `"
            ),
            Some(("pasword".to_owned(), vec!["password".to_owned(), "strict".to_owned()]))
        );
    }

    #[test]
    fn unknown_name_is_none_for_other_messages() {
        assert_eq!(
            unknown_name("invalid type: string, expected u32", "unknown field `"),
            None
        );
    }
}
