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
//! # Ok::<(), laminapdf_interchange::Error>(())
//! ```
//!
//! Added in 0.7.0.

#![forbid(unsafe_code)]

mod exchange;
mod export;
mod fdf;
mod import;
mod map;
mod markdown;
mod xfdf;

pub use exchange::{Comment, Exchange, ExchangedFile, Format, Popup, SkipReason, Skipped};
pub use export::exchange_of;
pub use import::{ImportMode, ImportOptions, ImportSummary, ImportWarning, import, preview};
use laminapdf_syntax::Limits;
pub use markdown::{MarkdownComment, MarkdownOptions, MarkdownReply, write_markdown};

/// Why an exchange file couldn't be read, or an import couldn't be applied.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The bytes are neither XFDF nor FDF.
    #[error("the file is neither XFDF nor FDF")]
    UnknownFormat,
    /// The file is XFDF or FDF that doesn't read.
    #[error("the file isn't valid {format} near line {line}: {reason}")]
    Malformed {
        /// Its format.
        format: Format,
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

impl Error {
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
pub type Result<T> = std::result::Result<T, Error>;

/// Reads an XFDF or FDF file, telling them apart by their first bytes.
///
/// # Errors
/// `interchange-unknown-format` for neither; `interchange-too-large` past
/// `limits.max_stream_decoded()`; `interchange-dtd` for an XFDF file with a DTD;
/// `interchange-malformed`, naming the line, for a file that doesn't read.
///
/// Added in 0.7.0.
pub fn read(bytes: &[u8], limits: &Limits) -> Result<Exchange> {
    match sniff(bytes) {
        Some(Format::Xfdf) => xfdf::read(bytes, limits),
        Some(Format::Fdf) => fdf::read(bytes, limits),
        _ => Err(Error::UnknownFormat),
    }
}

/// The format of `bytes`: FDF when `%FDF-` starts it, XFDF when its first markup is XML.
fn sniff(bytes: &[u8]) -> Option<Format> {
    let start = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let start = start.trim_ascii_start();
    if start.starts_with(b"%FDF-") {
        Some(Format::Fdf)
    } else if start.starts_with(b"<") {
        Some(Format::Xfdf)
    } else {
        None
    }
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
