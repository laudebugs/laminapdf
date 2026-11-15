//! The worker thread: opens, unlocks, measures and renders documents, one job at a time.
//!
//! One thread, because PDFium serialises every call anyway (`laminapdf-render-pdfium` holds one
//! process-wide lock) and the façade's `Document` is used from one thread. The router stays free
//! to deliver the bytes a job is waiting for.

use std::collections::BTreeMap;
use std::sync::Arc;

use laminapdf::OpenOptions;
use laminapdf_desktop_protocol::{
    DocId, EditCall, EngineLimits, FailureKind, FromEngine, MAX_FRAME, PageDimensions, Password, PrintBandRequest,
    ReqId, SaveCall, TileAppearance, TileFormat, TileRequest, ViewRotation,
};
use laminapdf_render::{
    ByteSource, CancelToken, PageAppearance, PixelRect, RenderBackend, RenderDocument, RenderOpenOptions,
    RenderRequest, Rgba8, Rotation,
};

use crate::limits;
use crate::outbox::Outbox;
use crate::queue::{Job, Queue};
use crate::remote::{Ranges, RemoteSource};

/// Room left in a tile's frame for its header and fixed fields.
const TILE_FRAME_SLACK: usize = 4096;

/// A document the engine holds.
enum Doc {
    /// Encrypted, waiting for its password.
    Locked {
        source: Arc<RemoteSource>,
    },
    Open(Box<Opened>),
}

/// An open document: Lamina PDF's model for facts, PDFium's for pixels.
pub(crate) struct Opened {
    pub(crate) lamina: laminapdf::Document,
    /// `None` without PDFium, or when PDFium couldn't open a file Lamina PDF could.
    pub(crate) render: Option<Box<dyn RenderDocument>>,
    /// The file as opened, which an edit's revision is appended to for PDFium.
    pub(crate) source: Arc<dyn ByteSource>,
    page_count: u32,
    pub(crate) encrypted: bool,
    repaired: bool,
    /// An edit failed and couldn't be taken back: the model and the pages drawn may differ.
    pub(crate) diverged: bool,
    /// A save's output, written and waiting for the main process to put it in place.
    pub(crate) pending: Option<crate::save::Pending>,
    /// The Properties dialog's facts, which read every object: kept until an edit, an undo, a
    /// redo or a save changes the document they describe.
    pub(crate) facts: Option<crate::properties::Facts>,
}

/// Why a document didn't open.
enum OpenFailed {
    NeedsPassword,
    Error(FailureKind, String),
}

pub(crate) struct Worker {
    out: Outbox,
    ranges: Arc<Ranges>,
    backend: Option<Box<dyn RenderBackend>>,
    limits: Box<EngineLimits>,
    docs: BTreeMap<DocId, Doc>,
}

impl Worker {
    pub(crate) fn new(
        out: Outbox,
        ranges: Arc<Ranges>,
        backend: Option<Box<dyn RenderBackend>>,
        limits: Box<EngineLimits>,
    ) -> Self {
        Self {
            out,
            ranges,
            backend,
            limits,
            docs: BTreeMap::new(),
        }
    }

    /// Runs jobs until the queue closes.
    pub(crate) fn run(mut self, queue: &Queue) {
        while let Some(job) = queue.pop() {
            let token = CancelToken::new();
            queue.start(job.req(), token.clone());
            match job {
                Job::Open { req, doc, len } => self.open(req, doc, len),
                Job::Unlock { req, doc, password } => self.unlock(req, doc, &password),
                Job::PageInfo { req, doc } => self.page_info(req, doc),
                Job::Properties { req, doc } => self.properties(req, doc),
                Job::Render(tile) => self.render(&tile, &token),
                Job::Edit { req, doc, call } => self.edit(req, doc, call),
                Job::Save { req, doc, call } => self.save(req, doc, call),
                Job::Print(band) => self.print_band(&band, &token),
                Job::Close { doc } => {
                    self.docs.remove(&doc);
                }
            }
            queue.finish();
        }
    }

