//! The PDF file structure (COS layer, ISO 32000-2 §7.5–7.7): a tolerant reader and the
//! overlay that records changes, over the object model of [`lamina_syntax`]. Writing lives in
//! `lamina-save`.
//!
//! ```
//! use lamina_cos::{Document, OpenOptions};
//!
//! let mut bytes = b"%PDF-1.7\n".to_vec();
//! let objects = [
//!     "<< /Type /Catalog /Pages 2 0 R >>",
//!     "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
//!     "<< /Type /Page /Parent 2 0 R >>",
//! ];
//! let mut offsets = Vec::new();
//! for (i, body) in objects.iter().enumerate() {
//!     offsets.push(bytes.len());
//!     bytes.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
//! }
//! let xref = bytes.len();
//! bytes.extend_from_slice(b"xref\n0 4\n0000000000 65535 f\r\n");
//! for offset in offsets {
//!     bytes.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
//! }
//! bytes.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
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
//! # Sources without an in-memory slice
//!
//! An in-memory source ([`ByteSource::as_slice`]: `Vec<u8>`, `Arc<[u8]>`, `&'static [u8]`) is
//! parsed in place. Any other source (a memory map, a `Blob`, an HTTP range cache) is never
//! read whole: the reader reads 64 KiB windows around the offsets it needs through
//! [`ByteSource::read_at`], doubling a window while an object runs past its end (up to
//! [`Limits::max_read_window`]), keeps the 64 most recently used per [`Source`], checks xref
//! entries with small probes, reads a stream's data only once its extent is known, and scans
//! for repair in overlapping chunks. Within [`Limits::max_read_window`] the results are the
//! same as for the same bytes in memory.
//!
//! A source that answers [`SourceError::Pending`] makes the call fail with
//! [`Error::DataPending`], which lists the ranges it still needs (merged, and collected so that
//! one round fetches what is known: the first window and the last kilobyte at open, the xref
//! entry probes together, every window a repair scan needs). Fetch them and call again. This
//! applies to lazy calls after open ([`Document::resolve`], [`Document::pages`],
//! [`Document::decode_stream`], …) too: they cache no object on failure, so the [`Document`]
//! stays usable.
//!
//! * [`Document::open`] with [`OpenOptions`] ([`Limits`], strict mode): classic xref tables
//!   and xref streams, `/Prev` chains, hybrid `/XRefStm` sections, object streams, repair by scanning when the stored
//!   xref is unusable, an object cache, and the page tree with inherited attributes.
//! * Encryption: files encrypted with the standard security handler (R2–R6, RC4 / AES-128 /
//!   AES-256, crypt filters, `/EncryptMetadata`) open with [`OpenOptions::with_password`] or the
//!   empty user password and are decrypted as objects load ([`Document::encryption`],
//!   [`Document::permissions`], [`Document::matched_password`]). Read only.
//! * The overlay ([`Document::edit`] → [`CosMut`]): every change lands there, never in the
//!   base; [`Document::rebase`] makes saved bytes the new base. The writers that turn it into
//!   bytes are in `lamina-save`.
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
//! * Writing (in `lamina-save`) is strict: an incremental update appends a section of the
//!   same kind as the newest one (classic table or xref stream), never shrinks `/Size`
//!   ([`Document::size_floor`]), and refuses repaired ([`Error::RepairedBase`]) and encrypted
//!   ([`Error::EncryptedBase`]) bases.
//! * Limits ([`Limits`], ADR 0003 §3): hard limits (nesting, string length, decoded bytes
//!   per stream and per call, filter chains, reference chains, page-tree depth) fail the call
//!   with [`Error::LimitExceeded`], or make a tolerant path skip the item with its usual
//!   warning; soft limits (`/Prev` sections, object numbers, warnings kept) drop the item
//!   with a warning. Loops over counts read from the file are bounded by the bytes actually
//!   present.
#![forbid(unsafe_code)]

mod error;
mod overlay;
mod reader;
mod scan;
mod source;
// Hand-built files, shared with the integration tests (unit tests can't use `lamina-save`'s
// writer: through the dev-dependency they would see a second copy of this crate).
#[cfg(test)]
#[path = "../tests/common/classic.rs"]
mod test_files;

pub use error::{Error, Result};
pub use lamina_syntax::crypt::{Cipher, PasswordKind, Permissions};
pub use lamina_syntax::{Dict, Limit, Limits, Name, ObjRef, Object, PdfString, Stream, StringFormat};
pub use overlay::{CosChange, CosMut, JournalMark, Overlay, OverlayEntry, Reserved};
pub use reader::{
    DirectPath, Document, Encryption, Inherited, OpenOptions, Page, PathSegment, Resolved, Warning, XREF_STREAM_KEYS,
    strip_crypt_filter,
};
pub use source::{ByteSource, Source, SourceError, merge_ranges, read_exact};
