//! [`Supervisor`]: the main process's side of one engine. It starts the engine, matches replies
//! to requests, serves the engine's reads, watches its heartbeat and restarts it after a crash.
//!
//! It knows nothing of Tauri, so the engine's tests drive it with the real binary and this
//! module's tests with an in-process fake ([`Spawner`]).
//!
//! **Restart policy** (ADR 0008 §2): an engine that exits or misses its heartbeats for
//! [`Config::heartbeat_timeout`] is killed and started again after [`Config::backoff`], doubled
//! for each recent crash. More than [`Config::max_restarts`] crashes within [`Config::window`]
//! mark the engine failed: requests then answer [`SupervisorError::Failed`] and the owner gets
//! [`Event::Failed`] instead of a restart loop. After each restart the owner gets
//! [`Event::Restarted`] and reopens its documents.
//!
//! Locks are never held while waiting for the engine: a request takes the state lock to register
//! its reply slot and releases it before writing.

use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::{DocId, ErrorKind, FromEngine, Limits, MAX_FRAME, PROTOCOL_VERSION, ProtocolError, ReqId, ToEngine};
use crate::{read_frame, write_frame};

/// The timings and limits of one supervised engine.
#[derive(Debug, Clone)]
pub struct Config {
    /// What every document of the engine opens with.
    pub limits: Limits,
    /// Silence after which a running engine is taken as hung: 6 s, three missed 2 s periods
    /// (the engine beats every second).
    pub heartbeat_timeout: Duration,
    /// How long a request waits for its reply.
    pub reply_timeout: Duration,
    /// How long a new engine has to answer `Hello`.
    pub start_timeout: Duration,
    /// The wait before the first restart, doubled for each further crash in [`Config::window`].
    pub backoff: Duration,
    /// Restarts allowed within [`Config::window`]; one more crash marks the engine failed.
    pub max_restarts: usize,
    /// The span [`Config::max_restarts`] counts over.
    pub window: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            heartbeat_timeout: Duration::from_secs(6),
            reply_timeout: Duration::from_secs(60),
            start_timeout: Duration::from_secs(10),
            backoff: Duration::from_millis(250),
            max_restarts: 3,
            window: Duration::from_secs(60),
        }
    }
}

/// What happened to the engine, for its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A new engine replaced one that crashed; reopen the documents.
    Restarted {
        /// The new engine's process id.
        pid: u32,
    },
    /// Too many crashes: the engine stays down.
    Failed,
    /// A line for the log (from the engine's stderr or about it). Never document content.
    Log(String),
}

/// Answers the engine's reads: the main process's files, by document.
pub trait RangeServer: Send + Sync {
    /// `len` bytes of `doc` from `offset`, short only at the end of the file.
    ///
    /// # Errors
    /// The document isn't this engine's, or the file can't be read.
    fn read(&self, doc: DocId, offset: u64, len: u32) -> io::Result<Vec<u8>>;
}

/// A started engine: its pipes and its process.
pub struct Spawned {
    /// The engine's stdin.
    pub stdin: Box<dyn Write + Send>,
    /// The engine's stdout.
    pub stdout: Box<dyn Read + Send>,
    /// The process.
    pub process: Box<dyn Process>,
}

/// An engine process, as the supervisor needs it.
pub trait Process: Send {
    /// Its process id.
    fn id(&self) -> u32;
    /// Ends it at once (`kill -9`, `TerminateProcess`).
    fn kill(&mut self);
    /// Waits for it to end and describes how it did.
    fn wait(&mut self) -> String;
}

/// Starts engines.
pub trait Spawner: Send + Sync {
    /// Starts one engine.
    ///
    /// # Errors
    /// The program couldn't start.
    fn spawn(&self) -> io::Result<Spawned>;
}