    fn open(&mut self, req: ReqId, doc: DocId, len: u64) {
        let source = Arc::new(RemoteSource::new(doc, len, Arc::clone(&self.ranges)));
        match self.open_source(&source, None) {
            Ok(opened) => {
                self.send(&FromEngine::Opened {
                    req,
                    doc,
                    page_count: opened.page_count,
                    encrypted: opened.encrypted,
                    needs_password: false,
                    repaired: opened.repaired,
                });
                self.docs.insert(doc, Doc::Open(Box::new(opened)));
            }
            Err(OpenFailed::NeedsPassword) => {
                self.docs.insert(doc, Doc::Locked { source });
                self.send(&FromEngine::Opened {
                    req,
                    doc,
                    page_count: 0,
                    encrypted: true,
                    needs_password: true,
                    repaired: false,
                });
            }
            Err(OpenFailed::Error(kind, message)) => {
                self.docs.remove(&doc);
                self.error(req, kind, message);
            }
        }
    }

    fn unlock(&mut self, req: ReqId, doc: DocId, password: &Password) {
        let source = match self.docs.get(&doc) {
            None => return self.error(req, FailureKind::UnknownDocument, "no such document".into()),
            Some(Doc::Open(opened)) => {
                // Already open (its password was empty, or a repeated unlock): nothing to do.
                let (page_count, repaired) = (opened.page_count, opened.repaired);
                return self.send(&FromEngine::Unlocked {
                    req,
                    doc,
                    page_count,
                    repaired,
                });
            }
            Some(Doc::Locked { source }) => Arc::clone(source),
        };
        match self.open_source(&source, Some(password)) {
            Ok(opened) => {
                self.send(&FromEngine::Unlocked {
                    req,
                    doc,
                    page_count: opened.page_count,
                    repaired: opened.repaired,
                });
                self.docs.insert(doc, Doc::Open(Box::new(opened)));
            }
            Err(OpenFailed::NeedsPassword) => self.error(req, FailureKind::WrongPassword, "wrong-password".into()),
            Err(OpenFailed::Error(kind, message)) => self.error(req, kind, message),
        }
    }

    fn page_info(&self, req: ReqId, doc: DocId) {
        let opened = match self.docs.get(&doc) {
            None => return self.error(req, FailureKind::UnknownDocument, "no such document".into()),
            Some(Doc::Locked { .. }) => return self.send(&FromEngine::NeedsPassword { req, doc }),
            Some(Doc::Open(opened)) => opened,
        };
        match opened.lamina.page_infos() {
            Ok(pages) => {
                let ids = pages.iter().map(|page| page.id.to_string()).collect();
                let sizes = pages.iter().map(dimensions).collect();
                self.send(&FromEngine::PageInfo { req, doc, sizes, ids });
            }
            Err(error) => self.error(req, kind_of(error.code()), error.code().into()),
        }
    }

    fn properties(&mut self, req: ReqId, doc: DocId) {
        let opened = match self.docs.get_mut(&doc) {
            None => return self.error(req, FailureKind::UnknownDocument, "no such document".into()),
            Some(Doc::Locked { .. }) => return self.send(&FromEngine::NeedsPassword { req, doc }),
            Some(Doc::Open(opened)) => opened,
        };
        match crate::properties::read(&opened.lamina, &mut opened.facts) {
            Ok(json) => self.send(&FromEngine::Properties { req, doc, json }),
            Err(code) => self.error(req, kind_of(&code), code),
        }
    }

    fn edit(&mut self, req: ReqId, doc: DocId, call: EditCall) {
        let opened = match self.docs.get_mut(&doc) {
            None => return self.error(req, FailureKind::UnknownDocument, "no such document".into()),
            Some(Doc::Locked { .. }) => return self.send(&FromEngine::NeedsPassword { req, doc }),
            Some(Doc::Open(opened)) => opened,
        };
        // A save's output written before this edit no longer matches the document, nor do the
        // Properties dialog's facts.
        opened.pending = None;
        opened.facts = None;
        match crate::edit::run(opened, self.backend.as_deref(), &self.limits, call) {
            Ok(report) => {
                opened.page_count = u32::try_from(report.pages.len()).unwrap_or(u32::MAX);
                self.send(&FromEngine::Edited {
                    req,
                    doc,
                    report: Box::new(report),
                });
            }
            Err((kind, code)) => self.error(req, kind, code),
        }
    }

