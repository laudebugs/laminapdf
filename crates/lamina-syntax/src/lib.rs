//! The PDF syntax layer: the object model, its exact serialization, the lexer, the object
//! parser and the stream filters (ISO 32000-2 §7.2–7.4).
//!
//! ```
//! use lamina_syntax::{Object, parse_object};
//!
//! let obj = parse_object(b"<< /Kids [3 0 R] /Count 1 >>")?;
//! assert_eq!(obj.as_dict().and_then(|d| d.get_int("Count")), Some(1));
//! assert_eq!(obj.to_bytes(), b"<< /Kids [3 0 R] /Count 1 >>");
//! # Ok::<(), lamina_syntax::Error>(())
//! ```
//!
//! This crate is part of lamina's pure core: it takes bytes and returns values, and never
//! touches the file system, network, clocks or threads, so it builds for native, WebAssembly
//! and mobile targets alike. Every parse path returns [`Result`]; no input panics. Recursion
//! only happens in the object parser, which is depth-limited ([`Limits::max_nesting_depth`]).
//!
//! * [`Object`] and friends: the object model and its byte-exact serialization.
//! * [`parse_object`] / [`Parser`]: the tolerant object parser (indirect objects and
//!   streams, with an `endstream` scan when `/Length` is wrong).
//! * [`Lexer`] / [`Token`]: the tokenizer the parser and the cross-reference reader share.
//! * [`filters`]: Flate encode/decode within [`Limits`], and PNG predictors.
//! * [`Limits`]: the resource limits of ADR 0003, shared by every layer that reads input.
//!
//! The file structure (cross-reference tables, object streams, repair, the page tree) and the
//! writer live one layer up, in `lamina-cos`.
#![forbid(unsafe_code)]

mod error;
pub mod filters;
mod lexer;
mod limits;
mod object;
mod parser;

pub use error::{Error, Result};
pub use lexer::{Lexer, Token, is_delimiter, is_regular, is_whitespace};
pub use limits::{DecodeBudget, Limit, Limits};
pub use object::{Dict, Name, ObjRef, Object, PdfString, Stream, StringFormat, format_real};
pub use parser::{IndirectObject, Parser, find, parse_object, parse_object_with};
