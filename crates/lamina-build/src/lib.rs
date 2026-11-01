//! Generating PDFs ([ADR 0005] §6): a builder that writes lamina `Object`s straight into a
//! document's overlay, reusing `pdf-writer`'s `Content` for content streams. A new document is
//! a [`lamina_ops::Document`] over an empty base; adding pages to an open document
//! ([`Builder::on`]) is the same code and saves incrementally.
//!
//! ```
//! use lamina_build::{BuildOptions, Builder, Color, FillRule, Image, PageSize, StrokeStyle};
//! use lamina_model::{PointPt, Pt, RectPt};
//!
//! let mut b = Builder::new(BuildOptions::default().with_id([1; 16]));
//! let mut page = b.page(PageSize::A4);
//! let logo = Image::rgba(2, 2, &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 128])?;
//! page.surface()
//!     // Page space: points from the top-left, y down (ADR 0027).
//!     .fill_color(Color::hex("#1e78c8")?)
//!     .path(|p| {
//!         p.rect(RectPt::from_xywh(Pt::new(56.0), Pt::new(56.0), Pt::new(200.0), Pt::new(120.0)));
//!     })
//!     .fill(FillRule::NonZero)
//!     .stroke_color(Color::gray(0.1))
//!     .path(|p| {
//!         p.move_to(PointPt::from_points(56.0, 200.0)).line_to(PointPt::from_points(539.0, 200.0));
//!     })
//!     .stroke(&StrokeStyle::default())
//!     .image(&logo, RectPt::from_xywh(Pt::new(440.0), Pt::new(56.0), Pt::new(100.0), Pt::new(100.0)));
//! page.finish()?;
//! let doc = b.finish()?; // a Document, not bytes: save it with lamina-save's `Save`
//! assert_eq!(doc.pages()?.len(), 1);
//! # Ok::<(), lamina_build::Error>(())
//! ```
//!
//! * [`Builder`] → [`PageBuilder`] (size, boxes, `/Rotate`, `/UserUnit`) → [`Surface`] →
//!   [`PageBuilder::finish`] → [`Builder::finish`]; [`Appender`] for open documents.
//! * [`Surface`]: `save`/`restore`, `transform` ([`Matrix`]), paths ([`PathBuilder`]: lines,
//!   cubic and quadratic curves, rectangles, ellipses), `stroke` ([`StrokeStyle`]), `fill` and
//!   `clip` ([`FillRule`]), colours ([`Color`]: gray, RGB, CMYK, `#rrggbb`, ICC-based with an
//!   [`IccProfile`]), `opacity` and [`BlendMode`] (`ExtGState`), images ([`Image`]: JPEG
//!   passed through, PNG with an `/SMask` for alpha, raw RGBA).
//! * Positions are page space ([ADR 0027]): [`Pt`], [`PointPt`], [`RectPt`] from
//!   `lamina-model`, re-exported here.
//! * Output is deterministic (ADR 0001): no clocks, no hash maps; resources are named per
//!   page in order of first use (`GS1`, `Im1`, `CS1`) and every image, profile and
//!   `ExtGState` is written once per document.
//!
//! * Text ([B5b]): [`Builder::font`] / [`Builder::standard_font`] give a [`Font`];
//!   [`Surface::text`] draws a shaped, bidi-resolved run with its **baseline at `y`** and
//!   returns a [`TextRun`]; [`Surface::text_box`] breaks a paragraph into a rectangle
//!   ([`TextBoxOptions`], [`TextLayout`]).
//! * Navigation and metadata ([B5b]): [`Surface::link`] ([`LinkTarget`], [`Fit`]),
//!   [`Builder::destination`], [`Builder::outline`] ([`OutlineEntry`]),
//!   [`Builder::page_labels`] ([`PageLabel`]), [`Builder::metadata`] (Info and XMP from one
//!   [`Metadata`]), [`Builder::lang`], [`Builder::viewer_preferences`].
//! * Tagged PDF ([B8a]): [`Surface::tag`] ([`Tag`], [`TagKind`], [`TableScope`]),
//!   [`TextRun::tag`], [`Surface::figure`] (`/Alt`), [`Surface::artifact`] ([`ArtifactKind`]);
//!   MCIDs per page, `/StructParents`, the `ParentTree`, `OBJR`s for links, the `RoleMap`
//!   and `/MarkInfo`; [`Builder::tagging_diagnostics`] ([`TagDiagnostic`]) for what is left
//!   untagged.
//! * Conformance: [`BuildOptions::with_profile`] ([`Profile`]: PDF/UA-1, PDF/A-2b,
//!   PDF/A-2u, PDF/A-3b) turns each machine-checkable rule the document breaks into a coded
//!   error at [`Builder::finish`], and writes the XMP identification, `/DisplayDocTitle` and
//!   the PDF/A output intent with an embedded sRGB profile; [`Builder::attach`]
//!   ([`Attachment`], [`AfRelationship`]) adds associated files for PDF/A-3b.
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
//! [B5b]: https://github.com/laudebugs/lamina/blob/main/docs/todos/done/B5b-builder-text-and-document-features.md
//! [B8a]: https://github.com/laudebugs/lamina/blob/main/docs/todos/done/B8a-structure-tree.md
#![forbid(unsafe_code)]

mod annot;
mod attach;
mod builder;
mod error;
mod finish;
mod meta;
mod nav;
mod page;
mod profile;
mod resources;
mod surface;
mod tag;

// What `fixtures/codes.json` lists, for the kind tests (shared with the other crates' tests).
#[cfg(test)]
#[path = "../../lamina-cos/tests/common/codes.rs"]
mod test_codes;

pub use attach::{AfRelationship, Attachment};
pub use builder::{Appender, BuildOptions, Builder, PdfVersion};
pub use error::{Error, ImageFormat, Result};
pub use lamina_model::{LabelStyle, OutlineItemId, PageFrame, PageId, PdfDate, PointPt, Pt, RectPt, Rotation};
#[cfg(feature = "text")]
pub use lamina_text::{Alignment, Direction, EmbedPolicy, Feature, Standard14};
pub use meta::{Metadata, PageLayout, PageMode, ViewerPreferences};
pub use nav::{Fit, LinkTarget, OutlineEntry, PageLabel};
pub use page::{PageBuilder, PageSize};
pub use profile::Profile;
pub use surface::color::{Color, IccProfile};
pub use surface::image::{Image, MAX_DECODED_IMAGE_BYTES, PngOptions};
pub use surface::path::{FillRule, Matrix, PathBuilder};
pub use surface::style::{BlendMode, Dash, LineCap, LineJoin, StrokeStyle};
#[cfg(feature = "text")]
pub use surface::text::{Font, TextRenderMode, TextRun};
#[cfg(feature = "text")]
pub use surface::textbox::{LineBox, Overflow, TextBoxOptions, TextLayout};
pub use surface::{PendingPath, Surface};
pub use tag::{ArtifactKind, ContentKind, TableScope, Tag, TagDiagnostic, TagKind};
