//! A minidump read into its summary: the crashing thread's frames as modules and offsets, the
//! modules they name, and why the process crashed. The raw dump holds the process's memory, so
//! possibly document bytes and paths; only this summary, after the main process's allow-list, is
//! ever sent.
//!
//! The types cross a process boundary as JSON: the engine reads a dump in its `--summarize-dump`
//! mode under the sandbox ([`summarize_bytes`], feature `summary`) and writes a [`DumpSummary`] to
//! its stdout; the main process reads it back without linking the minidump parser.

use serde::{Deserialize, Serialize};

/// The most frames kept, innermost first (the report's schema allows 64).
pub const MAX_FRAMES: usize = 64;

/// What a dump says about its crash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DumpSummary {
    /// The crashed process's id, to join the dump with the supervisor's `Crashed`.
    pub pid: Option<u32>,
    /// Why it crashed, as the OS put it (`EXC_BAD_ACCESS / KERN_INVALID_ADDRESS`,
    /// `SIGSEGV / SEGV_MAPERR`, `EXCEPTION_ACCESS_VIOLATION_WRITE`); unscrubbed.
    pub crash_reason: String,
    /// The crashing thread's frames that fall in a module, innermost first.
    pub frames: Vec<DumpFrame>,
    /// The modules those frames name, each once.
    pub modules: Vec<DumpModule>,
}

/// One frame: the module's file name (no folder) and the address within it. No function name: the
/// computer has no symbols, and a name from the dump's own memory could be document text, so
/// `cargo xtask crash symbolicate` names the frames from the symbol archive instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DumpFrame {
    /// The module's file name.
    pub module: String,
    /// The instruction's address minus the module's base.
    pub offset: u64,
}

/// One module: its file name, version and debug ID (Breakpad's form), the symbol archive's key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DumpModule {
    /// The file name, without its folder.
    pub name: String,
    /// Its version, where the OS records one (Windows).
    pub version: Option<String>,
    /// Breakpad's debug ID: 33 or more hex digits.
    pub debug_id: String,
}

#[cfg(feature = "summary")]
pub use read::{SummaryError, summarize_bytes, summarize_to_json};

#[cfg(feature = "summary")]
mod read {
    use std::task::{Context, Poll, Waker};

    use minidump::{Minidump, MinidumpLinuxProcStatus, MinidumpModule, Module as _};
    use minidump_processor::ProcessState;
    use minidump_unwind::{Symbolizer, simple_symbol_supplier};

    use super::{DumpFrame, DumpModule, DumpSummary, MAX_FRAMES};

