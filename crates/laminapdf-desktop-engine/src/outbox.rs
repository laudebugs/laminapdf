//! The engine's one way out: every thread sends its frames through [`Outbox`], one at a time.

use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};

use laminapdf_desktop_protocol::{FromEngine, ProtocolError, write_frame};

/// The engine's stdout (or a test's pipe), shared by the router, the worker, the byte sources
/// and the heartbeat. A frame is written whole under the lock, so frames never interleave.
#[derive(Clone)]
pub(crate) struct Outbox(Arc<Mutex<Box<dyn Write + Send>>>);

impl Outbox {
    pub(crate) fn new(out: Box<dyn Write + Send>) -> Self {
        Self(Arc::new(Mutex::new(out)))
    }

    /// Sends `msg` with `payload` and flushes it.
    pub(crate) fn send(&self, msg: &FromEngine, payload: &[u8]) -> Result<(), ProtocolError> {
        // A thread that panicked mid-write left a half frame; the next frame then fails to
        // parse on the other side, which restarts the engine. Nothing better is possible here.
        let mut out = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        write_frame(&mut *out, msg, payload)
    }

    /// Sends `msg` unless another frame is being written: for the panic hook, which may run on a
    /// thread that holds the lock already, where waiting would never end.
    pub(crate) fn try_send(&self, msg: &FromEngine) {
        let mut out = match self.0.try_lock() {
            Ok(out) => out,
            // A thread that panicked mid-write may have left half a frame: this one wouldn't parse.
            Err(std::sync::TryLockError::Poisoned(_) | std::sync::TryLockError::WouldBlock) => return,
        };
        let _ = write_frame(&mut *out, msg, &[]);
    }

    /// Sends `msg`, logging a failure: the pipe is gone, and the router sees that too.
    pub(crate) fn send_or_log(&self, msg: &FromEngine, payload: &[u8]) {
        if let Err(error) = self.send(msg, payload) {
            crate::log(&format!("could not send a message: {error}"));
        }
    }
}
