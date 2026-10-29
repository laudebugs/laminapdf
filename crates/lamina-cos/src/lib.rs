//! The PDF file structure (COS layer, ISO 32000-2 §7.5–7.7): a tolerant reader and a strict
//! writer, over the object model of [`lamina_syntax`].
//!
//! ```
//! use lamina_cos::{Dict, Document, Object, OpenOptions, Writer};
//!
//! let mut w = Writer::new(1, 7);
//! let pages = w.reserve();
//! let page = w.add([("Type", Object::name("Page")), ("Parent", pages.into())].into_iter().collect::<Dict>());
//! w.set(pages, [("Type", Object::name("Pages")), ("Kids", vec![page.into()].into()), ("Count", 1.into())]
//!     .into_iter().collect::<Dict>());
//! let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
//! let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
//!
//! let doc = Document::open(bytes, OpenOptions::default())?;
//! assert_eq!(doc.page_count()?, 1);
//! assert!(!doc.was_repaired());
//! # Ok::<(), lamina_cos::Error>(())
//! ```
//!
//! This crate is part of lamina's pure core: bytes in, bytes out. A [`Document`] owns its
//! input as a shared [`ByteSource`] (ADR 0005), parses objects lazily and never touches the
//! file system, network, clocks or threads.
//!
//! * [`Document::open`] with [`OpenOptions`] ([`Limits`], strict mode): classic xref tables
//!   and xref streams, `/Prev` chains, hybrid `/XRefStm` sections, object streams, repair by scanning when the stored
//!   xref is unusable, an object cache, and the page tree with inherited attributes.
//! * Encryption: files encrypted with the standard security handler (R2–R6, RC4 / AES-128 /
//!   AES-256, crypt filters, `/EncryptMetadata`) open with [`OpenOptions::with_password`] or the
//!   empty user password and are decrypted as objects load ([`Document::encryption`],
//!   [`Document::permissions`], [`Document::matched_password`]). Read only.
//! * [`Writer`] / [`incremental_update`]: classic, compact (object stream + xref stream) and
//!   incremental output.
//! * [`Warning`]: what the tolerant reader repaired, with stable codes (ADR 0003 §2).
//!
//! The object model types ([`Object`], [`Dict`], …) are re-exported from `lamina-syntax`.
//!
//! # Tolerance policy
//!
//! Real-world files are often malformed, so reading is tolerant and writing is strict. The
//! rules are ADR 0003's (ported from the `pdf-basics` spike, where the Rust and TypeScript
//! readers were aligned by differential fuzzing); every repair raises a [`Warning`], and
//! strict mode ([`OpenOptions::with_strict`]) turns each into [`Error::Strict`]:
//!
//! * Missing or free objects, and references whose generation number does not match the xref
//!   entry's, resolve to `null` (§7.3.10). A `null` value counts as absent: an inheritable
//!   page attribute that is `null` keeps the inherited value, and `/Contents` that is `null`
//!   contributes no content. The page-tree walk skips children that are missing, unparsable
//!   or not dictionaries; reference cycles are [`Error::ReferenceCycle`].
//! * The `%PDF-` header is optional: it is looked for in the first 1 024 bytes, and its
//!   absence is reported as [`Warning::MissingHeader`].
//! * The stored xref is used only when every in-file entry points at a matching `n g obj`
//!   header and the trailer's `/Root` resolves to a dictionary; otherwise the xref is rebuilt
//!   by scanning ([`Warning::XrefRepaired`]). Repair keeps, for each object number, the
//!   definition that comes last in the file (an object-stream member counts at its
//!   container's offset), and falls back to the newest `/Type /Catalog`. In hybrid files the
//!   classic table's in-use entries win over `/XRefStm`, which still fills in entries the
//!   table marks free.
//! * Writing is strict: an incremental update appends a section of the same kind as the
//!   newest one (classic table or xref stream), never shrinks `/Size`, and refuses repaired
//!   ([`Error::RepairedBase`]) and encrypted ([`Error::EncryptedBase`]) bases.
//! * Limits ([`Limits`], ADR 0003 §3): hard limits (nesting, string length, decoded bytes
//!   per stream and per call, filter chains, reference chains, page-tree depth) fail the call
//!   with [`Error::LimitExceeded`], or make a tolerant path skip the item with its usual
//!   warning; soft limits (`/Prev` sections, object numbers, warnings kept) drop the item
//!   with a warning. Loops over counts read from the file are bounded by the bytes actually
//!   present.
#![forbid(unsafe_code)]

mod error;
mod reader;
mod scan;
mod source;
mod writer;

pub use error::{Error, Result};
pub use lamina_syntax::crypt::{Cipher, PasswordKind, Permissions};
pub use lamina_syntax::{Dict, Limit, Limits, Name, ObjRef, Object, PdfString, Stream, StringFormat};
pub use reader::{Document, Encryption, OpenOptions, Page, Resolved, Warning};
pub use source::{ByteSource, Source, SourceError};
pub use writer::{Writer, XrefEol, incremental_update};
