//! The engine's panics, for crash reports (D7a): the binary's panic hook ([`report_panics`])
//! sends the main process a summary of the panic, then aborts, so the main process sees a crash and
//! restarts the engine as it does after any other (D2b). Without the abort, a panic in the worker
//! would leave an engine that beats but never answers.

use std::sync::OnceLock;

use laminapdf_desktop_protocol::FromEngine;
use laminapdf_desktop_protocol::crash::summarize;

use crate::outbox::Outbox;

/// Where the hook sends the summary: the main process, once [`crate::run`] has said `Ready`.
static TO_MAIN: OnceLock<Outbox> = OnceLock::new();

/// The engine's binary, as frames name it.
const MODULE: &str = "laminapdf-desktop-engine";

/// Makes every panic from here on end the engine, after telling the main process what it was.
/// The binary calls it; the library's tests don't, so a test's panic stays a test failure.
pub fn report_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // The default hook writes the message to stderr, which the main process keeps in its log.
        previous(info);
        if let Some(out) = TO_MAIN.get() {
            out.try_send(&FromEngine::Panic(Box::new(summarize(info, MODULE))));
        }
        std::process::abort();
    }));
}

/// The pipe to the main process, for the hook.
pub(crate) fn connect(out: &Outbox) {
    let _ = TO_MAIN.set(out.clone());
}

/// [`laminapdf_desktop_protocol::ToEngine::DebugPanic`]: a debug build panics, as the end-to-end
/// tests ask; a release build says no and carries on.
pub(crate) fn debug_panic(out: &Outbox) {
    #[cfg(debug_assertions)]
    {
        let _ = out;
        #[expect(clippy::panic, reason = "the end-to-end tests' forced engine panic, in debug builds only")]
        {
            panic!("a panic the end-to-end tests asked for");
        }
    }
    #[cfg(not(debug_assertions))]
    crate::report(out, "DebugPanic ignored: this is a release engine");
}
