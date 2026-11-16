//! Windows: the monitor watches the engines as their debugger, and writes a faulting engine's
//! minidump itself (ADR 0014 §10). The engine takes no part: it opens no socket, so it never
//! starts Winsock, which its sandbox counts on to keep it off the network; and since it sends no
//! request, it can't name another process to be dumped.
//!
//! The monitor finds the engines on its own, by polling: the processes whose parent is the main
//! process (the monitor's parent) and whose image is the engine binary beside the app. It attaches
//! to each, passes every exception on to it, and on a second-chance one (the engine is about to
//! end) writes the dump before letting it go. A fail-fast (`STATUS_STACK_BUFFER_OVERRUN`, which a
//! Rust panic's abort raises) leaves none: the panic's own report is the crash's.

use std::collections::BTreeSet;
use std::io;
use std::os::windows::io::AsRawHandle as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, DBG_CONTINUE, DBG_EXCEPTION_NOT_HANDLED, EXCEPTION_BREAKPOINT, HANDLE, INVALID_HANDLE_VALUE,
    STATUS_STACK_BUFFER_OVERRUN,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
    CONTEXT, CREATE_PROCESS_DEBUG_EVENT, ContinueDebugEvent, DEBUG_EVENT, DebugActiveProcess, DebugActiveProcessStop,
    DebugSetProcessKillOnExit, EXCEPTION_DEBUG_EVENT, EXCEPTION_POINTERS, EXCEPTION_RECORD, EXIT_PROCESS_DEBUG_EVENT,
    GetThreadContext, LOAD_DLL_DEBUG_EVENT, MINIDUMP_EXCEPTION_INFORMATION, MiniDumpNormal, MiniDumpWithThreadInfo,
    MiniDumpWriteDump, WaitForDebugEvent,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, OpenThread, PROCESS_DUP_HANDLE, PROCESS_QUERY_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ, QueryFullProcessImageNameW, THREAD_GET_CONTEXT,
    THREAD_QUERY_INFORMATION,
};

use super::Handler;

/// How often the monitor looks for new engines. One that faults sooner after it starts leaves no
/// dump: its crash is still reported, without the `native` part.
const SCAN_EVERY: Duration = Duration::from_millis(50);

/// How soon an engine the monitor couldn't attach to is tried again: one still starting refuses a
/// debugger for a moment.
const RETRY_EVERY: Duration = Duration::from_millis(20);

/// How long one wait for a debug event lasts, so the loop also scans and sees `stop`.
const WAIT_MS: u32 = 100;

/// Where the watcher says what it did and what failed (the monitor's goes nowhere; a test's to its
/// output).
pub(crate) type Log = fn(&str);

/// The engine binary's file name, beside the app's own executable.
const ENGINE_NAME: &str = "laminapdf-desktop-engine.exe";

/// The thread that debugs the engines; stops (detaching from every engine, which runs on) when
/// dropped.
pub(crate) struct EngineWatcher {
    stop: Arc<AtomicBool>,
    /// The engines attached now (read by tests, which wait for their engine to be watched).
    #[cfg_attr(
        not(feature = "test-support"),
        expect(dead_code, reason = "only the tests' watcher asks which engines are watched")
    )]
    watching: Arc<Mutex<BTreeSet<u32>>>,
    thread: Option<JoinHandle<()>>,
}

impl EngineWatcher {
    /// Watches the children of `parent` whose image is `image`, writing their dumps through
    /// `handler` (its folder, caps and `.tmp` names).
    pub(crate) fn start(parent: u32, image: PathBuf, handler: Handler, log: Log) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let watching = Arc::new(Mutex::new(BTreeSet::new()));
        let thread = {
            let (stop, watching) = (Arc::clone(&stop), Arc::clone(&watching));
            std::thread::Builder::new()
                .name("crash-debugger".into())
                .spawn(move || watch(parent, &image, &handler, &stop, &watching, log))?
        };
        Ok(Self {
            stop,
            watching,
            thread: Some(thread),
        })
    }

    /// Watches the app's engines: the children of the monitor's parent (the main process) whose
    /// image is the engine beside the monitor's own executable. Its log goes to the monitor's
    /// stderr, which the main process copies into its own log.
    pub(crate) fn for_the_app(handler: Handler) -> io::Result<Self> {
        let parent = parent_of(current_pid()).ok_or_else(|| io::Error::other("the monitor's parent is unknown"))?;
        let image = std::env::current_exe()?
            .parent()
            .map(|dir| dir.join(ENGINE_NAME))
            .ok_or_else(|| io::Error::other("the app's folder is unknown"))?;
        Self::start(parent, image, handler, |line| {
            use std::io::Write as _;
            let _ = writeln!(io::stderr(), "{line}");
        })
    }

    /// Whether engine `pid` is attached now.
    #[cfg(feature = "test-support")]
    pub(crate) fn watches(&self, pid: u32) -> bool {
        self.watching
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&pid)
    }

    /// Whether the watching thread has ended.
    pub(crate) fn finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

