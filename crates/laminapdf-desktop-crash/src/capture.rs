//! Native crash capture on Unix (ADR 0014 §10): the minidump servers the monitor hosts, and the
//! crash handler a faulting process installs to ask its server for a dump. Nothing here runs in the
//! main process except installing its own handler and spawning the monitor; the servers, which read
//! a hostile process's memory, run in the monitor, so a bad message crashes only the monitor.

#![expect(
    unsafe_code,
    reason = "FFI: crash-handler's unsafe callback and the open-socket probe"
)]

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::os::fd::RawFd;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crash_handler::{CrashEventResult, CrashHandler, make_crash_event};
use minidumper::{Client, LoopAction, MinidumpBinary, Server, ServerHandler, SocketName};

/// The argument that makes the app binary the crash monitor:
/// `--crash-monitor <main-socket> <main-dir> <engine-socket> <engine-dir>`.
pub const MONITOR_ARG: &str = "--crash-monitor";

/// How many times a server loop that fails (a client that broke the protocol ends `minidumper`'s
/// `run`) is started again within [`RESTART_WINDOW`] before the server stops; a quiet window resets
/// the count. When the engines' server stops this way the monitor exits, and the main process
/// starts a new one (at most 5 times a session: the app's `MAX_MONITOR_RESTARTS`).
const MAX_RESTARTS: u32 = 16;

/// The window [`MAX_RESTARTS`] counts in.
const RESTART_WINDOW: Duration = Duration::from_secs(60);

/// The most dumps a folder may hold (written or being written) before a new one is refused: a
/// compromised engine asking for dump after dump can't fill the disk or keep the reader busy.
const MAX_DUMPS_IN_FOLDER: usize = 4;

/// The most dumps one server writes per minute, whatever the folder holds.
const MAX_DUMPS_PER_MINUTE: usize = 6;

/// A dump being written: renamed to `.dmp` only once complete, so a reader never sees a partial one.
const WRITING_EXTENSION: &str = "tmp";

/// A complete dump, the only kind the main process reads.
pub const DUMP_EXTENSION: &str = "dmp";

/// How long the monitor waits for the main process to connect before it gives up and exits.
const MONITOR_WAIT: Duration = Duration::from_secs(30);

/// The sockets and dump folders the monitor serves. The main process makes a fresh set per monitor,
/// so a restarted monitor never races a dead one's socket files.
#[derive(Debug, Clone)]
pub struct MonitorPaths {
    /// The main process's crash socket.
    pub main_socket: PathBuf,
    /// Where the main process's dumps are written (read at the next launch).
    pub main_dir: PathBuf,
    /// The engines' crash socket, passed to each engine at spawn.
    pub engine_socket: PathBuf,
    /// Where the engines' dumps are written (read live by the main process).
    pub engine_dir: PathBuf,
}

impl MonitorPaths {
    /// A fresh set under `data_dir/crashes/dumps`.
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        let dumps = data_dir.join("crashes").join("dumps");
        Self {
            main_socket: socket_path("main"),
            main_dir: dumps.join("main"),
            engine_socket: socket_path("engines"),
            engine_dir: dumps.join("engines"),
        }
    }

    /// The monitor's command-line arguments (after the program name).
    #[must_use]
    pub fn args(&self) -> Vec<OsString> {
        vec![
            MONITOR_ARG.into(),
            self.main_socket.clone().into(),
            self.main_dir.clone().into(),
            self.engine_socket.clone().into(),
            self.engine_dir.clone().into(),
        ]
    }
}

/// Where a [`DumpServer`] writes dumps and how long it serves.
#[derive(Clone, Copy)]
enum Role {
    /// Any number of clients (the engines), until the server is dropped.
    Many,
    /// One client (the main process): stop when it disconnects, or when none came within the wait.
    One,
}

