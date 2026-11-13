//! Editing PDFs through typed, serialisable ops ([ADR 0006]): atomic transactions with
//! savepoints, and undo and redo computed from the current state.
//!
//! ```
//! use laminapdf_ops::{Document, Origin, RotatePage, UndoOutcome};
//! use laminapdf_model::Rotation;
//! # use laminapdf_cos::{Dict, Object};
//! # use laminapdf_save::test_support::Writer;
//! # let mut w = Writer::new(1, 7);
//! # let pages = w.reserve();
//! # let page = w.add([("Type", Object::name("Page")), ("Parent", pages.into())].into_iter().collect::<Dict>());
//! # w.set(pages, [("Type", Object::name("Pages")), ("Kids", vec![page.into()].into()), ("Count", 1.into())].into_iter().collect::<Dict>());
//! # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
//! # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
//!
//! let mut doc = Document::open(bytes, Default::default())?;
//! let page = doc.pages()?.get(0).map(|p| p.id().clone()).ok_or("no page")?;
//!
//! let ((), applied) = doc.transact(Origin::Local, |tx| {
//!     tx.apply(RotatePage::new(page.clone(), Rotation::Clockwise90))?;
//!     Ok(())
//! })?;
//! assert!(applied.touched_pages.contains(&page));
//! assert_eq!(doc.pages()?.get(0).map(|p| p.rotate()).transpose()?, Some(Rotation::Clockwise90));
//! assert!(doc.is_dirty() && doc.has_unsaved_changes());
//!
//! assert_eq!(doc.undo()?.map(|r| r.outcome), Some(UndoOutcome::Applied));
//! assert!(!doc.is_dirty()); // the overlay is empty again: a save would write nothing new
//! doc.redo()?;
//! assert_eq!(doc.pages()?.get(0).map(|p| p.rotate()).transpose()?, Some(Rotation::Clockwise90));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! This crate is part of Lamina PDF's pure core: bytes in, bytes out, no file system, network,
//! clocks or threads.
//!
//! * [`Op`]: [`SetMetadata`] (with [`MetadataPatch`]), [`RemoveMetadata`] (with
//!   [`MetadataKey`]), the save planner's [`UpdateSaveMetadata`], [`RotatePage`] and the
//!   deprecated [`SetInfo`] (with [`InfoPatch`]), each with a
//!   [`ModificationClass`] and [`Op::requires_rewrite`]; [`OpLog`] is the versioned JSON
//!   envelope `{ "v": 1, "ops": [ … ] }`.
//! * [`Document`]: wraps a [`laminapdf_model::Document`] (it dereferences to one) and is the
//!   only write path: [`Document::transact`] with an [`Origin`], [`Tx::apply`],
//!   [`Tx::savepoint`], the handle form `begin` / `apply` / `commit` / `rollback`, and
//!   [`Error::NestedTransaction`] for a transaction inside one.
//! * [`Applied`] carries the [`UndoRecord`] (each property's value before and after) and the
//!   touched pages; [`Document::undo`] / [`Document::redo`] return an [`UndoOutcome`].
//! * [`read_metadata`] → [`DocumentMetadata`]: Info and XMP read together, the side that wins
//!   and where they disagree; [`conformance_warnings`] → [`ConformanceWarning`]s for a PDF/A or
//!   PDF/UA claim the metadata breaks.
//! * [`PendingChanges`] ([`Document::pending_changes`]): the op classes not yet saved, which
//!   `laminapdf-save`'s planner checks against signatures (ADR 0007 §2).
//!
//! # How a change reaches the file
//!
//! Ops read through the model's views and write only through
//! [`laminapdf_cos::CosMut`], which records each change in the document's overlay and journal
//! ([ADR 0005] §3). A transaction is a journal mark: committing takes the changes since the
//! mark and derives the undo record from them, so every op gets undo without writing an
//! inverse by hand; failing rolls the journal back to the mark. Undoing to the saved state
//! leaves the overlay empty, because an entry equal to the base is dropped and dictionary
//! edits keep key order.
//!
//! `transact` lives here, on a wrapper over the model document, rather than on
//! `laminapdf_cos::Document`: the COS layer offers the overlay, journal marks and rollback, and
//! this crate turns them into transactions with origins and history (ADR 0006 §2).
//!
//! # Features
//!
//! * `tsify`: derives `tsify::Tsify` on the op types, so `laminapdf-wasm` can emit their
//!   TypeScript declarations (ADR 0002). Off by default.
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
#![forbid(unsafe_code)]

mod annot;
mod blob;
mod class;
mod document;
mod error;
mod history;
mod info;
mod metadata;
mod op;
mod pending;
mod record;
mod rotate;
mod tx;
mod wire;
// What `fixtures/codes.json` lists, for the kind tests.
#[cfg(test)]
use laminapdf_test_fixtures::codes as test_codes;

pub use annot::{DeleteAnnot, Offset, UpdateAnnot};
pub use blob::{BlobId, BlobStore, ParseBlobIdError, SourceId};
pub use class::ModificationClass;
pub use document::{Document, PendingRebase, SaveState};
pub use error::{Error, Result};
pub use history::{History, Reverted, UndoOutcome};
#[expect(deprecated, reason = "SetInfo stays public so v1 op logs still apply")]
pub use info::{InfoPatch, SetInfo};
pub use metadata::{
    ConformanceWarning, DocumentMetadata, MetadataKey, MetadataPatch, MetadataSource, PacketStatus, RemoveMetadata,
    SetMetadata, UpdateSaveMetadata, conformance_warnings, read_metadata,
};
pub use op::{Op, OpLog};
pub use pending::PendingChanges;
pub use record::{Change, Field, Property, PropertyValue, SkipReason, Skipped, Target, UndoRecord};
pub use rotate::RotatePage;
pub use tx::{Applied, Origin, PeerId, Tx};
