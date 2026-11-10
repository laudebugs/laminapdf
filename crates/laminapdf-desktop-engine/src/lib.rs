//! Lamina PDF's desktop engine process (ADR 0008 §2): parses and renders untrusted PDFs for the
//! main process, which owns every file and never parses one.
//!
//! The engine reads frames from its stdin and writes them to its stdout
//! (`laminapdf-desktop-protocol`); it logs to stderr, never document content. Three threads:
//!
//! - **The router** ([`run`]'s caller) only decodes and routes frames. File bytes go straight to
//!   the read waiting for them, a cancel reaches the queue, and every other request goes to the
//!   worker. It never parses or renders: a parse waiting for bytes would otherwise wait for a frame
//!   only the router can deliver.
//! - **The worker** runs one job at a time: documents first, then visible tiles, then prefetched
//!   ones. It reads each document through a source whose reads become `ReadRange` requests.
//! - **The heartbeat** sends `Heartbeat` every second, so the main process can tell a hung engine
//!   from a busy one.

mod limits;
mod outbox;
mod queue;
mod remote;
mod worker;

use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use laminapdf_desktop_protocol::{
    ErrorKind, FromEngine, MAX_FRAME, PROTOCOL_VERSION, ProtocolError, ToEngine, read_frame,
};
use laminapdf_render::RenderBackend;

use crate::outbox::Outbox;
use crate::queue::{Cancelled, Job, Queue};
use crate::remote::Ranges;
use crate::worker::Worker;

/// How often the engine says it is alive.
pub const HEARTBEAT: Duration = Duration::from_secs(1);

/// Serves the main process on `input` and `output` until it sends `Shutdown` or closes the pipe.
/// `backend` renders tiles; without one, documents open and report their pages but every tile
/// answers `RenderUnavailable`.
///
/// # Errors
/// [`ProtocolError`] when the first message isn't a `Hello` of this version, a frame is malformed,
/// or the pipe fails: the main process then restarts the engine.
pub fn run<R: Read, W: Write + Send + 'static>(
    mut input: R,
    output: W,
    backend: Option<Box<dyn RenderBackend>>,
) -> Result<(), ProtocolError> {
    let out = Outbox::new(Box::new(output));
    let limits = match read_frame(&mut input, MAX_FRAME)? {
        (ToEngine::Hello { version, limits }, _) if version == PROTOCOL_VERSION => limits,
        (ToEngine::Hello { version, .. }, _) => {
            report(&out, &format!("main process speaks protocol {version}"));
            return Err(ProtocolError::Version { found: version });
        }
        _ => {
            report(&out, "the first message was not Hello");
            return Err(ProtocolError::Malformed("the first message was not Hello"));
        }
    };
    out.send(
        &FromEngine::Ready {
            version: PROTOCOL_VERSION,
            pid: std::process::id(),
        },
        &[],
    )?;

    let ranges = Ranges::new(out.clone());
    let queue = Arc::new(Queue::default());
    let worker = {
        let (out, ranges, queue) = (out.clone(), Arc::clone(&ranges), Arc::clone(&queue));
        thread::Builder::new()
            .name("engine-worker".into())
            .spawn(move || Worker::new(out, ranges, backend, limits).run(&queue))?
    };
    let alive = Arc::new(AtomicBool::new(true));
    {
        let (out, alive) = (out.clone(), Arc::clone(&alive));
        thread::Builder::new().name("engine-heartbeat".into()).spawn(move || {
            while alive.load(Ordering::SeqCst) {
                thread::sleep(HEARTBEAT);
                if out.send(&FromEngine::Heartbeat, &[]).is_err() {
                    break;
                }
            }
        })?;
    }

    let result = route(&mut input, &out, &ranges, &queue);
    alive.store(false, Ordering::SeqCst);
    ranges.close();
    queue.close();
    if worker.join().is_err() {
        log("the worker thread panicked");
    }
    result
}

/// The router's loop: decodes each frame and hands it on, until `Shutdown` or the pipe closes.
fn route<R: Read>(input: &mut R, out: &Outbox, ranges: &Ranges, queue: &Queue) -> Result<(), ProtocolError> {
    loop {
        let (msg, payload) = match read_frame(input, MAX_FRAME) {
            Ok(frame) => frame,
            Err(ProtocolError::Closed) => return Ok(()),
            Err(error) => {
                report(out, &format!("unreadable frame: {error}"));
                return Err(error);
            }
        };
        match msg {
            ToEngine::RangeData { req, .. } => ranges.deliver(req, Ok(payload)),
            ToEngine::RangeFailed { req, message, .. } => ranges.deliver(req, Err(message)),
            ToEngine::Cancel { req } => {
                if matches!(queue.cancel(req), Cancelled::Dequeued) {
                    out.send_or_log(
                        &FromEngine::Error {
                            req: Some(req),
                            kind: ErrorKind::Cancelled,
                            message: "cancelled".into(),
                        },
                        &[],
                    );
                }
            }
            ToEngine::Shutdown => return Ok(()),
            ToEngine::Hello { .. } => report(out, "Hello sent twice"),
            ToEngine::OpenDocument { req, doc, len } => queue.push(Job::Open { req, doc, len }),
            ToEngine::UnlockDocument { req, doc, password } => queue.push(Job::Unlock { req, doc, password }),
            ToEngine::GetPageInfo { req, doc } => queue.push(Job::PageInfo { req, doc }),
            ToEngine::RenderTile(tile) => queue.push(Job::Render(tile)),
            ToEngine::CloseDocument { doc } => queue.push(Job::Close { doc }),
        }
    }
}

/// Logs `message` and tells the main process, which logs it too.
fn report(out: &Outbox, message: &str) {
    log(message);
    out.send_or_log(
        &FromEngine::Error {
            req: None,
            kind: ErrorKind::Protocol,
            message: message.into(),
        },
        &[],
    );
}

/// One line on stderr, which the main process copies into its log. Never document content.
pub(crate) fn log(message: &str) {
    let mut err = std::io::stderr().lock();
    // Nothing to do if stderr is gone.
    let _ = writeln!(err, "{message}");
}