/// Parses `--crash-monitor …` and, if that is what this process is, runs the monitor's two servers
/// until the main process goes away, then returns its exit code. Otherwise returns `None`.
#[must_use]
pub fn run_monitor(mut args: impl Iterator<Item = OsString>) -> Option<ExitCode> {
    if args.next().is_none_or(|arg| arg != MONITOR_ARG) {
        return None;
    }
    let (main_socket, main_dir, engine_socket, engine_dir) = match (args.next(), args.next(), args.next(), args.next())
    {
        (Some(a), Some(b), Some(c), Some(d)) => {
            (PathBuf::from(a), PathBuf::from(b), PathBuf::from(c), PathBuf::from(d))
        }
        _ => return Some(ExitCode::FAILURE),
    };
    // The engines' server runs for the monitor's whole life; the main server stops when the main
    // process disconnects, which ends the monitor.
    let Ok(engines) = DumpServer::start(engine_socket, engine_dir, Role::Many) else {
        return Some(ExitCode::FAILURE);
    };
    let Ok(main) = DumpServer::start(main_socket, main_dir, Role::One) else {
        return Some(ExitCode::FAILURE);
    };
    // Until the main process goes (the main server stops), or the engines' server gives up after
    // too many protocol failures: then the monitor exits and the main process starts a fresh one.
    let code = loop {
        if main.finished() {
            break ExitCode::SUCCESS;
        }
        if engines.finished() {
            break ExitCode::FAILURE;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    drop(main);
    drop(engines);
    Some(code)
}

/// A `minidumper` server on a thread, writing dumps into a private folder. Used only by the monitor.
pub(crate) struct DumpServer {
    socket: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DumpServer {
    fn start(socket: PathBuf, dir: PathBuf, role: Role) -> io::Result<Self> {
        private_dir(&dir)?;
        // A dump an earlier monitor left half written is never read: delete it.
        for path in files_with(&dir, WRITING_EXTENSION) {
            let _ = std::fs::remove_file(path);
        }
        // Bind once up front so a failure is reported before the thread starts.
        let server = bind(&socket)?;
        let stop = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let one = matches!(role, Role::One);
        if one {
            let (stop, connected) = (Arc::clone(&stop), Arc::clone(&connected));
            // Detached: it only flips the stop flag if no client ever connects.
            std::thread::Builder::new()
                .name("crash-monitor-wait".into())
                .spawn(move || {
                    std::thread::sleep(MONITOR_WAIT);
                    if !connected.load(Ordering::SeqCst) {
                        stop.store(true, Ordering::SeqCst);
                    }
                })?;
        }
        let handler = Handler {
            dir,
            one,
            connected,
            writing: Arc::new(Mutex::new(None)),
            written: Arc::new(Mutex::new(VecDeque::new())),
        };
        let thread = {
            let (socket, stop) = (socket.clone(), Arc::clone(&stop));
            std::thread::Builder::new()
                .name("crash-dumps".into())
                .spawn(move || serve_loop(server, &socket, &handler, &stop))?
        };
        Ok(Self {
            socket,
            stop,
            thread: Some(thread),
        })
    }

    /// The socket clients connect to (tests only).
    #[cfg(test)]
    fn socket(&self) -> &Path {
        &self.socket
    }

    /// Whether the server's thread has ended (stopped by itself, or given up).
    fn finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

impl Drop for DumpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        // minidumper's own `Server` removed the file on each bind; this is belt and braces.
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// Runs the server until stopped. A loop that fails — a client broke the protocol, which ends
/// `minidumper`'s `run` — is started again, at most [`MAX_RESTARTS`] times in [`RESTART_WINDOW`];
/// past that the loop ends (and the monitor with it). The old `Server` is dropped **before** the
/// new one binds, so the new socket file the bind just made is never removed by the old one's `Drop`.
fn serve_loop(mut server: Server, socket: &Path, handler: &Handler, stop: &AtomicBool) {
    let mut budget = RestartBudget::new(Instant::now());
    loop {
        let handler = Box::new(Shared(handler.clone()));
        // No stale timeout: a client sends nothing until it crashes, so a quiet connection is the
        // normal case. With one, minidumper dropped every connection after a minute: the engines'
        // capture ended silently, and the main process's drop ended the monitor.
        match server.run(handler, stop, None) {
            Ok(()) => return,
            Err(_) if stop.load(Ordering::SeqCst) => return,
            Err(_) => {
                if !budget.allow(Instant::now()) {
                    return;
                }
                drop(server);
                match bind(socket) {
                    Ok(again) => server = again,
                    Err(_) => return,
                }
            }
        }
    }
}

/// [`MAX_RESTARTS`] restarts per [`RESTART_WINDOW`]: a window with none left ends the server; a new
/// window starts the count again.
struct RestartBudget {
    used: u32,
    since: Instant,
}

impl RestartBudget {
    const fn new(now: Instant) -> Self {
        Self { used: 0, since: now }
    }

    /// Whether one more restart is allowed at `now`, counting it if so.
    fn allow(&mut self, now: Instant) -> bool {
        if now.duration_since(self.since) > RESTART_WINDOW {
            *self = Self::new(now);
        }
        if self.used >= MAX_RESTARTS {
            return false;
        }
        self.used += 1;
        true
    }
}

fn bind(socket: &Path) -> io::Result<Server> {
    Server::with_name(SocketName::Path(socket)).map_err(io::Error::other)
}

#[derive(Clone)]
struct Handler {
    dir: PathBuf,
    one: bool,
    connected: Arc<AtomicBool>,
    /// The `.tmp` file of the dump being written, removed if writing fails.
    writing: Arc<Mutex<Option<PathBuf>>>,
    /// When the last dumps were started, for [`MAX_DUMPS_PER_MINUTE`].
    written: Arc<Mutex<VecDeque<Instant>>>,
}

impl Handler {
    /// Whether one more dump may be written now: few enough in the folder and in the last minute.
    fn may_write(&self) -> bool {
        let held = files_with(&self.dir, DUMP_EXTENSION).len() + files_with(&self.dir, WRITING_EXTENSION).len();
        if held >= MAX_DUMPS_IN_FOLDER {
            return false;
        }
        let mut written = self.written.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();
        while written
            .front()
            .is_some_and(|at| now.duration_since(*at) > Duration::from_secs(60))
        {
            written.pop_front();
        }
        if written.len() >= MAX_DUMPS_PER_MINUTE {
            return false;
        }
        written.push_back(now);
        true
    }
}

/// The files in `dir` with extension `ext`.
fn files_with(dir: &Path, ext: &str) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == ext))
        .collect()
}

/// `minidumper` takes a boxed handler each `run`; this shares one across restarts.
struct Shared(Handler);

impl ServerHandler for Shared {
    fn create_minidump_file(&self) -> Result<(File, PathBuf), io::Error> {
        // Refused (minidumper logs it and answers the client) past the folder's or the minute's cap.
        if !self.0.may_write() {
            return Err(io::Error::other("too many crash dumps"));
        }
        let path = self.0.dir.join(unique_name(WRITING_EXTENSION));
        let file = File::options()
            .write(true)
            .read(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        *self.0.writing.lock().unwrap_or_else(PoisonError::into_inner) = Some(path.clone());
        Ok((file, path))
    }

    fn on_minidump_created(&self, result: Result<MinidumpBinary, minidumper::Error>) -> LoopAction {
        let writing = self.0.writing.lock().unwrap_or_else(PoisonError::into_inner).take();
        match result {
            // Complete: flushed, then renamed to `.dmp`, the only name the main process reads.
            Ok(binary) => {
                let _ = binary.file.sync_all();
                drop(binary.file);
                let _ = std::fs::rename(&binary.path, binary.path.with_extension(DUMP_EXTENSION));
            }
            Err(_) => {
                if let Some(path) = writing {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        LoopAction::Continue
    }

    fn on_message(&self, _kind: u32, _buffer: Vec<u8>) {}

    fn on_client_connected(&self, _clients: usize) -> LoopAction {
        self.0.connected.store(true, Ordering::SeqCst);
        LoopAction::Continue
    }

    fn on_client_disconnected(&self, clients: usize) -> LoopAction {
        if self.0.one && clients == 0 {
            LoopAction::Exit
        } else {
            LoopAction::Continue
        }
    }
}

/// A socket path unique to this process and moment, for a server named `role`: the user's runtime
/// folder on Linux (`XDG_RUNTIME_DIR`), else the user's temporary folder.
fn socket_path(role: &str) -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|_| cfg!(target_os = "linux"))
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir());
    let dir = runtime.unwrap_or_else(std::env::temp_dir);
    let path = dir.join(unique_name(&format!("{role}.sock")));
    // A Unix socket path holds about 100 bytes; fall back to /tmp if the chosen folder is too deep.
    if path.as_os_str().len() > 100 {
        return Path::new("/tmp").join(unique_name(&format!("{role}.sock")));
    }
    path
}

/// `lamina-<pid>-<time>-<n>.<suffix>`: no two the same in one process, nor across processes.
fn unique_name(suffix: &str) -> String {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.subsec_nanos() ^ u32::try_from(since.as_secs() % (1 << 32)).unwrap_or_default())
        .unwrap_or_default();
    format!(
        "lamina-{}-{nanos:08x}-{}.{suffix}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    )
}

/// Makes `dir`, private to the user.
fn private_dir(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

/// Set once a crash is reported some other way (the main process's panic hook), so the abort that
/// follows asks for no dump: one crash, one report.
static SKIP_DUMPS: AtomicBool = AtomicBool::new(false);

/// From now on, a crash in this process writes no dump.
pub fn skip_dumps() {
    SKIP_DUMPS.store(true, Ordering::SeqCst);
}

/// A faulting process's crash handler and its connection to a server. Dropping it detaches the
/// handler.
pub struct CrashCapture {
    _handler: CrashHandler,
    _client: Arc<Client>,
    descriptor: Option<RawFd>,
}

impl CrashCapture {
    /// The crash socket's descriptor: the one descriptor, besides its pipes, the engine's sandbox
    /// must let it send and receive on. `None` when it could not be identified: then the engine
    /// drops this capture (closing the socket, which the lock-down would otherwise count as a
    /// leaked descriptor) and runs without capture.
    #[must_use]
    pub const fn descriptor(&self) -> Option<RawFd> {
        self.descriptor
    }
}

impl std::fmt::Debug for CrashCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrashCapture")
            .field("descriptor", &self.descriptor)
            .finish_non_exhaustive()
    }
}

/// Why capture could not be set up (non-fatal: the process runs without it).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CaptureError {
    /// The server could not be reached.
    #[error("the crash socket could not be reached")]
    Connect,
    /// The crash handler could not be installed.
    #[error("the crash handler could not be installed: {0}")]
    Handler(String),
}

