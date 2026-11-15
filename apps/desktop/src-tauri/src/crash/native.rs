//! Native crashes (D7d; ADR 0014 §10): minidumps written by `laminapdf-desktop-sandbox`'s
//! [`DumpServer`]s, read on this computer into a report's [`Native`] part, and deleted.
//!
//! - **Engines**: [`EngineDumps`] serves every engine this app starts (their `--crash-socket`).
//!   Each dump is read on a thread of its own as soon as it is written, and its summary waits,
//!   by the crashed engine's pid, for the supervisor's `Crashed` ([`NativeCrashes::take`]).
//! - **The main process**: [`capture_main`] starts a monitor, this app's own binary run with
//!   [`MONITOR_ARG`] ([`monitor_main`]), which serves the main process alone and exits when it
//!   goes. A dump it leaves is read at the next launch ([`leftover_main_crashes`]) into a report
//!   held like a main-process panic's.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use laminapdf_desktop_protocol::crash::truncate_on_char_boundary;
use laminapdf_desktop_sandbox::capture::summary::{DumpSummary, summarize_dump};
use laminapdf_desktop_sandbox::capture::{
    CRASH_SOCKET_ARG, CrashCapture, DumpEvent, DumpServer, Serve, capture_crashes, socket_path,
};

use super::report::{DumpFrame, DumpModule, Native, module_name};
use super::scrub::scrub;

/// The argument that makes this binary the main process's crash monitor:
/// `--crash-monitor <socket> <dumps folder>`.
pub(super) const MONITOR_ARG: &str = "--crash-monitor";

/// The longest a crash report waits for its engine's dump to be read: the reading's own cap and a
/// little more, for writing it.
const WAIT_FOR_DUMP: Duration = Duration::from_secs(15);

/// How long the monitor waits for the main process to connect before it gives up.
const MONITOR_WAIT: Duration = Duration::from_secs(30);

/// How long the main process waits for its monitor's socket.
const MONITOR_START: Duration = Duration::from_secs(5);

/// The most read dumps kept unclaimed (a dump of no engine of ours is dropped once past it).
const MAX_UNCLAIMED: usize = 8;

/// The most bytes a crash reason keeps.
const MAX_REASON: usize = 128;

/// The dumps' folders, under the reports' own (`data_dir/crashes`).
fn dumps_dir(data_dir: &Path, whose: &str) -> PathBuf {
    data_dir.join("crashes").join("dumps").join(whose)
}

/// The engines' dumps, read as they come, waiting for their crash.
#[derive(Default)]
pub(crate) struct NativeCrashes {
    state: Mutex<Waiting>,
    changed: Condvar,
}

#[derive(Default)]
struct Waiting {
    /// Dumps being written or read.
    pending: usize,
    /// Dumps read, not yet claimed by a crash; the oldest go first.
    read: VecDeque<DumpSummary>,
}

