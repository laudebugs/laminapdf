//! Lamina PDF: create, read, edit and save PDFs, in pure Rust, bytes in and bytes out.
//!
//! An invoice, built, saved, reopened, edited and saved again:
//!
//! ```
//! # #[cfg(feature = "text")] {
//! use laminapdf::{
//!     BuildOptions, Builder, CancelToken, Color, Document, FillRule, LinkTarget, MetadataPatch, Op, OpenOptions,
//!     PageSize, PointPt, Pt, RectPt, SaveMode, SaveOptions, SetMetadata, Standard14, StrokeStyle,
//! };
//!
//! let at = PointPt::from_points;
//! let mut b = Builder::new(BuildOptions::new().with_id(*b"invoice-0000-042"));
//! let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
//! let bold = b.standard_font(Standard14::HelveticaBold)?;
//! let mut page = b.page(PageSize::A4);
//! let s = page.surface(); // page space: points from the top-left, y down; `at.y` is the baseline
//! s.text("Invoice 42", &bold, Pt::new(24.0), at(56.0, 80.0))?;
//! for (row, (item, amount)) in [("Widgets × 3", "€ 36.00"), ("Delivery", "€ 4.50")].iter().enumerate() {
//!     let y = 140.0 + 24.0 * row as f64;
//!     s.text(item, &font, Pt::new(11.0), at(56.0, y))?;
//!     s.text(amount, &font, Pt::new(11.0), at(460.0, y))?;
//! }
//! s.stroke_color(Color::gray(0.6))
//!     .path(|p| {
//!         p.move_to(at(56.0, 196.0)).line_to(at(539.0, 196.0));
//!     })
//!     .stroke(&StrokeStyle::default());
//! s.fill_color(Color::hex("#1e78c8")?)
//!     .path(|p| {
//!         p.rect(RectPt::from_xywh(Pt::new(0.0), Pt::new(0.0), Pt::new(595.0), Pt::new(12.0)));
//!     })
//!     .fill(FillRule::NonZero);
//! s
//!     .text("Pay online", &font, Pt::new(11.0), at(56.0, 230.0))?
//!     .link(LinkTarget::Uri("https://example.com/pay/42".into()), None);
//! page.finish()?;
//! let mut doc = b.finish()?; // a Document, not bytes
//! let bytes = doc.save(&SaveOptions::new())?.bytes;
//!
//! // Later: open the bytes, edit with ops (undoable, serialisable), save incrementally.
//! let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
//! assert_eq!((doc.page_count()?, doc.page_info(0)?.width.round()), (1, 595.0));
//! doc.apply(vec![Op::from(SetMetadata::new(MetadataPatch::new().subject(Some("Paid")), None))])?;
//! let options = SaveOptions::new().with_mode(SaveMode::Incremental);
//! let updated = doc.save(&options)?;
//! assert_eq!(doc.info()?.subject.as_deref(), Some("Paid"));
//! assert!(updated.bytes.len() > updated.report.append_offset().unwrap_or(0) as usize);
//! # }
//! # Ok::<(), laminapdf::Error>(())
//! ```
//!
//! # What is here
//!
//! * [`Document`]: [`open_bytes`](Document::open_bytes) → reads ([`page_count`](Document::page_count),
//!   [`page_info`](Document::page_info), [`info`](Document::info),
//!   [`metadata`](Document::metadata), [`warnings`](Document::warnings)) → edits
//!   ([`apply`](Document::apply) a list of [`Op`]s as one transaction, such as
//!   [`SetMetadata`] and [`RemoveMetadata`], which keep Info and XMP in step;
//!   [`undo`](Document::undo), [`redo`](Document::redo)) → [`plan`](Document::plan) shows how a
//!   save would go → [`save`](Document::save) with [`SaveOptions`] (incremental or a full
//!   rewrite, planned around signatures; [`SaveOptions::with_metadata_date`] stamps the
//!   save-time metadata).
//! * [`Builder`] (feature `text`, on by default): pages, paths, colour, images, shaped text in
//!   any script, text boxes, links, outlines, page labels and metadata, tagged PDF
//!   ([`Surface::tag`], [`Surface::figure`], [`Surface::artifact`]; [`Tag`], [`TagKind`]),
//!   finishing into a [`Document`].
//! * [`Error`]: one error type with a stable [`code`](Error::code), a [`kind`](Error::kind),
//!   a [`hint`](Error::hint), a [`location`](Error::location) and a
//!   [`docs_url`](Error::docs_url), serialising to the [ADR 0026] object ([`Diagnostic`]);
//!   [`Warning`] likewise.
//! * Jobs ([ADR 0028]): long work is a [`SaveJob`] stepped with a [`Budget`] of work units
//!   and stopped by a [`CancelToken`]; every step checks the document's revision.
//!
//! # Rules this API follows
//!
//! The façade is what every binding wraps one-to-one ([ADR 0028] §1): no lifetimes or
//! borrowed returns on [`Document`] (pages are addressed by index or [`PageId`]), no generics
//! or closures in its signatures, owned records of strings and numbers (`u32` counts), bytes
//! in and out as `Vec<u8>`, cursors instead of iterators ([`WarningCursor`]), one error type,
//! and jobs instead of callbacks. [`Document`] is `Send` but not `Sync`: the lock, when one
//! is needed, lives in the edge crate ([ADR 0028] §2). The [`Builder`] is the Rust form of
//! the builder (borrowed page builders, closures for paths); the edge crates define its
//! binding form.
//!
//! Coordinates are points from the top-left of the displayed page, y down ([ADR 0027]).
//! Output is byte-deterministic: the same calls give the same bytes on every platform.
//!
//! # Features
//!
//! * `build`: the builder without text (`laminapdf-build`): pages, paths, colour, images,
//!   links, navigation and metadata; the WASM base module's builder (ADR 0016 §4).
//! * `text` (default; implies `build`): fonts and text (`laminapdf-text`). Without either, the
//!   crate opens, reads, edits and saves only.
//!
//! This crate is part of Lamina PDF's pure core: no file system, network, clocks or threads.
//!
//! [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
//! [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
//! [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#![forbid(unsafe_code)]

