//! Typed, tolerant views over a PDF's COS graph ([ADR 0005] §4), the stable
//! [`LaminaPdfId`]s that ops, undo and sync address (§5), and the graphics values that
//! building and reading share ([`Matrix`], [`FillRule`], [`Dash`]).
//!
//! ```
//! use laminapdf_model::{Document, OpenOptions, Rotation};
//! use laminapdf_cos::{Dict, Object};
//! use laminapdf_save::test_support::Writer;
//!
//! let mut w = Writer::new(1, 7);
//! let pages = w.reserve();
//! let page = w.add(
//!     [("Type", Object::name("Page")), ("Parent", pages.into()), ("Rotate", 90.into())]
//!         .into_iter()
//!         .collect::<Dict>(),
//! );
//! let media_box = Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]);
//! w.set(pages, [("Type", Object::name("Pages")), ("Kids", vec![page.into()].into()), ("Count", 1.into()),
//!     ("MediaBox", media_box)].into_iter().collect::<Dict>());
//! let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
//! let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
//!
//! let doc = Document::open(bytes, OpenOptions::default())?;
//! let pages = doc.pages()?;
//! let first = pages.get(0).expect("one page");
//! assert_eq!(first.id().to_string(), "p2.0");
//! assert_eq!(first.media_box()?.map(|r| r.width()), Some(612.0)); // inherited from /Pages
//! assert_eq!(first.rotate()?, Rotation::Clockwise90);
//! assert_eq!(pages.by_id(&"p2.0".parse()?).map(|p| p.index()), Some(0));
//! assert!(doc.outline()?.is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! This crate is part of Lamina PDF's pure core: bytes in, bytes out, no file system, network,
//! clocks or threads.
//!
//! * [`Document`]: a [`laminapdf_cos::Document`] (it dereferences to one) plus the
//!   document-level [`IdAllocator`].
//! * [`Pages`] / [`Page`]: boxes as normalised [`UserSpaceRect`]s, [`Rotation`], user unit,
//!   the [`PageFrame`] (page space, [ADR 0027]),
//!   resources, contents and annotation references, page labels.
//! * [`Outline`] / [`OutlineItem`]: titles, nesting, open state, [`Destination`]s resolved
//!   to [`PageId`]s, action kinds.
//! * [`Info`]: the information dictionary, with dates as [`PdfDate`].
//! * [`Disclosure`]: what a document holds beyond its visible pages ([`Document::disclosure`]).
//! * [`xmp`]: the XMP metadata packet ([`Document::xmp`]): a tolerant reader, typed access to
//!   the properties Info mirrors, the Info ↔ XMP mapping table, and a writer that keeps every
//!   property it doesn't change.
//! * [`NameTree`] / [`NumberTree`] and [`PageLabels`].
//! * [`StructTree`] / [`StructElem`]: a minimal read-only view of the logical structure
//!   (types, attributes, MCIDs and `OBJR`s per element, the `RoleMap`).
//! * [`LaminaPdfId`], [`PageId`], [`OutlineItemId`], [`AnnotId`], [`FieldId`].
//! * Units and coordinates ([ADR 0027]): [`Pt`], [`PointPt`], [`RectPt`] (page space) and
//!   [`UserSpacePoint`], [`UserSpaceRect`] (raw user space).
//!
//! # Tolerance
//!
//! Views read their fields lazily and apply [ADR 0003]'s tolerance per field: a malformed
//! `/MediaBox` is `None` plus [`Warning::InvalidBox`], not a failed page. Warnings are
//! recorded on the document ([`laminapdf_cos::Document::warnings`]); strict mode turns each
//! into [`Error::Strict`]. Tree walks (outline, name and number trees) are iterative, keep a
//! visited set, and skip subtrees deeper than [`Limits::max_tree_depth`] with a warning.
//! Only strict-mode refusals and unavailable input ([`Error::DataPending`],
//! [`Error::Source`]) end a view's call; every other unreadable value counts as absent.
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
//! [ADR 0003]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
#![forbid(unsafe_code)]

mod copy;
mod date;
mod disclosure;
mod document;
#[cfg(feature = "edit")]
mod edit;
mod fonts;
mod geometry;
mod graphics;
mod id;
mod info;
mod labels;
mod link;
mod locate;
mod outline;
mod page;
#[cfg(feature = "edit")]
mod page_tree;
#[cfg(feature = "edit")]
mod references;
mod structure;
mod tree;
#[cfg(feature = "edit")]
mod tree_writer;
mod values;
mod widgets;
pub mod xmp;

pub use copy::{Copied, ObjectCopier};
pub use date::{InvalidDate, PdfDate};
pub use disclosure::{
    Action, Annotation, Disclosure, EmbeddedFile, EmbeddedFileHolder, FormField, Owner, Trigger, XmpPacket,
};
pub use document::Document;
#[cfg(feature = "edit")]
pub use edit::set_catalog_entry;
pub use fonts::{FontEncoding, FontResource};
pub use geometry::{LETTER, PageFrame, PageSize, PointPt, Pt, RectPt, UserSpacePoint, UserSpaceRect};
pub use graphics::{BlendMode, Dash, FillRule, Matrix};
pub use id::{AnnotId, FieldId, IdAllocator, Kind, LaminaPdfId, OutlineItemId, PageId, ParseIdError};
pub use info::{Info, InfoDate};
pub use labels::{LabelRange, LabelStyle, PageLabels};
pub use link::{Link, LinkAction};
pub use locate::ParentedPage;
pub use outline::{ActionKind, Destination, Outline, OutlineItem, View};
pub use page::{Page, Pages, Rotation};
#[cfg(feature = "edit")]
pub use page_tree::{PagePosition, PageTreeEditor, PageTreeError};
#[cfg(feature = "edit")]
pub use references::{NameStore, PageReference, PageReferences, ReferenceKind, Via, WidgetLink};
pub use structure::{StructElem, StructKid, StructTree};
pub use tree::{NameTree, NumberTree};
#[cfg(feature = "edit")]
pub use tree_writer::TreeWriter;
pub use widgets::PageWidget;

// The layers below, in façade order (ADR 0005 §1): what callers of the model need without
// naming `laminapdf-cos`.
pub use laminapdf_cos::{Error, Limit, Limits, OpenOptions, Result, Warning};