impl Drop for EngineWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The debug loop. A debugger's processes belong to the thread that attached them, so attaching,
/// waiting and detaching all happen here.
fn watch(parent: u32, image: &Path, handler: &Handler, stop: &AtomicBool, watching: &Mutex<BTreeSet<u32>>, log: Log) {
    let attached = || watching.lock().unwrap_or_else(PoisonError::into_inner);
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut retry = false;
    let mut scanned: Option<Instant> = None;
    log(&format!("watching the children of {parent} that run the engine"));
    while !stop.load(Ordering::SeqCst) {
        let every = if retry { RETRY_EVERY } else { SCAN_EVERY };
        if scanned.is_none_or(|at| at.elapsed() >= every) {
            retry = false;
            // Never a path in the log (it is the app's, and it holds no file's path at info level).
            let wanted = comparable(&image.to_string_lossy());
            for (pid, path) in children_of(parent) {
                if seen.insert(pid) {
                    let what = match path {
                        Some(path) if comparable(&path) == wanted => "the engine",
                        Some(_) => "another program",
                        None => "unreadable",
                    };
                    log(&format!("child {pid}: {what}"));
                }
            }
            for pid in children_with_image(parent, image) {
                if attached().contains(&pid) {
                    continue;
                }
                if attach(pid) {
                    log(&format!("attached to {pid}"));
                    attached().insert(pid);
                } else {
                    log(&format!(
                        "could not attach to {pid} yet: {}",
                        io::Error::last_os_error()
                    ));
                    retry = true;
                }
            }
            scanned = Some(Instant::now());
        }
        // SAFETY: an all-zero DEBUG_EVENT is a valid value for WaitForDebugEvent to overwrite.
        let mut event: DEBUG_EVENT = unsafe { std::mem::zeroed() };
        let wait = if retry { 20 } else { WAIT_MS };
        // SAFETY: `event` is a local the call fills in.
        if unsafe { WaitForDebugEvent(&raw mut event, wait) } == 0 {
            continue;
        }
        let status = handle(&event, handler, &mut attached(), log);
        // SAFETY: answers the event just received, on the thread that received it.
        unsafe { ContinueDebugEvent(event.dwProcessId, event.dwThreadId, status) };
    }
    for pid in std::mem::take(&mut *attached()) {
        // SAFETY: detaches a process this thread attached; it runs on without a debugger.
        unsafe { DebugActiveProcessStop(pid) };
    }
}

/// Attaches to `pid`, so its exceptions come here first; the engine is not ended when the monitor
/// is (a new monitor attaches again).
fn attach(pid: u32) -> bool {
    // SAFETY: plain calls on a process id; DebugSetProcessKillOnExit applies to this thread's
    // debuggees.
    unsafe { DebugActiveProcess(pid) != 0 && DebugSetProcessKillOnExit(0) != 0 }
}

/// What to answer `event`: every exception goes on to the engine (its own handlers first), but the
/// attach's breakpoint; a second-chance one is fatal, so its dump is written first.
fn handle(event: &DEBUG_EVENT, handler: &Handler, attached: &mut BTreeSet<u32>, log: Log) -> i32 {
    match event.dwDebugEventCode {
        EXCEPTION_DEBUG_EVENT => {
            // SAFETY: the event code says `u` holds the exception's information.
            let info = unsafe { event.u.Exception };
            let code = info.ExceptionRecord.ExceptionCode;
            if code == EXCEPTION_BREAKPOINT {
                return DBG_CONTINUE;
            }
            if info.dwFirstChance == 0 {
                log(&format!("{}: second-chance exception {code:#x}", event.dwProcessId));
                if code != STATUS_STACK_BUFFER_OVERRUN {
                    write_dump(event.dwProcessId, event.dwThreadId, &info.ExceptionRecord, handler, log);
                }
            }
            DBG_EXCEPTION_NOT_HANDLED
        }
        CREATE_PROCESS_DEBUG_EVENT => {
            // SAFETY: the event code says `u` holds the process's information; its file handle is
            // the debugger's to close (the process and thread handles are the system's).
            close(unsafe { event.u.CreateProcessInfo.hFile });
            DBG_CONTINUE
        }
        LOAD_DLL_DEBUG_EVENT => {
            // SAFETY: as above, for the library's file handle.
            close(unsafe { event.u.LoadDll.hFile });
            DBG_CONTINUE
        }
        EXIT_PROCESS_DEBUG_EVENT => {
            attached.remove(&event.dwProcessId);
            DBG_CONTINUE
        }
        _ => DBG_CONTINUE,
    }
}

