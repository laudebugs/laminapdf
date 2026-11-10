//! The worker thread: opens, unlocks, measures and renders documents, one job at a time.
//!
//! One thread, because PDFium serialises every call anyway (`laminapdf-render-pdfium` holds one
//! process-wide lock) and the façade's `Document` is used from one thread. The router stays free
//! to deliver the bytes a job is waiting for.

use std::collections::BTreeMap;
use std::sync::Arc;

use laminapdf::OpenOptions;
use laminapdf_desktop_protocol::{
    DocId, EngineLimits, FailureKind, FromEngine, MAX_FRAME, PageDimensions, Password, ReqId, TileFormat, TileRequest,
};
use laminapdf_render::{
    ByteSource, CancelToken, PixelRect, RenderBackend, RenderDocument, RenderOpenOptions, RenderRequest,
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
struct Opened {
    lamina: laminapdf::Document,
    /// `None` without PDFium, or when PDFium couldn't open a file Lamina PDF could.
    render: Option<Box<dyn RenderDocument>>,
    page_count: u32,
    encrypted: bool,
    repaired: bool,
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
                Job::Render(tile) => self.render(&tile, &token),
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
                let sizes = pages
                    .iter()
                    .map(|page| PageDimensions {
                        #[expect(clippy::cast_possible_truncation, reason = "page sizes in points fit an f32")]
                        width: page.width as f32,
                        #[expect(clippy::cast_possible_truncation, reason = "page sizes in points fit an f32")]
                        height: page.height as f32,
                        rotate: page.rotation,
                    })
                    .collect();
                self.send(&FromEngine::PageInfo { req, doc, sizes });
            }
            Err(error) => self.error(req, kind_of(error.code()), error.code().into()),
        }
    }

    fn render(&self, tile: &TileRequest, cancel: &CancelToken) {
        let req = tile.req;
        let opened = match self.docs.get(&tile.doc) {
            None => return self.error(req, FailureKind::UnknownDocument, "no such document".into()),
            Some(Doc::Locked { .. }) => return self.send(&FromEngine::NeedsPassword { req, doc: tile.doc }),
            Some(Doc::Open(opened)) => opened,
        };
        let Some(render) = opened.render.as_deref() else {
            let kind = if self.backend.is_some() {
                FailureKind::RenderFailed
            } else {
                FailureKind::RenderUnavailable
            };
            return self.error(req, kind, "no renderer for this document".into());
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a scale in thousandths is far below f32's exact range"
        )]
        let scale = tile.scale_milli as f32 / 1000.0;
        let request = RenderRequest::page(tile.page, scale).with_clip(PixelRect {
            x: tile.x,
            y: tile.y,
            width: tile.w,
            height: tile.h,
        });
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
                .open(shared, &options)
                .map_err(|error| crate::log(&format!("the renderer could not open a document: {}", error.code())))
                .ok()
        });
        Ok(Opened {
            encrypted: lamina.is_encrypted(),
            repaired: lamina.warnings().remaining() > 0,
            lamina,
            render,
            page_count,
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

/// The protocol's kind for a parser or renderer error code. Only codes cross the pipe, never an
/// error's text, which can quote the document.
fn kind_of(code: &str) -> FailureKind {
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
