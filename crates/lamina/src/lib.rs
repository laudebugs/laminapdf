//! lamina: create, read, edit and save PDFs, in pure Rust, bytes in and bytes out.
//!
//! An invoice, built, saved, reopened, edited and saved again:
//!
//! ```
//! # #[cfg(feature = "text")] {
//! use lamina::{
//!     BuildOptions, Builder, CancelToken, Color, Document, FillRule, InfoPatch, LinkTarget, Op, OpenOptions,
//!     PageSize, PointPt, Pt, RectPt, SaveMode, SaveOptions, SetInfo, Standard14, StrokeStyle,
//! };
//!
//! let at = PointPt::from_points;
//! let mut b = Builder::new(BuildOptions::default().with_id(*b"invoice-0000-042"));
//! let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
//! let bold = b.standard_font(Standard14::HelveticaBold)?;
//! let mut page = b.page(PageSize::A4);
//! let s = page.surface(); // page space: points from the top-left, y down; `at.y` is the baseline
//! let _ = s.text("Invoice 42", &bold, Pt::new(24.0), at(56.0, 80.0))?;
//! for (row, (item, amount)) in [("Widgets × 3", "€ 36.00"), ("Delivery", "€ 4.50")].iter().enumerate() {
//!     let y = 140.0 + 24.0 * row as f64;
//!     let _ = s.text(item, &font, Pt::new(11.0), at(56.0, y))?;
//!     let _ = s.text(amount, &font, Pt::new(11.0), at(460.0, y))?;
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
//! let _ = s
//!     .text("Pay online", &font, Pt::new(11.0), at(56.0, 230.0))?
//!     .link(LinkTarget::Uri("https://example.com/pay/42".into()), None);
//! page.finish()?;
//! let mut doc = b.finish()?; // a Document, not bytes
//! let bytes = doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes;
//!
//! // Later: open the bytes, edit with ops (undoable, serialisable), save incrementally.
//! let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
//! assert_eq!((doc.page_count()?, doc.page_info(0)?.width.round()), (1, 595.0));
//! doc.apply(vec![Op::from(SetInfo::new(InfoPatch::new().subject(Some("Paid")), None))])?;
//! let options = SaveOptions::new().with_mode(SaveMode::Incremental);
//! let updated = doc.save(&options, &CancelToken::new())?;
//! assert_eq!(doc.info()?.subject.as_deref(), Some("Paid"));
//! assert!(updated.bytes.len() > updated.report.append_offset().unwrap_or(0) as usize);
//! # }
//! # Ok::<(), lamina::Error>(())
//! ```
//!
//! # What is here
//!
//! * [`Document`]: [`open_bytes`](Document::open_bytes) → reads ([`page_count`](Document::page_count),
//!   [`page_info`](Document::page_info), [`info`](Document::info),
//!   [`warnings`](Document::warnings)) → edits ([`apply`](Document::apply) a list of [`Op`]s as
//!   one transaction, [`undo`](Document::undo), [`redo`](Document::redo)) →
//!   [`save`](Document::save) with [`SaveOptions`] (incremental or a full rewrite, planned
//!   around signatures).
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
//! * `build`: the builder without text (`lamina-build`): pages, paths, colour, images,
//!   links, navigation and metadata; the WASM base module's builder (ADR 0016 §4).
//! * `text` (default; implies `build`): fonts and text (`lamina-text`). Without either, the
//!   crate opens, reads, edits and saves only.
//!
//! This crate is part of lamina's pure core: no file system, network, clocks or threads.
//!
//! [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
//! [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
//! [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#![forbid(unsafe_code)]

#[cfg(feature = "build")]
mod builder;
mod codes;
mod data;
mod diagnostic;
mod document;
mod error;
#[cfg(feature = "extract")]
pub mod extract;
/// Generated by `cargo xtask gen codes` from `fixtures/codes.json`; never edited by hand.
mod generated {
    #[rustfmt::skip]
    pub(crate) mod codes;
}
mod job;
mod warning;
// What `fixtures/codes.json` lists, for the kind tests (shared with the other crates' tests).
#[cfg(test)]
#[path = "../../lamina-cos/tests/common/codes.rs"]
mod test_codes;

pub use diagnostic::Diagnostic;
pub use document::{Document, DocumentInfo, Edited, PageInfo, Saved};
pub use error::{Error, Result};
#[cfg(feature = "explain")]
pub use generated::codes::explain;
pub use generated::codes::{DOCS_URL_BASE, DOCS_URL_SUFFIX, ErrorCode, WarningCode};
pub use job::{Budget, CancelToken, Phase, SaveJob, Step};
pub use warning::{Warning, WarningCursor};

#[cfg(feature = "build")]
pub use lamina_build::Error as BuildError;
/// The wrapped errors, for matching on [`Error`]'s variants.
pub use lamina_cos::Error as CosError;
pub use lamina_ops::Error as OpsError;
pub use lamina_save::{SaveError, SinkError};
#[cfg(feature = "extract")]
pub use lamina_text::Error as TextError;

pub use lamina_cos::{ByteSource, ErrorKind, Limit, Limits, Location, OpenOptions, Position};
pub use lamina_model::{PageId, PdfDate, PointPt, Pt, RectPt, Rotation};
pub use lamina_ops::{InfoPatch, Op, OpLog, RotatePage, SetInfo, UndoOutcome};
pub use lamina_save::{IncrementalOutput, Renumber, SaveMode, SaveOptions, SaveReport, XrefEol, XrefKind};

#[cfg(feature = "build")]
pub use builder::Builder;
/// A font registered with a [`Builder`], to draw text with ([`Surface::text`]).
#[cfg(feature = "text")]
pub use lamina_build::Font as FontHandle;
#[cfg(feature = "text")]
pub use lamina_build::{
    Alignment, Direction, EmbedPolicy, Feature, LineBox, Overflow, Standard14, TextBoxOptions, TextLayout,
    TextRenderMode, TextRun,
};
#[cfg(feature = "build")]
pub use lamina_build::{
    ArtifactKind, BlendMode, BuildOptions, Color, ContentKind, Dash, FillRule, Fit, IccProfile, Image, ImageFormat,
    LabelStyle, LineCap, LineJoin, LinkTarget, Matrix, Metadata, OutlineEntry, OutlineItemId, PageBuilder, PageLabel,
    PageLayout, PageMode, PageSize, PathBuilder, PdfVersion, PendingPath, PngOptions, StrokeStyle, Surface, TableScope,
    Tag, TagDiagnostic, TagKind, ViewerPreferences,
};
/// A font loaded from its bytes (TrueType, OpenType/CFF, collections, variable instances).
#[cfg(feature = "text")]
pub use lamina_text::Font;