/// Writes the minidump of engine `pid`, stopped on `record` in thread `tid`.
fn write_dump(pid: u32, tid: u32, record: &EXCEPTION_RECORD, handler: &Handler, log: Log) {
    let (file, _) = match handler.new_dump() {
        Ok(new) => new,
        Err(error) => {
            log(&format!("{pid}: no dump file: {error}"));
            return;
        }
    };
    let written = dump_into(pid, tid, record, file.as_raw_handle());
    match &written {
        Ok(()) => log(&format!("{pid}: dump written")),
        Err(error) => log(&format!("{pid}: dump failed: {error}")),
    }
    handler.finish_dump(written.is_ok().then_some(file));
}

/// The dump itself: the thread's context and the exception's record, both copied into this
/// process (`ClientPointers` false), so nothing of the engine's is read as a pointer here.
fn dump_into(pid: u32, tid: u32, record: &EXCEPTION_RECORD, file: HANDLE) -> io::Result<()> {
    let failed = |step: &str| io::Error::other(format!("{step}: {}", io::Error::last_os_error()));
    let process = Opened::process(pid).ok_or_else(|| failed("OpenProcess"))?;
    let thread = Opened::thread(tid).ok_or_else(|| failed("OpenThread"))?;
    // SAFETY: an all-zero CONTEXT is valid before its flags are set and GetThreadContext fills it.
    let mut aligned = Aligned(unsafe { std::mem::zeroed() });
    let context = &mut aligned.0;
    context.ContextFlags = CONTEXT_ALL;
    // SAFETY: `thread` is open with THREAD_GET_CONTEXT and stopped by the debug event; `context` is
    // 16-byte aligned, as GetThreadContext requires (ERROR_NOACCESS otherwise).
    if unsafe { GetThreadContext(thread.0, &raw mut *context) } == 0 {
        return Err(failed("GetThreadContext"));
    }
    let mut record = *record;
    // The chained record lives in the engine's memory: not followed.
    record.ExceptionRecord = std::ptr::null_mut();
    let mut pointers = EXCEPTION_POINTERS {
        ExceptionRecord: &raw mut record,
        ContextRecord: &raw mut *context,
    };
    let exception = MINIDUMP_EXCEPTION_INFORMATION {
        ThreadId: tid,
        ExceptionPointers: &raw mut pointers,
        ClientPointers: 0,
    };
    // SAFETY: `process` is open with the rights MiniDumpWriteDump needs; `file` is an open, writable
    // file; `exception` and what it points to are locals alive for the call.
    let written = unsafe {
        MiniDumpWriteDump(
            process.0,
            pid,
            file,
            MiniDumpNormal | MiniDumpWithThreadInfo,
            &raw const exception,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if written == 0 {
        Err(failed("MiniDumpWriteDump"))
    } else {
        Ok(())
    }
}

/// A `CONTEXT` at the 16-byte alignment `GetThreadContext` requires, which the binding's type alone
/// doesn't give a local.
#[repr(C, align(16))]
struct Aligned(CONTEXT);

#[cfg(target_arch = "x86_64")]
const CONTEXT_ALL: u32 = windows_sys::Win32::System::Diagnostics::Debug::CONTEXT_ALL_AMD64;
#[cfg(target_arch = "aarch64")]
const CONTEXT_ALL: u32 = windows_sys::Win32::System::Diagnostics::Debug::CONTEXT_ALL_ARM64;

/// The processes whose parent is `parent` and whose image is `image` (by path, ignoring case and
/// which slash separates it: a path built from parts may mix `/` and `\`, Windows reports `\`).
fn children_with_image(parent: u32, image: &Path) -> Vec<u32> {
    let wanted = comparable(&image.to_string_lossy());
    children_of(parent)
        .into_iter()
        .filter(|(_, path)| path.as_deref().is_some_and(|path| comparable(path) == wanted))
        .map(|(pid, _)| pid)
        .collect()
}

/// `path` as the image comparison sees it: lower case, with `\` for every separator.
fn comparable(path: &str) -> String {
    path.to_lowercase().replace('/', "\\")
}

/// The children of `parent`, each with its image's path when it can be read.
fn children_of(parent: u32) -> Vec<(u32, Option<String>)> {
    processes()
        .into_iter()
        .filter(|entry| entry.th32ParentProcessID == parent)
        .map(|entry| (entry.th32ProcessID, image_of(entry.th32ProcessID)))
        .collect()
}

/// The parent of `pid`, from the system's process list.
fn parent_of(pid: u32) -> Option<u32> {
    processes()
        .into_iter()
        .find(|entry| entry.th32ProcessID == pid)
        .map(|entry| entry.th32ParentProcessID)
}

/// This process's id.
fn current_pid() -> u32 {
    // SAFETY: no arguments, no failure.
    unsafe { GetCurrentProcessId() }
}

/// A snapshot of the system's processes.
fn processes() -> Vec<PROCESSENTRY32W> {
    // SAFETY: a snapshot of every process; checked below and closed once.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    let mut found = Vec::new();
    // SAFETY: an all-zero PROCESSENTRY32W is valid once its size is set, as the calls require.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = u32::try_from(size_of::<PROCESSENTRY32W>()).unwrap_or(u32::MAX);
    // SAFETY: `snapshot` is open and `entry` sized; each call overwrites it.
    let mut more = unsafe { Process32FirstW(snapshot, &raw mut entry) } != 0;
    while more {
        found.push(entry);
        // SAFETY: as above.
        more = unsafe { Process32NextW(snapshot, &raw mut entry) } != 0;
    }
    close(snapshot);
    found
}

/// The full path of `pid`'s image, if it can be read.
fn image_of(pid: u32) -> Option<String> {
    // SAFETY: a query-only handle, closed by `Opened`.
    let process = Opened::new(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) })?;
    let mut buffer = [0u16; 1024];
    let mut len = u32::try_from(buffer.len()).unwrap_or(0);
    // SAFETY: `buffer` holds `len` wide characters; the call writes at most that and sets `len`.
    if unsafe { QueryFullProcessImageNameW(process.0, 0, buffer.as_mut_ptr(), &raw mut len) } == 0 {
        return None;
    }
    let len = usize::try_from(len).ok()?;
    Some(String::from_utf16_lossy(buffer.get(..len)?))
}

