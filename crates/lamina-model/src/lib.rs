//! Typed, tolerant views over a PDF's COS graph ([ADR 0005] §4) and the stable
//! [`LaminaId`]s that ops, undo and sync address (§5).
//!
//! ```
//! use lamina_model::{Document, OpenOptions, Rotation};
//! use lamina_cos::{Dict, Object};
//! use lamina_save::Writer;
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
//! This crate is part of lamina's pure core: bytes in, bytes out, no file system, network,
//! clocks or threads.
//!
//! * [`Document`]: a [`lamina_cos::Document`] (it dereferences to one) plus the
//!   document-level [`IdAllocator`].
//! * [`Pages`] / [`Page`]: boxes as normalised [`UserSpaceRect`]s, [`Rotation`], user unit,
//!   the [`PageFrame`] (page space, [ADR 0027]),
//!   resources, contents and annotation references, page labels.
//! * [`Outline`] / [`OutlineItem`]: titles, nesting, open state, [`Destination`]s resolved
//!   to [`PageId`]s, action kinds.
//! * [`Info`]: the information dictionary, with dates as [`PdfDate`].
//! * [`NameTree`] / [`NumberTree`] and [`PageLabels`].
//! * [`LaminaId`], [`PageId`], [`OutlineItemId`], [`AnnotId`], [`FieldId`].
//! * Units and coordinates ([ADR 0027]): [`Pt`], [`PointPt`], [`RectPt`] (page space) and
//!   [`UserSpacePoint`], [`UserSpaceRect`] (raw user space).
//!
//! # Tolerance
//!
//! Views read their fields lazily and apply [ADR 0003]'s tolerance per field: a malformed
//! `/MediaBox` is `None` plus [`Warning::InvalidBox`], not a failed page. Warnings are
//! recorded on the document ([`lamina_cos::Document::warnings`]); strict mode turns each
//! into [`Error::Strict`]. Tree walks (outline, name and number trees) are iterative, keep a
//! visited set, and skip subtrees deeper than [`Limits::max_tree_depth`] with a warning.
//! Only strict-mode refusals and unavailable input ([`Error::DataPending`],
//! [`Error::Source`]) end a view's call; every other unreadable value counts as absent.
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
//! [ADR 0003]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
#![forbid(unsafe_code)]

mod date;
mod document;
mod geometry;
mod id;
mod info;
mod labels;
mod outline;
mod page;
mod tree;

pub use date::{InvalidDate, PdfDate};
pub use document::Document;
pub use geometry::{LETTER, PageFrame, PointPt, Pt, RectPt, UserSpacePoint, UserSpaceRect};
pub use id::{AnnotId, FieldId, IdAllocator, Kind, LaminaId, OutlineItemId, PageId, ParseIdError};
pub use info::{Info, InfoDate};
pub use labels::{LabelRange, LabelStyle, PageLabels};
pub use outline::{ActionKind, Destination, Outline, OutlineItem, View};
pub use page::{Page, Pages, Rotation};
pub use tree::{NameTree, NumberTree};

// The layers below, in façade order (ADR 0005 §1): what callers of the model need without
// naming `lamina-cos`.
pub use lamina_cos::{Error, Limit, Limits, OpenOptions, Result, Warning};
