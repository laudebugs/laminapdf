//! Exit codes and the CLI's error ([ADR 0031] §5, §8).
//!
//! Every failure is a [`CliError`]: an ADR 0026 diagnostic with a code from
//! `fixtures/codes.json`, whether the `laminapdf` façade raised it or the CLI did
//! (`cliErrors`). Its exit code follows from its code alone ([`exit_code_for`]), so a script
//! can branch on either.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use std::fmt;

use laminapdf::{Diagnostic, ErrorCode, ErrorKind};
use serde_json::{Map, Value};

/// The hint for a missing or wrong password, in flags.
const PASSWORD_HINT: &str = "pass the password in a file with --password-file FILE, or set LAMINA_PDF_PASSWORD";

/// How a command ended, as the process's exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExitCode {
    /// `0`: success.
    Ok,
    /// `1`: the document could not be read, written or processed.
    Failure,
    /// `2`: a bad flag, range or argument, or binary output to a terminal.
    Usage,
    /// `3`: success with warnings (`--warnings-ok` makes it `0`).
    Warnings,
    /// `4`: a destructive action refused without `--force`.
    Refused,
    /// `5`: a password is required, or the one given is wrong.
    Password,
    /// `6`: `laminapdf reveal --fail-on` found something in a category it names.
    Found,
}

impl ExitCode {
    /// Every exit code, in numeric order.
    pub const ALL: [Self; 7] = [
        Self::Ok,
        Self::Failure,
        Self::Usage,
        Self::Warnings,
        Self::Refused,
        Self::Password,
        Self::Found,
    ];

    /// The process exit status.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Ok => 0,
            Self::Failure => 1,
            Self::Usage => 2,
            Self::Warnings => 3,
            Self::Refused => 4,
            Self::Password => 5,
            Self::Found => 6,
        }
    }

    /// What the code means, for `laminapdf help exit-codes`.
    #[must_use]
    pub const fn meaning(self) -> &'static str {
        match self {
            Self::Ok => "success",
            Self::Failure => "the document could not be read, written or processed",
            Self::Usage => "a bad flag, range or argument, or binary output to a terminal",
            Self::Warnings => "success with warnings; --warnings-ok turns it into 0",
            Self::Refused => "refused a destructive action that needs --force",
            Self::Password => "a password is required, or the one given is wrong",
            Self::Found => "the file holds something --fail-on named",
        }
    }

    /// Which code wins when several apply: 2 > 5 > 4 > 1 > 6 > 3 > 0.
    const fn priority(self) -> u8 {
        match self {
            Self::Ok => 0,
            Self::Warnings => 1,
            Self::Found => 2,
            Self::Failure => 3,
            Self::Refused => 4,
            Self::Password => 5,
            Self::Usage => 6,
        }
    }

    /// The code of the two that wins.
    #[must_use]
    pub const fn max(self, other: Self) -> Self {
        if other.priority() > self.priority() {
            other
        } else {
            self
        }
    }
}

/// The exit code of an error `code`: `5` for a missing or wrong password, `4` for a refused
/// overwrite, `2` for every code of kind `usage`, and `1` for the rest.
#[must_use]
pub fn exit_code_for(code: ErrorCode) -> ExitCode {
    match code {
        ErrorCode::PasswordRequired | ErrorCode::WrongPassword => ExitCode::Password,
        ErrorCode::OverwriteRefused => ExitCode::Refused,
        _ if code.kind() == ErrorKind::Usage => ExitCode::Usage,
        _ => ExitCode::Failure,
    }
}

/// A command's failure: the ADR 0026 diagnostic it reports (boxed: every `Result` of the
/// command layer carries one).
#[derive(Debug, Clone, PartialEq)]
pub struct CliError {
    diagnostic: Box<Diagnostic>,
    exit: ExitCode,
}

impl CliError {
    /// The CLI's own error `code` with `message`: one lowercase sentence that quotes neither
    /// the document nor the command line (ADR 0026 §4; caller text goes in [`Self::with`]).
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            diagnostic: Box::new(Diagnostic::error(code, message)),
            exit: exit_code_for(code),
        }
    }

    /// The same error with `key` set to `value` in its `data`.
    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        let mut data = std::mem::take(&mut self.diagnostic.data);
        data.insert(key.to_owned(), value.into());
        self.diagnostic = Box::new(self.diagnostic.with_data(data));
        self
    }

    /// The same error with `candidates` as its `didYouMean`.
    #[must_use]
    pub fn with_did_you_mean(mut self, candidates: Vec<String>) -> Self {
        self.diagnostic = Box::new(self.diagnostic.with_did_you_mean(candidates));
        self
    }

    /// The diagnostic, as the ADR 0026 §1 object serialises it.
    #[must_use]
    pub const fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }

    /// The exit code this error ends the command with.
    #[must_use]
    pub const fn exit_code(&self) -> ExitCode {
        self.exit
    }

    /// The line standard error holds under `--json`, which the MCP server returns as a tool
    /// error: `{"schema":"laminapdf.error/1","error":{…}}`, the diagnostic with the `input` (when
    /// there is one) and the `exitCode` (ADR 0031 §8).
    #[must_use]
    pub fn to_line(&self, input: Option<&str>) -> Value {
        let mut error = self.diagnostic.to_json();
        if let Some(map) = error.as_object_mut() {
            if let Some(input) = input {
                map.insert("input".to_owned(), input.into());
            }
            map.insert("exitCode".to_owned(), self.exit.number().into());
        }
        serde_json::json!({ "schema": "laminapdf.error/1", "error": error })
    }

    /// The `data` of the diagnostic.
    #[must_use]
    pub const fn data(&self) -> &Map<String, Value> {
        &self.diagnostic.data
    }
}

