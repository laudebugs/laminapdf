//! Errors across the boundary: always the [ADR 0026] §1 object (a plain JS object, never a
//! string), which `@laminapdf/sdk` turns into its `LaminaPdfError` (ADR 0026 §7).
//!
//! Core errors are [`laminapdf::Error::diagnostic`] as it is. The binding adds the codes only it
//! can raise (listed under `bindingErrors` in `fixtures/codes.json`): `invalid-argument` (a
//! value from JavaScript of the wrong shape), `unknown-option` (a key the record doesn't have,
//! with `didYouMean`; ADR 0028 F6), `builder-finished` (an owned page or the builder used after
//! the builder's `finish`) and `source-budget-exceeded` (a sparse source given more than its
//! budget). Their kind, hint and `docsUrl` come from the façade's generated
//! [`laminapdf::ErrorCode`] table, like every other code's.
//!
//! [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md

use serde_json::{Map, Value, json};
use wasm_bindgen::JsValue;

use laminapdf::{Diagnostic, ErrorCode};

use crate::convert::to_js;

/// What every binding call returns: the value, or the §1 object thrown in JavaScript.
pub(crate) type JsResult<T> = Result<T, JsValue>;

/// A core error as the §1 object.
pub(crate) fn lamina_error(e: &laminapdf::Error) -> JsValue {
    to_js(&e.diagnostic())
}

/// `data` of a binding error.
struct BindingData {
    /// The parameter that was wrong (Lamina PDF's own vocabulary).
    argument: &'static str,
    /// The key given (**untrusted**: caller text), for `unknown-option`.
    option: Option<String>,
    /// What the conversion said (**untrusted**: it may quote the value), for `invalid-argument`.
    detail: Option<String>,
}

impl BindingData {
    fn into_map(self) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert("argument".to_owned(), json!(self.argument));
        if let Some(option) = self.option {
            data.insert("option".to_owned(), json!(option));
        }
        if let Some(detail) = self.detail {
            data.insert("detail".to_owned(), json!(detail));
        }
        data
    }
}

/// The §1 object of a binding error. `message` and `hint` are fixed text; what the caller
/// passed appears only in `data` (ADR 0026 §4).
fn binding(code: ErrorCode, message: &'static str, did_you_mean: Vec<String>, data: BindingData) -> JsValue {
    to_js(
        &Diagnostic::error(code, message)
            .with_did_you_mean(did_you_mean)
            .with_data(data.into_map()),
    )
}

/// `builder-finished`: a builder or page was used after its `finish`.
#[cfg(feature = "build")]
pub(crate) fn builder_finished() -> JsValue {
    binding(
        ErrorCode::BuilderFinished,
        "the builder or page has already finished",
        Vec::new(),
        BindingData {
            argument: "builder",
            option: None,
            detail: None,
        },
    )
}

/// `invalid-argument`: `argument` isn't the shape its type declares; `did_you_mean` lists
/// the closest allowed values when it names an unknown one.
pub(crate) fn invalid_argument(argument: &'static str, detail: String, did_you_mean: Vec<String>) -> JsValue {
    binding(
        ErrorCode::InvalidArgument,
        "an argument does not have the type its declaration describes",
        did_you_mean,
        BindingData {
            argument,
            option: None,
            detail: Some(detail),
        },
    )
}

/// `source-budget-exceeded`: a sparse source would hold more than its budget (`detail` says how
/// much; Lamina PDF's own numbers).
pub(crate) fn source_budget_exceeded(detail: String) -> JsValue {
    binding(
        ErrorCode::SourceBudgetExceeded,
        "the source would hold more bytes than its budget allows",
        Vec::new(),
        BindingData {
            argument: "bytes",
            option: None,
            detail: Some(detail),
        },
    )
}

/// `unknown-option`: `argument` has a key `option` (when it is a top-level key) none of
/// `known` is.
pub(crate) fn unknown_option(argument: &'static str, option: Option<String>, known: &[String]) -> JsValue {
    let did_you_mean = option.as_deref().map(|o| did_you_mean(o, known)).unwrap_or_default();
    binding(
        ErrorCode::UnknownOption,
        "an options record has a key lamina does not know",
        did_you_mean,
        BindingData {
            argument,
            option,
            detail: None,
        },
    )
}

/// The known names within two edits of `given` (case-insensitive), closest first.
pub(crate) fn did_you_mean(given: &str, known: &[String]) -> Vec<String> {
    let given = given.to_ascii_lowercase();
    let mut close: Vec<(usize, &String)> = known
        .iter()
        .map(|k| (distance(&given, &k.to_ascii_lowercase()), k))
        .filter(|(d, _)| *d <= 2)
        .collect();
    close.sort_unstable();
    close.into_iter().map(|(_, k)| k.clone()).collect()
}

/// Levenshtein distance over chars.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = i;
        let mut next = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let above = row.get(j + 1).copied().unwrap_or(usize::MAX);
            let left = next.last().copied().unwrap_or(usize::MAX);
            let cost = diagonal + usize::from(ca != *cb);
            next.push(cost.min(above.saturating_add(1)).min(left.saturating_add(1)));
            diagonal = above;
        }
        row = next;
    }
    row.last().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::did_you_mean;

    /// Every `bindingErrors` code is in the generated `ErrorCode` table the binding takes its
    /// kind and hint from (the façade's tests check that table against `codes.json`).
    #[test]
    fn binding_codes_are_in_the_generated_table() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/codes.json");
        let codes: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let missing: Vec<&str> = codes["bindingErrors"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["code"].as_str())
            .filter(|c| laminapdf::ErrorCode::from_code(c).is_none())
            .collect();
        assert_eq!(missing, Vec::<&str>::new());
    }

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn did_you_mean_finds_a_misspelt_key() {
        assert_eq!(
            did_you_mean("pasword", &names(&["password", "strict", "limits"])),
            ["password"]
        );
    }

    #[test]
    fn did_you_mean_is_empty_for_a_far_key() {
        assert_eq!(
            did_you_mean("colour", &names(&["password", "strict"])),
            Vec::<String>::new()
        );
    }
}
