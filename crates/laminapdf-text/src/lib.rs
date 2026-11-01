//! Fonts and text layout for generated PDFs (todos B4a, B4b, B4d; [ADR 0005] §6): load TrueType/OpenType/CFF fonts and
//! collection members from caller-supplied bytes, pick variable-font instances, track the
//! glyphs a document uses, and at the end write each font as a deterministic subset: a Type 0
//! font with Identity-H, a `CIDFontType2` (TrueType outlines, `/FontFile2`,
//! `/CIDToGIDMap /Identity`, or a stream once a glyph has several CIDs) or `CIDFontType0` (CFF outlines, `/FontFile3 /CIDFontType0C`)
//! descendant with compressed `/W` widths, and a ToUnicode CMap, so text is selectable and
//! extractable (ISO 32000-2 §9.6–9.10; research 03 §3.5–6).
//!
//! ```
//! # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
//! use laminapdf_text::{Error, Font, FontRegistry, EmbedPolicy};
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
//! * [`FontUse`]: per font, one CID per (glyph, text) (dense, in order of first use) and
//!   CID → text, so every use extracts exactly. With CFF outlines a glyph keeps one CID and
//!   its first text, and a second text reports [`Warning::ToUnicodeConflict`].
//! * [`FontRegistry`]: per document; [`FontRegistry::reference`] reserves the font object in
//!   the overlay at first use, [`FontRegistry::encode`] / [`FontRegistry::use_glyph`] return
//!   the bytes to show ([`GlyphCode`]: a two-byte CID or a one-byte code),
//!   [`FontRegistry::usage`] reports what was drawn ([`FontUsage`]), and
//!   [`FontRegistry::finish_fonts`] writes every font through a [`laminapdf_cos::CosMut`].
//! * [`Font::standard`] / [`Standard14`] (B4c): the standard 14 fonts by name, with Adobe's
//!   AFM metrics bundled ([`AfmMetrics`]); [`Font::requires_embedding`] is `false` for them.
//!   They are written as non-embedded `/Type1` simple fonts: one-byte codes in
//!   `WinAnsiEncoding` (other Latin glyphs through `/Differences`, [`SimpleUse`]) or the
//!   built-in encoding, `/Widths`, a font descriptor and a ToUnicode CMap.
//! * [`Encoding`] (code ↔ glyph name for `StandardEncoding`, `WinAnsiEncoding`,
//!   `MacRomanEncoding`, `PDFDocEncoding`, Symbol and `ZapfDingbats`) and [`agl`] (glyph
//!   name → Unicode), shared with text extraction.
//!
//! Subset tags are six letters derived from a SHA-256 of the font bytes and the glyphs in
//! CID order, so the same text in the same font gives byte-identical output.
//!
//! Text layout (B4b; research 03 §7), for the builder to draw with:
//!
//! * [`Shaper`] / [`HarfRust`]: characters → [`ShapedRun`]s of glyphs with clusters,
//!   advances and offsets, with [`ShapeOptions`] (script, language, direction, [`Feature`]s
//!   such as `liga=0`).
//! * [`BidiParagraph`]: UAX #9 levels and per-line visual runs; [`line_breaks`]: UAX #14
//!   break opportunities; [`grapheme_boundaries`]: UAX #29.
//! * [`itemize`] and [`break_lines`]: runs of one script, level and font; greedy lines in a
//!   box, measured by shaping, reordered and aligned; draw a line's runs in
//!   [`Line::drawing_order`].
//! * [`pdf_run`]: a shaped run as `TJ` arrays, `Ts` rises, ToUnicode entries and `/ActualText`
//!   spans.
//!
//! ```
//! # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
//! use laminapdf_text::{EmbedPolicy, Font, FontRegistry, HarfRust, LayoutOptions, break_lines, pdf_run};
//!
//! let font = Font::from_bytes(font_bytes())?;
//! let text = "The office staff fill the affine waffle flask first.";
//! let lines = break_lines(&HarfRust, &[font.clone()], text, &LayoutOptions::new(150.0, 12.0))?;
//! let mut fonts = FontRegistry::new();
//! let key = fonts.add(font, EmbedPolicy::Respect)?;
//! for line in &lines {
//!     // Runs in logical order (each still at its own `x`), so extractors read right-to-left
//!     // lines in order.
//!     for run in line.drawing_order().into_iter().filter_map(|i| line.runs.get(i)) {
//!         let pdf = pdf_run(&run.run, text, &mut fonts, key)?;
//!         assert!(!pdf.pieces.is_empty()); // write each piece's segments as `Ts` + `TJ`
//!     }
//! }
//! # Ok::<(), laminapdf_text::Error>(())
//! ```
//!
//! The `surface.text()` API (B5b) builds on this crate. It is part of Lamina PDF's pure core:
//! bytes in, bytes out, no file system, network, clocks or threads; font bytes are parsed by
//! `skrifa`/`read-fonts` and `subsetter`, which bounds-check every read.
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
#![forbid(unsafe_code)]

pub mod agl;
mod bidi;
mod embed;
mod encoding;
mod error;
pub mod extract;
mod font;
mod fstype;
mod itemize;
mod layout;
mod linebreak;
mod pdfpos;
mod shape;
mod simple;
mod std14;
mod usage;
// What `fixtures/codes.json` lists, for the kind tests (shared with the other crates' tests).
#[cfg(test)]
#[path = "../../laminapdf-cos/tests/common/codes.rs"]
mod test_codes;
#[cfg(test)]
mod test_fonts;

pub use bidi::{BidiParagraph, BidiRun};
pub use embed::{EmbeddedFont, Encoded, FontKey, FontRegistry, FontUsage, GlyphCode};
pub use encoding::{Encoding, UnsupportedEncoding};
pub use error::{Error, Result, Warning};
pub use extract::{Glyph, PageText, Quad, TextExtractor, TextRun, UserSpaceVector};
pub use font::{Axis, Font, FontInfo, GlyphId, MAX_NAME_LEN, Tag};
pub use fstype::{EmbedPolicy, FsType};
pub use itemize::{Item, itemize};
pub use layout::{Alignment, LayoutOptions, Line, LineRun, break_lines};
pub use linebreak::{BreakOpportunity, grapheme_boundaries, is_hard_break, is_trailing_space, line_breaks};
pub use pdfpos::{PdfPiece, PdfRun, PdfSegment, TjElement, pdf_run};
pub use shape::{Direction, Feature, HarfRust, Script, ShapeOptions, ShapedGlyph, ShapedRun, Shaper};
pub use simple::SimpleUse;
pub use std14::{AfmMetrics, Standard14};
pub use usage::FontUse;
