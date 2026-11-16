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

    use minidump::{
        Minidump, MinidumpBreakpadInfo, MinidumpException, MinidumpLinuxProcStatus, MinidumpMiscInfo, MinidumpModule,
        MinidumpModuleList, MinidumpSystemInfo, MinidumpThreadList, Module as _, UnifiedMemoryList,
    };
    use minidump_unwind::{
        CallStack, FileError, FileKind, FillSymbolError, FrameSymbolizer, FrameWalker, StackFrame, SymbolProvider,
        SystemInfo, walk_stack,
    };

    use super::{DumpFrame, DumpModule, DumpSummary, MAX_FRAMES};

    /// Why a dump's bytes gave no summary.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum SummaryError {
        /// Not a minidump, or one without what a summary needs (system info, threads).
        #[error("the dump is not a minidump it can read: {0}")]
        Unreadable(String),
    }

    fn unreadable(error: impl std::fmt::Display) -> SummaryError {
        SummaryError::Unreadable(error.to_string())
    }

    /// [`summarize_bytes`] serialised to JSON, for the engine's `--summarize-dump` mode.
    ///
    /// # Errors
    /// [`SummaryError::Unreadable`] when the bytes aren't a minidump, or JSON encoding fails.
    pub fn summarize_to_json(bytes: Vec<u8>) -> Result<String, SummaryError> {
        let summary = summarize_bytes(bytes)?;
        serde_json::to_string(&summary).map_err(unreadable)
    }

    /// The summary of a dump's bytes: the crash reason, the crashing thread's frames and the
    /// modules they name. Read with `minidump` and unwound with `minidump-unwind` directly, not
    /// `minidump-processor`, which also analyses every thread and instruction and would add
    /// megabytes to the engine for fields the summary drops. Runs to completion; the caller (the
    /// engine's parent) caps the time by ending the process.
    ///
    /// # Errors
    /// [`SummaryError::Unreadable`] when the bytes aren't a minidump it can follow.
    pub fn summarize_bytes(bytes: Vec<u8>) -> Result<DumpSummary, SummaryError> {
        let dump = Minidump::read(bytes).map_err(unreadable)?;
        let system = dump.get_stream::<MinidumpSystemInfo>().map_err(unreadable)?;
        let threads = dump.get_stream::<MinidumpThreadList<'_>>().map_err(unreadable)?;
        let misc = dump.get_stream::<MinidumpMiscInfo>().ok();
        let exception = dump.get_stream::<MinidumpException<'_>>().ok();
        let modules = dump.get_stream::<MinidumpModuleList>().unwrap_or_default();
        let memory = dump.get_memory().unwrap_or_default();
        let crash_reason = exception.as_ref().map_or_else(
            || "unknown".to_owned(),
            |exception| exception.get_crash_reason(system.os, system.cpu).to_string(),
        );
        let frames = crashing_frames(
            &dump,
            &system,
            &threads,
            misc.as_ref(),
            exception.as_ref(),
            &modules,
            &memory,
        );
        // The process's ID, which the app joins the dump to its report by: on Linux the status
        // stream's `Tgid` (its `Pid` is the crashing thread's), which only Linux dumps carry; else
        // the misc-info stream's (macOS, Windows).
        let pid = linux_pid(&dump).or_else(|| misc.as_ref().and_then(|misc| misc.raw.process_id().copied()));
        Ok(summary_of(&frames, pid, crash_reason))
    }

    /// The crashing thread's frames, unwound without symbols (frame pointers and stack scanning):
    /// the exception's thread with the exception's context, else the thread that asked for the
    /// dump, else the first.
    fn crashing_frames(
        dump: &Minidump<'_, Vec<u8>>,
        system: &MinidumpSystemInfo,
        threads: &MinidumpThreadList<'_>,
        misc: Option<&MinidumpMiscInfo>,
        exception: Option<&MinidumpException<'_>>,
        modules: &MinidumpModuleList,
        memory: &UnifiedMemoryList<'_>,
    ) -> Vec<StackFrame> {
        let requesting = dump
            .get_stream::<MinidumpBreakpadInfo>()
            .ok()
            .and_then(|info| info.requesting_thread_id);
        let id = exception.map(MinidumpException::get_crashing_thread_id).or(requesting);
        let Some(thread) = id
            .and_then(|id| threads.get_thread(id))
            .or_else(|| threads.threads.first())
        else {
            return Vec::new();
        };
        let exception_context = exception
            .filter(|exception| Some(exception.get_crashing_thread_id()) == Some(thread.raw.thread_id))
            .and_then(|exception| exception.context(system, misc));
        let Some(context) = exception_context.or_else(|| thread.context(system, misc)) else {
            return Vec::new();
        };
        let context = context.into_owned();
        // The stack the context's stack pointer is in: the exception's may differ from the thread's.
        let pointer = context.get_stack_pointer();
        let stack_memory = thread
            .stack_memory(memory)
            .filter(|stack| stack.get_memory_at_address::<u64>(pointer).is_some())
            .or_else(|| memory.memory_at_address(pointer))
            .or_else(|| thread.stack_memory(memory));
        let info = SystemInfo {
            os: system.os,
            os_version: None,
            os_build: None,
            cpu: system.cpu,
            cpu_info: None,
            cpu_microcode_version: None,
            cpu_count: 1,
        };
        let mut stack = CallStack::with_context(context);
        block_on(walk_stack(0, (), &mut stack, stack_memory, modules, &info, &NoSymbols));
        stack.frames
    }

    /// The computer has no symbol files: every module answers "no symbols", as an empty
    /// Breakpad supplier would, so unwinding uses frame pointers and stack scanning alone. Not
    /// `Symbolizer`, whose symbol-file parser would be linked into the engine for nothing.
    struct NoSymbols;

    #[async_trait::async_trait]
    impl SymbolProvider for NoSymbols {
        async fn fill_symbol(
            &self,
            _module: &(dyn minidump::Module + Sync),
            _frame: &mut (dyn FrameSymbolizer + Send),
        ) -> Result<(), FillSymbolError> {
            Err(FillSymbolError {})
        }

        async fn walk_frame(
            &self,
            _module: &(dyn minidump::Module + Sync),
            _walker: &mut (dyn FrameWalker + Send),
        ) -> Option<()> {
            None
        }

        async fn get_file_path(
            &self,
            _module: &(dyn minidump::Module + Sync),
            _file_kind: FileKind,
        ) -> Result<std::path::PathBuf, FileError> {
            Err(FileError::NotFound)
        }
    }

    fn summary_of(walked: &[StackFrame], pid: Option<u32>, crash_reason: String) -> DumpSummary {
        let mut frames = Vec::new();
        let mut modules: Vec<DumpModule> = Vec::new();
        for frame in walked {
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

    /// Runs `future` to its end on this thread. Without a symbol server, the unwinder's futures
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
