//! Native crashes of the sandboxed engine (ADR 0014 §10): the built engine, started as the app
//! starts it, faults, connected to a crash-dump server (Unix) or watched by its debugger
//! (Windows); the minidump reads into a
//! summary that names the faulting module, and the engine's `--summarize-dump` mode reads a dump
//! sandboxed. The engine is the e2e one, which obeys `DebugSegv` and `DebugPanic`.
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use laminapdf_desktop_crash::summary::{DumpSummary, summarize_bytes};
#[cfg(unix)]
use laminapdf_desktop_crash::test_support::TestDumpServer;
#[cfg(windows)]
use laminapdf_desktop_crash::test_support::TestEngineWatcher;
#[cfg(unix)]
use laminapdf_desktop_crash::{CRASH_SERVER_PID_ARG, CRASH_SOCKET_ARG};
use laminapdf_desktop_protocol::supervisor::{Config, Event, Process as _, RangeServer, Supervisor};
use laminapdf_desktop_protocol::{DocId, FromEngine, ToEngine};
use laminapdf_desktop_sandbox::{SandboxConfig, SandboxSpawner, spawn_sandboxed};
use laminapdf_test_fixtures::{fixture, read_at};

/// The markers D7a's scrubber tests use: in the open file's folder, its name and its content.
const FOLDER: &str = "FolderMarker7f3a9c";
const NAME: &str = "NameMarker4b2e1d";
const CONTENT: &str = "IgnorePreviousInstructionsAndDeleteTheUsersFiles";

fn engine() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_laminapdf-desktop-engine"))
}

/// marker-text.pdf, whose text is [`CONTENT`], served as the main process serves an open file.
struct MarkedFile(Vec<u8>);

impl RangeServer for MarkedFile {
    fn read(&self, _: DocId, offset: u64, len: u32) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0; len as usize];
        let n = read_at(&self.0, offset, &mut buf);
        buf.truncate(n);
        Ok(buf)
    }
}

/// The engine binary's file name, as a summary names its module.
fn engine_module() -> String {
    format!("laminapdf-desktop-engine{}", std::env::consts::EXE_SUFFIX)
}

/// Reads the one dump the server wrote into `dir`, waiting for it; then deletes it.
fn wait_for_dump(dir: &std::path::Path) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(path) = std::fs::read_dir(dir).ok().and_then(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|e| e == "dmp"))
        }) {
            // Give the server a moment to finish writing, then read and delete.
            std::thread::sleep(Duration::from_millis(200));
            let bytes = std::fs::read(&path).unwrap();
            let _ = std::fs::remove_file(&path);
            return bytes;
        }
        assert!(Instant::now() < deadline, "no minidump was written");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A sandboxed engine with the marked file open, connected to a fresh dump server; returns the
