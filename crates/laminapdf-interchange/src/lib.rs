//! Import and export a PDF's comments: XFDF (ISO 19444-1) and FDF (ISO 32000-2 §12.7.8) both
//! ways, and Markdown out, for notes apps.
//!
//! An exchange file is read into an [`Exchange`]: the PDF it was made for, and each comment as
//! its annotation dictionary's entries, the way XFDF itself is defined in terms of PDF keys.
//! XFDF is read with `laminapdf_model::xml`'s guarded reader, which refuses a DTD and every
//! entity but XML's five; FDF with `laminapdf-syntax`'s tolerant parser.
//!
//! ```
//! use laminapdf_interchange::{read, write_fdf};
//! use laminapdf_syntax::Limits;
//!
//! let xfdf = br#"<?xml version="1.0" encoding="UTF-8"?>
//! <xfdf xmlns="http://ns.adobe.com/xfdf/"><annots>
//! <text page="0" rect="72,700,92,720" name="c1" title="Ada"><contents>Check this</contents></text>
//! </annots><f href="report.pdf"/></xfdf>"#;
//! let exchange = read(xfdf, &Limits::default())?;
//! assert_eq!((exchange.file.as_deref(), exchange.comments.len()), (Some("report.pdf"), 1));
//! let fdf = write_fdf(&exchange);
//! assert!(fdf.starts_with(b"%FDF-1.2"));
//! # Ok::<(), laminapdf_interchange::InterchangeError>(())
//! ```
//!
//! Added in 0.7.0.

#![forbid(unsafe_code)]

mod exchange;
mod export;
mod fdf;
mod fields;
mod form_data;
mod import;
mod json;
mod lines;
mod map;
mod markdown;
mod xfdf;

pub use exchange::{Comment, CommentFormat, Exchange, ExchangedFile, Popup, SkipCause, SkippedComment};
pub use export::{exchange_of, file_ids};
pub use fields::{DataValue, FieldData, FormDataFormat};
pub use form_data::{FormPlan, form_data_of, plan_form_data, reset_values};
pub use import::{ImportCommentsOptions, ImportMode, ImportSummary, ImportWarning, import, preview};
pub use json::FORM_DATA_SCHEMA;
use laminapdf_syntax::Limits;
pub use markdown::{CommentMarkdownOptions, MarkdownComment, MarkdownReply, write_markdown};

/// Why an exchange file couldn't be read, or an import couldn't be applied.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum InterchangeError {
    /// The bytes are in none of the formats asked for: XFDF or FDF for comments, and JSON too
    /// for form data.
    #[error("the file is in a format Lamina PDF doesn't read here")]
    UnknownFormat,
    /// The file is XFDF or FDF that doesn't read.
    #[error("the file isn't valid {format} near line {line}: {reason}")]
    Malformed {
        /// Its format.
        format: FormDataFormat,
        /// Where reading stopped, counted from 1.
        line: u32,
        /// What is wrong, in a few words.
        reason: &'static str,
    },
    /// An XFDF file that declares a DTD, which could define entities that expand without end
    /// or read other files.
    #[error("the XFDF file declares a DTD at line {line}")]
    Dtd {
        /// Where the `<!DOCTYPE` starts, counted from 1.
        line: u32,
    },
    /// The file is larger than the document's limits allow.
    #[error("the file is larger than the limits allow")]
    TooLarge,
    /// Applying the import failed; nothing was changed.
    #[error(transparent)]
    Op(#[from] laminapdf_ops::Error),
    /// Reading the document failed.
    #[error(transparent)]
    Document(#[from] laminapdf_cos::Error),
}

impl InterchangeError {
    /// The stable code (ADR 0026): `interchange-unknown-format`, `interchange-malformed`,
    /// `interchange-dtd`, `interchange-too-large`, or the wrapped error's code.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnknownFormat => "interchange-unknown-format",
            Self::Malformed { .. } => "interchange-malformed",
            Self::Dtd { .. } => "interchange-dtd",
            Self::TooLarge => "interchange-too-large",
            Self::Op(e) => e.code(),
            Self::Document(e) => e.code(),
        }
    }

    /// The code's kind (ADR 0026 §2): `input` for a file that doesn't read, `security` for a
    /// DTD, `limit` for a file too large; a wrapped error's own.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn kind(&self) -> laminapdf_syntax::ErrorKind {
        use laminapdf_syntax::ErrorKind;
        match self {
            Self::UnknownFormat | Self::Malformed { .. } => ErrorKind::Input,
            Self::Dtd { .. } => ErrorKind::Security,
            Self::TooLarge => ErrorKind::Limit,
            Self::Op(e) => e.kind(),
            Self::Document(e) => e.kind(),
        }
    }

    /// Where the error was found: a wrapped error's location; none for a file that doesn't read,
    /// whose line is in the error itself.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn location(&self) -> laminapdf_syntax::Location {
        match self {
            Self::Op(e) => e.location(),
            Self::Document(e) => e.location(),
            _ => laminapdf_syntax::Location::default(),
        }
    }
}