impl From<laminapdf::Error> for CliError {
    /// The façade's error, with the hints that name an API (`OpenOptions`, `Limits`) said
    /// in flags.
    fn from(error: laminapdf::Error) -> Self {
        let code = error.error_code();
        let diagnostic = error.diagnostic();
        let hint = match code {
            ErrorCode::PasswordRequired | ErrorCode::WrongPassword => Some(PASSWORD_HINT.to_owned()),
            ErrorCode::LimitExceeded => diagnostic.data.get("limit").and_then(Value::as_str).map(|limit| {
                format!(
                    "the input exceeds a safety limit; raise it with --{} only if you trust the input",
                    limit.replace('_', "-")
                )
            }),
            // `laminapdf build` without fonts sets the standard 14, Latin text only and never embedded.
            ErrorCode::MissingGlyph => Some(
                "a font lacks a character the text uses: give fonts that have it with --font regular=FILE (and bold, italic, mono, fallback)".to_owned(),
            ),
            ErrorCode::FontNotEmbedded => Some(
                "the profile needs embedded fonts: give them with --font regular=FILE (and bold, italic, mono)".to_owned(),
            ),
            _ => None,
        };
        let diagnostic = match hint {
            Some(hint) => diagnostic.with_hint(Some(hint)),
            None => diagnostic,
        };
        Self {
            exit: exit_code_for(code),
            diagnostic: Box::new(diagnostic),
        }
    }
}

impl From<laminapdf_render::RenderError> for CliError {
    /// A renderer's error, with the password hint said in flags as for the façade's.
    fn from(error: laminapdf_render::RenderError) -> Self {
        let code = error.error_code();
        let diagnostic = error.diagnostic();
        let diagnostic = match code {
            ErrorCode::PasswordRequired | ErrorCode::WrongPassword => {
                diagnostic.with_hint(Some(PASSWORD_HINT.to_owned()))
            }
            _ => diagnostic,
        };
        Self {
            exit: exit_code_for(code),
            diagnostic: Box::new(diagnostic),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.diagnostic.message)
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_should_be_zero_to_six() {
        assert_eq!(ExitCode::ALL.map(ExitCode::number), [0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn the_highest_priority_should_win() {
        let order = [
            ExitCode::Usage,
            ExitCode::Password,
            ExitCode::Refused,
            ExitCode::Failure,
            ExitCode::Found,
            ExitCode::Warnings,
            ExitCode::Ok,
        ];
        for (i, high) in order.iter().enumerate() {
            for low in order.iter().skip(i) {
                assert_eq!((high.max(*low), low.max(*high)), (*high, *high));
            }
        }
    }

    // Every code in fixtures/codes.json is an `ErrorCode` (the façade's code test checks that),
    // so this covers every code: passwords exit 5, a refused overwrite 4, usage errors 2.
    #[test]
    fn every_error_code_should_map_to_an_exit_code() {
        for code in ErrorCode::ALL {
            let expected = match code.as_str() {
                "password-required" | "wrong-password" => ExitCode::Password,
                "overwrite-refused" => ExitCode::Refused,
                _ if code.kind() == ErrorKind::Usage => ExitCode::Usage,
                _ => ExitCode::Failure,
            };
            assert_eq!(exit_code_for(*code), expected, "{}", code.as_str());
        }
        assert_eq!(exit_code_for(ErrorCode::LimitExceeded), ExitCode::Failure);
        assert_eq!(exit_code_for(ErrorCode::InvalidPageRange), ExitCode::Usage);
    }

    #[test]
    fn data_should_collect_every_key() {
        let error = CliError::new(ErrorCode::InvalidPageRange, "the document has 5 pages")
            .with("range", "1-9")
            .with("position", 3);
        assert_eq!(error.data().get("position"), Some(&Value::from(3)));
        assert_eq!(error.data().len(), 2);
        assert_eq!(
            (error.to_string(), error.exit_code()),
            ("the document has 5 pages".to_owned(), ExitCode::Usage)
        );
    }
}
