//! The engine's panics, for crash reports: the binary's panic hook ([`report_panics`])
//! sends the main process a summary of the panic (its message and place, without frames, which the
//! sandbox stops the engine naming), then aborts, so the main process sees a crash and
//! restarts the engine as it does after any other. Without the abort, a panic in the worker
//! would leave an engine that beats but never answers.

use std::sync::OnceLock;

use laminapdf_desktop_protocol::FromEngine;
use laminapdf_desktop_protocol::crash::summarize_without_frames;

use crate::outbox::Outbox;

/// Where the hook sends the summary: the main process, once [`crate::run`] has said `Ready`.
static TO_MAIN: OnceLock<Outbox> = OnceLock::new();

/// Makes every panic from here on end the engine, after telling the main process what it was.
/// The binary calls it once the sandbox is on and it starts serving; the selftest, whose probes
/// catch the panics of calls the sandbox refuses, and the library's tests don't.
pub fn report_panics() {
    std::panic::set_hook(Box::new(|info| {
        let summary = summarize_without_frames(info);
        if let Some(out) = TO_MAIN.get() {
            out.try_send(&FromEngine::Panic(Box::new(summary.clone())));
        }
        // The message goes to stderr, which the main process keeps in its log. Not through the
        // default hook: with RUST_BACKTRACE set it names the frames, which reads the binary from
        // disk, and Linux's seccomp ends the engine for that before the summary is sent.
        let at = summary
            .location
            .map(|at| format!(" at {}:{}:{}", at.file, at.line, at.column))
            .unwrap_or_default();
        crate::log(&format!("panicked{at}: {}", summary.message));
        // This report is the crash's: the abort asks the crash handler for no dump (on Windows the
        // abort is a fail-fast no handler sees anyway), as the main process's panic hook does.
        laminapdf_desktop_crash::skip_dumps();
        std::process::abort();
    }));
}

/// The pipe to the main process, for the hook.
pub(crate) fn connect(out: &Outbox) {
    let _ = TO_MAIN.set(out.clone());
}

/// `ToEngine::DebugPanic`: the end-to-end tests' forced engine panic, with their `message`.
#[cfg(feature = "e2e")]
pub(crate) fn debug_panic(message: &str) {
    #[expect(clippy::panic, reason = "the end-to-end tests' forced engine panic")]
    {
        panic!("{message}");
    }
}

/// `ToEngine::DebugSegv`: the end-to-end tests' forced native fault, a write through a null
/// pointer, which the crash handler turns into a minidump.
#[cfg(feature = "e2e")]
pub(crate) fn debug_segv() {
    laminapdf_desktop_crash::fault();
}
