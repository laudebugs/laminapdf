//! The messages: [`ToEngine`] from the main process, [`FromEngine`] from the engine.

use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// A document open in an engine. The main process chooses it and keeps it across engine
/// restarts, so a restarted engine reopens each document under the same id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DocId(pub u32);

/// A request's id, echoed by its reply. The main process numbers its requests and the engine
/// numbers its own [`FromEngine::ReadRange`]s; the two never meet in one table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReqId(pub u64);

/// A document's password: wiped from memory when dropped, never shown by `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(Zeroizing<String>);

impl Password {
    /// Wraps `password`, which is moved, not copied.
    #[must_use]
    pub fn new(password: String) -> Self {
        Self(Zeroizing::new(password))
    }

    /// The password's text, for the one call that needs it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(<redacted>)")
    }
}

impl Serialize for Password {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

/// The resource limits the engine opens and renders with. `None` keeps the engine's default
/// (ADR 0003's for parsing, ADR 0004's for rendering), so the main process needs to know none of
/// them. Names as in `laminapdf`'s `EngineLimits` and `laminapdf-render`'s `RenderLimits`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EngineLimits {
    /// Nested arrays and dictionaries.
    pub max_nesting_depth: Option<u64>,
    /// Bytes one stream may decode to.
    pub max_stream_decoded: Option<u64>,
    /// Bytes all streams may decode to.
    pub max_decoded_total: Option<u64>,
    /// Filters in one stream's chain.
    pub max_filter_chain: Option<u64>,
    /// Indirect references followed in one resolve.
    pub max_resolve_depth: Option<u64>,
    /// Levels of the page tree.
    pub max_page_tree_depth: Option<u64>,
    /// Levels of the other trees (outlines, names, structure).
    pub max_tree_depth: Option<u64>,
    /// Bytes in one string.
    pub max_string_length: Option<u64>,
    /// Cross-reference sections followed.
    pub max_xref_sections: Option<u64>,
    /// The largest object number read.
    pub max_object_number: Option<u64>,
    /// Warnings kept.
    pub max_warnings: Option<u64>,
    /// Content stream operators on one page.
    pub max_content_operators: Option<u64>,
    /// Nested form XObjects.
    pub max_xobject_depth: Option<u64>,
    /// Pixels in one image.
    pub max_image_pixels: Option<u64>,
    /// Bytes the parser reads in one window.
    pub max_read_window: Option<u64>,
    /// Pixels in one rendered tile.
    pub max_render_pixels: Option<u64>,
    /// Pixels on one side of a rendered page.
    pub max_render_side: Option<u64>,
    /// Bytes of a document the renderer may read whole.
    pub max_input_bytes: Option<u64>,
}

/// A tile's pixel format. The engine sends raw pixels; the main process encodes them for the
/// webview (WebP, ADR 0008 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileFormat {
    /// Straight-alpha RGBA, 8 bits per channel, rows top to bottom with no padding.
    Rgba8,
}

/// Which tiles go first: those on screen, then those rendered ahead of a scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Rendered ahead of time, in case the user scrolls there.
    Prefetch,
    /// On screen now.
    Visible,
}

/// One tile of one page: the rectangle `x, y, w, h` (device pixels) of the page drawn at
/// `scale_milli / 1000` pixels per point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileRequest {
    /// The request; the reply is [`FromEngine::Tile`] or [`FromEngine::Error`].
    pub req: ReqId,
    /// The document.
    pub doc: DocId,
    /// The page, 0-based.
    pub page: u32,
    /// Pixels per point, in thousandths (1000 is 72 dpi).
    pub scale_milli: u32,
    /// The tile's left edge in the page's pixels.
    pub x: u32,
    /// The tile's top edge in the page's pixels.
    pub y: u32,
    /// The tile's width in pixels.
    pub w: u32,
    /// The tile's height in pixels.
    pub h: u32,
    /// The pixel format of the reply.
    pub format: TileFormat,
    /// Whether the tile is on screen.
    pub priority: Priority,
}

/// A page's displayed size in points and its `/Rotate`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageDimensions {
    /// Displayed width in points.
    pub width: f32,
    /// Displayed height in points.
    pub height: f32,
    /// `/Rotate`: 0, 90, 180 or 270.
    pub rotate: u16,
}

/// Why a request failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureKind {
    /// The main process couldn't read the file (gone, truncated, unreadable).
    Io,
    /// A read the engine asked for wasn't answered in time.
    Timeout,
    /// The password doesn't open the document.
    WrongPassword,
    /// The bytes aren't a PDF Lamina PDF can read.
    Invalid,
    /// The document passed one of the resource limits.
    Limit,
    /// No document is open under that id.
    UnknownDocument,
    /// The page doesn't exist.
    PageOutOfRange,
    /// The tile is too large for the limits or for one frame.
    TooLarge,
    /// The request was cancelled.
    Cancelled,
    /// The engine has no renderer (PDFium wasn't found).
    RenderUnavailable,
    /// The renderer failed on this page.
    RenderFailed,
    /// A message the receiver can't act on.
    Protocol,
}