impl NativeCrashes {
    fn state(&self) -> MutexGuard<'_, Waiting> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn started(&self) {
        self.state().pending += 1;
    }

    fn finished(&self, summary: Option<DumpSummary>) {
        let mut state = self.state();
        state.pending = state.pending.saturating_sub(1);
        if let Some(summary) = summary {
            if state.read.len() == MAX_UNCLAIMED {
                state.read.pop_front();
            }
            state.read.push_back(summary);
        }
        drop(state);
        self.changed.notify_all();
    }

    /// The dump of engine `pid`, as the report's [`Native`]: at once if it was read, else after
    /// the dumps being read are done, waiting `wait` at most; `None` when the engine left none.
    pub(crate) fn take(&self, pid: u32, wait: Duration) -> Option<Native> {
        let deadline = Instant::now() + wait;
        let mut state = self.state();
        loop {
            if let Some(at) = state.read.iter().position(|summary| summary.pid == Some(pid)) {
                return state.read.remove(at).map(|summary| allow_list(&summary));
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if state.pending == 0 || left.is_zero() {
                return None;
            }
            state = self
                .changed
                .wait_timeout(state, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    /// [`Self::take`] with the app's wait.
    pub(crate) fn take_for_report(&self, pid: u32) -> Option<Native> {
        self.take(pid, WAIT_FOR_DUMP)
    }
}

/// The server every engine's crash handler connects to, and the dumps it read.
pub(crate) struct EngineDumps {
    server: DumpServer,
    pub(crate) crashes: Arc<NativeCrashes>,
}

impl EngineDumps {
    /// Starts the server, its dumps in `data_dir/crashes/dumps/engines`; any an earlier run left
    /// there unread are deleted. `None` (logged) when it can't start: engines then run uncaptured.
    pub(crate) fn start(data_dir: &Path) -> Option<Self> {
        let dir = dumps_dir(data_dir, "engines");
        delete_all(&dir);
        let crashes = Arc::new(NativeCrashes::default());
        let on_event = {
            let crashes = Arc::clone(&crashes);
            move |event| on_engine_dump(&crashes, event)
        };
        match DumpServer::start(socket_path("engines"), dir, Serve::Many, on_event) {
            Ok(server) => Some(Self { server, crashes }),
            Err(error) => {
                tracing::warn!("engine crashes won't leave dumps: {error}");
                None
            }
        }
    }

    /// The engine's arguments that connect it to this server.
    pub(crate) fn engine_args(&self) -> [OsString; 2] {
        [CRASH_SOCKET_ARG.into(), self.server.socket().into()]
    }
}

fn on_engine_dump(crashes: &Arc<NativeCrashes>, event: DumpEvent) {
    match event {
        DumpEvent::Started => crashes.started(),
        // Read off the server's thread: the next crashed engine mustn't wait for this one.
        DumpEvent::Written(path) => {
            let reader = Arc::clone(crashes);
            let read = std::thread::Builder::new()
                .name("crash-dump".into())
                .spawn(move || reader.finished(read_dump(&path)));
            if read.is_err() {
                crashes.finished(None);
            }
        }
        DumpEvent::Failed(error) => {
            tracing::warn!("an engine's crash dump failed: {error}");
            crashes.finished(None);
        }
        DumpEvent::Trouble(error) => tracing::warn!("{error}"),
    }
}

/// Reads and deletes one dump; its failure is logged, never its content.
fn read_dump(path: &Path) -> Option<DumpSummary> {
    match summarize_dump(path) {
        Ok(summary) => Some(summary),
        Err(error) => {
            tracing::warn!("a crash dump could not be read: {error}");
            None
        }
    }
}

/// The report's [`Native`] from a dump's summary: each string held to what the schema accepts.
pub(crate) fn allow_list(summary: &DumpSummary) -> Native {
    let modules = summary
        .modules
        .iter()
        .filter(|module| is_debug_id(&module.debug_id))
        .map(|module| DumpModule {
            name: module_name(&module.name),
            version: module.version.clone().filter(|version| is_version(version)),
            debug_id: module.debug_id.clone(),
        })
        .take(64)
        .collect();
    Native {
        crash_reason: truncate_on_char_boundary(scrub(&summary.crash_reason).into_owned(), MAX_REASON),
        frames: summary
            .frames
            .iter()
            .take(64)
            .map(|frame| DumpFrame {
                module: module_name(&frame.module),
                offset: frame.offset.min(MAX_OFFSET),
                // Never kept: the computer has no symbols, so a name could only come from the
                // dump's own memory, where nothing tells a function's name from a document's
                // text. `cargo xtask crash symbolicate` names the frames from the symbol archive.
                function: None,
            })
            .collect(),
        modules,
    }
}

/// A native stack as a report shows one, for Settings › Privacy's sample report.
pub(crate) fn sample() -> Native {
    let engine = format!("laminapdf-desktop-engine{}", std::env::consts::EXE_SUFFIX);
    Native {
        crash_reason: "EXC_BAD_ACCESS / KERN_INVALID_ADDRESS".into(),
        frames: [0x0044_016c, 0x0043_f2a0, 0x0002_8c4c]
            .into_iter()
            .map(|offset| DumpFrame {
                module: engine.clone(),
                offset,
                function: None,
            })
            .collect(),
        modules: vec![DumpModule {
            name: engine,
            version: None,
            debug_id: "5887580E1F1A3CE2BEAB358DCFAA73030".into(),
        }],
    }
}

/// The largest offset a report holds: JSON numbers stay exact up to 2^53 − 1 (the schema's cap).
const MAX_OFFSET: u64 = (1 << 53) - 1;

fn is_debug_id(id: &str) -> bool {
    (32..=40).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_version(version: &str) -> bool {
    (1..=32).contains(&version.len())
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'+' | b'-'))
}

fn delete_all(dir: &Path) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let _ = std::fs::remove_file(entry.path());
    }
}

/// The main process's crash handler, connected to its monitor; kept for the process's life.
static MAIN_CAPTURE: OnceLock<CrashCapture> = OnceLock::new();

/// Starts the main process's monitor and connects this process's crash handler to it, on a thread
/// of its own so the start-up doesn't wait. Its dumps go to `data_dir/crashes/dumps/main`.
pub(crate) fn capture_main(data_dir: &Path) {
    let dir = dumps_dir(data_dir, "main");
    let started = std::thread::Builder::new()
        .name("crash-monitor-start".into())
        .spawn(move || {
            if let Err(error) = start_monitor(&dir) {
                tracing::warn!("the main process's native crashes won't leave dumps: {error}");
            }
        });
    if let Err(error) = started {
        tracing::warn!("the crash monitor could not start: {error}");
    }
}