mod annotations;
#[cfg(feature = "build")]
mod builder;
mod codes;
mod data;
mod document;
mod error;
#[cfg(feature = "extract")]
pub mod extract;
/// Generated by `cargo xtask gen codes` from `fixtures/codes.json`; never edited by hand.
#[cfg(feature = "explain")]
mod generated {
    #[rustfmt::skip]
    pub(crate) mod explain;
}
mod job;
mod links;
mod metadata;
mod navigation;
mod properties;
#[cfg(feature = "extract")]
mod redaction;
#[cfg(feature = "extract")]
pub mod reveal;
/// Flowing layout ([ADR 0032]): `laminapdf-build`'s flow layer, for [`Builder::flow`].
///
/// [ADR 0032]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0032-flowing-layout-and-document-inputs.md
#[cfg(feature = "text")]
pub mod flow {
    pub use laminapdf_build::flow::*;
}
/// Markdown in, flowing pages out ([ADR 0032] §6): `laminapdf-build`'s Markdown front end, for
/// [`Builder::markdown`].
///
/// [ADR 0032]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0032-flowing-layout-and-document-inputs.md
#[cfg(feature = "markdown")]
pub mod markdown {
    pub use laminapdf_build::markdown::*;
}
mod revisions;
mod save_plan;
mod structure;
mod warning;
// What `fixtures/codes.json` lists, for the kind tests.
#[cfg(test)]
use laminapdf_test_fixtures::codes as test_codes;

pub use annotations::{AnnotationGeometry, PageAnnotation};
pub use document::{Document, DocumentInfo, Edited, PageInfo, Saved};
pub use error::{CoreError, Error, Result};
#[cfg(feature = "explain")]
pub use generated::explain::explain;
pub use laminapdf_cos::Cipher;
pub use laminapdf_model::FontEncoding;
pub use links::{LinkTo, PageLink};
pub use navigation::{NamedDestination, OutlineNode, PageTarget, PageTargetView};
pub use properties::{DocumentContents, DocumentPermissions, DocumentProperties, EncryptionSummary, FontSummary};
pub use structure::{StructureKid, StructureNode};

