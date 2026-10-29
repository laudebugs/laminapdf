//! Saving PDFs ([ADR 0007] §5–§7, [ADR 0005] §3): the writers that turn a document's overlay
//! into bytes, the sinks they stream to, and the rebase that makes the output the new base.
//!
//! ```
//! use lamina_model::Rotation;
//! use lamina_ops::{Document, Origin, RotatePage, SaveState};
//! use lamina_save::{Save, SaveOptions};
//! # use lamina_cos::{Dict, Object};
//! # use lamina_save::Writer;
//! # let mut w = Writer::new(1, 7);
//! # let pages = w.reserve();
//! # let page = w.add([("Type", Object::name("Page")), ("Parent", pages.into())].into_iter().collect::<Dict>());
//! # w.set(pages, [("Type", Object::name("Pages")), ("Kids", vec![page.into()].into()), ("Count", 1.into())].into_iter().collect::<Dict>());
//! # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
//! # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
//!
//! let mut doc = Document::open(bytes, Default::default())?;
//! let page = doc.pages()?.get(0).map(|p| p.id().clone()).expect("one page");
//! doc.apply_all(Origin::Local, [RotatePage::new(page, Rotation::Clockwise90).into()])?;
//!
//! // A full rewrite into a readable sink rebases the document onto the new file.
//! doc.save_full_and_rebase(Vec::new(), &SaveOptions::new())?;
//! assert_eq!(doc.save_state(), SaveState::Clean);
//! assert_eq!(doc.pages()?.get(0).map(|p| p.rotate()).transpose()?, Some(Rotation::Clockwise90));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! * [`IncrementalWriter`]: the base bytes unchanged, then the overlay as an appended
//!   revision with an xref section of the base's newest kind (§7.5.6).
//! * [`FullWriter`]: only what the trailer reaches; numbers kept or compacted; object streams
//!   and an xref stream, or a classic table.
//! * [`WriteSink`] / [`ReadableSink`]: append-only destinations (`Vec<u8>`, [`IoWrite`]),
//!   counted by [`CountingSink`]; nothing seeks.
//! * [`Save`] on [`lamina_ops::Document`]: `save_incremental`, `save_full`,
//!   `save_full_and_rebase` (B3b's names until B3c's `Document::save` and the planner),
//!   with the `Clean | Dirty | SavedNotRebased` states ([`SaveState`]).
//! * `/ID` (§14.4): `/ID[0]` kept (derived when missing), `/ID[1]` the first 16 bytes of a
//!   SHA-256 of the content written.
//! * [`Writer`] / [`incremental_update`]: the spike-era writer, moved here from `lamina-cos`
//!   with its golden tests, for building files from scratch.
//!
//! Both writers refuse an encrypted base ([`SaveError::EncryptedBase`]) and the incremental
//! one a repaired base ([`SaveError::RepairedBase`]), before writing anything. Output is
//! byte-deterministic (ADR 0001): no clocks, no hash maps, `/ID` from a hash.
//!
//! This crate is part of lamina's pure core: bytes in, bytes out, no file system, network,
//! clocks or threads.
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
#![forbid(unsafe_code)]

mod document;
mod error;
mod full;
mod id;
mod incremental;
mod legacy;
mod options;
mod sink;
mod xref;

pub use document::Save;
pub use error::{Result, SaveError};
pub use full::FullWriter;
pub use incremental::{IncrementalSave, IncrementalWriter};
pub use lamina_ops::SaveState;
pub use legacy::{Writer, XrefEol, incremental_update};
pub use options::{IncrementalOutput, Renumber, SaveMode, SaveOptions, SaveReport, XrefKind};
pub use sink::{CountingSink, IoWrite, ReadableSink, SinkError, WriteSink};