/// Connects to the server at `socket` and installs the crash handler that asks it for a dump.
/// `server` is the server process's pid (the monitor): on Linux it alone may trace this process
/// while it writes the dump (`PR_SET_PTRACER`, for Yama's `ptrace_scope` 1; never
/// `PR_SET_PTRACER_ANY`, and the engine's seccomp allows exactly this pid). Call it before the
/// sandbox goes on, from the only thread (the engine) or early (the main process).
///
/// # Errors
/// [`CaptureError`] when the server can't be reached or the handler can't be installed.
pub fn capture_crashes(socket: &Path, server: u32) -> Result<CrashCapture, CaptureError> {
    let before = open_sockets();
    let client = Arc::new(Client::with_name(SocketName::Path(socket)).map_err(|_| CaptureError::Connect)?);
    // minidumper gives no accessor for the client's socket fd, so the one new connected socket this
    // call opened is it. Exactly one must appear; otherwise we don't guess (the engine then logs and
    // runs without capture rather than allowing the wrong fd through seccomp).
    let after = open_sockets();
    let new: Vec<RawFd> = after.into_iter().filter(|fd| !before.contains(fd)).collect();
    let descriptor = match new.as_slice() {
        [fd] => Some(*fd),
        _ => None,
    };
    let asks = Arc::clone(&client);
    // SAFETY: the callback runs in a signal handler or on the exception thread while the process is
    // crashing. It only reads a flag and sends the crash context over the socket opened above, which
    // is what `minidumper::Client::request_dump` is made for; it allocates nothing.
    let event = unsafe {
        make_crash_event(move |context| {
            if SKIP_DUMPS.load(Ordering::SeqCst) {
                return CrashEventResult::Handled(false);
            }
            CrashEventResult::Handled(asks.request_dump(context).is_ok())
        })
    };
    let handler = CrashHandler::attach(event).map_err(|error| CaptureError::Handler(error.to_string()))?;
    #[cfg(target_os = "linux")]
    handler.set_ptracer(Some(server));
    #[cfg(not(target_os = "linux"))]
    let _ = server;
    Ok(CrashCapture {
        _handler: handler,
        _client: client,
        descriptor,
    })
}