/// The JSON Schema of the error and warning object, `laminapdf.error/1` (ADR 0026 §1), as
/// `cargo xtask gen codes` writes it. Tools that embed it in schemas of their own, such as
/// `laminapdf mcp`'s output schemas, read it from here.
///
/// Added in 0.2.0.
pub const ERROR_SCHEMA: &str = include_str!("../schemas/error.schema.json");
pub use job::{Budget, CancelToken, Phase, SaveJob, Step};
/// The error of the builder's own methods (`Surface`, `PageBuilder` and the rest come from
/// `laminapdf-build` as they are, ADR 0028): `?` turns it into [`Error`].
#[cfg(feature = "build")]
pub use laminapdf_build::Error as BuildError;
pub use laminapdf_syntax::{DOCS_URL_BASE, DOCS_URL_SUFFIX, Diagnostic, ErrorCode, WarningCode, did_you_mean};
#[cfg(feature = "extract")]
pub use redaction::RedactionWarning;
pub use warning::{Warning, WarningCursor};

pub use laminapdf_cos::{ByteSource, ErrorKind, Limit, Limits, Location, OpenOptions, Position};
pub use laminapdf_model::{AnnotId, LabelStyle, PageId, PdfDate, PointPt, Pt, RectPt, Rotation, SiblingPosition};
#[expect(deprecated, reason = "SetInfo stays public so v1 op logs still apply")]
pub use laminapdf_ops::{
    AddOutlineItem, ChildItems, ConformanceWarning, DeleteAnnot, DeleteNamedDestination, DeleteOutlineItem,
    DestinationUse, DestinationView, DocumentMetadata, ExplicitDestination, InfoPatch, MetadataKey, MetadataPatch,
    MetadataSource, ModificationClass, MoveOutlineItem, Offset, Op, OpLog, OutlineStyle, OutlineTarget, PacketStatus,
    PageLabelRange, RemoveMetadata, RenameLinks, RenameNamedDestination, Reverted, Rgb, RotatePage, SetInfo,
    SetMetadata, SetNamedDestination, SetPageLabels, UndoOutcome, UpdateAnnot, UpdateOutlineItem,
};
pub use laminapdf_save::{
    ConstraintKind, DocMdpPermission, IncrementalOutput, MetadataUpdate, Renumber, Restriction, RewriteReason,
    SaveMode, SaveOptions, SaveReport, SignatureInfo, SignatureKind, Violation, XrefEol, XrefKind,
};
pub use save_plan::PlannedSave;

#[cfg(feature = "build")]
pub use builder::{Builder, images_to_pdf};
/// A font registered with a [`Builder`], to draw text with ([`Surface::text`]).
#[cfg(feature = "text")]
pub use laminapdf_build::FontHandle;
#[cfg(feature = "build")]
pub use laminapdf_build::{
    AfRelationship, Artifact, ArtifactKind, Attachment, BlendMode, BuildOptions, Color, ContentKind, Dash, ElementId,
    FillRule, Fit, IccProfile, Image, ImageFormat, ImagePages, ImagesOptions, LineCap, LineJoin, LinkTarget, Matrix,
    Metadata, Orientation, OutlineEntry, OutlineItemId, PageBuilder, PageEdge, PageLabel, PageLayout, PageMode,
    PageSize, PathBuilder, PdfVersion, PendingPath, PngOptions, Profile, StrokeStyle, Surface, TableScope, Tag,
    TagDiagnostic, TagKind, ViewerPreferences,
};
#[cfg(feature = "text")]
pub use laminapdf_build::{
    Alignment, Direction, EmbedPolicy, Feature, LineBox, Overflow, Standard14, TextBoxOptions, TextLayout,
    TextRenderMode, TextRun,
};
/// The error of [`Font`]'s own methods (`from_bytes`, `from_collection`, `instance`): `Font`
/// comes from `laminapdf-text` as it is, like the builder's types, and `?` turns it into
/// [`Error`].
#[cfg(feature = "text")]
pub use laminapdf_text::Error as TextError;
/// A font loaded from its bytes (TrueType, OpenType/CFF, collections, variable instances).
#[cfg(feature = "text")]
pub use laminapdf_text::Font;
/// A four-byte OpenType tag (`wght`, `liga`, `cv01`): a variation axis for [`Font::instance`] (`OpenTypeTag::WEIGHT`)
/// or a [`Feature`]'s.
#[cfg(feature = "text")]
pub use laminapdf_text::OpenTypeTag;