/// A message from the main process to the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToEngine {
    /// The first message: the protocol version and the limits. The engine answers
    /// [`FromEngine::Ready`].
    Hello {
        /// [`crate::PROTOCOL_VERSION`].
        version: u16,
        /// The limits every document of this engine opens with (boxed: the other messages
        /// are small).
        limits: Box<EngineLimits>,
    },
    /// Open the document `doc`, `len` bytes long; the engine reads it with
    /// [`FromEngine::ReadRange`]. Answered by [`FromEngine::Opened`] or [`FromEngine::Error`].
    OpenDocument {
        /// The request.
        req: ReqId,
        /// The document's id, chosen by the main process.
        doc: DocId,
        /// The file's length in bytes.
        len: u64,
    },
    /// Unlock an encrypted document. Answered by [`FromEngine::Unlocked`] or a
    /// [`FromEngine::Error`] of kind [`FailureKind::WrongPassword`].
    UnlockDocument {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// The password the user typed (or the main process kept since they did).
        password: Password,
    },
    /// The answer to [`FromEngine::ReadRange`]: the bytes from `offset` follow the header as
    /// the frame's payload, short only at the end of the file.
    RangeData {
        /// The engine's request.
        req: ReqId,
        /// Where the bytes start.
        offset: u64,
    },
    /// The main process couldn't read a range (the file was truncated or is gone).
    RangeFailed {
        /// The engine's request.
        req: ReqId,
        /// [`FailureKind::Io`].
        kind: FailureKind,
        /// What went wrong, without the file's path.
        message: String,
    },
    /// The size and rotation of every page. Answered by [`FromEngine::PageInfo`].
    GetPageInfo {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
    },
    /// Render a tile. Answered by [`FromEngine::Tile`].
    RenderTile(TileRequest),
    /// Drop a queued request, or skip sending a running one's result. The engine still
    /// answers it, with [`FailureKind::Cancelled`].
    Cancel {
        /// The request.
        req: ReqId,
    },
    /// Forget a document. Not answered.
    CloseDocument {
        /// The document.
        doc: DocId,
    },
    /// Exit. Not answered.
    Shutdown,
}

/// A message from the engine to the main process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FromEngine {
    /// The answer to [`ToEngine::Hello`].
    Ready {
        /// The engine's [`crate::PROTOCOL_VERSION`].
        version: u16,
        /// The engine's process id.
        pid: u32,
    },
    /// The engine needs bytes of a document; answered by [`ToEngine::RangeData`] or
    /// [`ToEngine::RangeFailed`].
    ReadRange {
        /// The engine's request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// The first byte.
        offset: u64,
        /// How many bytes.
        len: u32,
    },
    /// The document opened. An encrypted document whose user password isn't empty opens
    /// locked: `needs_password` is set, `page_count` is 0, and every other request answers
    /// [`FromEngine::NeedsPassword`] until [`ToEngine::UnlockDocument`] succeeds.
    Opened {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// The number of pages (0 while locked).
        page_count: u32,
        /// Whether the file is encrypted.
        encrypted: bool,
        /// Whether it waits for a password.
        needs_password: bool,
        /// Whether reading it raised ADR 0003 warnings (a rebuilt xref, a dropped object).
        repaired: bool,
    },
    /// The answer to any request on a document that waits for its password.
    NeedsPassword {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
    },
    /// The password opened the document.
    Unlocked {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// The number of pages.
        page_count: u32,
        /// Whether reading it raised ADR 0003 warnings.
        repaired: bool,
    },
    /// Every page's size and rotation, in page order.
    PageInfo {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// One entry per page.
        sizes: Vec<PageDimensions>,
    },
    /// A rendered tile; its pixels follow the header as the frame's payload.
    Tile {
        /// The request.
        req: ReqId,
        /// Width in pixels.
        w: u32,
        /// Height in pixels.
        h: u32,
        /// The pixels' format.
        format: TileFormat,
    },
    /// A request failed, or (with no `req`) the engine hit a problem of its own.
    Error {
        /// The request, when the error answers one.
        req: Option<ReqId>,
        /// Why.
        kind: FailureKind,
        /// What happened, for the log. Never document content or a password.
        message: String,
    },
    /// Sent every second, so the main process can tell a hung engine from a busy one.
    Heartbeat,
}
