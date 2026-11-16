//! Native crashes (ADR 0014 §10): minidumps written by the monitor process, read on this computer
//! into a report's [`Native`] part, and deleted. Nothing hostile is handled in the main process: the
//! servers that read a crashing process's memory run in the monitor (the app's own binary started
//! with [`laminapdf_desktop_crash::MONITOR_ARG`]), and each dump is parsed by a fresh engine process
//! under the full sandbox, so the main process never links the minidump parser either.
//!
//! - **The monitor** hosts two servers: one for the main process, one for its engines. The main
//!   process spawns it, restarts it if it dies, at most [`MAX_MONITOR_RESTARTS`] times (a hostile
//!   message can crash only the monitor), and
//!   gives each engine the current engine socket at spawn.
//! - **Engines**: the monitor writes their dumps into `dumps/engines`. A polling thread
//!   ([`EngineDumps`]) picks up each new dump, reads it (size cap, delete, summarise sandboxed), and
//!   keeps the summary by the crashing engine's pid for the supervisor's `Crashed`.
//! - **The main process**: its own dumps go to `dumps/main`, read at the next launch
//!   ([`leftover_main_crashes`], the newest few, off the start-up path) into a held report.
//!
//! On Windows the monitor's dump server dumps only the process on the other end of the connection
//! (the workspace's patched `minidumper`, D7f), so it runs on every OS.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use laminapdf_desktop_crash::summary::DumpSummary;
use laminapdf_desktop_crash::{
    CRASH_SERVER_PID_ARG, CRASH_SOCKET_ARG, CrashCapture, DUMP_EXTENSION, MonitorPaths, SUMMARIZE_DUMP_ARG,
    capture_crashes_within,
};
use laminapdf_desktop_protocol::crash::truncate_on_char_boundary;
use laminapdf_desktop_protocol::supervisor::Process as _;
use laminapdf_desktop_sandbox::{SandboxConfig, spawn_sandboxed};

use super::report::{DumpFrame, DumpModule, Native, module_name};
use super::scrub::scrub;

/// A dump larger than this is dropped unread: the engine's dumps are a few megabytes.
const MAX_DUMP_BYTES: u64 = 64 << 20;

/// How long the reader subprocess may run before the parent ends it (the real 10 s cap).
const MAX_READING: Duration = Duration::from_secs(10);

/// The most bytes of summary JSON read from the reader subprocess: an allow-listed summary of 64
/// frames and 64 modules is a few kilobytes, so anything longer is a misbehaving reader and dropped.
const MAX_SUMMARY_BYTES: u64 = 1 << 20;

/// The longest a crash report waits for its engine's dump to be read into a summary.
const WAIT_FOR_DUMP: Duration = Duration::from_secs(8);

/// How often the poller scans the engines' dump folder.
const POLL_EVERY: Duration = Duration::from_millis(250);

/// The most main-process dumps an earlier run left that this launch reads; the rest are deleted.
const MAX_LEFTOVER: usize = 3;

/// The most bytes a crash reason keeps.
const MAX_REASON: usize = 128;

/// The largest offset a report holds: JSON numbers stay exact up to 2^53 − 1 (the schema's cap).
const MAX_OFFSET: u64 = (1 << 53) - 1;

/// Deletes a file when it goes out of scope, however reading it ends.
struct DeleteOnDrop(PathBuf);

impl Drop for DeleteOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Reads a dump into its summary by spawning a sandboxed engine, one at a time. The parent reads the
/// bytes and deletes the file first (keeping the deletion guarantee), caps the size before sending,
/// and caps the time by ending the child.
pub(crate) struct DumpReader {
    engine: PathBuf,
    config: SandboxConfig,
    one_at_a_time: Mutex<()>,
}

impl DumpReader {
    pub(crate) fn new(engine: PathBuf) -> Self {
        Self {
            engine,
            config: SandboxConfig::default(),
            one_at_a_time: Mutex::new(()),
        }
    }