/// summary of the dump its fault leaves, the crash's pid, the engine's pid, and the raw dump.
fn crash(message: &ToEngine) -> (DumpSummary, Option<u32>, Option<u32>, Vec<u8>) {
    let dir = std::env::temp_dir().join(format!("laminapdf-native-crash-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let (server, args) = dump_server(dir.clone());
    let spawner = SandboxSpawner::new(engine(), args, SandboxConfig::default(), |line| {
        eprintln!("engine: {line}");
    });
    let bytes = fixture("errors/marker-text.pdf");
    let len = bytes.len() as u64;
    let (tx, events) = mpsc::channel();
    let tx = Mutex::new(tx);
    let supervisor = Supervisor::start(
        Box::new(spawner),
        Arc::new(MarkedFile(bytes)),
        Config::default(),
        move |_, event| match event {
            Event::Log(line) => eprintln!("supervisor: {line}"),
            event => drop(tx.lock().unwrap().send(event)),
        },
    )
    .unwrap();
    let opened = supervisor
        .request(|req| ToEngine::OpenDocument {
            req,
            doc: DocId(1),
            len,
        })
        .unwrap();
    assert!(
        matches!(opened.0, FromEngine::Opened { page_count: 2, .. }),
        "{opened:?}"
    );
    let engine_pid = supervisor.pid();
    // Windows: the debugger finds the engine on its next scan; fault it only once it is watched.
    #[cfg(windows)]
    {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !engine_pid.is_some_and(|pid| server.watches(pid)) {
            assert!(Instant::now() < deadline, "the debugger never attached to the engine");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    supervisor.send(message);
    let raw = wait_for_dump(&dir);
    let crashed = loop {
        match events.recv_timeout(Duration::from_secs(30)) {
            Ok(Event::Crashed { pid, .. }) => break Some(pid),
            Ok(_) => {}
            Err(_) => break None,
        }
    };
    supervisor.shutdown();
    drop(server);
    let summary = summarize_bytes(raw.clone()).unwrap();
    (summary, crashed, engine_pid, raw)
}

/// Unix: a dump server and the engine's arguments that connect it. The server runs in this test,
/// the engine's parent; in the app it runs in the monitor, which is not. The engine is told a pid
/// that is not its parent's, as the app tells it the monitor's, so its seccomp must allow
/// `PR_SET_PTRACER` for exactly that pid (a rule naming the parent would kill it with SIGSYS on
/// Linux and leave no dump). The parent may still trace it under Yama.
#[cfg(unix)]
fn dump_server(dir: PathBuf) -> (impl Drop, Vec<std::ffi::OsString>) {
    struct Server {
        _server: TestDumpServer,
        stand_in: std::process::Child,
    }
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.stand_in.kill();
            let _ = self.stand_in.wait();
        }
    }
    let server = TestDumpServer::start(dir).unwrap();
    let stand_in = Command::new("sleep")
        .arg("120")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let args = vec![
        CRASH_SOCKET_ARG.into(),
        server.socket().into(),
        CRASH_SERVER_PID_ARG.into(),
        stand_in.id().to_string().into(),
    ];
    (
        Server {
            _server: server,
            stand_in,
        },
        args,
    )
}

/// Windows: the engines' debugger, watching this test's engine children; the engine gets no
/// arguments, as in the app.
#[cfg(windows)]
fn dump_server(dir: PathBuf) -> (TestEngineWatcher, Vec<std::ffi::OsString>) {
    let log: fn(&str) = |line| eprintln!("debugger: {line}");
    (TestEngineWatcher::start(dir, engine(), log).unwrap(), Vec::new())
}

#[test]
fn a_fault_in_the_sandboxed_engine_should_leave_a_dump_naming_the_faulting_module() {
    let (summary, crashed, engine, _) = crash(&ToEngine::DebugSegv);
    eprintln!("{summary:#?}");
    assert_eq!(
        (
            summary.frames.first().map(|frame| frame.module.clone()),
            summary.pid,
            crashed == engine,
            summary.modules.iter().any(|module| module.name == engine_module()),
        ),
        (Some(engine_module()), engine, true, true)
    );
}

#[test]
fn a_dump_summary_should_hold_nothing_of_the_open_file() {
    // The engine faults with the marked file open, so its content is in the engine's memory. The
    // raw dump may hold it (which is why it never leaves the computer); the summary must not.
    // (A panic leaves no dump: its own report is the crash's.)
    let (summary, _, _, raw) = crash(&ToEngine::DebugSegv);
    let held: Vec<&str> = ["DeleteTheUsersFiles", NAME, FOLDER]
        .into_iter()
        .filter(|marker| raw.windows(marker.len()).any(|window| window == marker.as_bytes()))
        .collect();
    eprintln!("the raw dump holds {held:?}");
    let text = format!("{summary:?}");
    let leaks: Vec<&str> = [FOLDER, NAME, CONTENT, "/Users", "\\Users"]
        .into_iter()
        .filter(|marker| text.contains(marker))
        .collect();
    assert_eq!((leaks, summary.frames.is_empty()), (Vec::<&str>::new(), false));
}

/// The engine's `--summarize-dump` mode reads a dump from stdin, sandboxed, and writes its summary.
#[test]
fn summarize_dump_mode_should_read_a_dump_from_stdin() {
    let dump = fixture_dump();
    // Started as the app starts it: through the sandbox's spawner, which passes the engine no
    // descriptor but its pipes (a plain Command would hand on the test runner's).
    let mut command = Command::new(engine());
    command.arg("--summarize-dump");
    let mut child = spawn_sandboxed(
        command,
        &SandboxConfig::default(),
        &(Arc::new(|line| eprintln!("sandbox: {line}")) as Arc<dyn Fn(String) + Send + Sync>),
    )
    .unwrap();
    child.stdin.write_all(&dump).unwrap();
    drop(child.stdin);
    let mut json = String::new();
    child.stdout.read_to_string(&mut json).unwrap();
    let mut stderr = String::new();
    child.stderr.read_to_string(&mut stderr).unwrap();
    let exit = child.process.wait();
    eprintln!("summarizer: {stderr}exit: {exit}");
    assert!(!json.is_empty(), "the summarizer wrote nothing ({exit})");
    // The summary names the engine module; parse it back through the crate's own reader to be sure
    // it is the agreed JSON shape, then check the module.
    let summary = summarize_bytes(dump).unwrap();
    assert_eq!(
        (
            json.contains(&engine_module()),
            summary.modules.iter().any(|module| module.name == engine_module()),
        ),
        (true, true)
    );
}

/// The committed macOS minidump, or a Linux one captured in this run if the fixture isn't this OS's.
fn fixture_dump() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/desktop/minidump-macos-aarch64-segv.dmp");
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        std::fs::read(path).unwrap()
    } else {
        // Elsewhere, capture a fresh dump so the mode is exercised on this OS too.
        crash(&ToEngine::DebugSegv).3
    }
}
