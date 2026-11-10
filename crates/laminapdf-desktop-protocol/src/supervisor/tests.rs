//! The supervisor against an in-process fake engine: two pipes and a thread that speaks the
//! protocol, and that can crash, hang or stay silent on cue.

use std::io::{PipeReader, PipeWriter};
use std::sync::atomic::AtomicU32;

use super::*;
use crate::{Password, TileFormat};

/// How a fake engine behaves.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Answers requests and beats.
    Healthy,
    /// Exits shortly after `Ready` (an engine that dies on its first document).
    CrashAfterReady,
    /// Answers `Hello`, then never beats or answers again.
    Silent,
    /// Exits as it says `Ready`.
    DieAtOnce,
}

struct FakeSpawner {
    mode: Mode,
    spawns: Arc<AtomicU32>,
}

/// The fake's stdout, which `kill` closes: the supervisor then reads the end of the stream.
type SharedOut = Arc<Mutex<Option<PipeWriter>>>;

struct FakeProcess {
    pid: u32,
    out: SharedOut,
}

impl Process for FakeProcess {
    fn id(&self) -> u32 {
        self.pid
    }

    fn kill(&mut self) {
        lock(&self.out).take();
    }

    fn wait(&mut self) -> String {
        "killed".into()
    }
}

impl Spawner for FakeSpawner {
    fn spawn(&self) -> io::Result<Spawned> {
        let pid = 1000 + self.spawns.fetch_add(1, Ordering::SeqCst);
        let (stdin_r, stdin_w) = io::pipe()?;
        let (stdout_r, stdout_w) = io::pipe()?;
        let out: SharedOut = Arc::new(Mutex::new(Some(stdout_w)));
        let mode = self.mode;
        let engine_out = Arc::clone(&out);
        thread::spawn(move || fake_engine(mode, pid, stdin_r, &engine_out));
        Ok(Spawned {
            stdin: Box::new(stdin_w),
            stdout: Box::new(stdout_r),
            process: Box::new(FakeProcess { pid, out }),
        })
    }
}

fn send(out: &SharedOut, msg: &FromEngine) -> bool {
    lock(out)
        .as_mut()
        .is_some_and(|w| write_frame(w, msg, &[]).is_ok())
}

/// Speaks the protocol: `Ready`, then one answer per request (a document of 3 pages).
fn fake_engine(mode: Mode, pid: u32, mut stdin: PipeReader, out: &SharedOut) {
    if !matches!(read_frame::<_, ToEngine>(&mut stdin, MAX_FRAME), Ok((ToEngine::Hello { .. }, _))) {
        return;
    }
    send(
        out,
        &FromEngine::Ready {
            version: PROTOCOL_VERSION,
            pid,
        },
    );
    match mode {
        Mode::CrashAfterReady => {
            thread::sleep(Duration::from_millis(30));
            lock(out).take();
            return;
        }
        Mode::DieAtOnce => {
            lock(out).take();
            return;
        }
        Mode::Silent => {
            // Keep the pipes open without a word until the supervisor kills it.
            while read_frame::<_, ToEngine>(&mut stdin, MAX_FRAME).is_ok() {}
            return;
        }
        Mode::Healthy => {}
    }
    let beats = Arc::clone(out);
    thread::spawn(move || while send(&beats, &FromEngine::Heartbeat) {
        thread::sleep(Duration::from_millis(20));
    });
    while let Ok((msg, _)) = read_frame::<_, ToEngine>(&mut stdin, MAX_FRAME) {
        let reply = match msg {
            ToEngine::OpenDocument { req, doc, .. } => FromEngine::Opened {
                req,
                doc,
                page_count: 3,
                encrypted: false,
                needs_password: false,
                repaired: false,
            },
            ToEngine::UnlockDocument { req, doc, password } if password.expose() == "secret" => FromEngine::Unlocked {
                req,
                doc,
                page_count: 3,
                repaired: false,
            },
            ToEngine::UnlockDocument { req, .. } => FromEngine::Error {
                req: Some(req),
                kind: ErrorKind::WrongPassword,
                message: "wrong-password".into(),
            },
            ToEngine::RenderTile(tile) => FromEngine::Tile {
                req: tile.req,
                w: tile.w,
                h: tile.h,
                format: TileFormat::Rgba8,
            },
            ToEngine::Shutdown => return,
            _ => continue,
        };
        if !send(out, &reply) {
            return;
        }
    }
}

struct NoFiles;

impl RangeServer for NoFiles {
    fn read(&self, _: DocId, _: u64, _: u32) -> io::Result<Vec<u8>> {
        Err(io::Error::other("no files"))
    }
}

fn fast_config() -> Config {
    Config {
        heartbeat_timeout: Duration::from_millis(300),
        reply_timeout: Duration::from_secs(5),
        start_timeout: Duration::from_secs(5),
        backoff: Duration::from_millis(10),
        ..Config::default()
    }
}

/// A supervisor of fakes in `mode`, the number of engines spawned, and its events.
fn start(mode: Mode) -> (Supervisor, Arc<AtomicU32>, mpsc::Receiver<Event>) {
    let spawns = Arc::new(AtomicU32::new(0));
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let spawner = FakeSpawner {
        mode,
        spawns: Arc::clone(&spawns),
    };
    let supervisor = Supervisor::start(Box::new(spawner), Arc::new(NoFiles), fast_config(), move |_, event| {
        if !matches!(event, Event::Log(_)) {
            let _ = lock(&tx).send(event);
        }
    });
    match supervisor {
        Ok(supervisor) => (supervisor, spawns, rx),
        Err(error) => panic!("the fake engine did not start: {error}"),
    }
}

