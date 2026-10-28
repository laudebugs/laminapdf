//! `lamina-pdf-basics` — research spike for lamina's PDF core.
//!
//! Bytes in, bytes out: no file system, network, threads, clocks or randomness.
//! Every parse path returns [`Result`]; no input panics.
//!
//! * [`object`]: the object model and its exact serialization.
//! * [`parse_object`]: the tolerant object parser.
//! * [`Writer`] / [`incremental_update`]: classic, compact (object + xref streams) and
//!   incremental output.
//! * [`open`] / [`Document`]: xref tables and streams, `/Prev` chains, object streams,
//!   repair, and the page tree.
//! * [`ContentBuilder`]: content-stream operators.
//!
//! ```
//! let bytes = lamina_pdf_basics::samples::hello()?;
//! let doc = lamina_pdf_basics::open(&bytes)?;
//! assert_eq!(doc.page_count()?, 1);
//! # Ok::<(), lamina_pdf_basics::Error>(())
//! ```
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod content;
pub mod error;
pub mod filters;
mod lexer;
pub mod object;
mod parser;
pub mod reader;
pub mod samples;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
pub mod writer;

pub use content::{ContentBuilder, Num, fixed};
pub use error::{Error, Result};
pub use object::{Dict, Name, ObjRef, Object, PdfString, Stream, StringFormat, format_real};
pub use parser::{MAX_DEPTH, parse_object};
pub use reader::{Document, OpenOptions, Page, Warning, open, open_with};
pub use writer::{Writer, XrefEol, incremental_update};
