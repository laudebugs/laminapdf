//! Saving PDFs ([ADR 0007], [ADR 0005] §3): the save planner that decides how a document may be
//! saved, the writers that turn its overlay into bytes, the sinks they stream to, and the
//! rebase that makes the output the new base.
//!
//! ```
//! use laminapdf_model::Rotation;
//! use laminapdf_ops::{Document, Origin, RotatePage, SaveState};
//! use laminapdf_save::{Save, SaveMode, SaveOptions};
//! # use laminapdf_cos::{Dict, Object};
//! # use laminapdf_save::Writer;
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
//! doc.save_and_rebase(Vec::new(), &SaveOptions::new().with_mode(SaveMode::FullRewrite))?;
//! assert_eq!(doc.save_state(), SaveState::Clean);
//! assert_eq!(doc.pages()?.get(0).map(|p| p.rotate()).transpose()?, Some(Rotation::Clockwise90));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! * [`SavePlanner`] ([ADR 0007] §1–§3): the [`SaveConstraint`] (signatures, `DocMDP`, rewrite
//!   reasons, violations; [`ConstraintKind`]), [`SavePlan`] with the mode resolved from
//!   [`SaveMode::Auto`] or checked against the requested one, the explicit overrides
//!   (invalidating signatures, removing encryption), and
//!   [`would_violate`](SavePlanner::would_violate) for the editor.
//! * [`Save`] on [`laminapdf_ops::Document`]: [`Save::plan`], [`Save::save`] and
//!   [`Save::save_and_rebase`], with the `Clean | Dirty | SavedNotRebased` states
//!   ([`SaveState`]).
//! * [`IncrementalWriter`]: the base bytes unchanged, then the overlay as an appended
//!   revision with an xref section of the base's newest kind (§7.5.6).
//! * [`FullWriter`]: only what the trailer reaches; numbers kept or compacted; object streams
//!   and an xref stream, or a classic table.
//! * [`WriteSink`] / [`ReadableSink`]: append-only destinations (`Vec<u8>`, [`IoWrite`]),
//!   counted by [`CountingSink`]; nothing seeks.
//! * `/ID` (§14.4): `/ID[0]` kept (derived when missing), `/ID[1]` the first 16 bytes of a
//!   SHA-256 of the content written.
//! * [`Writer`] / [`incremental_update`]: the spike-era writer, moved here from `laminapdf-cos`
//!   with its golden tests, for building files from scratch.
//!
//! Safe by default: no save breaks a signature, appends to a repaired base or drops encryption
//! unless the caller opted in ([`SaveError::WouldInvalidateSignatures`],
//! [`SaveError::RequiresRewrite`], [`SaveError::EncryptedBase`]), and the writers themselves
//! refuse an encrypted base (unless removing the encryption) and the incremental one a repaired
//! base, before writing anything. Output is byte-deterministic (ADR 0001): no clocks, no hash
//! maps, `/ID` from a hash.
//!
//! This crate is part of Lamina PDF's pure core: bytes in, bytes out, no file system, network,
//! clocks or threads.
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
#![forbid(unsafe_code)]

mod constraint;
mod detect;
mod document;
mod error;
mod full;
mod id;
mod incremental;
mod legacy;
mod options;
mod planner;
mod resolve;
mod sink;
mod xref;
// What `fixtures/codes.json` lists, for the kind tests.
#[cfg(test)]
use laminapdf_test_fixtures::codes as test_codes;

pub use constraint::{
    ConstraintKind, DocMdp, DocMdpPermission, Restriction, RewriteReason, SaveConstraint, SignatureInfo, SignatureKind,
    Violation,
};
pub use document::Save;
pub use error::{Result, SaveError, SavedOutput};
pub use full::FullWriter;
pub use incremental::{IncrementalSave, IncrementalWriter};
pub use laminapdf_ops::{PendingRebase, SaveState};
pub use legacy::{Writer, incremental_update};
pub use options::{IncrementalOutput, Renumber, SaveMode, SaveOptions, SaveReport, XrefEol, XrefKind};
pub use planner::{SavePlan, SavePlanner};
pub use sink::{CountingSink, IoWrite, ReadableSink, SinkError, WriteSink};