/// The result of this crate's functions.
///
/// Added in 0.7.0.
pub type Result<T> = std::result::Result<T, InterchangeError>;

/// Reads an XFDF or FDF file, telling them apart by their first bytes.
///
/// # Errors
/// `interchange-unknown-format` for neither; `interchange-too-large` past
/// `limits.max_stream_decoded()`; `interchange-dtd` for an XFDF file with a DTD;
/// `interchange-malformed`, naming the line, for a file that doesn't read.
///
/// Added in 0.7.0.
pub fn read(bytes: &[u8], limits: &Limits) -> Result<Exchange> {
    match form_data_format(bytes) {
        Some(FormDataFormat::Xfdf) => xfdf::read(bytes, limits),
        Some(FormDataFormat::Fdf) => fdf::read(bytes, limits),
        _ => Err(InterchangeError::UnknownFormat),
    }
}

/// Reads a form data file's fields, telling XFDF, FDF and JSON apart by their first bytes. An
/// XFDF or FDF file's comments are left out.
///
/// # Errors
/// As [`read`]; for JSON, `interchange-malformed` for text that isn't JSON or a value of no
/// kind a field takes.
///
/// Added in 0.7.0.
pub fn read_form_data(bytes: &[u8], limits: &Limits) -> Result<Vec<FieldData>> {
    match form_data_format(bytes) {
        // Past the byte-order mark only, so serde counts lines as the file has them.
        Some(FormDataFormat::Json) => json::read(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes), limits),
        _ => Ok(read(bytes, limits)?.fields),
    }
}

/// Writes `fields` as a form data file in `format`, naming the PDF they came from (`file`, its
/// `ids`) in XFDF and FDF, as those formats do. [`DataValue::Checked`] is written as `Yes` or
/// `Off` in XFDF and FDF, which name a check box's states: give a [`DataValue::State`] for a
/// check box whose on-state has another name, as [`form_data_of`] does.
///
/// Added in 0.7.0.
#[must_use]
pub fn write_form_data(
    format: FormDataFormat,
    fields: Vec<FieldData>,
    file: Option<String>,
    ids: Option<[Vec<u8>; 2]>,
) -> Vec<u8> {
    match format {
        FormDataFormat::Json => json::write(&fields),
        FormDataFormat::Xfdf | FormDataFormat::Fdf => {
            let exchange = Exchange {
                file,
                ids,
                fields,
                ..Exchange::default()
            };
            if format == FormDataFormat::Xfdf {
                xfdf::write(&exchange)
            } else {
                fdf::write(&exchange)
            }
        }
    }
}

/// The format of a file, told by its first bytes after a byte-order mark and white space: FDF
/// when `%FDF-` starts it, XFDF when its first markup is XML, JSON when it is `{`.
///
/// Added in 0.7.0.
#[must_use]
pub fn form_data_format(bytes: &[u8]) -> Option<FormDataFormat> {
    let start = start_of(bytes);
    if start.starts_with(b"%FDF-") {
        Some(FormDataFormat::Fdf)
    } else if start.starts_with(b"<") {
        Some(FormDataFormat::Xfdf)
    } else if start.starts_with(b"{") {
        Some(FormDataFormat::Json)
    } else {
        None
    }
}

/// `bytes` past a UTF-8 byte-order mark and leading white space.
fn start_of(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes).trim_ascii_start()
}

/// Writes `exchange` as XFDF (ISO 19444-1), UTF-8.
///
/// Added in 0.7.0.
#[must_use]
pub fn write_xfdf(exchange: &Exchange) -> Vec<u8> {
    xfdf::write(exchange)
}

/// Writes `exchange` as FDF (ISO 32000-2 §12.7.8).
///
/// Added in 0.7.0.
#[must_use]
pub fn write_fdf(exchange: &Exchange) -> Vec<u8> {
    fdf::write(exchange)
}

#[cfg(test)]
mod tests {
    use laminapdf_test_fixtures::codes::assert_kinds_match;

    use super::{FormDataFormat, ImportWarning, InterchangeError, SkipCause, SkippedComment};

    #[test]
    fn every_code_and_kind_is_the_one_codes_json_lists() {
        let errors = [
            InterchangeError::UnknownFormat,
            InterchangeError::Malformed {
                format: FormDataFormat::Xfdf,
                line: 1,
                reason: "",
            },
            InterchangeError::Dtd { line: 1 },
            InterchangeError::TooLarge,
        ];
        let warnings = [
            ImportWarning::Skipped(SkippedComment {
                page: None,
                kind: String::new(),
                cause: SkipCause::Unsupported,
                line: None,
            }),
            ImportWarning::DocumentMismatch { file: None },
            ImportWarning::DuplicateName {
                page: 0,
                name: String::new(),
            },
        ];
        assert_kinds_match(
            errors
                .iter()
                .map(|e| (e.code(), e.kind().as_str()))
                .chain(warnings.iter().map(|w| (w.code(), w.kind().as_str()))),
        );
    }
}
