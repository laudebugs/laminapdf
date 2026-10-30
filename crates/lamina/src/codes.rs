//! Hints, summaries and `docsUrl`s, from the generated [`ErrorCode`] and [`WarningCode`]
//! tables (`src/generated/codes.rs`, written by `cargo xtask gen codes` from
//! `fixtures/codes.json`).

use lamina_cos::ErrorKind;

use crate::generated::codes::{DOCS_URL_BASE, DOCS_URL_SUFFIX, ErrorCode, WarningCode};

/// The kind `codes.json` gives `code`. Every code a façade type raises is listed (checked by
/// the tests); an unlisted one would be a lamina bug, hence `internal`.
pub(crate) const fn kind(code: &str) -> ErrorKind {
    match (ErrorCode::from_code(code), WarningCode::from_code(code)) {
        (Some(c), _) => c.kind(),
        (None, Some(c)) => c.kind(),
        (None, None) => ErrorKind::Internal,
    }
}

/// The hint template of `code`.
fn hint_template(code: &str) -> Option<&'static str> {
    ErrorCode::from_code(code)
        .map(ErrorCode::hint)
        .or_else(|| WarningCode::from_code(code).map(WarningCode::hint))
}

/// The hint of `code` with `{limit}` filled in (ADR 0026 §5: the only parameters are lamina's
/// own vocabulary). A parameter with no value leaves the hint out rather than show a brace.
pub(crate) fn hint(code: &str, limit: Option<&'static str>) -> Option<String> {
    let template = hint_template(code)?;
    let filled = match limit {
        Some(limit) => template.replace("{limit}", limit),
        None => template.to_owned(),
    };
    (!filled.contains('{')).then_some(filled)
}

/// `base + code + suffix` (ADR 0026 §6).
pub(crate) fn docs_url(code: &str) -> String {
    format!("{DOCS_URL_BASE}{code}{DOCS_URL_SUFFIX}")
}

/// The summary of `code`: a fixed sentence, used as a warning's `message`.
pub(crate) fn summary(code: &str) -> Option<&'static str> {
    WarningCode::from_code(code)
        .map(WarningCode::summary)
        .or_else(|| ErrorCode::from_code(code).map(ErrorCode::summary))
}

/// `a == b`, usable in `const fn` (the generated `from_code`).
pub(crate) const fn same(a: &str, b: &str) -> bool {
    let (mut a, mut b) = (a.as_bytes(), b.as_bytes());
    loop {
        match (a, b) {
            ([x, rest_a @ ..], [y, rest_b @ ..]) => {
                if *x != *y {
                    return false;
                }
                a = rest_a;
                b = rest_b;
            }
            ([], []) => return true,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, WarningCode, same};

    #[test]
    fn same_should_compare_whole_strings() {
        assert_eq!(
            (same("abc", "abc"), same("abc", "abd"), same("ab", "abc"), same("", "")),
            (true, false, false, true)
        );
    }

    #[test]
    fn from_code_should_round_trip_every_code() {
        assert!(
            ErrorCode::ALL
                .iter()
                .all(|c| ErrorCode::from_code(c.as_str()) == Some(*c))
                && WarningCode::ALL
                    .iter()
                    .all(|c| WarningCode::from_code(c.as_str()) == Some(*c))
        );
    }

    #[test]
    fn from_code_should_return_none_for_an_unknown_code() {
        assert_eq!(ErrorCode::from_code("not-a-code"), None);
    }

    /// The generated tables say what `codes.json` says (the generator's `--check` keeps them
    /// in step; this catches a generator bug).
    #[test]
    fn generated_tables_match_codes_json() {
        let codes = crate::test_codes::codes_json();
        let mut listed = Vec::new();
        for (group, entries) in codes.as_object().into_iter().flatten() {
            if ["version", "docsUrl", "limits", "packageErrors"].contains(&group.as_str()) {
                continue;
            }
            for e in entries.as_array().into_iter().flatten() {
                let field = |k: &str| e[k].as_str().unwrap_or_default().to_owned();
                listed.push((field("code"), field("kind"), field("summary"), field("hint")));
            }
        }
        listed.sort();
        let mut generated: Vec<_> = ErrorCode::ALL
            .iter()
            .map(|c| (c.as_str(), c.kind(), c.summary(), c.hint()))
            .chain(
                WarningCode::ALL
                    .iter()
                    .map(|c| (c.as_str(), c.kind(), c.summary(), c.hint())),
            )
            .map(|(c, k, s, h)| (c.to_owned(), k.as_str().to_owned(), s.to_owned(), h.to_owned()))
            .collect();
        generated.sort();
        assert_eq!(generated, listed);
    }
}
