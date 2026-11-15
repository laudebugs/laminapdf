//! Native crash capture (ADR 0014 §10): a process that faults asks another process, which
//! is not sandboxed, to write its minidump. Embark's `crash-handler` catches the fault (a signal on
//! Linux, a Mach exception on macOS, an unhandled exception on Windows) and `minidumper` carries the
//! request to a [`DumpServer`], which reads the crashed process's memory from outside and writes
//! the dump into a private folder. The dump never leaves the computer: the main process reads it
//! into a [`summary`] (feature `summary`) and deletes it.
//!
//! - **Engines**: the main process runs one [`DumpServer`] for all of them and passes its socket in
//!   [`CRASH_SOCKET_ARG`]. Each engine calls [`capture_crashes`] before it locks itself down, so
//!   the connection exists before the sandbox would refuse it; the lock-down then allows only what
//!   the crash path needs (on Linux: the socket's own descriptor, `PR_SET_PTRACER` for the server).
//! - **The main process**: a monitor (the app's own binary, started with a flag) runs a
//!   [`DumpServer`] for it alone and exits when the main process goes away.

#![expect(
    unsafe_code,
    reason = "FFI: crash-handler's unsafe callback, the fd probe, the deliberate fault, the Windows label"
)]

#[cfg(feature = "summary")]
pub mod summary;

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crash_handler::{CrashEventResult, CrashHandler, make_crash_event};
use minidumper::{Client, LoopAction, MinidumpBinary, Server, ServerHandler, SocketName};

/// The engine's argument that names the main process's [`DumpServer`] socket.
pub const CRASH_SOCKET_ARG: &str = "--crash-socket";

/// How many times a server whose loop failed (a client that broke the protocol) is started again.
const MAX_RESTARTS: u32 = 8;

/// Why capture couldn't be set up. Capture is best effort: the app runs without it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CaptureError {
    /// The server's socket couldn't be made, or the client couldn't reach it.
    #[error("the crash socket failed")]
    Socket(#[source] minidumper::Error),
    /// The crash handler couldn't be installed.
    #[error("the crash handler could not be installed: {0}")]
    Handler(String),
    /// The dumps folder couldn't be made.
    #[error("the dumps folder could not be made")]
    Folder(#[source] io::Error),
}

/// What a [`DumpServer`] tells its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumpEvent {
    /// A crashed process asked for a dump; its file is being written.
    Started,
    /// The dump was written to this file, which the owner now owns (and must delete).
    Written(PathBuf),
    /// Writing the dump failed; its file is already deleted.
    Failed(String),
    /// The server's loop failed and was started again (or gave up, after [`MAX_RESTARTS`]).
    Trouble(String),
}

/// Whom a [`DumpServer`] serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Serve {
    /// Any number of clients, until the server is dropped: the main process's engines.
    Many,
    /// One client: the server stops when it disconnects, or when none connected within `wait`.
    One {
        /// How long to wait for the client to connect.
        wait: Duration,
    },
}

