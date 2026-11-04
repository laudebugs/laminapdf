//! Errors across the boundary: always the [ADR 0026] §1 object (a plain JS object, never a
//! string), which `@laminapdf/sdk` turns into its `LaminaPdfError` (ADR 0026 §7).
//!
//! Core errors are [`laminapdf::Error::diagnostic`] as it is. The binding adds the codes only it
//! can raise (listed under `bindingErrors` in `fixtures/codes.json`): `invalid-argument` (a
//! value from JavaScript of the wrong shape), `unknown-option` (a key the record doesn't have,
//! with `didYouMean`; ADR 0028 F6), `builder-finished` (an owned page or the builder used after
//! the builder's `finish`), `source-budget-exceeded` (a sparse source given more than its
//! budget) and `disposed` (a page or save job used after its document was freed). Their kind, hint and `docsUrl` come from the façade's generated
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

/// `disposed`: a page or save job was used after its document was freed (no `data`, as when
/// `@laminapdf/sdk` raises it).
pub(crate) fn disposed() -> JsValue {
    to_js(&Diagnostic::error(
        ErrorCode::Disposed,
        "the page or save job's document has been freed",
    ))
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

/// The known names within two edits of `given`, closest first; names at the same distance in
/// code-point order. Edits count chars, after lowercasing ASCII letters only
/// (`fixtures/did-you-mean.json` pins the rule for both languages).
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

    /// One case of `fixtures/did-you-mean.json`.
    #[derive(serde::Deserialize)]
    struct Case {
        about: String,
        given: String,
        known: Vec<String>,
        expected: Vec<String>,
    }

    /// Every `bindingErrors` code is in the generated `ErrorCode` table the binding takes its
    /// kind and hint from (the façade's tests check that table against `codes.json`).
    #[test]
    fn binding_codes_are_in_the_generated_table() {
        let codes = laminapdf_test_fixtures::codes::codes_json();
        let missing: Vec<&str> = codes["bindingErrors"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["code"].as_str())
            .filter(|c| laminapdf::ErrorCode::from_code(c).is_none())
            .collect();
        assert_eq!(missing, Vec::<&str>::new());
    }

    /// Every case of `fixtures/did-you-mean.json`, which `@laminapdf/sdk`'s
    /// `test/did-you-mean.test.ts` runs too, so both languages suggest the same names. Every
    /// failing case is listed, as Vitest's `it.each` lists them.
    #[test]
    fn did_you_mean_gives_the_shared_fixture_s_names() {
        let fixture = laminapdf_test_fixtures::json("did-you-mean.json");
        let cases: Vec<Case> = serde_json::from_value(fixture["cases"].clone()).unwrap();
        let failing: Vec<(String, Vec<String>, Vec<String>)> = cases
            .into_iter()
            .map(|case| (did_you_mean(&case.given, &case.known), case))
            .filter(|(suggested, case)| *suggested != case.expected)
            .map(|(suggested, case)| (case.about, suggested, case.expected))
            .collect();
        assert!(failing.is_empty(), "(about, suggested, expected): {failing:#?}");
    }
}
