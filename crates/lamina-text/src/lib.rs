//! Fonts for generated PDFs (todo B4a; [ADR 0005] §6): load TrueType/OpenType/CFF fonts and
//! collection members from caller-supplied bytes, pick variable-font instances, track the
//! glyphs a document uses, and at the end write each font as a deterministic subset: a Type 0
//! font with Identity-H, a `CIDFontType2` (TrueType outlines, `/FontFile2`,
//! `/CIDToGIDMap /Identity`) or `CIDFontType0` (CFF outlines, `/FontFile3 /CIDFontType0C`)
//! descendant with compressed `/W` widths, and a ToUnicode CMap, so text is selectable and
//! extractable (ISO 32000-2 §9.6–9.10; research 03 §3.5–6).
//!
//! ```
//! # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
//! use lamina_text::{Error, Font, FontRegistry, EmbedPolicy};
//!
//! let font = Font::from_bytes(font_bytes())?;
//! let mut fonts = FontRegistry::new();
//! let key = fonts.add(font, EmbedPolicy::Respect)?;
//!
//! // Two bytes per glyph (the CID in the subset), and the advance in 1/1000 of the font size.
//! let encoded = fonts.encode(key, "Hi")?;
//! assert_eq!(encoded.bytes, [0, 1, 0, 2]);
//!
//! // Characters the font has no glyph for are errors, never a silent `.notdef`.
//! let err = fonts.encode(key, "中").unwrap_err();
//! assert_eq!(err.code(), "missing-glyph");
//! # Ok::<(), Error>(())
//! ```
//!
//! * [`Font`]: [`Font::from_bytes`], [`Font::from_collection`], [`Font::instance`] for
//!   variable fonts, [`FontInfo`] (names, metrics, flags, axes, `fsType`),
//!   [`Font::glyph_for`].
//! * [`FsType`] / [`EmbedPolicy`]: a restricted-licence or bitmap-only font fails with
//!   [`Error::FontNotEmbeddable`] unless the caller overrides; a "no subsetting" font is
//!   embedded whole.
//! * [`FontUse`]: per font, glyph → CID (dense, in order of first use) and CID → text.
//!   One glyph used for two texts keeps the first and reports [`Warning::ToUnicodeConflict`].
//! * [`FontRegistry`]: per document; [`FontRegistry::reference`] reserves the Type 0 object in
//!   the overlay at first use, [`FontRegistry::finish_fonts`] subsets with `subsetter` and
//!   fills it through a [`lamina_cos::CosMut`].
//!
//! Subset tags are six letters derived from a SHA-256 of the font bytes and the glyphs in
//! CID order, so the same text in the same font gives byte-identical output.
//!
//! Shaping (B4b), simple fonts and the standard 14 (B4c) and the `surface.text()` API (B5b)
//! build on this crate. It is part of lamina's pure core: bytes in, bytes out, no file
//! system, network, clocks or threads; font bytes are parsed by `skrifa`/`read-fonts` and
//! `subsetter`, which bounds-check every read.
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
#![forbid(unsafe_code)]

mod embed;
mod error;
mod font;
mod fstype;
mod usage;

pub use embed::{EmbeddedFont, Encoded, FontKey, FontRegistry};
pub use error::{Error, Result, Warning};
pub use font::{Axis, Font, FontInfo, GlyphId, Tag};
pub use fstype::{EmbedPolicy, FsType};
pub use usage::FontUse;