/// A `minidumper` server on a thread, writing dumps into a private folder.
pub struct DumpServer {
    socket: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DumpServer {
    /// Listens on `socket` (see [`socket_path`]) and writes each dump into `dumps` (made, private
    /// to the user, if missing). `on_event` runs on the server's thread: it should hand long work
    /// to another thread, since a crashed process waits for the server's answer.
    ///
    /// # Errors
    /// [`CaptureError`] when the folder or the socket can't be made.
    pub fn start(
        socket: PathBuf,
        dumps: PathBuf,
        serve: Serve,
        on_event: impl Fn(DumpEvent) + Send + Sync + 'static,
    ) -> Result<Self, CaptureError> {
        private_folder(&dumps).map_err(CaptureError::Folder)?;
        let server = listen(&socket)?;
        let stop = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(Handler {
            dumps,
            writing: Mutex::new(None),
            serve,
            connected: AtomicBool::new(false),
            on_event: Box::new(on_event),
        });
        if let Serve::One { wait } = serve {
            let (stop, handler) = (Arc::clone(&stop), Arc::clone(&handler));
            // Detached: it only flips the flag, and does nothing once the client has connected.
            let _ = std::thread::Builder::new()
                .name("crash-monitor-wait".into())
                .spawn(move || {
                    std::thread::sleep(wait);
                    if !handler.connected.load(Ordering::SeqCst) {
                        stop.store(true, Ordering::SeqCst);
                    }
                });
        }
        let thread = {
            let (socket, stop) = (socket.clone(), Arc::clone(&stop));
            std::thread::Builder::new()
                .name("crash-dumps".into())
                .spawn(move || serve_loop(server, &socket, &handler, &stop))
                .map_err(|error| CaptureError::Socket(error.into()))?
        };
        Ok(Self {
            socket,
            stop,
            thread: Some(thread),
        })
    }

    /// The socket clients connect to.
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// Waits until the server stops by itself ([`Serve::One`]).
    pub fn wait(mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for DumpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// Runs the server until it is stopped; a loop that fails (a client broke the protocol, which
/// ends `minidumper`'s loop) starts again on a new socket, a few times at most.
fn serve_loop(mut server: Server, socket: &Path, handler: &Arc<Handler>, stop: &AtomicBool) {
    let mut restarts = 0;
    loop {
        match server.run(Box::new(Shared(Arc::clone(handler))), stop, None) {
            Ok(()) => return,
            Err(error) => {
                restarts += 1;
                (handler.on_event)(DumpEvent::Trouble(format!("the crash server stopped: {error}")));
                if restarts > MAX_RESTARTS || stop.load(Ordering::SeqCst) {
                    return;
                }
                match listen(socket) {
                    Ok(again) => server = again,
                    Err(error) => {
                        (handler.on_event)(DumpEvent::Trouble(format!("the crash server can't restart: {error}")));
                        return;
                    }
                }
            }
        }
    }
}

fn listen(socket: &Path) -> Result<Server, CaptureError> {
    let server = Server::with_name(SocketName::Path(socket)).map_err(CaptureError::Socket)?;
    #[cfg(windows)]
    windows::let_low_integrity_connect(socket);
    Ok(server)
}

struct Handler {
    dumps: PathBuf,
    /// The file being written, deleted if writing fails.
    writing: Mutex<Option<PathBuf>>,
    serve: Serve,
    connected: AtomicBool,
    on_event: Box<dyn Fn(DumpEvent) + Send + Sync>,
}

/// `minidumper` takes its handler boxed; the server keeps it across restarts.
struct Shared(Arc<Handler>);

impl ServerHandler for Shared {
    fn create_minidump_file(&self) -> Result<(File, PathBuf), io::Error> {
        (self.0.on_event)(DumpEvent::Started);
        let path = self.0.dumps.join(unique_name("dmp"));
        let file = create_private(&path)?;
        *self.0.writing.lock().unwrap_or_else(PoisonError::into_inner) = Some(path.clone());
        Ok((file, path))
    }

    fn on_minidump_created(&self, result: Result<MinidumpBinary, minidumper::Error>) -> LoopAction {
        let writing = self.0.writing.lock().unwrap_or_else(PoisonError::into_inner).take();
        let event = match result {
            Ok(binary) => {
                let _ = binary.file.sync_all();
                drop(binary.file);
                DumpEvent::Written(binary.path)
            }
            Err(error) => {
                if let Some(path) = writing {
                    let _ = std::fs::remove_file(path);
                }
                DumpEvent::Failed(error.to_string())
            }
        };
        (self.0.on_event)(event);
        LoopAction::Continue
    }

    fn on_message(&self, _kind: u32, _buffer: Vec<u8>) {}

    fn on_client_connected(&self, _clients: usize) -> LoopAction {
        self.0.connected.store(true, Ordering::SeqCst);
        LoopAction::Continue
    }

    fn on_client_disconnected(&self, clients: usize) -> LoopAction {
        if clients == 0 && matches!(self.0.serve, Serve::One { .. }) {
            LoopAction::Exit
        } else {
            LoopAction::Continue
        }
    }
}

/// A socket path unique to this process and moment, for a server named `role`: in the user's
/// runtime folder on Linux (`XDG_RUNTIME_DIR`), else the user's temporary folder (private to the
/// user on macOS and Windows). Kept short: a Unix socket's path holds about 100 bytes.
#[must_use]
pub fn socket_path(role: &str) -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|_| cfg!(target_os = "linux"))
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir());
    let dir = runtime.unwrap_or_else(std::env::temp_dir);
    let path = dir.join(unique_name(&format!("{role}.sock")));
    if cfg!(unix) && path.as_os_str().len() > 100 {
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

fn private_folder(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn create_private(path: &Path) -> io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).read(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)
}

/// Set once a crash is reported some other way (the main process's panic hook), so the abort
/// that follows asks for no dump: one crash, one report.
static SKIP_DUMPS: AtomicBool = AtomicBool::new(false);

/// From now on, a crash in this process writes no dump.
pub fn skip_dumps() {
    SKIP_DUMPS.store(true, Ordering::SeqCst);
}

/// This process's crash handler and its connection to a [`DumpServer`]; dropping it detaches the
/// handler.
pub struct CrashCapture {
    _handler: CrashHandler,
    _client: Arc<Client>,
    descriptor: Option<i32>,
}

impl CrashCapture {
    /// The socket's descriptor (Unix): the one descriptor the engine's sandbox lets it keep and
    /// write to besides its pipes.
    pub const fn descriptor(&self) -> Option<i32> {
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

/// Connects to the [`DumpServer`] at `socket` and installs the crash handler that asks it for a
/// dump. `server` is the server's process id: on Linux the handler lets it trace this process
/// while it writes the dump (`PR_SET_PTRACER`, for Yama's `ptrace_scope` 1). Call it before
/// the sandbox goes on, from the only thread (the engine) or early (the main process).
///
/// # Errors
/// [`CaptureError`] when the server can't be reached or the handler can't be installed.
pub fn capture_crashes(socket: &Path, server: Option<u32>) -> Result<CrashCapture, CaptureError> {
    let before = open_descriptors();
    let client = Arc::new(Client::with_name(SocketName::Path(socket)).map_err(CaptureError::Socket)?);
    let descriptor = open_descriptors().into_iter().find(|fd| !before.contains(fd));
    let asks = Arc::clone(&client);
    // SAFETY: the callback runs in a signal handler or on the exception thread while the process
    // is crashing. It only reads a flag and sends the crash context over the socket opened above,
    // which is what `minidumper::Client::request_dump` is made for; it allocates nothing.
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
    handler.set_ptracer(server);
    #[cfg(not(target_os = "linux"))]
    let _ = server;
    Ok(CrashCapture {
        _handler: handler,
        _client: client,
        descriptor,
    })
}

/// The open descriptors above 2 (Unix; none elsewhere), to find the one the client opened.
fn open_descriptors() -> Vec<i32> {
    #[cfg(unix)]
    {
        (3..1024)
            // SAFETY: F_GETFD on any number only asks whether it is an open descriptor.
            .filter(|&fd| unsafe { libc::fcntl(fd, libc::F_GETFD) } != -1)
            .collect()
    }
    #[cfg(not(unix))]
    Vec::new()
}

/// Faults on purpose: a write through a null pointer, so the end-to-end tests see a real native
/// crash (`ToEngine::DebugSegv`), not a signal sent from outside. Feature `debug-fault`, which only
/// the e2e engine turns on.
#[cfg(feature = "debug-fault")]
pub fn fault() {
    let null = std::ptr::null_mut::<u64>();
    // SAFETY: none: this is the deliberate fault the end-to-end tests ask for. The write through a
    // null pointer faults (SIGSEGV, EXC_BAD_ACCESS, an access violation) and the process ends in
    // its crash handler; `write_volatile` keeps the compiler from removing it.
    unsafe { null.write_volatile(0x1a3e_57ed) };
}

#[cfg(windows)]
mod windows {
    use std::os::windows::ffi::OsStrExt as _;
    use std::path::Path;

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1, SE_FILE_OBJECT, SetNamedSecurityInfoW,
    };
    use windows_sys::Win32::Security::{
        ACL, GetSecurityDescriptorSacl, LABEL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    };

    /// Labels the socket's file Low integrity, so the engine (Low integrity) may connect to it;
    /// its DACL still admits only the user. Best effort: without it the engine runs uncaptured.
    pub(super) fn let_low_integrity_connect(socket: &Path) {
        let path: Vec<u16> = socket.as_os_str().encode_wide().chain([0]).collect();
        // The mandatory label "Low, no write up": S:(ML;;NW;;;LW).
        let sddl: Vec<u16> = "S:(ML;;NW;;;LW)".encode_utf16().chain([0]).collect();
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: `sddl` is NUL-terminated; `descriptor` is a valid out-pointer, freed below.
        let made = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &raw mut descriptor,
                std::ptr::null_mut(),
            )
        };
        if made == 0 {
            return;
        }
        let (mut present, mut defaulted) = (0, 0);
        let mut sacl: *mut ACL = std::ptr::null_mut();
        // SAFETY: `descriptor` is the valid descriptor made above; the out-pointers are valid.
        let got = unsafe { GetSecurityDescriptorSacl(descriptor, &raw mut present, &raw mut sacl, &raw mut defaulted) };
        if got != 0 && present != 0 {
            // SAFETY: `path` is NUL-terminated; `sacl` points into `descriptor`, alive until freed.
            unsafe {
                SetNamedSecurityInfoW(
                    path.as_ptr(),
                    SE_FILE_OBJECT,
                    LABEL_SECURITY_INFORMATION,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    sacl,
                );
            }
        }
        // SAFETY: `descriptor` was allocated by the conversion above and is freed once.
        unsafe { LocalFree(descriptor) };
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

