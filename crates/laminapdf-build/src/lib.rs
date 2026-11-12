//! Generating PDFs ([ADR 0005] §6): a builder that writes Lamina PDF `Object`s straight into a
//! document's overlay, reusing `pdf-writer`'s `Content` for content streams. A new document is
//! a [`laminapdf_ops::Document`] over an empty base; adding pages to an open document
//! ([`Builder::on`]) is the same code and saves incrementally.
//!
//! ```
//! use laminapdf_build::{BuildOptions, Builder, Color, FillRule, Image, PageSize, StrokeStyle};
//! use laminapdf_model::{PointPt, Pt, RectPt};
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
//! let doc = b.finish()?; // a Document, not bytes: save it with laminapdf-save's `Save`
//! assert_eq!(doc.pages()?.len(), 1);
//! # Ok::<(), laminapdf_build::Error>(())
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
//!   `laminapdf-model`, re-exported here.
//! * Output is deterministic (ADR 0001): no clocks, no hash maps; resources are named per
//!   page in order of first use (`GS1`, `Im1`, `CS1`) and every image, profile and
//!   `ExtGState` is written once per document.
//!
//! * Text: [`Builder::font`] / [`Builder::standard_font`] give a [`FontHandle`];
//!   [`Surface::text`] draws a shaped, bidi-resolved run with its **baseline at `y`** and
//!   returns a [`TextRun`]; [`Surface::text_box`] breaks a paragraph into a rectangle
//!   ([`TextBoxOptions`], [`TextLayout`]).
//! * Navigation and metadata: [`Surface::link`] ([`LinkTarget`], [`Fit`]),
//!   [`Builder::destination`], [`Builder::outline`] ([`OutlineEntry`]),
//!   [`Builder::page_labels`] ([`PageLabel`]), [`Builder::metadata`] (Info and XMP from one
//!   [`Metadata`]), [`Builder::lang`], [`Builder::viewer_preferences`].
//! * Tagged PDF: [`Surface::tag`] ([`Tag`], [`TagKind`], [`TableScope`]),
//!   [`TextRun::tag`], [`Surface::figure`] (`/Alt`), [`Surface::artifact`] ([`ArtifactKind`],
//!   or an [`Artifact`] such as [`Artifact::footer`] with its `/Subtype` and [`PageEdge`]s);
//!   elements that continue on later pages ([`Surface::open_element`] → [`ElementId`],
//!   [`Surface::resume`], [`Builder::close_element`]); MCIDs per page, `/StructParents`, the
//!   `ParentTree`, `OBJR`s for links, the `RoleMap` and `/MarkInfo`;
//!   [`Builder::tagging_diagnostics`] ([`TagDiagnostic`]) for what is left untagged.
//! * Flowing layout: [`Builder::flow`] pours a [`flow::Flow`] of headings, paragraphs, lists
//!   and figures through a page template into as many pages as it needs, with keep rules,
//!   running headers and footers and "Page X of Y" ([`flow`]).
//! * Conformance: [`BuildOptions::with_profile`] ([`Profile`]: PDF/UA-1, PDF/A-2b,
//!   PDF/A-2u, PDF/A-3b) turns each machine-checkable rule the document breaks into a coded
//!   error at [`Builder::finish`], and writes the XMP identification, `/DisplayDocTitle` and
//!   the PDF/A output intent with an embedded sRGB profile; [`Builder::attach`]
//!   ([`Attachment`], [`AfRelationship`]) adds associated files for PDF/A-3b.
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
#![forbid(unsafe_code)]

mod annot;
mod attach;
mod builder;
mod error;
mod finish;
#[cfg(feature = "text")]
pub mod flow;
mod images;
#[cfg(feature = "markdown")]
pub mod markdown;
mod meta;
mod nav;
mod page;
mod profile;
mod resources;
mod surface;
mod tag;

/// A text string (ISO 32000-2 §7.9.2.2): PDFDocEncoding when it can hold `s`, else UTF-16BE.
pub(crate) fn text_string(s: &str) -> laminapdf_cos::PdfString {
    laminapdf_cos::PdfString::literal(laminapdf_syntax::text::encode_text_string(s))
}

// What `fixtures/codes.json` lists, for the kind tests.
#[cfg(test)]
use laminapdf_test_fixtures::codes as test_codes;

pub use attach::{AfRelationship, Attachment};
pub use builder::{Appender, BuildOptions, Builder, PdfVersion};
pub use error::{Error, ImageFormat, Result, Warning};
pub use images::{ImagePages, ImagesOptions, images_to_pdf};
pub use laminapdf_model::{
    BlendMode, Dash, FillRule, LabelStyle, Matrix, OutlineItemId, PageFrame, PageId, PdfDate, PointPt, Pt, RectPt,
    Rotation,
};
#[cfg(feature = "text")]
pub use laminapdf_text::{Alignment, Direction, EmbedPolicy, Feature, Standard14};
pub use meta::{Metadata, PageLayout, PageMode, ViewerPreferences};
pub use nav::{Fit, LinkTarget, OutlineEntry, PageLabel};
pub use page::{PageBuilder, PageSize};
pub use profile::Profile;
pub use surface::color::{Color, IccProfile};
pub use surface::image::{Image, MAX_DECODED_IMAGE_BYTES, PngOptions};
pub use surface::path::PathBuilder;
pub use surface::style::{LineCap, LineJoin, StrokeStyle};
#[cfg(feature = "text")]
pub use surface::text::{FontHandle, TextRenderMode, TextRun};
#[cfg(feature = "text")]
pub use surface::textbox::{LineBox, Overflow, TextBoxOptions, TextLayout};
pub use surface::{Orientation, PendingPath, Surface};
pub use tag::{Artifact, ArtifactKind, ContentKind, ElementId, PageEdge, TableScope, Tag, TagDiagnostic, TagKind};
