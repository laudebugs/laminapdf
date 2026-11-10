//! The supervisor against the in-process fake engine (`fake`).

use super::fake::{FakeSpawner, Mode};
use super::*;
use crate::{Password, TileFormat};

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

/// A supervisor of fakes in `mode`, its spawner (which counts engines), and its events.
fn start(mode: Mode) -> (Supervisor, FakeSpawner, mpsc::Receiver<Event>) {
    let spawner = FakeSpawner::new(mode);
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let supervisor = Supervisor::start(
        Box::new(spawner.clone()),
        Arc::new(NoFiles),
        fast_config(),
        move |_, event| {
            if !matches!(event, Event::Log(_)) {
                let _ = lock(&tx).send(event);
            }
        },
    );
    match supervisor {
        Ok(supervisor) => (supervisor, spawner, rx),
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
        len: 11,
    })
}

#[test]
fn a_request_should_get_its_own_reply() {
    let (supervisor, _, _) = start(Mode::Healthy);
    assert!(matches!(
        open(&supervisor),
        Ok((FromEngine::Opened { page_count: 3, .. }, _))
    ));
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
    assert!(matches!(next_event(&events), Some(Event::Restarted { .. })) && spawns.spawns() == 2);
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
        (restarts, seen.last(), spawns.spawns(), supervisor.has_failed()),
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
fn an_engine_that_dies_as_it_starts_should_never_stay_installed() {
    // It dies right after `Ready`: either the start fails, or the death counts as a crash and
    // every restart dies too, until the engine is marked failed. Never a dead engine in place.
    let spawner = FakeSpawner::new(Mode::DieAtOnce);
    let started = Supervisor::start(Box::new(spawner), Arc::new(NoFiles), fast_config(), |_, _| {});
    let settled = match started {
        Err(_) => true,
        Ok(supervisor) => {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !supervisor.has_failed() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            supervisor.has_failed()
        }
    };
    assert!(settled);
}

#[test]
fn requests_from_other_threads_should_wait_out_the_reopening() {
    let spawner = FakeSpawner::new(Mode::Healthy);
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let supervisor = Supervisor::start(
        Box::new(spawner),
        Arc::new(NoFiles),
        fast_config(),
        move |engine, event| {
            if matches!(event, Event::Restarted { .. }) {
                // The handler's own request goes through; another thread's, made now, does not.
                let own = open(engine).is_ok();
                let other = {
                    let engine = engine.clone();
                    thread::spawn(move || open(&engine)).join()
                };
                let _ = lock(&tx).send((own, matches!(other, Ok(Err(SupervisorError::Lost)))));
            }
        },
    )
    .unwrap();
    supervisor.kill();
    assert_eq!(rx.recv_timeout(Duration::from_secs(5)).ok(), Some((true, true)));
}

#[test]
fn a_panicking_restart_handler_should_not_lock_out_requests() {
    let spawner = FakeSpawner::new(Mode::Healthy);
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let supervisor = Supervisor::start(Box::new(spawner), Arc::new(NoFiles), fast_config(), move |_, event| {
        if matches!(event, Event::Restarted { .. }) {
            let _ = lock(&tx).send(());
            panic!("the owner's handler failed");
        }
    })
    .unwrap();
    supervisor.kill();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // The handler's thread unwinds just after it signalled; give a slow runner time.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut reply = open(&supervisor);
    while reply.is_err() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
        reply = open(&supervisor);
    }
    assert!(reply.is_ok());
}

#[test]
fn shutdown_should_not_restart_the_engine() {
    let (supervisor, spawns, events) = start(Mode::Healthy);
    supervisor.shutdown();
    assert!(
        events.recv_timeout(Duration::from_millis(300)).is_err() && spawns.spawns() == 1 && supervisor.pid().is_none()
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