    fn save(&mut self, req: ReqId, doc: DocId, call: SaveCall) {
        let opened = match self.docs.get_mut(&doc) {
            None => return self.error(req, FailureKind::UnknownDocument, "no such document".into()),
            Some(Doc::Locked { .. }) => return self.send(&FromEngine::NeedsPassword { req, doc }),
            Some(Doc::Open(opened)) => opened,
        };
        // An adopted save changes the file the facts describe (its size, its revisions).
        if matches!(call, SaveCall::Adopt) {
            opened.facts = None;
        }
        // Room in the frame for the header besides the bytes.
        let max_read = MAX_FRAME.saturating_sub(u32::try_from(TILE_FRAME_SLACK).unwrap_or(u32::MAX));
        match crate::save::run(opened, self.backend.as_deref(), &self.limits, call, max_read) {
            Ok((answer, payload)) => self.send_payload(
                &FromEngine::Saved {
                    req,
                    doc,
                    answer: Box::new(answer),
                },
                &payload,
            ),
            Err((kind, code)) => self.error(req, kind, code),
        }
    }

    /// The renderer of `doc`, or `None` once the failure is answered.
    fn renderer(&self, req: ReqId, doc: DocId) -> Option<&dyn RenderDocument> {
        let opened = match self.docs.get(&doc) {
            None => {
                self.error(req, FailureKind::UnknownDocument, "no such document".into());
                return None;
            }
            Some(Doc::Locked { .. }) => {
                self.send(&FromEngine::NeedsPassword { req, doc });
                return None;
            }
            Some(Doc::Open(opened)) => opened,
        };
        let render = opened.render.as_deref();
        if render.is_none() {
            let kind = if self.backend.is_some() {
                FailureKind::RenderFailed
            } else {
                FailureKind::RenderUnavailable
            };
            self.error(req, kind, "no renderer for this document".into());
        }
        render
    }

    fn render(&self, tile: &TileRequest, cancel: &CancelToken) {
        let req = tile.req;
        let Some(render) = self.renderer(req, tile.doc) else {
            return;
        };
        let request = RenderRequest::page(tile.page, tile.scale)
            .with_clip(PixelRect {
                x: tile.x,
                y: tile.y,
                width: tile.w,
                height: tile.h,
            })
            .with_rotation(rotation(tile.rotation))
            .with_appearance(appearance(tile.appearance));
        let plan = match render.plan(&request) {
            Ok(plan) => plan,
            Err(error) => return self.error(req, kind_of(error.code()), error.code().into()),
        };
        let len = plan.byte_len();
        if len.saturating_add(TILE_FRAME_SLACK) > MAX_FRAME as usize {
            return self.error(req, FailureKind::TooLarge, "the tile is larger than one frame".into());
        }
        let mut pixels = vec![0_u8; len];
        match render.render(&request, &mut pixels, cancel) {
            Ok(plan) => self.send_payload(
                &FromEngine::Tile {
                    req,
                    w: plan.tile.width,
                    h: plan.tile.height,
                    format: TileFormat::Rgba8,
                },
                &pixels,
            ),
            Err(error) => self.error(req, kind_of(error.code()), error.code().into()),
        }
    }

    /// Renders one band of a page for printing ([`crate::print`] does the maths).
    fn print_band(&self, band: &PrintBandRequest, cancel: &CancelToken) {
        let req = band.req;
        let Some(render) = self.renderer(req, band.doc) else {
            return;
        };
        let planned = crate::print::plan(render, band);
        let (request, reply, len) = match planned {
            Ok(planned) => planned,
            Err((kind, message)) => return self.error(req, kind, message),
        };
        let mut pixels = vec![0_u8; len];
        match render.render(&request, &mut pixels, cancel) {
            Ok(_) => {
                if band.format == TileFormat::Bgra8 {
                    crate::print::rgba_to_bgra(&mut pixels);
                }
                self.send_payload(&reply, &pixels);
            }
            Err(error) => self.error(req, kind_of(error.code()), error.code().into()),
        }
    }