fn start_monitor(dir: &Path) -> Result<(), String> {
    let socket = socket_path("main");
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = Command::new(exe);
    command
        .arg(MONITOR_ARG)
        .arg(&socket)
        .arg(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW: a debug build is a console program; the monitor shows no console.
        command.creation_flags(0x0800_0000);
    }
    let monitor = command.spawn().map_err(|error| error.to_string())?;
    let deadline = Instant::now() + MONITOR_START;
    loop {
        match capture_crashes(&socket, Some(monitor.id())) {
            Ok(capture) => {
                let _ = MAIN_CAPTURE.set(capture);
                return Ok(());
            }
            Err(error) if Instant::now() > deadline => return Err(error.to_string()),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

/// From now on, a crash of the main process asks its monitor for no dump: the panic hook calls it,
/// since a panic already leaves its own report and may end in an abort.
pub(crate) fn skip_main_dumps() {
    laminapdf_desktop_sandbox::capture::skip_dumps();
}

/// The monitor's `main`: with [`MONITOR_ARG`] first, serves one client, the main process, until
/// it goes, and returns the exit code; without it, `None`.
pub fn monitor_main(mut args: impl Iterator<Item = OsString>) -> Option<ExitCode> {
    if args.next().is_none_or(|arg| arg != MONITOR_ARG) {
        return None;
    }
    let (Some(socket), Some(dir)) = (args.next(), args.next()) else {
        return Some(ExitCode::FAILURE);
    };
    let serve = Serve::One { wait: MONITOR_WAIT };
    match DumpServer::start(PathBuf::from(socket), PathBuf::from(dir), serve, |_| {}) {
        Ok(server) => {
            server.wait();
            Some(ExitCode::SUCCESS)
        }
        Err(_) => Some(ExitCode::FAILURE),
    }
}

/// The main process's native crashes an earlier run left, each read, deleted and returned as its
/// report's [`Native`].
pub(crate) fn leftover_main_crashes(data_dir: &Path) -> Vec<Native> {
    let dir = dumps_dir(data_dir, "main");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let dump = path.extension().is_some_and(|ext| ext == "dmp");
        match dump.then(|| read_dump(&path)).flatten() {
            Some(summary) => found.push(allow_list(&summary)),
            // Not a dump, or unreadable: gone either way (read_dump deleted the dump already).
            None => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use laminapdf_desktop_sandbox::capture::summary::{DumpFrame as Frame, DumpModule as Module};

    use super::*;

    fn summary(pid: u32) -> DumpSummary {
        DumpSummary {
            pid: Some(pid),
            crash_reason: "EXC_BAD_ACCESS / KERN_INVALID_ADDRESS".into(),
            frames: vec![Frame {
                module: "laminapdf-desktop-engine".into(),
                offset: 0x1a2b,
                function: None,
            }],
            modules: vec![Module {
                name: "laminapdf-desktop-engine".into(),
                version: None,
                debug_id: "5887580E1F1A3CE2BEAB358DCFAA73030".into(),
            }],
        }
    }

    #[test]
    fn a_read_dump_should_be_taken_by_its_engines_pid_only() {
        let crashes = NativeCrashes::default();
        crashes.started();
        crashes.finished(Some(summary(7)));
        let other = crashes.take(8, Duration::ZERO);
        let own = crashes.take(7, Duration::ZERO);
        assert_eq!((other, own.map(|native| native.frames.len())), (None, Some(1)));
    }

    #[test]
    fn a_crash_should_wait_for_a_dump_still_being_read() {
        let crashes = Arc::new(NativeCrashes::default());
        crashes.started();
        let reader = {
            let crashes = Arc::clone(&crashes);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                crashes.finished(Some(summary(7)));
            })
        };
        let native = crashes.take(7, Duration::from_secs(5));
        let _ = reader.join();
        assert!(native.is_some());
    }

    #[test]
    fn a_crash_without_a_dump_should_not_wait() {
        let started = Instant::now();
        let native = NativeCrashes::default().take(7, Duration::from_secs(5));
        assert_eq!((native, started.elapsed() < Duration::from_secs(1)), (None, true));
    }

    #[test]
    fn the_allow_list_should_keep_only_what_the_schema_accepts() {
        let mut dump = summary(7);
        dump.crash_reason = "SIGSEGV at /Users/ana/FolderMarker7f3a9c/NameMarker4b2e1d.pdf".into();
        dump.frames.push(Frame {
            module: "/usr/lib/libc.so.6".into(),
            offset: u64::MAX,
            function: Some("open(/home/ana/a/b.pdf) IgnorePreviousInstructions".into()),
        });
        dump.modules.push(Module {
            name: "evil.dll".into(),
            version: Some("1.0 /Users/ana".into()),
            debug_id: "not hex".into(),
        });
        let native = allow_list(&dump);
        let json = serde_json::to_string(&native).unwrap_or_default();
        assert_eq!(
            (
                ["/Users", "/home", "Marker", "Ignore"]
                    .iter()
                    .any(|marker| json.contains(marker)),
                native.frames.get(1).map(|frame| (frame.module.as_str(), frame.offset)),
                native.modules.len(),
            ),
            (false, Some(("<unknown>", MAX_OFFSET)), 1)
        );
    }

    #[test]
    fn the_monitor_flag_should_be_needed_to_run_the_monitor() {
        assert!(monitor_main(["--other".into()].into_iter()).is_none());
    }
}