/// The open connected-socket descriptors above 2 this process holds, to find the one the client
/// opened. Only sockets, so an unrelated file opened in between is not mistaken for it.
fn open_sockets() -> Vec<RawFd> {
    (3..1024).filter(|&fd| is_socket(fd)).collect()
}

/// Whether `fd` is an open socket descriptor.
fn is_socket(fd: RawFd) -> bool {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `fstat` on any number only reads the descriptor's kind into `stat`, or fails; on
    // failure `stat` is left uninitialised and we return false without reading it.
    if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
        return false;
    }
    // SAFETY: `fstat` succeeded above, so it wrote a complete `libc::stat`.
    let mode = unsafe { stat.assume_init() }.st_mode;
    mode & libc::S_IFMT == libc::S_IFSOCK
}

/// A crash-dump server for other crates' tests (the engine's native-crash test): the role the
/// monitor gives the engines, so a sandboxed engine can connect and fault against it.
#[cfg(feature = "test-support")]
pub mod test_support {
    use std::io;
    use std::path::{Path, PathBuf};

    use super::{DumpServer, Role, socket_path};

    /// A running engine-dump server writing to `dir`.
    pub struct TestDumpServer(DumpServer);

    impl TestDumpServer {
        /// Starts the server, writing dumps into `dir`.
        ///
        /// # Errors
        /// When the folder or socket can't be made.
        pub fn start(dir: PathBuf) -> io::Result<Self> {
            Ok(Self(DumpServer::start(socket_path("test"), dir, Role::Many)?))
        }