/// Why a request got no reply.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SupervisorError {
    /// The engine crashed too often and stays down.
    #[error("the document process stopped after crashing repeatedly")]
    Failed,
    /// The engine exited (or is restarting) before it answered.
    #[error("the document process restarted")]
    Lost,
    /// No reply within [`Config::reply_timeout`].
    #[error("the document process did not answer in time")]
    Timeout,
    /// A new engine couldn't be started.
    #[error("the document process could not start: {0}")]
    Start(String),
}

/// A request's outcome.
pub type Reply = Result<(FromEngine, Vec<u8>), SupervisorError>;

/// The main process's handle on one engine; cheap to clone.
#[derive(Clone)]
pub struct Supervisor {
    inner: Arc<Inner>,
}

type EventSink = Arc<dyn Fn(&Supervisor, Event) + Send + Sync>;

struct Inner {
    config: Config,
    spawner: Box<dyn Spawner>,
    ranges: Arc<dyn RangeServer>,
    events: EventSink,
    next_req: AtomicU64,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    generation: u64,
    engine: Option<Engine>,
    waiting: BTreeMap<ReqId, mpsc::Sender<Reply>>,
    crashes: VecDeque<Instant>,
    failed: bool,
    stopping: bool,
}

/// The running engine.
struct Engine {
    generation: u64,
    pid: u32,
    stdin: Arc<Mutex<Box<dyn Write + Send>>>,
    process: Arc<Mutex<Box<dyn Process>>>,
}

impl Supervisor {
    /// Starts an engine and waits for its `Ready`. `ranges` answers its reads; `events` hears of
    /// restarts, failure and log lines (on the supervisor's threads: don't block in it for long).
    ///
    /// # Errors
    /// [`SupervisorError::Start`] when the engine doesn't start or answer `Hello`.
    pub fn start(
        spawner: Box<dyn Spawner>,
        ranges: Arc<dyn RangeServer>,
        config: Config,
        events: impl Fn(&Self, Event) + Send + Sync + 'static,
    ) -> Result<Self, SupervisorError> {
        let supervisor = Self {
            inner: Arc::new(Inner {
                config,
                spawner,
                ranges,
                events: Arc::new(events),
                next_req: AtomicU64::new(1),
                state: Mutex::new(State::default()),
            }),
        };
        if let Err(error) = supervisor.launch() {
            // Nothing may restart behind a caller that saw the start fail.
            supervisor.shutdown();
            return Err(error);
        }
        Ok(supervisor)
    }

