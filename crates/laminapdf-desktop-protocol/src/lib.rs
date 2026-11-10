//! The protocol between Lamina PDF's desktop main process and its engine process (ADR 0008 §3).
//!
//! The main process owns files and windows; the engine parses and renders. They talk over the
//! engine's stdin and stdout in frames ([`write_frame`], [`read_frame`]): a length prefix, the
//! protocol version, a `postcard` header holding one message ([`ToEngine`] or [`FromEngine`]),
//! then raw bytes (a range of the file, a tile's pixels) that are never serialised byte by byte.
//!
//! - **Requests and replies.** Every request from the main process carries a [`ReqId`], and
//!   every reply carries the same one, so one table matches replies to waiters. The engine's
//!   own requests ([`FromEngine::ReadRange`]) use the engine's ids, a separate space.
//! - **Passwords** travel only in [`ToEngine::UnlockDocument`], inside a [`Password`] whose
//!   `Debug` redacts it and whose memory is wiped when dropped. [`read_frame`] and
//!   [`write_frame`] also wipe the header bytes that held it.
//! - **Untrusted input.** The engine reads documents a stranger may have crafted, so the main
//!   process treats every frame from it as untrusted too: [`read_frame`] checks the length
//!   against a limit before allocating and turns every malformed frame into a
//!   [`ProtocolError`], never a panic.
//!
//! This crate depends only on `serde`, `postcard`, `thiserror` and `zeroize`, so the main
//! process never links the parser or PDFium.

mod frame;
mod msg;

pub use frame::{MAX_FRAME, ProtocolError, read_frame, write_frame};
pub use msg::{
    DocId, ErrorKind, FromEngine, Limits, PageSize, Password, Priority, ReqId, TileFormat, TileRequest, ToEngine,
};

/// The protocol's version: in every frame and in the `Hello`/`Ready` handshake. Both sides come
/// from the same build, so a mismatch means a stale engine binary next to a newer app.
pub const PROTOCOL_VERSION: u16 = 1;
