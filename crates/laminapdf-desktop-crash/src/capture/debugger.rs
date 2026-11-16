//! Windows: the monitor watches the engines as their debugger, and writes a faulting engine's
//! minidump itself (ADR 0014 §10, D7f). The engine takes no part: it opens no socket, so it never
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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
const SCAN_EVERY: Duration = Duration::from_millis(250);

/// How long one wait for a debug event lasts, so the loop also scans and sees `stop`.
const WAIT_MS: u32 = 100;

/// The engine binary's file name, beside the app's own executable.
const ENGINE_NAME: &str = "laminapdf-desktop-engine.exe";

/// The thread that debugs the engines; stops (detaching from every engine, which runs on) when
/// dropped.
pub(crate) struct EngineWatcher {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EngineWatcher {
    /// Watches the children of `parent` whose image is `image`, writing their dumps through
    /// `handler` (its folder, caps and `.tmp` names).
    pub(crate) fn start(parent: u32, image: PathBuf, handler: Handler) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("crash-debugger".into())
                .spawn(move || watch(parent, &image, &handler, &stop))?
        };
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    /// Watches the app's engines: the children of the monitor's parent (the main process) whose
    /// image is the engine beside the monitor's own executable.
    pub(crate) fn for_the_app(handler: Handler) -> io::Result<Self> {
        let parent = parent_of(current_pid()).ok_or_else(|| io::Error::other("the monitor's parent is unknown"))?;
        let image = std::env::current_exe()?
            .parent()
            .map(|dir| dir.join(ENGINE_NAME))
            .ok_or_else(|| io::Error::other("the app's folder is unknown"))?;
        Self::start(parent, image, handler)
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
fn watch(parent: u32, image: &Path, handler: &Handler, stop: &AtomicBool) {
    let mut attached: BTreeSet<u32> = BTreeSet::new();
    let mut scanned: Option<Instant> = None;
    while !stop.load(Ordering::SeqCst) {
        if scanned.is_none_or(|at| at.elapsed() >= SCAN_EVERY) {
            for pid in children_with_image(parent, image) {
                if !attached.contains(&pid) && attach(pid) {
                    attached.insert(pid);
                }
            }
            scanned = Some(Instant::now());
        }
        // SAFETY: an all-zero DEBUG_EVENT is a valid value for WaitForDebugEvent to overwrite.
        let mut event: DEBUG_EVENT = unsafe { std::mem::zeroed() };
        // SAFETY: `event` is a local the call fills in.
        if unsafe { WaitForDebugEvent(&raw mut event, WAIT_MS) } == 0 {
            continue;
        }
        let status = handle(&event, handler, &mut attached);
        // SAFETY: answers the event just received, on the thread that received it.
        unsafe { ContinueDebugEvent(event.dwProcessId, event.dwThreadId, status) };
    }
    for pid in attached {
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
fn handle(event: &DEBUG_EVENT, handler: &Handler, attached: &mut BTreeSet<u32>) -> i32 {
    match event.dwDebugEventCode {
        EXCEPTION_DEBUG_EVENT => {
            // SAFETY: the event code says `u` holds the exception's information.
            let info = unsafe { event.u.Exception };
            let code = info.ExceptionRecord.ExceptionCode;
            if code == EXCEPTION_BREAKPOINT {
                return DBG_CONTINUE;
            }
            if info.dwFirstChance == 0 && code != STATUS_STACK_BUFFER_OVERRUN {
                write_dump(event.dwProcessId, event.dwThreadId, &info.ExceptionRecord, handler);
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
fn write_dump(pid: u32, tid: u32, record: &EXCEPTION_RECORD, handler: &Handler) {
    let Ok((file, _)) = handler.new_dump() else {
        return;
    };
    let written = dump_into(pid, tid, record, file.as_raw_handle());
    handler.finish_dump(written.then_some(file));
}

/// The dump itself: the thread's context and the exception's record, both copied into this
/// process (`ClientPointers` false), so nothing of the engine's is read as a pointer here.
fn dump_into(pid: u32, tid: u32, record: &EXCEPTION_RECORD, file: HANDLE) -> bool {
    let Some(process) = Opened::process(pid) else {
        return false;
    };
    let Some(thread) = Opened::thread(tid) else {
        return false;
    };
    // SAFETY: an all-zero CONTEXT is valid before its flags are set and GetThreadContext fills it.
    let mut context: CONTEXT = unsafe { std::mem::zeroed() };
    context.ContextFlags = CONTEXT_ALL;
    // SAFETY: `thread` is open with THREAD_GET_CONTEXT and stopped by the debug event; `context` is
    // a local with the alignment CONTEXT declares.
    if unsafe { GetThreadContext(thread.0, &raw mut context) } == 0 {
        return false;
    }
    let mut record = *record;
    // The chained record lives in the engine's memory: not followed.
    record.ExceptionRecord = std::ptr::null_mut();
    let mut pointers = EXCEPTION_POINTERS {
        ExceptionRecord: &raw mut record,
        ContextRecord: &raw mut context,
    };
    let exception = MINIDUMP_EXCEPTION_INFORMATION {
        ThreadId: tid,
        ExceptionPointers: &raw mut pointers,
        ClientPointers: 0,
    };
    // SAFETY: `process` is open with the rights MiniDumpWriteDump needs; `file` is an open, writable
    // file; `exception` and what it points to are locals alive for the call.
    unsafe {
        MiniDumpWriteDump(
            process.0,
            pid,
            file,
            MiniDumpNormal | MiniDumpWithThreadInfo,
            &raw const exception,
            std::ptr::null(),
            std::ptr::null(),
        ) != 0
    }
}

#[cfg(target_arch = "x86_64")]
const CONTEXT_ALL: u32 = windows_sys::Win32::System::Diagnostics::Debug::CONTEXT_ALL_AMD64;
#[cfg(target_arch = "aarch64")]
const CONTEXT_ALL: u32 = windows_sys::Win32::System::Diagnostics::Debug::CONTEXT_ALL_ARM64;

/// The processes whose parent is `parent` and whose image is `image` (by path, ignoring case).
fn children_with_image(parent: u32, image: &Path) -> Vec<u32> {
    let wanted = image.to_string_lossy().to_lowercase();
    processes()
        .into_iter()
        .filter(|entry| entry.th32ParentProcessID == parent)
        .map(|entry| entry.th32ProcessID)
        .filter(|&pid| image_of(pid).is_some_and(|path| path.to_lowercase() == wanted))
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