    /// Reads and deletes the dump at `path`: too-large or unreadable gives `None`, and the file is
    /// gone in every case.
    fn summarize_file(&self, path: &Path) -> Option<DumpSummary> {
        let guard = DeleteOnDrop(path.to_owned());
        let len = std::fs::metadata(path).ok()?.len();
        let bytes = (len <= MAX_DUMP_BYTES).then(|| std::fs::read(path).ok()).flatten();
        drop(guard); // the file is deleted now, before the dump is parsed
        self.summarize_bytes(bytes?)
    }

    /// Summarises `bytes` in a fresh sandboxed engine; `None` on any failure or timeout.
    fn summarize_bytes(&self, bytes: Vec<u8>) -> Option<DumpSummary> {
        let _one = self.one_at_a_time.lock().unwrap_or_else(PoisonError::into_inner);
        let mut command = Command::new(&self.engine);
        command.arg(SUMMARIZE_DUMP_ARG);
        let log: Arc<dyn Fn(String) + Send + Sync> =
            Arc::new(|line| tracing::debug!(target: "laminapdf_desktop::crash", "{line}"));
        let mut child = spawn_sandboxed(command, &self.config, &log).ok()?;
        // Feed the dump on a thread so a full pipe can't deadlock with reading the answer.
        let mut stdin = child.stdin;
        let feeder = std::thread::spawn(move || {
            let _ = stdin.write_all(&bytes);
            drop(stdin);
        });
        let mut stdout = child.stdout;
        let (tx, rx) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            let _ = tx.send(read_capped(&mut stdout, MAX_SUMMARY_BYTES));
        });
        let answer = rx.recv_timeout(MAX_READING).ok().flatten();
        // Over time or over size: end the child, so neither cap waits on it.
        if answer.is_none() {
            child.process.kill();
        }
        let _ = child.process.wait();
        let _ = feeder.join();
        let _ = reader.join();
        serde_json::from_slice(&answer?).ok()
    }
}

/// Everything `from` gives, at most `cap` bytes; `None` when there is more than that (or reading
/// fails), so an oversized answer is dropped rather than cut.
fn read_capped(from: &mut impl std::io::Read, cap: u64) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    from.take(cap.saturating_add(1)).read_to_end(&mut out).ok()?;
    (out.len() as u64 <= cap).then_some(out)
}

/// The engines' dumps, read as they come, waiting for their crash by pid.
#[derive(Default)]
pub(crate) struct NativeCrashes {
    read: Mutex<VecDeque<DumpSummary>>,
    arrived: Condvar,
}

impl NativeCrashes {
    fn read(&self) -> MutexGuard<'_, VecDeque<DumpSummary>> {
        self.read.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn push(&self, summary: DumpSummary) {
        let mut read = self.read();
        if read.len() == 16 {
            read.pop_front();
        }
        read.push_back(summary);
        drop(read);
        self.arrived.notify_all();
    }