        /// The socket a client connects to.
        #[must_use]
        pub fn socket(&self) -> &Path {
            &self.0.socket
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::fd::AsRawFd as _;

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("laminapdf-crash-{}-{name}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Raw bytes at a crash socket, as a hostile client might send; a stream to a seqpacket
    /// listener may even fail to connect, which is itself garbage the server must survive.
    fn send_garbage(path: &Path) {
        let Some(addr) = os_socket_addr(path) else {
            return;
        };
        // SAFETY: a SOCK_SEQPACKET socket of the server's family, connected to `addr` (a valid
        // sockaddr_un), then sent 64 junk bytes and closed; every call's arguments are checked, and
        // the fd is closed once. A wrong-type connect would hang, but the type matches the server's.
        unsafe {
            let fd = libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET, 0);
            if fd < 0 {
                return;
            }
            let _ = libc::connect(fd, (&raw const addr.0).cast(), addr.1);
            let junk = [0xffu8; 64];
            let _ = libc::send(fd, junk.as_ptr().cast(), junk.len(), 0);
            libc::close(fd);
        }
    }

    /// A `sockaddr_un` for `path`, and its length, or `None` if the path is too long.
    fn os_socket_addr(path: &Path) -> Option<(libc::sockaddr_un, libc::socklen_t)> {
        use std::mem::size_of;
        use std::os::unix::ffi::OsStrExt as _;
        let bytes = path.as_os_str().as_bytes();
        // SAFETY: a zeroed sockaddr_un is valid; only the path bytes are written, leaving the NUL.
        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        if bytes.len() >= addr.sun_path.len() {
            return None;
        }
        addr.sun_family = libc::sa_family_t::try_from(libc::AF_UNIX).ok()?;
        // `c_char` is signed on some targets and unsigned on others (Linux on Arm64).
        for (slot, &byte) in addr.sun_path.iter_mut().zip(bytes) {
            *slot = libc::c_char::from_ne_bytes([byte]);
        }
        let len = libc::socklen_t::try_from(size_of::<libc::sa_family_t>() + bytes.len() + 1).ok()?;
        Some((addr, len))
    }

    #[test]
    fn a_one_client_server_should_stop_when_its_client_disconnects() {
        let server = DumpServer::start(socket_path("one"), scratch("one"), Role::One).unwrap();
        let client = Client::with_name(SocketName::Path(server.socket())).unwrap();
        let started = Instant::now();
        drop(client);
        while !server.finished() && started.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(server.finished());
    }

    #[test]
    fn restarts_should_be_bounded_per_window_and_reset_after_a_quiet_one() {
        let start = Instant::now();
        let mut budget = RestartBudget::new(start);
        let burst = (0..=MAX_RESTARTS).filter(|_| budget.allow(start)).count();
        let after_quiet = budget.allow(start + RESTART_WINDOW + Duration::from_secs(1));
        assert_eq!((burst, after_quiet), (MAX_RESTARTS as usize, true));
    }

    fn handler_for(dir: PathBuf) -> Shared {
        Shared(Handler {
            dir,
            one: false,
            connected: Arc::new(AtomicBool::new(false)),
            writing: Arc::new(Mutex::new(None)),
            written: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    #[test]
    fn a_dump_should_be_written_as_tmp_and_renamed_to_dmp_only_when_complete() {
        let dir = scratch("rename");
        for path in files_with(&dir, DUMP_EXTENSION) {
            let _ = std::fs::remove_file(path);
        }
        let handler = handler_for(dir.clone());
        let (file, path) = handler.create_minidump_file().unwrap();
        // While it is written, the reader's `.dmp` glob sees nothing.
        let during = (
            path.extension().is_some_and(|e| e == WRITING_EXTENSION),
            files_with(&dir, DUMP_EXTENSION).len(),
        );
        let action = handler.on_minidump_created(Ok(MinidumpBinary {
            file,
            path: path.clone(),
            contents: None,
        }));
        assert_eq!(
            (
                during,
                action == LoopAction::Continue,
                path.exists(),
                path.with_extension(DUMP_EXTENSION).exists()
            ),
            ((true, 0), true, false, true)
        );
    }

    #[test]
    fn a_failed_dump_should_leave_no_file() {
        let dir = scratch("failed");
        let handler = handler_for(dir);
        let (_file, path) = handler.create_minidump_file().unwrap();
        handler.on_minidump_created(Err(minidumper::Error::ProtocolError("test")));
        assert!(!path.exists());
    }

    #[test]
    fn new_dumps_should_be_refused_once_the_folder_holds_its_cap() {
        let dir = scratch("cap");
        let handler = handler_for(dir.clone());
        let made: Vec<bool> = (0..MAX_DUMPS_IN_FOLDER + 2)
            .map(|_| handler.create_minidump_file().is_ok())
            .collect();
        for path in files_with(&dir, WRITING_EXTENSION) {
            let _ = std::fs::remove_file(path);
        }
        assert_eq!(made.iter().filter(|ok| **ok).count(), MAX_DUMPS_IN_FOLDER);
    }

    #[test]
    fn new_dumps_should_be_refused_past_the_per_minute_cap() {
        let handler = handler_for(scratch("rate"));
        // Each dump is completed and taken away, so only the rate can refuse.
        let made: Vec<bool> = (0..MAX_DUMPS_PER_MINUTE + 2)
            .map(|_| {
                let made = handler.create_minidump_file();
                if let Ok((_, path)) = &made {
                    let _ = std::fs::remove_file(path);
                }
                made.is_ok()
            })
            .collect();
        assert_eq!(made.iter().filter(|ok| **ok).count(), MAX_DUMPS_PER_MINUTE);
    }

    #[test]
    fn a_server_should_keep_serving_after_a_client_sends_garbage() {
        let server = DumpServer::start(socket_path("garbage"), scratch("garbage"), Role::Many).unwrap();
        let socket = server.socket().to_owned();
        send_garbage(&socket);
        send_garbage(&socket);
        // A proper client must still connect and get a pong: the server survived the garbage.
        let mut pinged = false;
        for _ in 0..50 {
            if let Ok(client) = Client::with_name(SocketName::Path(&socket))
                && client.ping().is_ok()
            {
                pinged = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(server);
        assert!(pinged, "the server did not serve a client after garbage");
    }

    #[test]
    fn a_server_should_remove_its_socket_when_dropped() {
        let server = DumpServer::start(socket_path("drop"), scratch("drop"), Role::Many).unwrap();
        let socket = server.socket().to_owned();
        drop(server);
        assert!(!socket.exists());
    }

    #[test]
    fn socket_paths_should_differ_and_fit_a_unix_socket() {
        let (a, b) = (socket_path("engines"), socket_path("engines"));
        assert_eq!((a != b, a.as_os_str().len() <= 104), (true, true));
    }

    #[test]
    fn the_monitor_flag_should_be_needed_to_run_the_monitor() {
        assert!(run_monitor(["--other".into()].into_iter()).is_none());
    }

    #[test]
    fn open_sockets_should_find_a_socket_and_skip_a_plain_file() {
        // A socket and a regular file, both open: only the socket's fd is reported.
        let mut pair = [0; 2];
        // SAFETY: `socketpair` writes two fds into `pair` on success; checked below, closed once.
        let made = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, pair.as_mut_ptr()) };
        assert_eq!(made, 0);
        let file = File::open("/dev/null").unwrap();
        let open = open_sockets();
        assert_eq!(
            (
                open.contains(&pair[0]),
                open.contains(&pair[1]),
                open.contains(&file.as_raw_fd())
            ),
            (true, true, false)
        );
        // SAFETY: both fds came from the socketpair above and are closed once here.
        unsafe {
            libc::close(pair[0]);
            libc::close(pair[1]);
        }
    }

    #[test]
    fn connecting_should_install_a_handler() {
        // The server runs in this process, so its accepted fd also opens: the descriptor diff is
        // reliable only in the single-threaded engine with an out-of-process monitor, so here we
        // check only that capture connects and installs its handler.
        let server = DumpServer::start(socket_path("fd"), scratch("fd"), Role::Many).unwrap();
        let capture = capture_crashes(server.socket(), std::process::id());
        assert!(capture.is_ok());
        drop(server);
    }
}
