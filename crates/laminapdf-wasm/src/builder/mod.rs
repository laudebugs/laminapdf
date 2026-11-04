//! The binding form of the builder (B5c left it to this crate, ADR 0028 F1–F3):
//! [`WasmBuilder`] owns a `laminapdf::Builder`; [`WasmPageBuilder`] is an *owned* page that
//! draws on its own `Surface` (paths as segment records instead of closures) and is written
//! into the builder by [`WasmPageBuilder::finish`]. The surface is the one
//! `laminapdf-build` uses, moved into the builder's page at the end, so the output is
//! byte-identical to the Rust API's.
//!
//! The text methods (fonts, `text`, `textBox`) exist only in the text module (feature
//! `text`, ADR 0016 §4).

mod document;
mod image;
mod page;
#[cfg(feature = "text")]
mod text;

pub use document::WasmBuilder;
pub use image::WasmImage;
pub use page::WasmPageBuilder;
#[cfg(feature = "text")]
pub use text::WasmFont;