    /// Opens `source` with Lamina PDF, then with the renderer when there is one.
    fn open_source(&self, source: &Arc<RemoteSource>, password: Option<&Password>) -> Result<Opened, OpenFailed> {
        let shared: Arc<dyn ByteSource> = Arc::clone(source) as Arc<dyn ByteSource>;
        let mut options = OpenOptions::new().with_limits(limits::parse(&self.limits));
        if let Some(password) = password {
            options = options.with_password(password.expose().as_bytes());
        }
        let lamina = match laminapdf::Document::open_source(Arc::clone(&shared), options) {
            Ok(doc) => doc,
            Err(error) if error.code() == "password-required" => return Err(OpenFailed::NeedsPassword),
            Err(error) if error.code() == "wrong-password" && password.is_none() => {
                return Err(OpenFailed::NeedsPassword);
            }
            Err(error) => return Err(OpenFailed::Error(kind_of(error.code()), error.code().into())),
        };
        let page_count = lamina
            .page_count()
            .map_err(|error| OpenFailed::Error(kind_of(error.code()), error.code().into()))?;
        let render = self.backend.as_ref().and_then(|backend| {
            let mut options = RenderOpenOptions::new().with_limits(limits::render(&self.limits));
            if let Some(password) = password {
                options = options.with_password(password.expose());
            }
            backend
                .open(Arc::clone(&shared), &options)
                .map_err(|error| crate::log(&format!("the renderer could not open a document: {}", error.code())))
                .ok()
        });
        Ok(Opened {
            encrypted: lamina.is_encrypted(),
            repaired: lamina.warnings().remaining() > 0,
            lamina,
            render,
            source: shared,
            page_count,
            diverged: false,
            pending: None,
            facts: None,
        })
    }

    fn send(&self, msg: &FromEngine) {
        self.out.send_or_log(msg, &[]);
    }

    fn send_payload(&self, msg: &FromEngine, payload: &[u8]) {
        self.out.send_or_log(msg, payload);
    }

    fn error(&self, req: ReqId, kind: FailureKind, message: String) {
        self.send(&FromEngine::Error {
            req: Some(req),
            kind,
            message,
        });
    }
}

/// A page's size and rotation, as the protocol carries them.
pub(crate) fn dimensions(page: &laminapdf::PageInfo) -> PageDimensions {
    PageDimensions {
        #[expect(clippy::cast_possible_truncation, reason = "page sizes in points fit an f32")]
        width: page.width as f32,
        #[expect(clippy::cast_possible_truncation, reason = "page sizes in points fit an f32")]
        height: page.height as f32,
        rotate: page.rotation,
    }
}

/// The protocol's kind for a parser or renderer error code. Only codes cross the pipe, never an
/// error's text, which can quote the document.
pub(crate) fn kind_of(code: &str) -> FailureKind {
    match code {
        "wrong-password" | "password-required" => FailureKind::WrongPassword,
        "limit-exceeded" => FailureKind::Limit,
        "render-too-large" => FailureKind::TooLarge,
        "page-index-out-of-range" => FailureKind::PageOutOfRange,
        "source" | "data-pending" => FailureKind::Io,
        "cancelled" => FailureKind::Cancelled,
        "render-backend-unavailable" => FailureKind::RenderUnavailable,
        "render-failed" => FailureKind::RenderFailed,
        "invalid-argument" => FailureKind::Protocol,
        _ => FailureKind::Invalid,
    }
}

/// The protocol's view rotation as the renderer's.
const fn rotation(rotation: ViewRotation) -> Rotation {
    match rotation {
        ViewRotation::None => Rotation::None,
        ViewRotation::Quarter => Rotation::Clockwise90,
        ViewRotation::Half => Rotation::Half,
        ViewRotation::ThreeQuarters => Rotation::Clockwise270,
    }
}

/// The protocol's page appearance as the renderer's.
const fn appearance(appearance: TileAppearance) -> PageAppearance {
    const fn rgba([r, g, b, a]: [u8; 4]) -> Rgba8 {
        Rgba8 { r, g, b, a }
    }
    match appearance {
        TileAppearance::Original => PageAppearance::Original,
        TileAppearance::Sepia => PageAppearance::Sepia,
        TileAppearance::Dark => PageAppearance::Dark,
        TileAppearance::HighContrast { ink, paper } => PageAppearance::HighContrast {
            ink: rgba(ink),
            paper: rgba(paper),
        },
    }
}