    /// The dump of engine `pid`, as a report's [`Native`]: at once if read, else waiting up to
    /// [`WAIT_FOR_DUMP`] for the poller to read it; `None` when the engine left none.
    pub(crate) fn take_for_report(&self, pid: u32) -> Option<Native> {
        let deadline = Instant::now() + WAIT_FOR_DUMP;
        let mut read = self.read();
        loop {
            if let Some(at) = read.iter().position(|summary| summary.pid == Some(pid)) {
                return read.remove(at).map(|summary| allow_list(&summary));
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return None;
            }
            read = self
                .arrived
                .wait_timeout(read, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// The monitor process and the servers it hosts, plus the poller that reads the engines' dumps.
pub(crate) struct EngineDumps {
    monitor: Arc<Monitor>,
    reader: Arc<DumpReader>,
    pub(crate) crashes: Arc<NativeCrashes>,
    poll_stop: Arc<AtomicBool>,
    poller: Mutex<Option<JoinHandle<()>>>,
}

impl EngineDumps {
    /// Starts the monitor (which connects the main process's own crash handler) and the poller over
    /// `data_dir/crashes/dumps/engines`. `None` (logged) when the monitor can't start.
    pub(crate) fn start(data_dir: &Path) -> Option<Self> {
        let exe = std::env::current_exe().ok()?;
        let monitor = Monitor::start(exe.clone(), data_dir.to_owned())?;
        let reader = Arc::new(DumpReader::new(engine_program(&exe)));
        let crashes = Arc::new(NativeCrashes::default());
        let engine_dir = monitor.engine_dir();
        let _ = std::fs::create_dir_all(&engine_dir);
        let poll_stop = Arc::new(AtomicBool::new(false));
        let poller = {
            let (reader, crashes, stop) = (Arc::clone(&reader), Arc::clone(&crashes), Arc::clone(&poll_stop));
            std::thread::Builder::new()
                .name("crash-dump-poll".into())
                .spawn(move || poll_dumps(&engine_dir, &reader, &crashes, &stop))
                .ok()?
        };
        Some(Self {
            monitor,
            reader,
            crashes,
            poll_stop,
            poller: Mutex::new(Some(poller)),
        })
    }

    /// The engine's arguments that connect it to the current monitor: the socket and the monitor's
    /// pid (for `PR_SET_PTRACER`). Called for each engine as it spawns, so an engine started after
    /// a monitor restart gets the new socket; none once the monitor is down for good. The engine
    /// waits briefly for a monitor that is still starting (`capture_crashes_within`).
    pub(crate) fn engine_args(&self) -> Vec<OsString> {
        let Some((socket, pid)) = self.monitor.engine_endpoint() else {
            return Vec::new();
        };
        vec![
            CRASH_SOCKET_ARG.into(),
            socket.into(),
            CRASH_SERVER_PID_ARG.into(),
            pid.to_string().into(),
        ]
    }

    /// The reader used for leftover main-process dumps.
    pub(crate) fn reader(&self) -> Arc<DumpReader> {
        Arc::clone(&self.reader)
    }
}

impl Drop for EngineDumps {
    fn drop(&mut self) {
        self.poll_stop.store(true, Ordering::SeqCst);
        if let Some(poller) = self.poller.lock().unwrap_or_else(PoisonError::into_inner).take() {
            let _ = poller.join();
        }
    }
}

/// Scans `dir` for dumps and reads each one (which deletes it), pushing its summary.
fn poll_dumps(dir: &Path, reader: &DumpReader, crashes: &NativeCrashes, stop: &AtomicBool) {
    while !stop.load(Ordering::SeqCst) {
        for path in dumps_in(dir) {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            if let Some(summary) = reader.summarize_file(&path) {
                crashes.push(summary);
            }
        }
        std::thread::sleep(POLL_EVERY);
    }
}

/// The `.dmp` files in `dir`, oldest first.
fn dumps_in(dir: &Path) -> Vec<PathBuf> {
    let mut dumps: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            // Only complete dumps: the monitor writes `.tmp` and renames when done.
            let dump = path.extension().is_some_and(|ext| ext == DUMP_EXTENSION);
            let when = entry.metadata().ok()?.modified().ok()?;
            dump.then_some((when, path))
        })
        .collect();
    dumps.sort_by_key(|(when, _)| *when);
    dumps.into_iter().map(|(_, path)| path).collect()
}

/// The engine binary beside the app's own executable (Tauri places the sidecar there).
fn engine_program(exe: &Path) -> PathBuf {
    let name = format!("laminapdf-desktop-engine{}", std::env::consts::EXE_SUFFIX);
    exe.parent().map_or_else(|| PathBuf::from(&name), |dir| dir.join(&name))
}

/// The main process's monitor: spawned, restarted if it dies, and the home of the main handler.
struct Monitor {
    exe: PathBuf,
    data_dir: PathBuf,
    engine_socket: Mutex<PathBuf>,
    engine_dir: PathBuf,
    pid: AtomicU32,
    /// Whether a monitor is running: cleared when one exits, set again when its restart spawns.
    /// Engines are given its socket only while one is.
    alive: AtomicBool,
    /// The main process's own crash handler, reconnected to each monitor; only one at a time.
    capture: Mutex<Option<CrashCapture>>,
    stop: AtomicBool,
    supervisor: Mutex<Option<JoinHandle<()>>>,
}

/// How long the main process waits for a monitor it just spawned to listen: a little under the
/// monitor's own 30 s wait for it, so a slow start (a debug build loading `WebKitGTK`) still connects
/// before the monitor gives up. The wait runs on the supervisor's thread, off the start-up path.
const MONITOR_READY: Duration = Duration::from_secs(25);

/// How many times a dead monitor is restarted before the main process gives up on capture.
const MAX_MONITOR_RESTARTS: u32 = 5;

impl Monitor {
    fn start(exe: PathBuf, data_dir: PathBuf) -> Option<Arc<Self>> {
        let (paths, child) = spawn_monitor(&exe, &data_dir)?;
        let this = Arc::new(Self {
            exe,
            data_dir,
            engine_dir: paths.engine_dir.clone(),
            engine_socket: Mutex::new(paths.engine_socket.clone()),
            pid: AtomicU32::new(child.id()),
            alive: AtomicBool::new(true),
            capture: Mutex::new(None),
            stop: AtomicBool::new(false),
            supervisor: Mutex::new(None),
        });
        // Connected on the supervisor's thread: the monitor needs a moment to listen, and the
        // start-up path shouldn't wait for it.
        let supervisor = {
            let this = Arc::clone(&this);
            std::thread::Builder::new()
                .name("crash-monitor".into())
                .spawn(move || {
                    this.connect_main(&paths, child.id());
                    this.supervise(child);
                })
                // On Linux the handler's alternate signal stack is this thread's (see
                // `capture_crashes`); the main thread keeps the standard library's.
                .ok()
        };
        *this.supervisor.lock().unwrap_or_else(PoisonError::into_inner) = supervisor;
        Some(this)
    }

    /// The engines' socket and the monitor's pid, while a monitor is running.
    fn engine_endpoint(&self) -> Option<(PathBuf, u32)> {
        self.alive.load(Ordering::SeqCst).then(|| {
            (
                self.engine_socket
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone(),
                self.pid.load(Ordering::SeqCst),
            )
        })
    }

    fn engine_dir(&self) -> PathBuf {
        self.engine_dir.clone()
    }

    /// Reconnects the main process's crash handler to `paths`' main socket, dropping the old one
    /// first (crash-handler allows only one installed handler at a time). The monitor was just
    /// spawned, so its socket may not be listening yet: the connection waits for it, up to
    /// [`MONITOR_READY`]. Connecting at once failed on every start, and the monitor, with no main
    /// process connected, exited after its wait and was restarted until the restarts ran out.
    fn connect_main(&self, paths: &MonitorPaths, monitor_pid: u32) {
        let mut capture = self.capture.lock().unwrap_or_else(PoisonError::into_inner);
        *capture = None;
        match capture_crashes_within(&paths.main_socket, monitor_pid, MONITOR_READY) {
            Ok(installed) => *capture = Some(installed),
            Err(error) => tracing::warn!("the main process's native crashes won't leave a dump: {error}"),
        }
    }

    /// Waits on the monitor; restarts it if it dies unexpectedly, bounded.
    fn supervise(&self, mut child: std::process::Child) {
        let mut restarts = 0u32;
        loop {
            let _ = child.wait();
            self.alive.store(false, Ordering::SeqCst);
            if self.stop.load(Ordering::SeqCst) || restarts >= MAX_MONITOR_RESTARTS {
                return;
            }
            restarts += 1;
            match spawn_monitor(&self.exe, &self.data_dir) {
                Some((paths, next)) => {
                    self.engine_socket
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .clone_from(&paths.engine_socket);
                    self.pid.store(next.id(), Ordering::SeqCst);
                    self.alive.store(true, Ordering::SeqCst);
                    self.connect_main(&paths, next.id());
                    child = next;
                }
                None => return,
            }
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Dropping the main handler disconnects its client, so the monitor's main server stops and
        // the monitor exits; the supervisor then sees the exit and, with `stop` set, returns.
        *self.capture.lock().unwrap_or_else(PoisonError::into_inner) = None;
        if let Some(supervisor) = self.supervisor.lock().unwrap_or_else(PoisonError::into_inner).take() {
            let _ = supervisor.join();
        }
    }
}

/// Spawns the monitor (this binary with [`MonitorPaths::args`]) with a fresh set of sockets.
fn spawn_monitor(exe: &Path, data_dir: &Path) -> Option<(MonitorPaths, std::process::Child)> {
    let paths = MonitorPaths::new(data_dir);
    let child = Command::new(exe)
        .args(paths.args())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| tracing::warn!("the crash monitor could not start: {error}"))
        .ok()?;
    Some((paths, child))
}

/// The main process's native crashes an earlier run left: the newest [`MAX_LEFTOVER`], each read,
/// deleted and returned as its report's [`Native`]; the rest are deleted unread. Runs off the
/// start-up path (`reader` summarises in a subprocess).
pub(crate) fn leftover_main_crashes(data_dir: &Path, reader: &DumpReader) -> Vec<Native> {
    let dir = MonitorPaths::new(data_dir).main_dir;
    let mut dumps = dumps_in(&dir);
    // Newest first; keep the newest few, delete the rest unread.
    dumps.reverse();
    let mut natives = Vec::new();
    for (index, path) in dumps.into_iter().enumerate() {
        if index < MAX_LEFTOVER {
            if let Some(summary) = reader.summarize_file(&path) {
                natives.push(allow_list(&summary));
            }
        } else {
            let _ = std::fs::remove_file(path);
        }
    }
    natives
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
                // Never a name: the computer has no symbols, and a name from the dump's own memory
                // could be document text; `cargo xtask crash symbolicate` names the frames.
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

fn is_debug_id(id: &str) -> bool {
    (32..=40).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_version(version: &str) -> bool {
    (1..=32).contains(&version.len())
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'+' | b'-'))
}

#[cfg(test)]
mod tests {
    use laminapdf_desktop_crash::summary::{DumpFrame as Frame, DumpModule as Module};

    use super::*;

    fn summary(pid: u32) -> DumpSummary {
        DumpSummary {
            pid: Some(pid),
            crash_reason: "EXC_BAD_ACCESS / KERN_INVALID_ADDRESS".into(),
            frames: vec![Frame {
                module: "laminapdf-desktop-engine".into(),
                offset: 0x1a2b,
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
        crashes.push(summary(7));
        let other = crashes.take_for_report(8);
        let own = crashes.take_for_report(7);
        // `other` waits the whole bound (no pid 8) and returns None; `own` is there at once.
        assert_eq!(
            (other.is_none(), own.map(|native| native.frames.len())),
            (true, Some(1))
        );
    }

    #[test]
    fn a_crash_should_wait_for_a_dump_still_being_read() {
        let crashes = Arc::new(NativeCrashes::default());
        let pusher = {
            let crashes = Arc::clone(&crashes);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                crashes.push(summary(7));
            })
        };
        let native = crashes.take_for_report(7);
        let _ = pusher.join();
        assert!(native.is_some());
    }

    #[test]
    fn an_answer_within_the_cap_should_be_read_whole() {
        let answer = read_capped(&mut std::io::Cursor::new(vec![b'x'; 10]), 10);
        assert_eq!(answer.map(|bytes| bytes.len()), Some(10));
    }

    #[test]
    fn an_answer_over_the_cap_should_be_dropped() {
        // An endless reader: only the cap and one byte more are ever read.
        let answer = read_capped(
            &mut std::io::Read::take(std::io::repeat(b'x'), 1 << 30),
            MAX_SUMMARY_BYTES,
        );
        assert!(answer.is_none());
    }

    #[test]
    fn the_poller_should_see_only_complete_dumps() {
        let dir = std::env::temp_dir().join(format!("laminapdf-native-poll-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.tmp"), b"half written").unwrap_or_default();
        std::fs::write(dir.join("b.dmp"), b"complete").unwrap_or_default();
        let seen: Vec<String> = dumps_in(&dir)
            .iter()
            .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(seen, ["b.dmp"]);
    }

    #[test]
    fn the_allow_list_should_keep_only_what_the_schema_accepts() {
        let mut dump = summary(7);
        dump.crash_reason = "SIGSEGV at /Users/ana/FolderMarker7f3a9c/NameMarker4b2e1d.pdf".into();
        dump.frames.push(Frame {
            module: "/usr/lib/libc.so.6".into(),
            offset: u64::MAX,
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
                ["/Users", "/home", "Marker"].iter().any(|marker| json.contains(marker)),
                native.frames.get(1).map(|frame| (frame.module.as_str(), frame.offset)),
                native.modules.len(),
            ),
            (false, Some(("<unknown>", MAX_OFFSET)), 1)
        );
    }
}
