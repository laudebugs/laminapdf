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
    /// The same, in the order B, G, R, A: what GDI's DIBs and cairo's `ARGB32` (on a
    /// little-endian machine) take as they are. Print bands only; the pixels are opaque.
    Bgra8,
}

/// Which tiles go first: those on screen, then those rendered ahead of a scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Rendered ahead of time, in case the user scrolls there.
    Prefetch,
    /// On screen now.
    Visible,
}

/// The view rotation a tile is drawn at, clockwise (the viewer's rotate command), on top of the
/// page's own `/Rotate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewRotation {
    /// Upright.
    #[default]
    None,
    /// A quarter turn clockwise.
    Quarter,
    /// Upside down.
    Half,
    /// Three quarter turns clockwise.
    ThreeQuarters,
}

/// How a tile's colours are drawn: the reader's page appearance (`@laminapdf/render`'s
/// `PageAppearance`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TileAppearance {
    /// The page's own colours.
    #[default]
    Original,
    /// Warm paper and brown ink.
    Sepia,
    /// Light ink on dark paper.
    Dark,
    /// Two colours, straight RGBA: what black becomes (`ink`) and what white becomes (`paper`).
    HighContrast {
        /// What black becomes.
        ink: [u8; 4],
        /// What white becomes.
        paper: [u8; 4],
    },
}

/// One tile of one page: the rectangle `x, y, w, h` (device pixels) of the page drawn at `scale`
/// pixels per point, turned by `rotation` and coloured by `appearance`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TileRequest {
    /// The request; the reply is [`FromEngine::Tile`] or [`FromEngine::Error`].
    pub req: ReqId,
    /// The document.
    pub doc: DocId,
    /// The page, 0-based.
    pub page: u32,
    /// Pixels per point, device pixel ratio applied (1.0 is 72 dpi): the viewer's own scale, so
    /// the engine's page size in pixels matches the viewer's to the pixel.
    pub scale: f32,
    /// The view rotation.
    pub rotation: ViewRotation,
    /// How the page's colours are drawn.
    pub appearance: TileAppearance,
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

/// One band of a page rendered for printing ([`ToEngine::RenderPrintBand`]): the page at `dpi`,
/// with the print intent (annotations' `Print` flags, PDFium's `FPDF_PRINTING`), its own colours
/// and no view rotation, on white. The engine splits the page into bands of its own height, so
/// a page is never held whole; band `band` is the `band`-th from the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintBandRequest {
    /// The request; the reply is [`FromEngine::PrintBand`] or [`FromEngine::Error`].
    pub req: ReqId,
    /// The document.
    pub doc: DocId,
    /// The page, 0-based.
    pub page: u32,
    /// Pixels per inch. The engine lowers it for a page too large to render at it, and says so
    /// in the reply.
    pub dpi: u16,
    /// Which band, from 0 at the top.
    pub band: u32,
    /// The pixel format of the reply.
    pub format: TileFormat,
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

/// What an edit call asks of a document's edit session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditCall {
    /// Apply an op log (`{ "v": 1, "ops": [ … ] }`, ADR 0006 §6) as one transaction.
    Apply {
        /// The op log as JSON: ops are tagged values, which only a self-describing format carries.
        log: String,
    },
    /// Undo the newest transaction.
    Undo,
    /// Redo the newest undone transaction.
    Redo,
}

/// How an undo or redo went (the façade's `UndoOutcome`). Its names are camelCase for the
/// webview, which reads them as JSON; the pipe's format numbers the variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Reversal {
    /// Every property was restored.
    Applied,
    /// Some were; another change replaced the rest since, or their target is gone.
    Partial,
    /// None could be.
    Skipped,
}

/// A page after an edit: its stable ID and its size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditedPage {
    /// The page's stable ID (`p12.0`), kept across page moves.
    pub id: String,
    /// Its size and rotation.
    pub size: PageDimensions,
}

/// What an edit call did. The pages it touched are drawn from the edited file from then on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditReport {
    /// Whether the document changed: false for an op log that only set what was there, and
    /// for an undo or redo with nothing to undo or redo (`reversal` is then `None`).
    pub changed: bool,
    /// How an undo or redo went.
    pub reversal: Option<Reversal>,
    /// The IDs of the pages whose content or attributes changed.
    pub touched: Vec<String>,
    /// The page order an applied op log reports, when it changed the page list.
    pub page_order: Option<Vec<String>>,
    /// Every page after the call, in page order.
    pub pages: Vec<EditedPage>,
    /// The bookmarks the op log added, by ID.
    pub created_outline_items: Vec<String>,
    /// The annotations the op log added, by ID.
    pub created_annotations: Vec<String>,
    /// The op log found the outline's links broken and rewrote them: read it again.
    pub outline_repaired: bool,
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
    /// The document can't take an edit (it is encrypted), or an op refused it; nothing changed.
    /// The message is the error's code.
    EditRefused,
    /// A save was refused, or its output can't be read or adopted; nothing changed. The message
    /// is the error's code.
    SaveRefused,
    /// An edit failed and couldn't be taken back: the document and the pages drawn may differ,
    /// and it takes no more edits until it opens again.
    EditDiverged,
    /// The engine has no renderer (PDFium wasn't found).
    RenderUnavailable,
    /// The renderer failed on this page.
    RenderFailed,
    /// A message the receiver can't act on.
    Protocol,
}

