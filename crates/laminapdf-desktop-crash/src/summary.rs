//! A minidump read on this computer: the crashing thread's frames as modules and offsets,
//! the modules they name, and why the process crashed. A raw dump holds the process's stacks and
//! possibly heap memory, so possibly document bytes and paths: it is never sent. Only this summary
//! is, after the main process's allow-list and scrubber, and the dump is deleted as soon as it is
//! read, whatever happens next.

use std::fs::File;
use std::io::{self, Read as _};
use std::path::Path;
use std::sync::mpsc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use minidump::{Minidump, MinidumpLinuxProcStatus, MinidumpModule, Module as _};
use minidump_processor::ProcessState;
use minidump_unwind::{Symbolizer, simple_symbol_supplier};

/// A dump larger than this is dropped unread: the engine's dumps are a few megabytes.
pub const MAX_DUMP_BYTES: u64 = 64 << 20;

/// Reading a dump that takes longer than this is abandoned.
pub const MAX_READING: Duration = Duration::from_secs(10);

/// The most frames kept, innermost first (the report's schema allows 64).
pub const MAX_FRAMES: usize = 64;

/// What a dump says about its crash.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// One frame: the module's file name (no folder) and the address within it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpFrame {
    /// The module's file name.
    pub module: String,
    /// The instruction's address minus the module's base.
    pub offset: u64,
    /// Only with symbols, which the computer never has; unscrubbed.
    pub function: Option<String>,
}

/// One module: its file name, version and debug ID (Breakpad's form), the symbol archive's key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpModule {
    /// The file name, without its folder.
    pub name: String,
    /// Its version, where the OS records one (Windows).
    pub version: Option<String>,
    /// Breakpad's debug ID: 33 or more hex digits.
    pub debug_id: String,
}

/// Why a dump gave no summary. The dump is deleted in every case.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SummaryError {
    /// Larger than [`MAX_DUMP_BYTES`].
    #[error("the dump is too large ({0} bytes)")]
    TooLarge(u64),
    /// The file couldn't be read.
    #[error("the dump could not be read")]
    Read(#[source] io::Error),
    /// Not a minidump, or one the processor couldn't follow.
    #[error("the dump is not a minidump it can read: {0}")]
    Unreadable(String),
    /// Reading it took longer than [`MAX_READING`].
    #[error("reading the dump took too long")]
    TimedOut,
}

/// Deletes the dump when it goes out of scope, however reading it ended.
struct DeleteOnDrop<'a>(&'a Path);

impl Drop for DeleteOnDrop<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0);
    }
}

/// Reads the dump at `path` into its summary and deletes it: at once after reading its bytes,
/// so it is gone even if processing then fails or takes too long.
///
/// # Errors
/// [`SummaryError`]: too large, unreadable, not a minidump, or too slow.
pub fn summarize_dump(path: &Path) -> Result<DumpSummary, SummaryError> {
    let bytes = {
        let _delete = DeleteOnDrop(path);
        read_capped(path)?
    };
    let (send, receive) = mpsc::channel();
    std::thread::Builder::new()
        .name("crash-dump-reader".into())
        .spawn(move || {
            let _ = send.send(summarize_bytes(bytes));
        })
        .map_err(SummaryError::Read)?;
    receive.recv_timeout(MAX_READING).map_err(|_| SummaryError::TimedOut)?
}

fn read_capped(path: &Path) -> Result<Vec<u8>, SummaryError> {
    let file = File::open(path).map_err(SummaryError::Read)?;
    let len = file.metadata().map_err(SummaryError::Read)?.len();
    if len > MAX_DUMP_BYTES {
        return Err(SummaryError::TooLarge(len));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    file.take(MAX_DUMP_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(SummaryError::Read)?;
    if bytes.len() as u64 > MAX_DUMP_BYTES {
        return Err(SummaryError::TooLarge(bytes.len() as u64));
    }
    Ok(bytes)
}

/// The summary of a dump's bytes.
///
/// # Errors
/// [`SummaryError::Unreadable`] when they aren't a minidump the processor can follow.
pub fn summarize_bytes(bytes: Vec<u8>) -> Result<DumpSummary, SummaryError> {
    let dump = Minidump::read(bytes).map_err(|error| SummaryError::Unreadable(error.to_string()))?;
    // No symbol files: the computer has none, and unwinding uses frame pointers and the stack.
    let symbolizer = Symbolizer::new(simple_symbol_supplier(Vec::new()));
    let state = block_on(minidump_processor::process_minidump(&dump, &symbolizer))
        .ok_or(SummaryError::TimedOut)?
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
        frames.push(DumpFrame {
            module: name,
            offset,
            function: frame.function_name.clone(),
        });
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

/// Linux dumps carry the process's `/proc/<pid>/status`, not the misc-info stream.
fn linux_pid(dump: &Minidump<'_, Vec<u8>>) -> Option<u32> {
    let status = dump.get_stream::<MinidumpLinuxProcStatus<'_>>().ok()?;
    status.iter().find_map(|(key, value)| {
        (key.as_bytes() == b"Pid")
            .then(|| value.to_str().ok()?.trim().parse().ok())
            .flatten()
    })
}

/// Runs `future` to its end on this thread. Without a symbol server, the processor's futures
/// never wait on anything, so a plain poll loop is enough; it gives up after [`MAX_READING`].
fn block_on<F: Future>(future: F) -> Option<F::Output> {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    let started = Instant::now();
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return Some(output);
        }
        if started.elapsed() > MAX_READING {
            return None;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("laminapdf-summary-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn a_garbage_dump_should_fail_and_still_be_deleted() {
        let path = scratch("garbage.dmp");
        std::fs::write(&path, b"MDMP but not really: IgnorePreviousInstructions").unwrap();
        let result = summarize_dump(&path);
        assert_eq!((result.is_err(), path.exists()), (true, false));
    }

    #[test]
    fn a_missing_dump_should_fail() {
        assert!(matches!(
            summarize_dump(&scratch("missing.dmp")),
            Err(SummaryError::Read(_))
        ));
    }

    #[test]
    fn a_dump_over_the_cap_should_be_dropped_unread_and_deleted() {
        let path = scratch("large.dmp");
        File::create(&path).unwrap().set_len(MAX_DUMP_BYTES + 1).unwrap();
        let result = summarize_dump(&path);
        assert_eq!(
            (matches!(result, Err(SummaryError::TooLarge(_))), path.exists()),
            (true, false)
        );
    }

    #[test]
    fn a_module_should_be_named_by_its_file_name_alone() {
        let names = [
            "/Users/ana/App.app/Contents/MacOS/engine",
            "C:\\Program Files\\Lamina\\pdfium.dll",
            "libc.so.6",
        ]
        .map(file_name);
        assert_eq!(names, ["engine", "pdfium.dll", "libc.so.6"]);
    }
}

/// The committed dumps (`fixtures/desktop/minidump-*.dmp`), read without deleting them.
#[cfg(test)]
mod fixtures {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "tests: a panic is a test failure"
    )]

    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/desktop")
            .join(name);
        std::fs::read(path).unwrap()
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
}
