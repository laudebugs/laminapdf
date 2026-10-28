//! The PDF file structure (COS layer, ISO 32000-2 §7.5–7.7): a tolerant reader and a strict
//! writer, over the object model of [`lamina_syntax`].
//!
//! ```
//! use lamina_cos::{Dict, Object, Writer, open};
//!
//! let mut w = Writer::new(1, 7);
//! let pages = w.reserve();
//! let page = w.add([("Type", Object::name("Page")), ("Parent", pages.into())].into_iter().collect::<Dict>());
//! w.set(pages, [("Type", Object::name("Pages")), ("Kids", vec![page.into()].into()), ("Count", 1.into())]
//!     .into_iter().collect::<Dict>());
//! let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
//! let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
//!
//! let doc = open(&bytes)?;
//! assert_eq!(doc.page_count()?, 1);
//! assert!(!doc.was_repaired());
//! # Ok::<(), lamina_cos::Error>(())
//! ```
//!
//! This crate is part of lamina's pure core: bytes in, bytes out. It borrows the input,
//! parses objects lazily and never touches the file system, network, clocks or threads.
//!
//! * [`open`] / [`open_with`] → [`Document`]: classic xref tables and xref streams, `/Prev`
//!   chains, hybrid `/XRefStm` sections, object streams, repair by scanning when the stored
//!   xref is unusable, an object cache, and the page tree with inherited attributes.
//! * [`Writer`] / [`incremental_update`]: classic, compact (object stream + xref stream) and
//!   incremental output.
//!
//! The object model types ([`Object`], [`Dict`], …) are re-exported from `lamina-syntax`.
//!
//! # Tolerance policy
//!
//! Real-world files are often malformed, so reading is tolerant and writing is strict. The
//! rules (ported from the `pdf-basics` spike, where the Rust and TypeScript readers were
//! aligned by differential fuzzing):
//!
//! * Missing or free objects resolve to `null` (§7.3.10). The page-tree walk skips children
//!   that are missing, unparsable or not dictionaries; reference cycles are
//!   [`Error::ReferenceCycle`].
//! * The `%PDF-` header is optional: it is looked for in the first 1 024 bytes, and its
//!   absence is reported as [`Warning::MissingHeader`].
//! * The stored xref is used only when every in-file entry points at a matching `n g obj`
//!   header and the trailer's `/Root` resolves to a dictionary; otherwise the xref is rebuilt
//!   by scanning ([`Warning::XrefRepaired`]). In hybrid files the classic table's in-use
//!   entries win over `/XRefStm`, which still fills in entries the table marks free.
//! * Limits: nesting depth [`lamina_syntax::MAX_DEPTH`], decoded stream size
//!   [`OpenOptions::max_decoded_size`], and bounded `/Prev` chains and reference chains.
//!   Loops over counts read from the file are bounded by the bytes actually present.
#![forbid(unsafe_code)]

mod error;
mod reader;
mod scan;
mod writer;

pub use error::{Error, Result};
pub use lamina_syntax::{Dict, Name, ObjRef, Object, PdfString, Stream, StringFormat};
pub use reader::{Document, OpenOptions, Page, Resolved, Warning, open, open_with};
pub use writer::{Writer, XrefEol, incremental_update};
