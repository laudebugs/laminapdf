//! The render backend contract ([ADR 0004]): every lamina surface renders pages through
//! [`RenderBackend`] and [`RenderDocument`], which draw bounded tiles of a page at a scale into
//! a caller-supplied straight-alpha RGBA8 buffer.
//!
//! - Requests are validated once, by [`TilePlan::new`], so every backend agrees on sizes and
//!   refuses oversized tiles (`render-too-large`) before allocating.
//! - The pure-Rust adapter, [`HayroBackend`], is behind the `hayro` feature: ADR 0004's CI
//!   oracle and debug choice. The PDFium adapter is the edge crate `lamina-render-pdfium`.
//! - Errors follow the ADR 0026 contract ([`RenderError`]).
//!
//! [ADR 0004]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0004-render-backend.md
#![forbid(unsafe_code)]

mod backend;
mod error;
#[cfg(feature = "hayro")]
mod hayro;
mod request;
mod source;

pub use backend::{RenderBackend, RenderDocument};
pub use error::RenderError;
#[cfg(feature = "hayro")]
pub use hayro::HayroBackend;
pub use lamina_cos::{ByteSource, SourceError, merge_ranges};
pub use lamina_syntax::CancelToken;
pub use request::{
    BYTES_PER_PIXEL, PixelRect, RenderFlags, RenderLimits, RenderOpenOptions, RenderPage, RenderRequest, Rgba8,
    TilePlan, ViewRotation,
};
pub use source::read_all;