fn next_event(rx: &mpsc::Receiver<Event>) -> Option<Event> {
    rx.recv_timeout(Duration::from_secs(5)).ok()
}

fn open(supervisor: &Supervisor) -> Reply {
    supervisor.request(|req| ToEngine::OpenDocument {
        req,
        doc: DocId(1),
        len: 10,
    })
}

#[test]
fn a_request_should_get_its_own_reply() {
    let (supervisor, _, _) = start(Mode::Healthy);
    assert!(matches!(open(&supervisor), Ok((FromEngine::Opened { page_count: 3, .. }, _))));
}

#[test]
fn concurrent_requests_should_each_get_their_reply() {
    let (supervisor, _, _) = start(Mode::Healthy);
    let threads: Vec<_> = (1..=8_u32)
        .map(|w| {
            let supervisor = supervisor.clone();
            thread::spawn(move || {
                supervisor.request(|req| {
                    ToEngine::RenderTile(crate::TileRequest {
                        req,
                        doc: DocId(1),
                        page: 0,
                        scale_milli: 1000,
                        x: 0,
                        y: 0,
                        w,
                        h: 1,
                        format: TileFormat::Rgba8,
                        priority: crate::Priority::Visible,
                    })
                })
            })
        })
        .collect();
    let widths: Vec<_> = threads
        .into_iter()
        .map(|t| match t.join() {
            Ok(Ok((FromEngine::Tile { w, .. }, _))) => w,
            _ => 0,
        })
        .collect();
    assert_eq!(widths, (1..=8).collect::<Vec<_>>());
}

#[test]
fn a_killed_engine_should_be_restarted_with_a_new_pid() {
    let (supervisor, _, events) = start(Mode::Healthy);
    let before = supervisor.pid();
    supervisor.kill();
    let restarted = next_event(&events);
    assert!(matches!(restarted, Some(Event::Restarted { pid }) if Some(pid) != before));
}

#[test]
fn a_request_after_a_restart_should_reach_the_new_engine() {
    let (supervisor, _, events) = start(Mode::Healthy);
    supervisor.kill();
    let _ = next_event(&events);
    assert!(open(&supervisor).is_ok());
}

#[test]
fn a_crash_should_fail_the_requests_in_flight() {
    let (supervisor, _, _) = start(Mode::Silent);
    let waiting = {
        let supervisor = supervisor.clone();
        thread::spawn(move || open(&supervisor))
    };
    thread::sleep(Duration::from_millis(50));
    supervisor.kill();
    assert!(matches!(waiting.join(), Ok(Err(SupervisorError::Lost))));
}

#[test]
fn an_engine_that_misses_its_heartbeats_should_be_restarted() {
    let (_supervisor, spawns, events) = start(Mode::Silent);
    assert!(matches!(next_event(&events), Some(Event::Restarted { .. })) && spawns.load(Ordering::SeqCst) == 2);
}

#[test]
fn a_fourth_crash_within_the_window_should_mark_the_engine_failed() {
    let (supervisor, spawns, events) = start(Mode::CrashAfterReady);
    let mut seen = Vec::new();
    while let Some(event) = next_event(&events) {
        let failed = event == Event::Failed;
        seen.push(event);
        if failed {
            break;
        }
    }
    let restarts = seen.iter().filter(|e| matches!(e, Event::Restarted { .. })).count();
    assert_eq!(
        (restarts, seen.last(), spawns.load(Ordering::SeqCst), supervisor.has_failed()),
        (3, Some(&Event::Failed), 4, true)
    );
}

#[test]
fn a_failed_engine_should_refuse_requests() {
    let (supervisor, _, events) = start(Mode::CrashAfterReady);
    while next_event(&events).is_some_and(|e| e != Event::Failed) {}
    assert!(matches!(open(&supervisor), Err(SupervisorError::Failed)));
}

#[test]
fn backoff_should_double_with_each_recent_crash() {
    let (_supervisor, _, events) = start(Mode::CrashAfterReady);
    let started = Instant::now();
    while next_event(&events).is_some_and(|e| e != Event::Failed) {}
    // 10 + 20 + 40 ms of backoff, plus 30 ms of life for each of the four engines.
    assert!(started.elapsed() >= Duration::from_millis(190));
}

#[test]
fn an_engine_that_dies_before_it_is_installed_should_fail_the_start() {
    let spawner = FakeSpawner {
        mode: Mode::DieAtOnce,
        spawns: Arc::new(AtomicU32::new(0)),
    };
    let result = Supervisor::start(Box::new(spawner), Arc::new(NoFiles), fast_config(), |_, _| {});
    assert!(result.is_err());
}

#[test]
fn shutdown_should_not_restart_the_engine() {
    let (supervisor, spawns, events) = start(Mode::Healthy);
    supervisor.shutdown();
    assert!(
        events.recv_timeout(Duration::from_millis(300)).is_err()
            && spawns.load(Ordering::SeqCst) == 1
            && supervisor.pid().is_none()
    );
}

#[test]
fn an_unlock_should_carry_its_password() {
    let (supervisor, _, _) = start(Mode::Healthy);
    let reply = supervisor.request(|req| ToEngine::UnlockDocument {
        req,
        doc: DocId(1),
        password: Password::new("secret".into()),
    });
    assert!(matches!(reply, Ok((FromEngine::Unlocked { .. }, _))));
}