/// A handle closed when dropped.
struct Opened(HANDLE);

impl Opened {
    fn new(handle: HANDLE) -> Option<Self> {
        (!handle.is_null() && handle != INVALID_HANDLE_VALUE).then_some(Self(handle))
    }

    /// `pid`, open as the dump writer needs it.
    fn process(pid: u32) -> Option<Self> {
        // SAFETY: opens a process by id; the handle is checked and closed by `Opened`.
        Self::new(unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_DUP_HANDLE, 0, pid) })
    }

    /// `tid`, open to read its context.
    fn thread(tid: u32) -> Option<Self> {
        // SAFETY: opens a thread by id; the handle is checked and closed by `Opened`.
        Self::new(unsafe { OpenThread(THREAD_GET_CONTEXT | THREAD_QUERY_INFORMATION, 0, tid) })
    }
}

impl Drop for Opened {
    fn drop(&mut self) {
        close(self.0);
    }
}

/// Closes `handle`, if it is one.
fn close(handle: HANDLE) {
    if !handle.is_null() && handle != INVALID_HANDLE_VALUE {
        // SAFETY: a handle this module owns, closed once.
        unsafe { CloseHandle(handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::comparable;

    #[test]
    fn image_paths_should_match_whatever_their_slashes_and_case() {
        assert_eq!(
            comparable(r"D:\a\Lamina/target\debug\laminapdf-desktop-engine.exe"),
            comparable(r"d:\a\lamina\target\debug\LAMINAPDF-DESKTOP-ENGINE.EXE")
        );
    }
}
