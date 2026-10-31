//! The generated [`ErrorCode`] and [`WarningCode`] tables (`src/generated/codes.rs`, written by
//! `cargo xtask gen codes` from `fixtures/codes.json`) as the façade uses them: the code of a
//! wrapped crate's error or warning, checked at compile time, and the hint with its parameter.

use crate::generated::codes::{ErrorCode, WarningCode};

/// Whether every code in `codes` is in the generated [`ErrorCode`] table.
const fn all_errors_listed(codes: &[&str]) -> bool {
    let mut rest = codes;
    while let [code, tail @ ..] = rest {
        if ErrorCode::from_code(code).is_none() {
            return false;
        }
        rest = tail;
    }
    true
}

/// Whether every code in `codes` is in the generated [`WarningCode`] table.
const fn all_warnings_listed(codes: &[&str]) -> bool {
    let mut rest = codes;
    while let [code, tail @ ..] = rest {
        if WarningCode::from_code(code).is_none() {
            return false;
        }
        rest = tail;
    }
    true
}

// Every code a wrapped crate declares (its `CODES`, which that crate's tests tie to its
// variants' `code()`) is in the generated tables: a code missing from `codes.json`, or a stale
// generated file, fails the build here rather than at run time.
const _: () = assert!(
    all_errors_listed(lamina_cos::Error::CODES),
    "a lamina-cos error code is not in codes.json"
);
const _: () = assert!(
    all_errors_listed(lamina_syntax::Error::CODES),
    "a lamina-syntax error code is not in codes.json"
);
const _: () = assert!(
    all_errors_listed(lamina_ops::Error::CODES),
    "a lamina-ops error code is not in codes.json"
);
const _: () = assert!(
    all_errors_listed(lamina_save::SaveError::CODES),
    "a lamina-save error code is not in codes.json"
);
#[cfg(feature = "text")]
const _: () = assert!(
    all_errors_listed(lamina_text::Error::CODES),
    "a lamina-text error code is not in codes.json"
);
#[cfg(feature = "build")]
const _: () = assert!(
    all_errors_listed(lamina_build::Error::CODES),
    "a lamina-build error code is not in codes.json"
);
const _: () = assert!(
    all_errors_listed(crate::Error::CODES),
    "a façade error code is not in codes.json"
);
const _: () = assert!(
    all_warnings_listed(lamina_cos::Warning::CODES),
    "a lamina-cos warning code is not in codes.json"
);
#[cfg(feature = "text")]
const _: () = assert!(
    all_warnings_listed(lamina_text::Warning::CODES),
    "a lamina-text warning code is not in codes.json"
);

/// The [`ErrorCode`] of a wrapped crate's error code. Every such code is in the table (the
/// assertions above), so the fallback is never taken; it is `unset-object`, kind `internal`,
/// whose hint says to report a lamina bug.
pub(crate) const fn error_code(code: &str) -> ErrorCode {
    match ErrorCode::from_code(code) {
        Some(c) => c,
        None => ErrorCode::UnsetObject,
    }
}

/// The [`WarningCode`] of a wrapped crate's warning code. Every such code is in the table (the
/// assertions above), so the fallback is never taken.
pub(crate) const fn warning_code(code: &str) -> WarningCode {
    match WarningCode::from_code(code) {
        Some(c) => c,
        None => WarningCode::InvalidEntry,
    }
}

/// `template` with `{limit}` filled in (ADR 0026 §5: the only parameters are lamina's own
/// vocabulary). A parameter with no value leaves the hint out rather than show a brace.
pub(crate) fn fill_hint(template: &str, limit: Option<&'static str>) -> Option<String> {
    let filled = match limit {
        Some(limit) => template.replace("{limit}", limit),
        None => template.to_owned(),
    };
    (!filled.contains('{')).then_some(filled)
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, WarningCode, fill_hint};
    use crate::test_codes::code_groups::{NOT_CODE_GROUPS, TS_ONLY_GROUPS};

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

    #[test]
    fn fill_hint_should_leave_out_a_hint_with_an_unfilled_parameter() {
        assert_eq!(
            (
                fill_hint("raise {limit}", Some("max_x")),
                fill_hint("raise {limit}", None)
            ),
            (Some("raise max_x".to_owned()), None)
        );
    }

    /// The generated tables say what `codes.json` says (the generator's `--check` keeps them
    /// in step; this catches a generator bug).
    #[test]
    fn generated_tables_match_codes_json() {
        let codes = crate::test_codes::codes_json();
        let mut listed = Vec::new();
        for (group, entries) in codes.as_object().into_iter().flatten() {
            if NOT_CODE_GROUPS.contains(&group.as_str()) || TS_ONLY_GROUPS.contains(&group.as_str()) {
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