    /// Sends the request `make` builds (given its id) and waits for the reply.
    ///
    /// # Errors
    /// [`SupervisorError::Failed`], `Lost` (also while a restart is under way) and `Timeout`.
    pub fn request(&self, make: impl FnOnce(ReqId) -> ToEngine) -> Reply {
        let req = ReqId(self.inner.next_req.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::channel();
        let stdin = {
            let mut state = self.state();
            if state.failed {
                return Err(SupervisorError::Failed);
            }
            let Some(engine) = &state.engine else {
                return Err(SupervisorError::Lost);
            };
            let stdin = Arc::clone(&engine.stdin);
            state.waiting.insert(req, tx);
            stdin
        };
        if write_frame(&mut *lock(&stdin), &make(req), &[]).is_err() {
            self.state().waiting.remove(&req);
            return Err(SupervisorError::Lost);
        }
        match rx.recv_timeout(self.inner.config.reply_timeout) {
            Ok(reply) => reply,
            Err(RecvTimeoutError::Timeout) => {
                self.state().waiting.remove(&req);
                self.send(&ToEngine::Cancel { req });
                Err(SupervisorError::Timeout)
            }
            Err(RecvTimeoutError::Disconnected) => Err(SupervisorError::Lost),
        }
    }

    /// Sends a message that has no reply (`CloseDocument`, `Cancel`); dropped while no engine
    /// runs.
    pub fn send(&self, msg: &ToEngine) {
        let stdin = self.state().engine.as_ref().map(|engine| Arc::clone(&engine.stdin));
        if let Some(stdin) = stdin {
            // A failed write means the engine is gone; its reader thread handles that.
            let _ = write_frame(&mut *lock(&stdin), msg, &[]);
        }
    }

    /// The running engine's process id.
    pub fn pid(&self) -> Option<u32> {
        self.state().engine.as_ref().map(|engine| engine.pid)
    }

    /// Whether the engine crashed too often and stays down.
    pub fn has_failed(&self) -> bool {
        self.state().failed
    }

    /// Kills the running engine, as a crash would: it restarts per the policy.
    pub fn kill(&self) {
        let process = self.state().engine.as_ref().map(|engine| Arc::clone(&engine.process));
        if let Some(process) = process {
            lock(&process).kill();
        }
    }

    /// Stops the engine for good: asks it to exit, then kills it.
    pub fn shutdown(&self) {
        let engine = {
            let mut state = self.state();
            state.stopping = true;
            state.engine.take()
        };
        let waiting = std::mem::take(&mut self.state().waiting);
        for (_, tx) in waiting {
            let _ = tx.send(Err(SupervisorError::Lost));
        }
        if let Some(engine) = engine {
            let _ = write_frame(&mut *lock(&engine.stdin), &ToEngine::Shutdown, &[]);
            let mut process = lock(&engine.process);
            process.kill();
            let status = process.wait();
            self.log(format!("engine {} stopped: {status}", engine.pid));
        }
    }

    /// Starts a new engine: spawns it, says `Hello`, waits for `Ready`, installs it.
    fn launch(&self) -> Result<u32, SupervisorError> {
        let spawned = self
            .inner
            .spawner
            .spawn()
            .map_err(|e| SupervisorError::Start(e.to_string()))?;
        let generation = {
            let mut state = self.state();
            state.generation += 1;
            state.generation
        };
        let stdin = Arc::new(Mutex::new(spawned.stdin));
        let process = Arc::new(Mutex::new(spawned.process));
        let beat = Arc::new(Mutex::new(Instant::now()));
        let (ready_tx, ready_rx) = mpsc::channel();
        // Set by the reader when the engine's stdout closes, so an engine that dies between its
        // `Ready` and being installed below is still seen as a crash.
        let done = Arc::new(AtomicBool::new(false));
        let hello = ToEngine::Hello {
            version: PROTOCOL_VERSION,
            limits: Box::new(self.inner.config.limits),
        };
        let started = write_frame(&mut *lock(&stdin), &hello, &[]).map_err(|e| e.to_string());
        let reader = {
            let (this, stdin, beat, done) = (self.clone(), Arc::clone(&stdin), Arc::clone(&beat), Arc::clone(&done));
            let stdout = spawned.stdout;
            thread::Builder::new().name("engine-reader".into()).spawn(move || {
                this.read_loop(stdout, &stdin, &beat, &ready_tx);
                drop(ready_tx);
                done.store(true, Ordering::SeqCst);
                this.exited(generation);
            })
        };
        let pid = started.and_then(|()| reader.map_err(|e| e.to_string())).and_then(|_| {
            match ready_rx.recv_timeout(self.inner.config.start_timeout) {
                Ok(pid) => Ok(pid),
                Err(_) => Err("no Ready".to_owned()),
            }
        });
        let pid = match pid {
            Ok(pid) => pid,
            Err(why) => {
                let mut process = lock(&process);
                process.kill();
                let status = process.wait();
                return Err(SupervisorError::Start(format!("{why} ({status})")));
            }
        };
        let stopping = {
            let mut state = self.state();
            state.engine = Some(Engine {
                generation,
                pid,
                stdin,
                process: Arc::clone(&process),
            });
            state.stopping
        };
        if stopping {
            // `shutdown` ran while this engine was starting.
            self.shutdown();
            return Err(SupervisorError::Lost);
        }
        if done.load(Ordering::SeqCst) {
            self.exited(generation);
            return Err(SupervisorError::Lost);
        }
        let this = self.clone();
        let _ = thread::Builder::new()
            .name("engine-watchdog".into())
            .spawn(move || this.watch(generation, &beat, &process));
        Ok(pid)
    }

    /// Reads the engine's frames until it exits: replies to their waiters, reads to the files.
    fn read_loop(
        &self,
        mut stdout: Box<dyn Read + Send>,
        stdin: &Mutex<Box<dyn Write + Send>>,
        beat: &Mutex<Instant>,
        ready: &mpsc::Sender<u32>,
    ) {
        loop {
            let (msg, payload) = match read_frame::<_, FromEngine>(&mut stdout, MAX_FRAME) {
                Ok(frame) => frame,
                Err(ProtocolError::Closed) => break,
                Err(error) => {
                    self.log(format!("unreadable frame from the engine: {error}"));
                    break;
                }
            };
            match msg {
                FromEngine::Ready { version, pid } if version == PROTOCOL_VERSION => {
                    let _ = ready.send(pid);
                }
                FromEngine::Ready { version, .. } => {
                    self.log(format!("the engine speaks protocol {version}"));
                    break;
                }
                FromEngine::Heartbeat => *lock(beat) = Instant::now(),
                FromEngine::ReadRange { req, doc, offset, len } => {
                    if !self.serve(stdin, req, doc, offset, len) {
                        break;
                    }
                }
                FromEngine::Error { req: None, message, .. } => self.log(format!("engine: {message}")),
                reply => match reply_req(&reply) {
                    Some(req) => self.deliver(req, Ok((reply, payload))),
                    None => self.log("a reply without a request".into()),
                },
            }
        }
    }

    /// Answers one `ReadRange`; `false` when the engine's stdin is gone.
    fn serve(&self, stdin: &Mutex<Box<dyn Write + Send>>, req: ReqId, doc: DocId, offset: u64, len: u32) -> bool {
        let written = match self.inner.ranges.read(doc, offset, len) {
            Ok(bytes) => write_frame(&mut *lock(stdin), &ToEngine::RangeData { req, offset }, &bytes),
            Err(error) => {
                let msg = ToEngine::RangeFailed {
                    req,
                    kind: ErrorKind::Io,
                    message: error.to_string(),
                };
                write_frame(&mut *lock(stdin), &msg, &[])
            }
        };
        written.is_ok()
    }

    /// Kills the engine of `generation` when its heartbeat stops.
    fn watch(&self, generation: u64, beat: &Mutex<Instant>, process: &Mutex<Box<dyn Process>>) {
        let period = (self.inner.config.heartbeat_timeout / 4).max(Duration::from_millis(10));
        loop {
            thread::sleep(period);
            if self.state().engine.as_ref().is_none_or(|e| e.generation != generation) {
                return;
            }
            if lock(beat).elapsed() > self.inner.config.heartbeat_timeout {
                self.log("the engine missed its heartbeats; killing it".into());
                lock(process).kill();
                return;
            }
        }
    }

    /// The engine of `generation` is gone: fail its requests, then restart it or give up.
    fn exited(&self, generation: u64) {
        let (engine, waiting, stopping) = {
            let mut state = self.state();
            let Some(engine) = state.engine.take_if(|e| e.generation == generation) else {
                return;
            };
            (engine, std::mem::take(&mut state.waiting), state.stopping)
        };
        for (_, tx) in waiting {
            let _ = tx.send(Err(SupervisorError::Lost));
        }
        let status = {
            let mut process = lock(&engine.process);
            process.kill();
            process.wait()
        };
        self.log(format!("engine {} exited: {status}", engine.pid));
        if !stopping {
            self.crashed();
        }
    }

    /// Counts a crash; restarts after the backoff, or marks the engine failed.
    fn crashed(&self) {
        let delay = {
            let mut state = self.state();
            let now = Instant::now();
            let window = self.inner.config.window;
            state.crashes.retain(|t| now.duration_since(*t) < window);
            state.crashes.push_back(now);
            if state.crashes.len() > self.inner.config.max_restarts {
                state.failed = true;
                None
            } else {
                let doublings = u32::try_from(state.crashes.len() - 1).unwrap_or(u32::MAX).min(16);
                Some(self.inner.config.backoff.saturating_mul(1 << doublings))
            }
        };
        let Some(delay) = delay else {
            self.emit(Event::Failed);
            return;
        };
        let this = self.clone();
        let _ = thread::Builder::new().name("engine-restart".into()).spawn(move || {
            thread::sleep(delay);
            if this.state().stopping {
                return;
            }
            match this.launch() {
                Ok(pid) => this.emit(Event::Restarted { pid }),
                // Installed, then gone: `exited` has already counted the crash.
                Err(SupervisorError::Lost) => {}
                Err(error) => {
                    this.log(error.to_string());
                    this.crashed();
                }
            }
        });
    }

    fn deliver(&self, req: ReqId, reply: Reply) {
        let tx = self.state().waiting.remove(&req);
        if let Some(tx) = tx {
            // The waiter is gone only if it timed out.
            let _ = tx.send(reply);
        }
    }

    fn emit(&self, event: Event) {
        (self.inner.events)(self, event);
    }

    fn log(&self, line: String) {
        self.emit(Event::Log(line));
    }

    fn state(&self) -> MutexGuard<'_, State> {
        lock(&self.inner.state)
    }
}