    /// Why a dump's bytes gave no summary.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum SummaryError {
        /// Not a minidump, or one the processor couldn't follow.
        #[error("the dump is not a minidump it can read: {0}")]
        Unreadable(String),
    }

    /// [`summarize_bytes`] serialised to JSON, for the engine's `--summarize-dump` mode.
    ///
    /// # Errors
    /// [`SummaryError::Unreadable`] when the bytes aren't a minidump, or JSON encoding fails.
    pub fn summarize_to_json(bytes: Vec<u8>) -> Result<String, SummaryError> {
        let summary = summarize_bytes(bytes)?;
        serde_json::to_string(&summary).map_err(|error| SummaryError::Unreadable(error.to_string()))
    }

    /// The summary of a dump's bytes. Runs to completion; the caller (the engine's parent) caps the
    /// time by ending the process, so this needs no timeout of its own.
    ///
    /// # Errors
    /// [`SummaryError::Unreadable`] when the bytes aren't a minidump the processor can follow.
    pub fn summarize_bytes(bytes: Vec<u8>) -> Result<DumpSummary, SummaryError> {
        let dump = Minidump::read(bytes).map_err(|error| SummaryError::Unreadable(error.to_string()))?;
        // No symbol files: the computer has none, and unwinding uses frame pointers and the stack.
        let symbolizer = Symbolizer::new(simple_symbol_supplier(Vec::new()));
        let state = block_on(minidump_processor::process_minidump(&dump, &symbolizer))
            .map_err(|error| SummaryError::Unreadable(error.to_string()))?;
        let pid = state.process_id.or_else(|| linux_pid(&dump));
        Ok(summary_of(&state, pid))
    }

    fn summary_of(state: &ProcessState, pid: Option<u32>) -> DumpSummary {
        let crash_reason = state
            .exception_info
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), |info| info.reason.to_string());
        let thread = state
            .requesting_thread
            .and_then(|index| state.threads.get(index))
            .or_else(|| state.threads.first());
        let mut frames = Vec::new();
        let mut modules: Vec<DumpModule> = Vec::new();
        for frame in thread.map(|thread| thread.frames.as_slice()).unwrap_or_default() {
            let Some(module) = &frame.module else {
                continue;
            };
            let Some(offset) = frame.instruction.checked_sub(module.base_address()) else {
                continue;
            };
            let name = file_name(&module.code_file());
            if let Some(debug_id) = module.debug_identifier()
                && !modules.iter().any(|known| known.name == name)
            {
                modules.push(DumpModule {
                    name: name.clone(),
                    version: version_of(module),
                    debug_id: debug_id.breakpad().to_string(),
                });
            }
            frames.push(DumpFrame { module: name, offset });
            if frames.len() == MAX_FRAMES {
                break;
            }
        }
        DumpSummary {
            pid,
            crash_reason,
            frames,
            modules,
        }
    }

    fn version_of(module: &MinidumpModule) -> Option<String> {
        module.version().map(std::borrow::Cow::into_owned)
    }

    /// The last part of a path, Unix or Windows: a module's file name without its folder.
    fn file_name(path: &str) -> String {
        path.rsplit(['/', '\\']).next().unwrap_or_default().to_owned()
    }

    /// Linux dumps carry a `/proc/<id>/status`, not the misc-info stream. Its `Pid` is the crashing
    /// thread's ID; the process's is `Tgid` (the thread group), which is what the app joins on.
    fn linux_pid(dump: &Minidump<'_, Vec<u8>>) -> Option<u32> {
        let status = dump.get_stream::<MinidumpLinuxProcStatus<'_>>().ok()?;
        let field = |name: &[u8]| {
            status.iter().find_map(|(key, value)| {
                (key.as_bytes() == name)
                    .then(|| value.to_str().ok()?.trim().parse().ok())
                    .flatten()
            })
        };
        field(b"Tgid").or_else(|| field(b"Pid"))
    }

    /// Runs `future` to its end on this thread. Without a symbol server, the processor's futures
    /// never wait on anything, so a plain poll loop finishes at once.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
            std::thread::yield_now();
        }
    }

    #[cfg(test)]
    mod tests {
        #![allow(
            clippy::unwrap_used,
            clippy::indexing_slicing,
            reason = "tests: a panic is a test failure"
        )]

        use std::path::Path;

        use super::{file_name, summarize_bytes};

        fn fixture(name: &str) -> Vec<u8> {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/desktop")
                .join(name);
            std::fs::read(path).unwrap()
        }

        #[test]
        fn a_garbage_dump_should_be_unreadable() {
            assert!(summarize_bytes(b"MDMP but not really".to_vec()).is_err());
        }

        /// The sandboxed engine's `DebugSegv` on macOS (Apple silicon).
        #[test]
        fn the_macos_engine_fault_should_name_the_engine_and_the_bad_access() {
            let summary = summarize_bytes(fixture("minidump-macos-aarch64-segv.dmp")).unwrap();
            let engine = summary
                .modules
                .iter()
                .find(|module| module.name == "laminapdf-desktop-engine");
            assert_eq!(
                (
                    summary.crash_reason.as_str(),
                    summary.frames[0].module.as_str(),
                    engine.map(|module| module.debug_id.len()),
                ),
                (
                    "EXC_BAD_ACCESS / KERN_INVALID_ADDRESS",
                    "laminapdf-desktop-engine",
                    Some(33)
                )
            );
        }

        #[test]
        fn a_module_should_be_named_by_its_file_name_alone() {
            let names = [
                "/Users/ana/App.app/Contents/MacOS/engine",
                "C:\\Lamina\\pdfium.dll",
                "libc.so.6",
            ]
            .map(file_name);
            assert_eq!(names, ["engine", "pdfium.dll", "libc.so.6"]);
        }
    }
}