/// One system font face the main process can serve (ADR 0008 §5, fonts): the engine matches a
/// PDF's font name against these and asks for the face's bytes by `id`, never by path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontFace {
    /// The main process's number for it.
    pub id: u32,
    /// The family name (`name` ID 1), e.g. `Helvetica`.
    pub family: String,
    /// The style name (`name` ID 2), e.g. `Bold Italic`.
    pub style: String,
    /// `OS/2` `usWeightClass`: 400 regular, 700 bold.
    pub weight: u16,
    /// Italic or oblique.
    pub italic: bool,
    /// The scripts it covers, from `OS/2` `ulCodePageRange1`.
    pub charsets: Charsets,
}

/// A set of the character sets PDFium asks fonts for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Charsets(pub u8);

impl Charsets {
    /// Latin 1.
    pub const ANSI: Self = Self(1);
    /// Symbol.
    pub const SYMBOL: Self = Self(1 << 1);
    /// Japanese (Shift JIS).
    pub const SHIFT_JIS: Self = Self(1 << 2);
    /// Simplified Chinese (GB 2312).
    pub const GB2312: Self = Self(1 << 3);
    /// Traditional Chinese (Big5).
    pub const BIG5: Self = Self(1 << 4);
    /// Korean (Hangul).
    pub const HANGUL: Self = Self(1 << 5);

    /// Whether every set in `other` is in this one.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Both sets together.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
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
    /// What the Properties dialog shows of the document, with its edits so far. Answered by
    /// [`FromEngine::Properties`].
    GetProperties {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
    },
    /// Render a tile. Answered by [`FromEngine::Tile`].
    RenderTile(TileRequest),
    /// Apply, undo or redo an edit of the document. Answered by [`FromEngine::Edited`], or a
    /// [`FromEngine::Error`] after which the document and its pages are as they were.
    Edit {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// What to do.
        call: EditCall,
    },
    /// Plan, write, read or adopt a save of the document (`crate::SaveCall`). Answered by
    /// [`FromEngine::Saved`], or a [`FromEngine::Error`] after which the document is as it was.
    Save {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// What to do.
        call: crate::SaveCall,
    },
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
    /// The system's font faces the main process can serve, sent once, right after `Ready` and
    /// before any document: the engine's PDFium matches fonts a PDF doesn't embed against them.
    Fonts {
        /// Every face, by id.
        faces: Vec<FontFace>,
    },
    /// The answer to [`FromEngine::FontRequest`]: the face as a standalone font file follows as
    /// the frame's payload.
    FontData {
        /// The engine's request.
        req: ReqId,
    },
    /// The main process couldn't serve a face (gone, too large, unreadable).
    FontFailed {
        /// The engine's request.
        req: ReqId,
        /// What went wrong, without the file's path.
        message: String,
    },
    /// Render one band of a page for printing. Answered by [`FromEngine::PrintBand`].
    RenderPrintBand(PrintBandRequest),
    /// Panic with `message`, as a crash would: the end-to-end tests' forced engine panic. Only the
    /// e2e builds have it (feature `debug-crash`).
    #[cfg(feature = "debug-crash")]
    DebugPanic {
        /// The panic's message, which the test fills with what a report must not hold.
        message: String,
    },
    /// Fault, as a native crash would: a write through a null pointer, which the engine's crash
    /// handler turns into a minidump. Only the e2e builds have it (feature `debug-crash`).
    #[cfg(feature = "debug-crash")]
    DebugSegv,
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
        /// What the sandbox enforces; sent last, once every layer is on (ADR 0008 §5).
        sandboxed: crate::SandboxLevel,
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
        /// Each page's stable ID (`p12.0`), in the same order.
        ids: Vec<String>,
    },
    /// What the Properties dialog shows of the document: its properties, its metadata and its
    /// XMP packet, as one JSON object (`laminapdf-desktop-engine`'s `properties` module), since the
    /// pipe's format can't carry a self-describing value. The webview reads it as it is.
    Properties {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// The JSON object.
        json: String,
    },
    /// What an edit call did.
    Edited {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// What changed (boxed: the other messages are small).
        report: Box<EditReport>,
    },
    /// What a save call did; [`crate::SaveAnswer::Bytes`] carries the output's bytes as the
    /// frame's payload.
    Saved {
        /// The request.
        req: ReqId,
        /// The document.
        doc: DocId,
        /// The answer (boxed: the other messages are small).
        answer: Box<crate::SaveAnswer>,
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
    /// The engine panicked: sent from its panic hook, just before it aborts, so the main process
    /// can build a crash report. Unscrubbed: the main process scrubs it.
    Panic(Box<crate::crash::PanicSummary>),
    /// The engine needs a system font face's bytes; answered by [`ToEngine::FontData`] or
    /// [`ToEngine::FontFailed`].
    FontRequest {
        /// The engine's request.
        req: ReqId,
        /// The face's [`FontFace::id`].
        font: u32,
    },
    /// A band of a page rendered for printing; its pixels (`width × rows × 4` bytes) follow the
    /// header as the frame's payload.
    PrintBand {
        /// The request.
        req: ReqId,
        /// The resolution the page was rendered at: the request's, or lower for a page too large
        /// to render at it.
        dpi: u16,
        /// The whole page's width in pixels, which is the band's width too.
        width: u32,
        /// The whole page's height in pixels.
        height: u32,
        /// The band's first row on the page.
        y: u32,
        /// The band's height in rows.
        rows: u32,
        /// How many bands the page has.
        bands: u32,
        /// The pixels' format.
        format: TileFormat,
    },
}
