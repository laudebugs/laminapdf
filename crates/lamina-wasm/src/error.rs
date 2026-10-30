//! Errors across the boundary: always the [ADR 0026] §1 object (a plain JS object, never a
//! string), which `@lamina/pdf` turns into its `LaminaError` (ADR 0026 §7).
//!
//! Core errors are [`lamina::Error::diagnostic`] as it is. The binding adds the two codes only
//! it can raise (listed under `bindingErrors` in `fixtures/codes.json`): `invalid-argument`
//! (a value from JavaScript of the wrong shape), `unknown-option` (a key the record doesn't
//! have, with `didYouMean`; ADR 0028 F6) and `builder-finished` (an owned page or the
//! builder used after the builder's `finish`).
//!
//! [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md

use serde::Serialize;
use wasm_bindgen::JsValue;

use crate::convert::to_js;

/// What every binding call returns: the value, or the §1 object thrown in JavaScript.
pub(crate) type JsResult<T> = Result<T, JsValue>;

/// `docsUrl` = base + code + suffix (`codes.json` `docsUrl`; checked by the tests below).
const DOCS_BASE: &str = "https://github.com/laudebugs/lamina/blob/main/docs/errors/";
const DOCS_SUFFIX: &str = ".md";

/// The codes the binding raises (its `bindingErrors` group), with their kind and hint.
pub(crate) const CODES: [(&str, &str, &str); 3] = [
    (
        "builder-finished",
        "state",
        "create a new builder: a finished builder takes no more pages, fonts or settings",
    ),
    (
        "invalid-argument",
        "usage",
        "pass the value the type declarations describe; the data field says what was wrong",
    ),
    (
        "unknown-option",
        "usage",
        "remove the key or correct its spelling; didYouMean lists the closest known names",
    ),
];

/// A core error as the §1 object.
pub(crate) fn lamina_error(e: &lamina::Error) -> JsValue {
    to_js(&e.diagnostic())
}

/// The §1 object of a binding error. `message` and `hint` are fixed text; what the caller
/// passed appears only in `data` (ADR 0026 §4).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BindingDiagnostic {
    code: &'static str,
    kind: &'static str,
    message: &'static str,
    hint: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    did_you_mean: Vec<String>,
    data: BindingData,
    docs_url: String,
}

/// `data` of a binding error.
#[derive(Serialize)]
struct BindingData {
    /// The parameter that was wrong (lamina's own vocabulary).
    argument: &'static str,
    /// The key given (**untrusted**: caller text), for `unknown-option`.
    #[serde(skip_serializing_if = "Option::is_none")]
    option: Option<String>,
    /// What the conversion said (**untrusted**: it may quote the value), for `invalid-argument`.
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

fn binding(code: &'static str, message: &'static str, did_you_mean: Vec<String>, data: BindingData) -> JsValue {
    let (kind, hint) = CODES
        .iter()
        .find(|(c, ..)| *c == code)
        .map_or(("usage", ""), |(_, kind, hint)| (*kind, *hint));
    to_js(&BindingDiagnostic {
        code,
        kind,
        message,
        hint,
        did_you_mean,
        data,
        docs_url: format!("{DOCS_BASE}{code}{DOCS_SUFFIX}"),
    })
}

/// `builder-finished`: the builder was used after its `finish`.
#[cfg(feature = "build")]
pub(crate) fn builder_finished() -> JsValue {
    binding(
        "builder-finished",
        "the builder has already finished",
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
        "invalid-argument",
        "an argument does not have the type its declaration describes",
        did_you_mean,
        BindingData {
            argument,
            option: None,
            detail: Some(detail),
        },
    )
}

/// `unknown-option`: `argument` has a key `option` none of `known` is.
pub(crate) fn unknown_option(argument: &'static str, option: String, known: &[String]) -> JsValue {
    let did_you_mean = did_you_mean(&option, known);
    binding(
        "unknown-option",
        "an options record has a key lamina does not know",
        did_you_mean,
        BindingData {
            argument,
            option: Some(option),
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
    use super::{CODES, DOCS_BASE, DOCS_SUFFIX, did_you_mean};

    fn codes_json() -> serde_json::Value {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/codes.json");
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn binding_codes_match_codes_json() {
        let codes = codes_json();
        let listed: Vec<(String, String, String)> = codes["bindingErrors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let field = |k: &str| e[k].as_str().unwrap().to_owned();
                (field("code"), field("kind"), field("hint"))
            })
            .collect();
        let ours: Vec<(String, String, String)> = CODES
            .iter()
            .map(|(c, k, h)| ((*c).to_owned(), (*k).to_owned(), (*h).to_owned()))
            .collect();
        assert_eq!(ours, listed);
    }

    #[test]
    fn docs_url_parts_match_codes_json() {
        let codes = codes_json();
        assert_eq!(
            (codes["docsUrl"]["base"].as_str(), codes["docsUrl"]["suffix"].as_str()),
            (Some(DOCS_BASE), Some(DOCS_SUFFIX))
        );
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
