//! `laminapdf explain`: the page of an error or warning code, from `docs/errors/`.

use laminapdf::{ErrorCode, ErrorKind, WarningCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::schema_id;
use crate::exit::CliError;

/// `laminapdf explain <code>`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExplainArgs {
    /// The code, as an error or warning shows it (`xref-repaired`).
    pub code: String,
}

/// Whether a code names an error or a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CodeType {
    /// The call failed.
    Error,
    /// The call went ahead, after a repair or with a default.
    Warning,
}

/// `laminapdf.explain/1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExplainReport {
    /// `laminapdf.explain/1`.
    #[schemars(schema_with = "explain_id")]
    pub schema: &'static str,
    /// The code.
    pub code: String,
    /// Error or warning.
    #[serde(rename = "type")]
    pub code_type: CodeType,
    /// One of the nine kinds of ADR 0026 §2.
    pub kind: String,
    /// What the code means, in one sentence.
    pub summary: String,
    /// What to do about it, in one sentence.
    pub hint: String,
    /// The code's page online.
    pub docs_url: String,
    /// The code's page, as Markdown.
    pub page: String,
}

impl ExplainReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.explain/1";
}

fn explain_id(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schema_id(ExplainReport::SCHEMA)
}

/// The page of `args.code`.
///
/// # Errors
///
/// `unknown-code`, with the known codes within two edits as `didYouMean`.
pub fn explain(args: &ExplainArgs) -> Result<ExplainReport, CliError> {
    let code = args.code.as_str();
    let known = ErrorCode::from_code(code)
        .map(|c| (CodeType::Error, c.kind(), c.summary(), c.hint(), c.docs_url()))
        .or_else(|| {
            WarningCode::from_code(code).map(|c| (CodeType::Warning, c.kind(), c.summary(), c.hint(), c.docs_url()))
        });
    let (Some((code_type, kind, summary, hint, docs_url)), Some(page)) = (known, laminapdf::explain(code)) else {
        let every: Vec<String> = ErrorCode::ALL
            .iter()
            .map(|c| c.as_str())
            .chain(WarningCode::ALL.iter().map(|c| c.as_str()))
            .map(str::to_owned)
            .collect();
        return Err(CliError::new(ErrorCode::UnknownCode, "not a Lamina PDF code")
            .with("code", code)
            .with_did_you_mean(laminapdf::did_you_mean(code, &every)));
    };
    Ok(ExplainReport {
        schema: ExplainReport::SCHEMA,
        code: code.to_owned(),
        code_type,
        kind: kind_name(kind),
        summary: summary.to_owned(),
        hint: hint.to_owned(),
        docs_url,
        page: page.to_owned(),
    })
}

fn kind_name(kind: ErrorKind) -> String {
    kind.as_str().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn explain_code(code: &str) -> Result<ExplainReport, CliError> {
        explain(&ExplainArgs { code: code.to_owned() })
    }

    #[test]
    fn a_warning_code_should_give_its_page() {
        let report = explain_code("xref-repaired").unwrap();
        assert_eq!((report.code_type, report.kind.as_str()), (CodeType::Warning, "input"));
        assert!(report.page.starts_with("# `xref-repaired`"));
        assert!(report.docs_url.ends_with("xref-repaired.md"));
    }

    #[test]
    fn the_clis_own_codes_should_have_pages() {
        let report = explain_code("overwrite-refused").unwrap();
        assert_eq!((report.code_type, report.kind.as_str()), (CodeType::Error, "state"));
    }

    #[test]
    fn a_misspelt_code_should_suggest_the_close_ones() {
        let error = explain_code("xref-repaird").unwrap_err();
        assert_eq!(error.diagnostic().code, "unknown-code");
        assert_eq!(error.diagnostic().did_you_mean, ["xref-repaired"]);
        assert_eq!(error.exit_code(), crate::exit::ExitCode::Usage);
    }
}
