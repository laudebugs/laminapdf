//! Read-only content-stream interpretation ([ADR 0023]): what a page draws, operator by
//! operator, with the graphics and text state in effect.
//!
//! ```
//! use lamina_content::{ContentIter, Lexed, Operator};
//!
//! // The tokenizer alone: operands followed by an operator (ISO 32000-2 §7.8.2).
//! let mut ops = ContentIter::new(b"0.5 g 10 10 100 50 re f");
//! let Some(Ok(Lexed::Op(first))) = ops.next() else { panic!("an operator") };
//! assert_eq!(first.operator, Operator::FillGray);
//! assert_eq!(first.operands.len(), 1);
//! ```
//!
//! * [`ContentIter`]: a tolerant tokenizer over decoded content bytes, yielding [`Op`]s (an
//!   [`Operator`] of ISO 32000-2 Annex A and its operands; inline images read whole) and
//!   [`Issue`]s for what it skipped.
//! * [`interpret_page`] / [`interpret_content`]: run content against a [`GraphicsState`]
//!   (CTM, colours, line style, an `ExtGState` subset, the [`TextState`]), the `q`/`Q` stack,
//!   text objects, resources (with a Form XObject's falling back to its parent's), Form
//!   XObjects (recursively, bounded, with cycle detection) and marked content, and report
//!   events to a [`Visitor`]: text shows, path segments and paints, shadings, images,
//!   marked-content sequences, form entry and exit, and warnings.
//!
//! Glyph decoding is `lamina-text`'s job: the interpreter hands each text show to the visitor
//! with the font and matrices, and the visitor returns how far the glyphs moved the text
//! matrix ([`TextAdvance`]).
//!
//! # Tolerance and limits
//!
//! Interpretation follows [ADR 0003]: every repair raises a [`Warning`] (reported to the
//! visitor and recorded on the document; strict mode turns it into [`Error::Strict`]).
//! Unknown operators are skipped (silently inside `BX`/`EX`), operators with too few or
//! mistyped operands are skipped and extra operands ignored, unbalanced `q`/`Q`, `BT`/`ET` and
//! `BMC`/`EMC` are repaired per content stream, a missing resource skips its operator, and an
//! inline image's end is found from `/L`, its computed size, or a heuristic. Hard limits end
//! the call with [`Error::LimitExceeded`]: [`Limits::max_content_operators`] (all operators of
//! one call, forms counted each time they are drawn), [`Limits::max_xobject_depth`],
//! [`Limits::max_nesting_depth`] (saved graphics states per stream) and
//! [`Limits::max_decoded_total`] (bytes interpreted).
//!
//! This crate is part of lamina's pure core: bytes in, bytes out, no file system, network,
//! clocks or threads.
//!
//! [ADR 0023]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0023-content-stream-reading.md
//! [ADR 0003]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
#![forbid(unsafe_code)]

mod interp;
mod lexer;
mod operator;
mod resources;
mod state;
mod visitor;

pub use interp::{Summary, interpret_content, interpret_page};
pub use lexer::{ContentIter, InlineImage, Issue, IssueKind, Lexed, MAX_OPERANDS, Op};
pub use operator::{Checked, Operand, Operator, Signature};
pub use state::{Color, ColorSpace, Dash, FontRef, GraphicsState, Matrix, TextAdvance, TextState};
pub use visitor::{
    FillRule, FormXObject, ImageDraw, ImageSource, MarkedContent, Paint, PathPaint, PathSegment, ShadingPaint,
    TextItem, TextShow, Visitor,
};

// The layers below, in façade order: what callers need without naming `lamina-cos`.
pub use lamina_cos::{Error, Limit, Limits, Result, Warning};
