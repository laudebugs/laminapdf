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
//! Text, links, outlines and metadata are [B5b]'s.
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
//! [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
//! [B5b]: https://github.com/laudebugs/lamina/blob/main/docs/todos/ready/B5b-builder-text-and-document-features.md
#![forbid(unsafe_code)]

mod builder;
mod error;
mod finish;
mod page;
mod resources;
mod surface;

// What `fixtures/codes.json` lists, for the kind tests (shared with the other crates' tests).
#[cfg(test)]
#[path = "../../lamina-cos/tests/common/codes.rs"]
mod test_codes;

pub use builder::{Appender, BuildOptions, Builder, PdfVersion};
pub use error::{Error, ImageFormat, Result};
pub use lamina_model::{PageFrame, PageId, PointPt, Pt, RectPt, Rotation};
pub use page::{PageBuilder, PageSize};
pub use surface::Surface;
pub use surface::color::{Color, IccProfile};
pub use surface::image::{Image, MAX_DECODED_IMAGE_BYTES, PngOptions};
pub use surface::path::{FillRule, Matrix, PathBuilder};
pub use surface::style::{BlendMode, Dash, LineCap, LineJoin, StrokeStyle};