/// The request a reply answers.
const fn reply_req(msg: &FromEngine) -> Option<ReqId> {
    match msg {
        FromEngine::Opened { req, .. }
        | FromEngine::NeedsPassword { req, .. }
        | FromEngine::Unlocked { req, .. }
        | FromEngine::PageInfo { req, .. }
        | FromEngine::Tile { req, .. } => Some(*req),
        FromEngine::Error { req, .. } => *req,
        FromEngine::Ready { .. } | FromEngine::ReadRange { .. } | FromEngine::Heartbeat => None,
    }
}

fn lock<T: ?Sized>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Starts the engine binary with piped stdio, copying its stderr lines to `log`.
pub struct CommandSpawner {
    program: PathBuf,
    args: Vec<OsString>,
    log: Arc<dyn Fn(String) + Send + Sync>,
}

impl CommandSpawner {
    /// `program` with `args`; each line the engine writes to stderr goes to `log`.
    pub fn new(program: PathBuf, args: Vec<OsString>, log: impl Fn(String) + Send + Sync + 'static) -> Self {
        Self {
            program,
            args,
            log: Arc::new(log),
        }
    }
}

impl Spawner for CommandSpawner {
    fn spawn(&self) -> io::Result<Spawned> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as _;
            // CREATE_NO_WINDOW: the engine is a console program; the app shows no console.
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn()?;
        let missing = || io::Error::other("a pipe was not set up");
        let stdin = child.stdin.take().ok_or_else(missing)?;
        let stdout = child.stdout.take().ok_or_else(missing)?;
        if let Some(stderr) = child.stderr.take() {
            let log = Arc::clone(&self.log);
            let _ = thread::Builder::new().name("engine-stderr".into()).spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    log(line);
                }
            });
        }
        Ok(Spawned {
            stdin: Box::new(stdin),
            stdout: Box::new(BufReader::new(stdout)),
            process: Box::new(ChildProcess(child)),
        })
    }
}

/// A real engine process.
struct ChildProcess(Child);

impl Process for ChildProcess {
    fn id(&self) -> u32 {
        self.0.id()
    }

    fn kill(&mut self) {
        // Fails only when it has already exited.
        let _ = self.0.kill();
    }

    fn wait(&mut self) -> String {
        match self.0.wait() {
            Ok(status) => status.to_string(),
            Err(error) => format!("unknown ({error})"),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod fake;
#[cfg(test)]
mod tests;