    use std::time::Instant;

    use super::*;

    fn dumps(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("laminapdf-capture-{}-{name}", std::process::id()))
    }

    /// The main process's monitor: it serves one client and stops once that client goes.
    #[test]
    fn a_one_client_server_should_stop_when_its_client_disconnects() {
        let wait = Duration::from_secs(30);
        let server = DumpServer::start(socket_path("one"), dumps("one"), Serve::One { wait }, |_| {}).unwrap();
        let client = Client::with_name(SocketName::Path(server.socket())).unwrap();
        let started = Instant::now();
        drop(client);
        server.wait();
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_one_client_server_should_stop_when_no_client_comes() {
        let wait = Duration::from_millis(200);
        let server = DumpServer::start(socket_path("none"), dumps("none"), Serve::One { wait }, |_| {}).unwrap();
        let started = Instant::now();
        server.wait();
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_server_should_remove_its_socket_when_dropped() {
        let server = DumpServer::start(socket_path("drop"), dumps("drop"), Serve::Many, |_| {}).unwrap();
        let socket = server.socket().to_owned();
        drop(server);
        assert!(!socket.exists());
    }

    #[test]
    fn socket_paths_should_differ_and_fit_a_unix_socket() {
        let (a, b) = (socket_path("engines"), socket_path("engines"));
        assert_eq!((a != b, a.as_os_str().len() <= 104), (true, true));
    }
}
